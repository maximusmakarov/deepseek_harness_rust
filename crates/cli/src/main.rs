#![forbid(unsafe_code)]

mod config_cmd;
mod doctor;
mod mcp_setup;
mod plugin_cmd;
mod prompt_cmd;
mod run_cmd;
mod schedule_cmd;
mod serve_cmd;
mod sessions;
mod subagent_runner;
mod tui_cmd;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dshr_config::ConfigLoader;
use dshr_core::Result;

/// Rust-native agent harness: TUI, headless, and HTTP surfaces over one engine.
#[derive(Debug, Parser)]
#[command(name = "dshr", version, about)]
struct Cli {
    /// Harness home directory (defaults to $DSH_HOME or ~/.dshr)
    #[arg(long, global = true, env = "DSH_HOME", value_name = "DIR")]
    home: Option<PathBuf>,

    /// Profile name
    #[arg(
        long,
        global = true,
        env = "DSH_PROFILE",
        default_value = "default",
        value_name = "NAME"
    )]
    profile: String,

    /// Extra configuration layer applied after the user config (repeatable)
    #[arg(long, global = true, value_name = "FILE")]
    patch: Vec<PathBuf>,

    /// Log level for the default filter (RUST_LOG still wins)
    #[arg(long, global = true, env = "DSH_LOG", value_name = "LEVEL")]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check the local installation and configuration
    Doctor(doctor::DoctorArgs),
    /// Inspect the resolved configuration
    Config(config_cmd::ConfigArgs),
    /// Send one prompt and print the streamed answer
    Run(run_cmd::RunArgs),
    /// List, show, or export sessions
    Sessions(sessions::SessionsArgs),
    /// Interactive terminal UI
    Tui(tui_cmd::TuiArgs),
    /// Print the assembled system prompt
    Prompt(prompt_cmd::PromptArgs),
    /// Manage MCP plugins
    Plugin(plugin_cmd::PluginArgs),
    /// Run the HTTP/SSE server and embedded web UI
    Serve(serve_cmd::ServeArgs),
    /// Manage and run cron schedules
    Schedule(schedule_cmd::ScheduleArgs),
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(cli.log_level.as_deref());

    match run(cli).await {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error[{}]: {err}", err.code());
            ExitCode::from(2)
        }
    }
}

async fn run(cli: Cli) -> Result<u8> {
    let home = ConfigLoader::resolve_home(cli.home)?;
    let loader = ConfigLoader::new(home, cli.profile).with_patches(cli.patch);

    match cli.command {
        Command::Doctor(args) => doctor::run(&loader, &args),
        Command::Config(args) => config_cmd::run(&loader, &args),
        Command::Run(args) => run_cmd::execute(&loader, &args).await,
        Command::Sessions(args) => sessions::execute(&loader, &args),
        Command::Tui(args) => tui_cmd::execute(&loader, &args).await,
        Command::Prompt(args) => prompt_cmd::execute(&loader, &args).await,
        Command::Plugin(args) => plugin_cmd::execute(&loader, &args),
        Command::Serve(args) => serve_cmd::execute(&loader, &args).await,
        Command::Schedule(args) => schedule_cmd::execute(&loader, &args).await,
    }
}

fn init_logging(level: Option<&str>) {
    let requested = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| level.map(ToOwned::to_owned))
        .unwrap_or_else(|| "info".to_owned());
    let filter = tracing_subscriber::EnvFilter::try_new(requested)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_writer(std::io::stderr)
        .try_init();
}
