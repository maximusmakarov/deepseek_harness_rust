use std::sync::Arc;

use crate::provider_router;
use clap::Args;
use dshr_agent::PermissionMode;
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result};
use dshr_llm::LlmProvider;
use dshr_prompt::PromptContext;
use dshr_sandbox::{NetworkMode as SandboxNetwork, Sandbox, SandboxMode, SandboxPolicy};
use dshr_session_jsonl::SessionStore;
use dshr_skills::SkillRegistry;
use dshr_tools::{BashTool, ToolRegistry};

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

pub async fn execute(loader: &ConfigLoader, args: &TuiArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let provider = build_provider(args, &config, loader.home())?;
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

    let mut tools = if args.no_tools {
        ToolRegistry::new()
    } else {
        ToolRegistry::with_builtin()
    };
    let skills = Arc::new(SkillRegistry::discover(&[
        loader.home().join("skills"),
        cwd.join(".dshr/skills"),
    ]));
    if !skills.is_empty() {
        tools.register_arc(Arc::new(skills.tool()));
    }
    crate::mcp_setup::register(&mut tools, &config, Some(Arc::clone(&provider))).await;
    dshr_office::register_tools(&mut tools);
    dshr_browser::register_tools(&mut tools, Arc::new(dshr_browser::BrowserManager::new()));
    crate::wasm_setup::register(
        &mut tools,
        loader.home(),
        std::path::Path::new(&cwd),
        &config.plugins,
    )?;
    crate::cordis_setup::register(&tools, &config, loader.home()).await;
    crate::lsp_setup::register(&mut tools, &config, cwd.clone());
    let sandbox = Arc::new(Sandbox::platform(SandboxPolicy {
        mode: if args.danger {
            SandboxMode::DangerFull
        } else {
            SandboxMode::WorkspaceWrite
        },
        workspace: cwd.clone(),
        network: SandboxNetwork::Deny,
    }));
    tools.register(BashTool::new(Some(Arc::clone(&sandbox))));
    let jobs = Arc::new(dshr_jobs::JobManager::with_state(
        Some(sandbox),
        loader.home().join("jobs"),
    ));
    dshr_jobs::register_tools(&mut tools, jobs);
    let mode = if args.danger {
        PermissionMode::DangerFull
    } else {
        PermissionMode::WorkspaceWrite
    };

    let tools = tools.filtered(&config.tools.allowed, &config.tools.denied);
    let system_prompt = build_system_prompt(
        loader.home(),
        &cwd_string,
        provider.name(),
        provider.model(),
        &tools,
        &skills,
    );

    dshr_tui::run(dshr_tui::TuiOptions {
        provider,
        tools: Arc::new(tools),
        store,
        session,
        system_prompt,
        max_steps: args.max_steps.max(1),
        max_tokens: args.max_tokens,
        output_limit: config.tools.output_limit_chars,
        mode,
        lang: dshr_i18n::Lang::parse(&config.ui.language),
        auto_title: config.session.auto_title,
    })?;
    Ok(0)
}

fn build_provider(
    args: &TuiArgs,
    config: &Config,
    home: &std::path::Path,
) -> Result<Arc<dyn LlmProvider>> {
    provider_router::build(
        &provider_router::Request {
            cassette: args.cassette.as_deref(),
            provider: args.provider.as_deref(),
            model: args.model.as_deref(),
            max_tokens: Some(args.max_tokens),
        },
        config,
        home,
    )
}

fn build_system_prompt(
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
