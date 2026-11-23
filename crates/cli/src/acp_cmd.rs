use std::path::PathBuf;
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
pub struct AcpArgs {
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub cassette: Option<PathBuf>,
    /// Allow shell and writes outside the workspace
    #[arg(long)]
    pub danger: bool,
    #[arg(long, default_value_t = 8)]
    pub max_steps: usize,
    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,
}

pub async fn execute(loader: &ConfigLoader, args: &AcpArgs) -> Result<u8> {
    let config = loader.load()?.config;
    let provider = build_provider(args, &config, loader.home())?;
    let cwd = std::env::current_dir().map_err(DshrError::Io)?;
    let session_dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let store = Arc::new(SessionStore::new(session_dir));

    let mut tools = ToolRegistry::with_builtin();
    let fs = Arc::new(dshr_acp::AcpFsClient::new());
    tools.register(dshr_acp::AcpReadTool::new(Arc::clone(&fs)));
    tools.register(dshr_acp::AcpWriteTool::new(Arc::clone(&fs)));
    let terminal = Arc::new(dshr_acp::AcpTerminalClient::new());
    tools.register(dshr_acp::AcpBashTool::new(Arc::clone(&terminal)));
    let skills = Arc::new(SkillRegistry::discover(&[
        loader.home().join("skills"),
        cwd.join(".dshr/skills"),
    ]));
    if !skills.is_empty() {
        tools.register_arc(Arc::new(skills.tool()));
    }
    crate::mcp_setup::register(&mut tools, &config, Some(Arc::clone(&provider))).await;
    crate::lsp_setup::register(&mut tools, &config, cwd.clone());
    dshr_office::register_tools(&mut tools);
    dshr_browser::register_tools(&mut tools, Arc::new(dshr_browser::BrowserManager::new()));
    crate::wasm_setup::register(&mut tools, loader.home(), &cwd, &config.plugins)?;
    crate::cordis_setup::register(&tools, &config, loader.home()).await;
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
    let tools = tools.filtered(&config.tools.allowed, &config.tools.denied);

    let today = dshr_session::now_rfc3339();
    let system_prompt = dshr_prompt::build(&PromptContext {
        cwd: cwd.clone(),
        home: loader.home().to_path_buf(),
        provider: provider.name().to_owned(),
        model: provider.model().to_owned(),
        date: today.get(..10).unwrap_or(&today).to_owned(),
        tools: tools.names(),
        skills: skills.catalog(),
    });

    dshr_acp::AcpServer::new(
        provider,
        Arc::new(tools),
        store,
        dshr_acp::AcpConfig {
            system_prompt,
            mode: if args.danger {
                PermissionMode::DangerFull
            } else {
                PermissionMode::WorkspaceWrite
            },
            max_steps: args.max_steps.max(1),
            max_tokens: args.max_tokens,
            output_limit: config.tools.output_limit_chars,
        },
    )
    .with_fs(fs)
    .with_terminal(terminal)
    .serve(tokio::io::stdin(), tokio::io::stdout())
    .await
    .map_err(|error| DshrError::config(error.to_string()))?;
    Ok(0)
}

fn build_provider(
    args: &AcpArgs,
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
