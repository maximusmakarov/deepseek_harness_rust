use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use dshr_agent::PermissionMode;
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider};
use dshr_llm_deepseek::DeepSeekProvider;
use dshr_llm_mock::ReplayProvider;
use dshr_llm_openai::OpenAiProvider;
use dshr_prompt::PromptContext;
use dshr_sandbox::{NetworkMode as SandboxNetwork, Sandbox, SandboxMode, SandboxPolicy};
use dshr_server::ServerConfig;
use dshr_session_jsonl::SessionStore;
use dshr_skills::SkillRegistry;
use dshr_tools::{BashTool, ToolRegistry};

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,

    /// Port (0 selects a free port)
    #[arg(long, default_value_t = 0)]
    pub port: u16,

    /// Bearer token; generated when omitted
    #[arg(long, env = "DSHR_API_TOKEN")]
    pub token: Option<String>,

    /// Provider route override
    #[arg(long)]
    pub provider: Option<String>,

    /// Model override
    #[arg(long)]
    pub model: Option<String>,

    /// Replay a cassette instead of calling a provider
    #[arg(long, value_name = "FILE")]
    pub cassette: Option<PathBuf>,

    /// Allow shell and out-of-workspace writes without approval
    #[arg(long)]
    pub danger: bool,

    /// Disable built-in tools
    #[arg(long)]
    pub no_tools: bool,

    #[arg(long, default_value_t = 8)]
    pub max_steps: usize,

    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,

    /// Requests per minute per bearer token (0 disables)
    #[arg(long, env = "DSHR_RATE_LIMIT", default_value_t = 0)]
    pub rate_limit: u32,

    /// Disable Cordis plugin hot reload (entry and config watching)
    #[arg(long)]
    pub no_watch_plugins: bool,
}

pub async fn execute(loader: &ConfigLoader, args: &ServeArgs) -> Result<u8> {
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

    // Plugin tools are registered after the serving registry is filtered, so
    // the supervisor must apply the same allow/deny lists itself.
    let serving = Arc::new(tools.filtered(&config.tools.allowed, &config.tools.denied));
    let manager = if crate::runtime_flags::safe_mode() {
        None
    } else {
        let manager = dshr_cordis::CordisManager::new(
            Arc::clone(&serving),
            loader.home().to_path_buf(),
            cwd.clone(),
            dshr_cordis::manager::ToolFilter {
                allowed: config.tools.allowed.clone(),
                denied: config.tools.denied.clone(),
            },
        );
        manager.sync(crate::cordis_setup::specs(&config)).await;
        if !args.no_watch_plugins {
            let watch_loader = loader.clone();
            let refresh = Arc::new(move || {
                watch_loader
                    .load()
                    .map(|loaded| crate::cordis_setup::specs(&loaded.config))
                    .unwrap_or_default()
            });
            let _watch = manager.spawn_watch(dshr_cordis::WatchOptions {
                interval: std::time::Duration::from_millis(750),
                config_files: vec![loader.profile_path(), loader.user_path()],
                refresh: Some(refresh),
            });
        }
        Some(manager)
    };
    let system_prompt = build_system_prompt(
        loader.home(),
        &cwd_string,
        provider.name(),
        provider.model(),
        &serving,
        &skills,
    );
    let token = args
        .token
        .clone()
        .or_else(|| std::env::var("DSHR_API_TOKEN").ok())
        .unwrap_or_else(|| {
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            )
        });
    let bind: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .map_err(|error| DshrError::invalid_input(format!("invalid bind address: {error}")))?;

    dshr_server::serve(
        ServerConfig {
            bind,
            token,
            workspace: cwd,
            mode: if args.danger {
                PermissionMode::DangerFull
            } else {
                PermissionMode::WorkspaceWrite
            },
            system_prompt,
            max_steps: args.max_steps.max(1),
            max_tokens: args.max_tokens,
            output_limit: config.tools.output_limit_chars,
            rate_limit_per_min: args.rate_limit,
            auto_title: config.session.auto_title,
            home: loader.home().to_path_buf(),
            profile: config.profile.clone(),
            cordis_plugins: config
                .plugins
                .cordis
                .iter()
                .map(|plugin| (plugin.name.clone(), plugin.enabled))
                .collect(),
            plugins: manager,
        },
        provider,
        serving,
        store,
    )
    .await
    .map_err(DshrError::config)?;
    Ok(0)
}

fn build_provider(args: &ServeArgs, config: &Config) -> Result<Arc<dyn LlmProvider>> {
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
        cwd: PathBuf::from(cwd),
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
