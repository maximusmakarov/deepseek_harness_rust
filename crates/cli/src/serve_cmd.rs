use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use dshr_agent::PermissionMode;
use dshr_config::credentials;
use dshr_config::{
    Config, ConfigLoader, ModelsDocument, Protocol, ResolvedProvider, RetryMode as ConfigRetryMode,
};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider, RetryMode as LlmRetryMode, RetryPlan};
use dshr_llm_deepseek::{Auth, DeepSeekConfig, DeepSeekProvider};
use dshr_llm_mock::ReplayProvider;
use dshr_llm_openai::{OpenAiConfig, OpenAiProvider};
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

    /// Allow plugin host routes (C1) when binding a non-loopback address
    #[arg(long)]
    pub allow_lan_plugin_routes: bool,
}

pub async fn execute(loader: &ConfigLoader, args: &ServeArgs) -> Result<u8> {
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
    let wasm = crate::wasm_setup::manager(
        tools.clone(),
        loader.home(),
        std::path::Path::new(&cwd),
        &config.plugins,
    )?;
    wasm.sync().await;
    wasm.spawn_watch(std::time::Duration::from_secs(2));
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
                    .map_err(|error| error.to_string())
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
            allow_plugin_routes_lan: args.allow_lan_plugin_routes,
            internal_token: None,
            wasm: Some(wasm),
        },
        provider,
        serving,
        store,
    )
    .await
    .map_err(DshrError::config)?;
    Ok(0)
}

fn build_provider(
    args: &ServeArgs,
    config: &Config,
    home: &std::path::Path,
) -> Result<Arc<dyn LlmProvider>> {
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
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.model.name.clone());
    // The Models page stores per-provider profiles in `models.yaml`; a route
    // without a profile falls back to the adapter's environment/defaults.
    let document =
        ModelsDocument::load(home).map_err(|error| DshrError::config(error.to_string()))?;
    let resolved = document
        .resolve(&route, config.model.base_url.as_deref())
        .ok_or_else(|| {
            DshrError::config(format!(
                "unknown provider route '{route}'; add it in Settings -> Models or use deepseek or openai-compatible"
            ))
        })?;
    if resolved.models_from_profile
        && !resolved.models.is_empty()
        && !resolved.models.iter().any(|entry| entry.id == model)
    {
        tracing::warn!(
            provider = %route,
            model = %model,
            "the active model is not in the provider's model list; serving it anyway"
        );
    }
    let base_url = resolved
        .effective_base_url
        .clone()
        .unwrap_or_else(|| default_base_url(resolved.protocol).to_owned());
    let reference = resolved
        .api_key_env
        .clone()
        .unwrap_or_else(|| default_key_reference(&route).to_owned());
    let retry = retry_plan(&resolved);
    let (secret, account_token) = read_secret(&reference, &resolved, home)
        .ok_or_else(|| {
            LlmError::MissingCredentials(format!(
                "set {reference} in the environment or save it in Settings -> Models"
            ))
        })
        .map_err(llm_error)?;
    match resolved.protocol {
        Protocol::AnthropicMessages => {
            let auth = if account_token {
                Auth::AccountToken(secret)
            } else {
                Auth::ApiKey(secret)
            };
            Ok(Arc::new(
                DeepSeekProvider::new(DeepSeekConfig {
                    base_url,
                    model,
                    max_tokens: args.max_tokens.max(1),
                    auth,
                    retry: retry.clone(),
                })
                .map_err(llm_error)?,
            ))
        }
        Protocol::OpenAiChat => Ok(Arc::new(
            OpenAiProvider::new(OpenAiConfig {
                base_url,
                model,
                max_tokens: args.max_tokens.max(1),
                api_key: secret,
                retry,
            })
            .map_err(llm_error)?,
        )),
    }
}

fn retry_plan(resolved: &ResolvedProvider) -> RetryPlan {
    let retry = &resolved.retry;
    RetryPlan::new(
        match retry.mode {
            ConfigRetryMode::Normal => LlmRetryMode::Normal,
            ConfigRetryMode::Always => LlmRetryMode::Always,
        },
        retry.max_retries,
        retry.retryable_codes.clone(),
        retry.initial_delay_ms,
        retry.max_delay_ms,
        retry.jitter_ratio as f32,
    )
}

fn default_base_url(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::AnthropicMessages => "https://api.deepseek.com/anthropic",
        Protocol::OpenAiChat => "https://api.openai.com/v1",
    }
}

fn default_key_reference(route: &str) -> &'static str {
    match route {
        "deepseek" | "deepseek-account" => "DSH_DEEPSEEK_TOKEN",
        "deepseek-official" => "DEEPSEEK_API_KEY",
        "routerai" => "ROUTERAI_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

/// Reads the profile's reference first, then the adapter's historical
/// fallbacks, so an existing `desktop.env` keeps working untouched.
/// Returns the secret and whether it is the account grant (which the DeepSeek
/// adapter sends on a different header than an API key).
fn read_secret(
    reference: &str,
    resolved: &ResolvedProvider,
    home: &std::path::Path,
) -> Option<(String, bool)> {
    let read =
        |name: &str| credentials::resolve(home, name).map(|resolved| resolved.value().to_owned());
    if let Some(value) = read(reference) {
        return Some((value, reference == "DSH_DEEPSEEK_TOKEN"));
    }
    if resolved.protocol == Protocol::AnthropicMessages {
        return read("DEEPSEEK_API_KEY")
            .map(|value| (value, false))
            .or_else(|| read("DSH_DEEPSEEK_TOKEN").map(|value| (value, true)));
    }
    read("OPENAI_API_KEY")
        .or_else(|| read("ROUTERAI_API_KEY"))
        .map(|value| (value, false))
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
