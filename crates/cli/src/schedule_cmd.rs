use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use dshr_agent::{Agent, AgentConfig, ApprovalPolicy, AutoApprover, NullObserver, PermissionMode};
use dshr_config::{Config, ConfigLoader};
use dshr_core::{DshrError, Result};
use dshr_llm::{LlmError, LlmProvider};
use dshr_llm_deepseek::DeepSeekProvider;
use dshr_llm_mock::ReplayProvider;
use dshr_llm_openai::OpenAiProvider;
use dshr_prompt::PromptContext;
use dshr_sandbox::{NetworkMode as SandboxNetwork, Sandbox, SandboxMode, SandboxPolicy};
use dshr_schedule::{ScheduleEntry, ScheduleStore, format_time, parse_time};
use dshr_session_jsonl::SessionStore;
use dshr_tools::{BashTool, ToolRegistry};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Args)]
pub struct ScheduleArgs {
    #[command(subcommand)]
    pub action: ScheduleAction,
}

#[derive(Debug, Subcommand)]
pub enum ScheduleAction {
    /// Add a cron schedule
    Add {
        #[arg(long)]
        cron: String,
        #[arg(long)]
        prompt: String,
        /// Stable id (generated when omitted)
        #[arg(long)]
        name: Option<String>,
        /// Resume this session instead of creating a new one
        #[arg(long)]
        session: Option<String>,
        /// Workspace for the scheduled run
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
    /// List schedules
    List {
        #[arg(long)]
        json: bool,
    },
    /// Remove a schedule
    Remove { id: String },
    /// Enable a schedule
    Enable { id: String },
    /// Disable a schedule
    Disable { id: String },
    /// Run schedules whose next_run is due
    RunDue {
        /// Evaluate at this RFC3339 instant (default: now)
        #[arg(long)]
        now: Option<String>,
        #[command(flatten)]
        run: RunOptions,
    },
    /// Poll for due schedules forever
    Daemon {
        #[arg(long, default_value_t = 30)]
        interval_secs: u64,
        #[command(flatten)]
        run: RunOptions,
    },
}

#[derive(Debug, Args, Clone)]
pub struct RunOptions {
    #[arg(long)]
    pub cassette: Option<PathBuf>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    /// Run schedules with danger-full-access
    #[arg(long)]
    pub danger: bool,
    #[arg(long, default_value_t = 8)]
    pub max_steps: usize,
}

pub async fn execute(loader: &ConfigLoader, args: &ScheduleArgs) -> Result<u8> {
    let path = loader.home().join("schedules.yaml");
    match &args.action {
        ScheduleAction::Add {
            cron,
            prompt,
            name,
            session,
            workspace,
        } => {
            let mut store = ScheduleStore::open(&path).map_err(schedule_error)?;
            let entry = store
                .add(
                    ScheduleEntry {
                        id: name.clone().unwrap_or_default(),
                        cron: cron.clone(),
                        prompt: prompt.clone(),
                        enabled: true,
                        session_id: session.clone(),
                        workspace: workspace.clone(),
                        last_run: None,
                        next_run: None,
                    },
                    Utc::now(),
                )
                .map_err(schedule_error)?;
            println!(
                "added {} next={}",
                entry.id,
                entry.next_run.as_deref().unwrap_or("-")
            );
            Ok(0)
        }
        ScheduleAction::List { json } => {
            let store = ScheduleStore::open(&path).map_err(schedule_error)?;
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(store.entries())
                        .map_err(|error| DshrError::config(error.to_string()))?
                );
                return Ok(0);
            }
            if store.entries().is_empty() {
                println!("(no schedules)");
                return Ok(0);
            }
            for entry in store.entries() {
                println!(
                    "{} {} cron={} next={} prompt={}",
                    entry.id,
                    if entry.enabled { "enabled" } else { "disabled" },
                    entry.cron,
                    entry.next_run.as_deref().unwrap_or("-"),
                    entry.prompt.chars().take(60).collect::<String>()
                );
            }
            Ok(0)
        }
        ScheduleAction::Remove { id } => {
            let mut store = ScheduleStore::open(&path).map_err(schedule_error)?;
            store.remove(id).map_err(schedule_error)?;
            println!("removed {id}");
            Ok(0)
        }
        ScheduleAction::Enable { id } | ScheduleAction::Disable { id } => {
            let enabled = matches!(args.action, ScheduleAction::Enable { .. });
            let mut store = ScheduleStore::open(&path).map_err(schedule_error)?;
            store.set_enabled(id, enabled).map_err(schedule_error)?;
            println!("{} {id}", if enabled { "enabled" } else { "disabled" });
            Ok(0)
        }
        ScheduleAction::RunDue { now, run } => {
            let now = parse_now(now.as_deref())?;
            let mut store = ScheduleStore::open(&path).map_err(schedule_error)?;
            run_due(loader, &mut store, now, run).await?;
            Ok(0)
        }
        ScheduleAction::Daemon { interval_secs, run } => {
            let interval = Duration::from_secs((*interval_secs).max(1));
            loop {
                let mut store = ScheduleStore::open(&path).map_err(schedule_error)?;
                run_due(loader, &mut store, Utc::now(), run).await?;
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = tokio::time::sleep(interval) => {}
                }
            }
            Ok(0)
        }
    }
}

