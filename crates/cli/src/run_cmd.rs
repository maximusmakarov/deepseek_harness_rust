use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use clap::Args;
use dshr_agent::{
    Agent, AgentConfig, AgentObserver, ApprovalPolicy, AutoApprover, DenyApprover, PermissionMode,
    ToolCallInfo,
};
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider};
use dshr_llm_deepseek::DeepSeekProvider;
use dshr_llm_mock::{RecordingProvider, ReplayProvider};
use dshr_llm_openai::OpenAiProvider;
use dshr_prompt::PromptContext;
use dshr_session_jsonl::SessionStore;
use dshr_skills::SkillRegistry;
use dshr_tools::{ToolOutcome, ToolRegistry};
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

    /// Maximum output tokens per model request
    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,

    /// Maximum model requests in one turn
    #[arg(long, default_value_t = 8)]
    pub max_steps: usize,

    /// Allow shell commands and writes outside the workspace
    #[arg(long)]
    pub danger: bool,

    /// Auto-approve every request that the policy escalates
    #[arg(long)]
    pub yes: bool,

    /// Disable built-in tools
    #[arg(long)]
    pub no_tools: bool,
}

pub async fn execute(loader: &ConfigLoader, args: &RunArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let provider = build_provider(args, &config)?;
    let recorder = args
        .record
        .as_ref()
        .map(|_| Arc::new(RecordingProvider::new(provider.clone())));
    let provider: Arc<dyn LlmProvider> = match &recorder {
        Some(recorder) => Arc::clone(recorder) as Arc<dyn LlmProvider>,
        None => provider,
    };

    let session_dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let store = Arc::new(SessionStore::new(session_dir));
    let cwd = std::env::current_dir()
        .map_err(DshrError::Io)?
        .display()
        .to_string();

    let session = match &args.session {
        Some(id) => store.open(id)?,
        None => store.create(provider.name(), provider.model(), &cwd)?,
    };

    let mut tools = if args.no_tools {
        ToolRegistry::new()
    } else {
        ToolRegistry::with_builtin()
    };
    let skills = Arc::new(SkillRegistry::discover(&[
        loader.home().join("skills"),
        std::path::PathBuf::from(&cwd).join(".dshr/skills"),
    ]));
    if !skills.is_empty() {
        tools.register_arc(Arc::new(skills.tool()));
    }

    let mode = if args.danger {
        PermissionMode::DangerFull
    } else {
        PermissionMode::WorkspaceWrite
    };
    let approver: Arc<dyn dshr_agent::Approver> = if args.yes || args.danger {
        Arc::new(AutoApprover)
    } else {
        Arc::new(DenyApprover)
    };
    let policy = ApprovalPolicy {
        mode,
        workspace: std::path::PathBuf::from(&cwd),
        approver,
    };

    let system_prompt = build_system_prompt(
        loader.home(),
        &cwd,
        provider.name(),
        provider.model(),
        &tools,
        &skills,
    );
    let mut agent = Agent::new(
        provider,
        Arc::new(tools),
        store,
        session,
        policy,
        AgentConfig {
            system_prompt,
            max_steps: args.max_steps.max(1),
            max_tokens: args.max_tokens,
            output_limit: config.tools.output_limit_chars,
            compaction: Some(dshr_compaction::CompactionConfig::default()),
        },
    );

    let cancel = CancellationToken::new();
    let signal = tokio::spawn({
        let cancel = cancel.clone();
        async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.cancel();
            }
        }
    });

    let observer = HeadlessObserver { json: args.json };
    let outcome = agent.run_turn(&args.prompt, cancel, &observer).await;
    signal.abort();

    match outcome {
        Ok(report) => {
            if let (Some(recorder), Some(path)) = (&recorder, &args.record) {
                recorder
                    .cassette(agent.session().header.model.clone())
                    .write_file(path)
                    .map_err(llm_error)?;
            }
            let text = report.text;
            if args.json {
                let payload = serde_json::json!({
                    "session_id": agent.session().header.id.as_str(),
                    "provider": agent.session().header.provider,
                    "model": agent.session().header.model,
                    "text": text,
                    "usage": report.usage,
                    "steps": report.steps,
                    "tool_calls": report.tool_calls,
                });
                let encoded = serde_json::to_string_pretty(&payload)
                    .map_err(|error| DshrError::config(format!("serialize result: {error}")))?;
                println!("{encoded}");
            } else {
                println!();
                eprintln!(
                    "session {} · {} in / {} out · {} step(s), {} tool call(s)",
                    agent.session().header.id,
                    report.usage.input_tokens,
                    report.usage.output_tokens,
                    report.steps,
                    report.tool_calls
                );
            }
            Ok(0)
        }
        Err(dshr_agent::AgentError::Cancelled) => {
            eprintln!("cancelled; session {}", agent.session().header.id);
            Ok(130)
        }
        Err(error) => Err(DshrError::config(error.to_string())),
    }
}

struct HeadlessObserver {
    json: bool,
}

#[async_trait]
impl AgentObserver for HeadlessObserver {
    async fn on_text(&self, text: &str) {
        if !self.json {
            print!("{text}");
            let _ = std::io::stdout().flush();
        }
    }

    async fn on_tool_start(&self, call: &ToolCallInfo) {
        if !self.json {
            println!();
            eprintln!("→ {}({})", call.name, summarize(&call.input));
        }
    }

    async fn on_tool_end(&self, call: &ToolCallInfo, outcome: &ToolOutcome) {
        if !self.json {
            eprintln!(
                "← {} {}",
                call.name,
                if outcome.is_error { "error" } else { "ok" }
            );
        }
    }
}

fn summarize(value: &serde_json::Value) -> String {
    let text = value.to_string();
    let mut end = text.len().min(80);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &text[..end], if text.len() > 80 { "…" } else { "" })
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

pub(crate) fn build_system_prompt(
    home: &std::path::Path,
    cwd: &str,
    provider: &str,
    model: &str,
    tools: &ToolRegistry,
    skills: &SkillRegistry,
) -> String {
    let today = dshr_session::now_rfc3339();
    dshr_prompt::build(&PromptContext {
        cwd: std::path::PathBuf::from(cwd),
        home: home.to_path_buf(),
        provider: provider.to_owned(),
        model: model.to_owned(),
        date: today.get(..10).unwrap_or(&today).to_owned(),
        tools: tools.names(),
        skills: skills.catalog(),
    })
}

fn llm_error(error: LlmError) -> DshrError {
    DshrError::config(format!("llm: {error}"))
}
