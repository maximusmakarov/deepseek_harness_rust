use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use dshr_config::ConfigLoader;
use dshr_core::{DshrError, Result};
use dshr_i18n::current;
use serde_json::{Value, json};

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub action: PluginAction,
}

#[derive(Debug, Subcommand)]
pub enum PluginAction {
    /// List configured MCP plugins
    List {
        #[arg(long)]
        json: bool,
    },
    /// Add or replace an MCP plugin
    Add {
        /// Plugin/server name
        name: String,
        /// Command to launch a stdio server
        #[arg(long)]
        command: Option<String>,
        /// Argument (repeatable)
        #[arg(long = "arg", allow_hyphen_values = true)]
        args: Vec<String>,
        /// Environment entry KEY=VALUE (repeatable)
        #[arg(long = "env")]
        env: Vec<String>,
        /// Streamable HTTP endpoint instead of a command
        #[arg(long)]
        url: Option<String>,
    },
    /// Remove an MCP plugin
    Remove {
        /// Plugin/server name
        name: String,
    },
}

pub fn execute(loader: &ConfigLoader, args: &PluginArgs) -> Result<u8> {
    match &args.action {
        PluginAction::List { json } => list(loader, *json),
        PluginAction::Add {
            name,
            command,
            args,
            env,
            url,
        } => add(loader, name, command.as_deref(), args, env, url.as_deref()),
        PluginAction::Remove { name } => remove(loader, name),
    }
}

fn list(loader: &ConfigLoader, json: bool) -> Result<u8> {
    let config = loader.load()?.config;
    if json {
        let encoded = serde_json::to_string_pretty(&config.mcp.servers)
            .map_err(|error| DshrError::config(format!("serialize plugins: {error}")))?;
        println!("{encoded}");
        return Ok(0);
    }
    if config.mcp.servers.is_empty() {
        println!("{}", current().no_plugins());
        return Ok(0);
    }
    for server in config.mcp.servers {
        println!(
            "{}: {} {} ({})",
            server.name,
            server.url.as_deref().unwrap_or(&server.command),
            server.args.join(" "),
            if server.enabled {
                current().enabled()
            } else {
                current().disabled()
            }
        );
    }
    Ok(0)
}

fn add(
    loader: &ConfigLoader,
    name: &str,
    command: Option<&str>,
    args: &[String],
    env: &[String],
    url: Option<&str>,
) -> Result<u8> {
    if command.is_none() && url.is_none() {
        return Err(DshrError::invalid_input(
            "plugin add requires --command or --url",
        ));
    }
    let mut root = load_raw(loader)?;
    let mut environment = BTreeMap::new();
    for entry in env {
        let Some((key, value)) = entry.split_once('=') else {
            return Err(DshrError::invalid_input(format!(
                "invalid --env '{entry}', expected KEY=VALUE"
            )));
        };
        environment.insert(key.to_owned(), value.to_owned());
    }

    let servers = servers_mut(&mut root)?;
    servers.retain(|server| server["name"].as_str() != Some(name));
    servers.push(json!({
        "name": name,
        "command": command.unwrap_or_default(),
        "args": args,
        "env": environment,
        "url": url,
        "enabled": true
    }));
    save_raw(loader, &root)?;
    println!("{}", current().plugin_added(name));
    Ok(0)
}

fn remove(loader: &ConfigLoader, name: &str) -> Result<u8> {
    let mut root = load_raw(loader)?;
    let servers = servers_mut(&mut root)?;
    let before = servers.len();
    servers.retain(|server| server["name"].as_str() != Some(name));
    if servers.len() == before {
        return Err(DshrError::not_found(format!("plugin '{name}' not found")));
    }
    save_raw(loader, &root)?;
    println!("{}", current().plugin_removed(name));
    Ok(0)
}

fn load_raw(loader: &ConfigLoader) -> Result<Value> {
    let path = loader.user_path();
    if !path.is_file() {
        return Ok(json!({
            "version": 1,
            "mcp": {"servers": []}
        }));
    }
    let text = std::fs::read_to_string(&path)?;
    serde_saphyr::from_str(&text)
        .map_err(|error| DshrError::config(format!("parse {}: {error}", path.display())))
}

fn save_raw(loader: &ConfigLoader, value: &Value) -> Result<()> {
    let path = loader.user_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_saphyr::to_string(value)
        .map_err(|error| DshrError::config(format!("serialize config: {error}")))?;
    std::fs::write(&path, text)?;
    Ok(())
}

fn servers_mut(root: &mut Value) -> Result<&mut Vec<Value>> {
    if !root.is_object() {
        return Err(DshrError::config("configuration root must be a mapping"));
    }
    let mcp = root
        .as_object_mut()
        .expect("object")
        .entry("mcp")
        .or_insert_with(|| json!({"servers": []}));
    let servers = mcp
        .as_object_mut()
        .ok_or_else(|| DshrError::config("mcp must be a mapping"))?
        .entry("servers")
        .or_insert_with(|| json!([]));
    servers
        .as_array_mut()
        .ok_or_else(|| DshrError::config("mcp.servers must be a list"))
}