async fn run_due(
    loader: &ConfigLoader,
    store: &mut ScheduleStore,
    now: DateTime<Utc>,
    options: &RunOptions,
) -> Result<usize> {
    let config = loader.load()?.config;
    let provider = build_provider(options, &config)?;
    let session_dir = config
        .session
        .dir
        .clone()
        .ok_or_else(|| DshrError::config("session.dir is not resolved"))?;
    let sessions = Arc::new(SessionStore::new(session_dir));
    let due = store.due(now);
    if due.is_empty() {
        println!("no due schedules at {}", format_time(now));
        return Ok(0);
    }

    for entry in due {
        let workspace = entry
            .workspace
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let mut tools = ToolRegistry::with_builtin();
        let sandbox = Arc::new(Sandbox::platform(SandboxPolicy {
            mode: if options.danger {
                SandboxMode::DangerFull
            } else {
                SandboxMode::WorkspaceWrite
            },
            workspace: workspace.clone(),
            network: SandboxNetwork::Deny,
        }));
        tools.register(BashTool::new(Some(sandbox)));
        let system_prompt = build_prompt(
            loader,
            &workspace,
            provider.name(),
            provider.model(),
            &tools,
        );
        let cwd = workspace.display().to_string();

        let session = match &entry.session_id {
            Some(id) => sessions.open(id)?,
            None => sessions.create(provider.name(), provider.model(), &cwd)?,
        };
        let title = format!("schedule: {}", entry.id);
        let session = sessions
            .set_title(session.header.id.as_str(), &title)
            .unwrap_or(session);

        let mut agent = Agent::new(
            Arc::clone(&provider),
            Arc::new(tools),
            Arc::clone(&sessions),
            session,
            ApprovalPolicy {
                mode: if options.danger {
                    PermissionMode::DangerFull
                } else {
                    PermissionMode::WorkspaceWrite
                },
                workspace,
                approver: Arc::new(AutoApprover),
            },
            AgentConfig {
                system_prompt,
                max_steps: options.max_steps.max(1),
                max_tokens: 2048,
                output_limit: config.tools.output_limit_chars,
                compaction: Some(dshr_compaction::CompactionConfig::default()),
            },
        );

        let result = agent
            .run_turn(&entry.prompt, CancellationToken::new(), &NullObserver)
            .await;
        match result {
            Ok(report) => println!("ran {}: {}", entry.id, first_line(&report.text)),
            Err(error) => println!("failed {}: {error}", entry.id),
        }
        let next = store.mark_run(&entry.id, now).map_err(schedule_error)?;
        if let Some(next) = next {
            println!("next {} at {}", entry.id, format_time(next));
        }
    }
    Ok(0)
}

fn build_provider(options: &RunOptions, config: &Config) -> Result<Arc<dyn LlmProvider>> {
    if let Some(path) = &options.cassette {
        return Ok(Arc::new(
            ReplayProvider::from_file(path).map_err(llm_error)?,
        ));
    }
    let route = options
        .provider
        .clone()
        .unwrap_or_else(|| config.model.provider.clone());
    let model = options
        .model
        .clone()
        .or_else(|| Some(config.model.name.clone()));
    match route.as_str() {
        "deepseek" | "deepseek-account" | "deepseek-official" => Ok(Arc::new(
            DeepSeekProvider::from_env(model, None).map_err(llm_error)?,
        )),
        "openai" | "openai-compatible" | "routerai" => Ok(Arc::new(
            OpenAiProvider::from_env(model, None).map_err(llm_error)?,
        )),
        other => Err(DshrError::config(format!(
            "unknown provider route '{other}'; expected deepseek or openai-compatible"
        ))),
    }
}

fn build_prompt(
    loader: &ConfigLoader,
    workspace: &std::path::Path,
    provider: &str,
    model: &str,
    tools: &ToolRegistry,
) -> String {
    let today = dshr_session::now_rfc3339();
    dshr_prompt::build(&PromptContext {
        cwd: workspace.to_path_buf(),
        home: loader.home().to_path_buf(),
        provider: provider.to_owned(),
        model: model.to_owned(),
        date: today.get(..10).unwrap_or(&today).to_owned(),
        tools: tools.names(),
        skills: Vec::new(),
    })
}

fn parse_now(value: Option<&str>) -> Result<DateTime<Utc>> {
    match value {
        Some(value) => parse_time(value)
            .ok_or_else(|| DshrError::invalid_input(format!("invalid RFC3339 instant '{value}'"))),
        None => Ok(Utc::now()),
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("(empty)")
        .chars()
        .take(120)
        .collect()
}

fn schedule_error(error: dshr_schedule::ScheduleError) -> DshrError {
    DshrError::invalid_input(error.to_string())
}

fn llm_error(error: LlmError) -> DshrError {
    DshrError::config(format!("llm: {error}"))
}
