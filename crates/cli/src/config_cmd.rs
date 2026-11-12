use clap::{Args, Subcommand};
use dshr_config::ConfigLoader;
use dshr_core::{DshrError, Result};
use serde_json::Value;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the merged configuration
    Show(ShowArgs),
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Emit JSON instead of YAML
    #[arg(long, conflicts_with = "schema")]
    pub json: bool,

    /// Print the JSON Schema of the configuration document
    #[arg(long)]
    pub schema: bool,

    /// Print the layer resolution chain to stderr
    #[arg(long)]
    pub sources: bool,
}

pub fn run(loader: &ConfigLoader, args: &ConfigArgs) -> Result<u8> {
    match &args.action {
        ConfigAction::Show(show) => show_config(loader, show),
    }
}

fn show_config(loader: &ConfigLoader, args: &ShowArgs) -> Result<u8> {
    if args.schema {
        print_json(&dshr_config::json_schema())?;
        return Ok(0);
    }

    let loaded = loader.load()?;
    tracing::debug!(
        home = %loader.home().display(),
        profile = loader.profile(),
        "configuration resolved"
    );

    if args.sources {
        for source in &loaded.sources {
            let path = source.path.as_ref().map_or_else(
                || "(built-in)".to_owned(),
                |path| path.display().to_string(),
            );
            eprintln!(
                "{:<16} {:<8} {}",
                source.layer,
                if source.found { "found" } else { "missing" },
                path
            );
        }
    }

    if args.json {
        print_json(&serde_json::to_value(&loaded.config).map_err(serialize_error)?)?;
    } else {
        let yaml = serde_saphyr::to_string(&loaded.config).map_err(serialize_error)?;
        print!("{yaml}");
    }

    Ok(0)
}

fn print_json(value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(serialize_error)?;
    println!("{text}");
    Ok(())
}

fn serialize_error(err: impl std::fmt::Display) -> DshrError {
    DshrError::config(format!("cannot serialize configuration: {err}"))
}
