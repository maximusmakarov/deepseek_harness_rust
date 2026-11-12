use std::sync::Arc;

use clap::Args;
use dshr_agent::PermissionMode;
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider};
use dshr_llm_deepseek::DeepSeekProvider;
use dshr_llm_mock::ReplayProvider;
use dshr_llm_openai::OpenAiProvider;
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;

#[derive(Debug, Args)]
pub struct TuiArgs {
    /// Resume an existing session
    #[arg(long, value_name = "ID")]
    pub session: Option<String>,

    /// Replay a recorded cassette instead of calling a provider
    #[arg(long, value_name = "FILE")]
    pub cassette: Option<std::path::PathBuf>,

    /// Override the provider route from config
    #[arg(long, value_name = "NAME")]
    pub provider: Option<String>,

    /// Override the model id
    #[arg(long, value_name = "NAME")]
    pub model: Option<String>,

    /// Allow shell commands and writes outside the workspace
    #[arg(long)]
    pub danger: bool,

    /// Disable built-in tools
    #[arg(long)]
    pub no_tools: bool,

    /// Maximum model requests per turn
    #[arg(long, default_value_t = 8)]
    pub max_steps: usize,

    /// Maximum output tokens per request
    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,
}

pub fn execute(loader: &ConfigLoader, args: &TuiArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let provider = build_provider(args, &config)?;
    let session_dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let store = Arc::new(SessionStore::new(session_dir));
    let cwd = std::env::current_dir().map_err(DshrError::Io)?;
    let cwd_string = cwd.display().to_string();
    let session = match &args.session {
        Some(id) => store.open(id)?,
        None => store.create(provider.name(), provider.model(), &cwd_string)?,
    };

    let tools = if args.no_tools {
        ToolRegistry::new()
    } else {
        ToolRegistry::with_builtin()
    };
    let mode = if args.danger {
        PermissionMode::DangerFull
    } else {
        PermissionMode::WorkspaceWrite
    };

    dshr_tui::run(dshr_tui::TuiOptions {
        provider,
        tools: Arc::new(tools),
        store,
        session,
        system_prompt: system_prompt(&cwd_string),
        max_steps: args.max_steps.max(1),
        max_tokens: args.max_tokens,
        output_limit: config.tools.output_limit_chars,
        mode,
    })?;
    Ok(0)
}

fn build_provider(args: &TuiArgs, config: &Config) -> Result<Arc<dyn LlmProvider>> {
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
            "unknown provider route '{other}'; expected deepseek or openai-compatible"
        ))),
    }
}

fn system_prompt(cwd: &str) -> String {
    format!(
        "You are dshr, a terminal coding agent. Working directory: {cwd}. \
Use the available tools to inspect and modify files. Prefer small, verifiable steps. \
Be concise and precise."
    )
}

fn llm_error(error: LlmError) -> DshrError {
    DshrError::config(format!("llm: {error}"))
}
