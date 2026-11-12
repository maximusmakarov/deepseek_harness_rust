use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result, TurnId};
use dshr_llm::{BlockAssembler, ContentBlock, LlmError, LlmProvider, Request, StreamChunk};
use dshr_llm_deepseek::DeepSeekProvider;
use dshr_llm_mock::{Cassette, ReplayProvider};
use dshr_llm_openai::OpenAiProvider;
use dshr_session::{SessionEvent, TurnEndReason, now_rfc3339};
use dshr_session_jsonl::SessionStore;
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Prompt for this turn
    #[arg(value_name = "PROMPT")]
    pub prompt: String,

    /// Resume an existing session instead of starting a new one
    #[arg(long, value_name = "ID")]
    pub session: Option<String>,

    /// Override the provider route from config
    #[arg(long, value_name = "NAME")]
    pub provider: Option<String>,

    /// Override the model id
    #[arg(long, value_name = "NAME")]
    pub model: Option<String>,

    /// Replay a recorded cassette instead of calling a provider
    #[arg(long, value_name = "FILE")]
    pub cassette: Option<PathBuf>,

    /// Record the normalized chunk stream to a cassette
    #[arg(long, value_name = "FILE")]
    pub record: Option<PathBuf>,

    /// Emit one JSON object instead of streaming text
    #[arg(long)]
    pub json: bool,

    /// Maximum output tokens
    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,
}

pub async fn execute(loader: &ConfigLoader, args: &RunArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let provider = build_provider(args, &config)?;

    let session_dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let store = SessionStore::new(session_dir);
    let cwd = std::env::current_dir()
        .map_err(DshrError::Io)?
        .display()
        .to_string();

    let mut session = match &args.session {
        Some(id) => store.open(id)?,
        None => store.create(provider.name(), provider.model(), &cwd)?,
    };

    let turn = TurnId::new();
    let seq = session.next_seq();
    store.append(
        &mut session,
        SessionEvent::TurnStart {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
        },
    )?;
    let seq = session.next_seq();
    store.append(
        &mut session,
        SessionEvent::UserMessage {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
            content: vec![ContentBlock::text(args.prompt.clone())],
        },
    )?;

    let request = Request {
        model: provider.model().to_owned(),
        system: Some(system_prompt(&cwd)),
        messages: session.derive_messages(),
        max_tokens: args.max_tokens,
        ..Request::default()
    };

    let cancel = CancellationToken::new();
    let signal = tokio::spawn({
        let cancel = cancel.clone();
        async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
            }
        }
    });

    let mut stream = provider
        .stream(request, cancel.clone())
        .await
        .map_err(llm_error)?;

    let mut assembler = BlockAssembler::new();
    let mut recorded = Vec::new();
    let mut cancelled = false;
    let mut failure = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                if !args.json
                    && let StreamChunk::TextDelta { text } = &chunk
                {
                    print!("{text}");
                    let _ = std::io::stdout().flush();
                }
                recorded.push(chunk.clone());
                assembler.push(chunk);
            }
            Err(LlmError::Cancelled) => {
                cancelled = true;
                break;
            }
            Err(err) => {
                failure = Some(err);
                break;
            }
        }
    }
    signal.abort();

    if let Some(err) = failure {
        append_attempt(&store, &mut session, &turn, &err.to_string())?;
        append_turn_end(&store, &mut session, &turn, TurnEndReason::Error)?;
        return Err(llm_error(err));
    }

    if cancelled {
        append_attempt(&store, &mut session, &turn, "cancelled")?;
        append_turn_end(&store, &mut session, &turn, TurnEndReason::Cancelled)?;
        eprintln!("cancelled; session {}", session.header.id);
        return Ok(130);
    }

    let output = assembler.finish().map_err(llm_error)?;
    if output.content.is_empty() {
        append_attempt(&store, &mut session, &turn, "empty response")?;
        append_turn_end(&store, &mut session, &turn, TurnEndReason::Error)?;
        return Err(DshrError::config("provider returned an empty response"));
    }

    let seq = session.next_seq();
    store.append(
        &mut session,
        SessionEvent::AssistantMessage {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
            content: output.content.clone(),
            usage: output.usage,
            stop_reason: output.stop_reason.clone(),
        },
    )?;
    append_turn_end(&store, &mut session, &turn, TurnEndReason::Completed)?;

    if let Some(path) = &args.record {
        Cassette::new(provider.model().to_owned(), recorded)
            .write_file(path)
            .map_err(llm_error)?;
    }

    let text = text_of(&output.content);
    if args.json {
        let payload = serde_json::json!({
            "session_id": session.header.id.as_str(),
            "provider": provider.name(),
            "model": provider.model(),
            "text": text,
            "usage": output.usage,
            "stop_reason": output.stop_reason,
        });
        let encoded = serde_json::to_string_pretty(&payload)
            .map_err(|err| DshrError::config(format!("serialize result: {err}")))?;
        println!("{encoded}");
    } else {
        println!();
        eprintln!(
            "session {} · {} in / {} out",
            session.header.id, output.usage.input_tokens, output.usage.output_tokens
        );
    }

    Ok(0)
}

fn build_provider(args: &RunArgs, config: &Config) -> Result<Arc<dyn LlmProvider>> {
    if let Some(path) = &args.cassette {
        return Ok(Arc::new(
            ReplayProvider::from_file(path).map_err(llm_error)?,
        ));
    }

    let route = args
        .provider
        .clone()
        .unwrap_or_else(|| config.model.provider.clone());
    let model = args
        .model
        .clone()
        .or_else(|| Some(config.model.name.clone()));

    match route.as_str() {
        "deepseek" | "deepseek-account" | "deepseek-official" => Ok(Arc::new(
            DeepSeekProvider::from_env(model, Some(args.max_tokens)).map_err(llm_error)?,
        )),
        "openai" | "openai-compatible" | "routerai" => Ok(Arc::new(
            OpenAiProvider::from_env(model, Some(args.max_tokens)).map_err(llm_error)?,
        )),
        other => Err(DshrError::config(format!(
            "unknown provider route '{other}'; expected deepseek, openai-compatible, or replay"
        ))),
    }
}

fn append_attempt(
    store: &SessionStore,
    session: &mut dshr_session::Session,
    turn: &TurnId,
    error: &str,
) -> Result<()> {
    let seq = session.next_seq();
    store.append(
        session,
        SessionEvent::AssistantAttempt {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
            error: error.to_owned(),
        },
    )
}

fn append_turn_end(
    store: &SessionStore,
    session: &mut dshr_session::Session,
    turn: &TurnId,
    reason: TurnEndReason,
) -> Result<()> {
    let seq = session.next_seq();
    store.append(
        session,
        SessionEvent::TurnEnd {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
            reason,
        },
    )
}

fn system_prompt(cwd: &str) -> String {
    format!(
        "You are dshr, a terminal coding agent. Working directory: {cwd}. Be concise and precise."
    )
}

fn text_of(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn llm_error(error: LlmError) -> DshrError {
    DshrError::config(format!("llm: {error}"))
}
