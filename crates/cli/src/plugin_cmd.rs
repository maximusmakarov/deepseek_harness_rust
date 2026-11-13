use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// Generate an Ed25519 key pair for signing WASM plugins
    Keygen {
        /// Where to write the secret key (hex). The public key is printed.
        #[arg(long)]
        out: PathBuf,
    },
    /// Sign a WASM plugin manifest
    Sign {
        /// Path to the plugin manifest (*.yaml)
        manifest: PathBuf,
        /// Secret key file written by `plugin keygen`
        #[arg(long)]
        key: PathBuf,
    },
    /// Verify a WASM plugin manifest signature
    Verify {
        /// Path to the plugin manifest (*.yaml)
        manifest: PathBuf,
        /// Public key (hex); defaults to the manifest `signer` field
        #[arg(long)]
        key: Option<String>,
    },
    /// Install and manage npm/Cordis plugins
    Npm {
        #[command(subcommand)]
        action: NpmAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum NpmAction {
    /// Install an npm package as a Cordis plugin
    Add {
        /// npm spec (package, @scope/name@version, file:…)
        spec: String,
        /// Config name and package name when it cannot be derived from the spec
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove an installed Cordis plugin
    Remove {
        /// Plugin name
        name: String,
    },
    /// Update installed Cordis plugins
    Update {
        /// Plugin name; all plugins when omitted
        name: Option<String>,
    },
    /// List configured Cordis plugins
    List {
        #[arg(long)]
        json: bool,
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
        PluginAction::Keygen { out } => keygen(out),
        PluginAction::Sign { manifest, key } => sign(manifest, key),
        PluginAction::Verify { manifest, key } => verify(manifest, key.as_deref()),
        PluginAction::Npm { action } => npm(loader, action),
    }
}

fn npm(loader: &ConfigLoader, action: &NpmAction) -> Result<u8> {
    match action {
        NpmAction::Add { spec, name } => npm_add(loader, spec, name.as_deref()),
        NpmAction::Remove { name } => npm_remove(loader, name),
        NpmAction::Update { name } => npm_update(loader, name.as_deref()),
        NpmAction::List { json } => npm_list(loader, *json),
    }
}

fn npm_binary() -> String {
    std::env::var("DSHR_NPM").unwrap_or_else(|_| "npm".to_owned())
}

fn install_dir(loader: &ConfigLoader, name: &str) -> PathBuf {
    let home = PathBuf::from(loader.home());
    let home = if home.is_absolute() {
        home
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&home))
            .unwrap_or(home)
    };
    home.join("cordis").join("packages").join(name)
}

fn npm_add(loader: &ConfigLoader, spec: &str, requested_name: Option<&str>) -> Result<u8> {
    let package = package_name(spec)
        .or_else(|| requested_name.map(ToOwned::to_owned))
        .ok_or_else(|| {
            DshrError::invalid_input(
                "cannot derive the package name; pass --name for file/git specs",
            )
        })?;
    let name = requested_name.unwrap_or(&package).to_owned();
    let dir = install_dir(loader, &name);
    std::fs::create_dir_all(&dir)?;
    let npm = npm_binary();
    run_npm(&npm, &["init", "-y"], &dir)?;
    run_npm(
        &npm,
        &["install", "--prefix", &dir.display().to_string(), spec],
        &dir,
    )?;
    let entry = dir.join("node_modules").join(&package);
    if !entry.is_dir() {
        return Err(DshrError::invalid_input(format!(
            "npm installed no package directory at {}",
            entry.display()
        )));
    }
    let mut root = load_raw(loader)?;
    let plugins = cordis_mut(&mut root)?;
    plugins.retain(|plugin| plugin["name"].as_str() != Some(&name));
    plugins.push(json!({
        "name": name,
        "entry": entry.display().to_string(),
        "enabled": true
    }));
    save_raw(loader, &root)?;
    println!("{}", current().plugin_added(&name));
    Ok(0)
}

fn npm_remove(loader: &ConfigLoader, name: &str) -> Result<u8> {
    let mut root = load_raw(loader)?;
    let plugins = cordis_mut(&mut root)?;
    let before = plugins.len();
    plugins.retain(|plugin| plugin["name"].as_str() != Some(name));
    if plugins.len() == before {
        return Err(DshrError::not_found(format!(
            "Cordis plugin '{name}' not found"
        )));
    }
    save_raw(loader, &root)?;
    let dir = install_dir(loader, name);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }
    println!("{}", current().plugin_removed(name));
    Ok(0)
}

fn npm_update(loader: &ConfigLoader, name: Option<&str>) -> Result<u8> {
    let config = loader.load()?.config;
    let plugins: Vec<_> = config
        .plugins
        .cordis
        .iter()
        .filter(|plugin| name.is_none_or(|name| plugin.name == name))
        .collect();
    if plugins.is_empty() {
        return Err(DshrError::not_found(format!(
            "Cordis plugin '{}' not found",
            name.unwrap_or("*")
        )));
    }
    let npm = npm_binary();
    for plugin in plugins {
        let dir = install_dir(loader, &plugin.name);
        run_npm(
            &npm,
            &["update", "--prefix", &dir.display().to_string()],
            &dir,
        )?;
        println!("updated {}", plugin.name);
    }
    Ok(0)
}

fn npm_list(loader: &ConfigLoader, json: bool) -> Result<u8> {
    let config = loader.load()?.config;
    if json {
        let encoded = serde_json::to_string_pretty(&config.plugins.cordis)
            .map_err(|error| DshrError::config(error.to_string()))?;
        println!("{encoded}");
        return Ok(0);
    }
    if config.plugins.cordis.is_empty() {
        println!("{}", current().no_plugins());
        return Ok(0);
    }
    for plugin in config.plugins.cordis {
        println!(
            "{}: {} ({})",
            plugin.name,
            plugin.entry.display(),
            if plugin.enabled {
                current().enabled()
            } else {
                current().disabled()
            }
        );
    }
    Ok(0)
}

fn run_npm(npm: &str, args: &[&str], dir: &Path) -> Result<()> {
    let output = std::process::Command::new(npm)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|error| DshrError::invalid_input(format!("run {npm}: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr
            .chars()
            .rev()
            .take(600)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(DshrError::invalid_input(format!(
            "{npm} {} failed: {tail}",
            args.join(" ")
        )));
    }
    Ok(())
}

fn package_name(spec: &str) -> Option<String> {
    let spec = spec.trim();
    if spec.is_empty()
        || spec.starts_with("file:")
        || spec.starts_with('.')
        || spec.starts_with('/')
        || spec.starts_with("git")
        || spec.starts_with("http")
    {
        return None;
    }
    if let Some(rest) = spec.strip_prefix('@') {
        let slash = rest.find('/')?;
        let after = &rest[slash + 1..];
        let end = after.find('@').unwrap_or(after.len());
        Some(format!("@{}/{}", &rest[..slash], &after[..end]))
    } else {
        let end = spec.find('@').unwrap_or(spec.len());
        Some(spec[..end].to_owned())
    }
}

fn keygen(out: &Path) -> Result<u8> {
    let (public, secret) = dshr_wasm::generate_keypair()
        .map_err(|error| DshrError::invalid_input(error.to_string()))?;
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, format!("{secret}\n"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o600))?;
    }
    println!(
        "{}",
        current().plugin_key_written(&out.display().to_string())
    );
    println!("{}", current().plugin_public_key(&public));
    Ok(0)
}

fn sign(manifest: &Path, key_path: &Path) -> Result<u8> {
    let secret = std::fs::read_to_string(key_path)?.trim().to_owned();
    let public = dshr_wasm::sign_manifest(manifest, &secret)
        .map_err(|error| DshrError::invalid_input(error.to_string()))?;
    println!(
        "{}",
        current().plugin_signed(&manifest.display().to_string())
    );
    println!("{}", current().plugin_public_key(&public));
    Ok(0)
}

fn verify(manifest: &Path, key: Option<&str>) -> Result<u8> {
    let key = match key {
        Some(key) => key.to_owned(),
        None => {
            let text = std::fs::read_to_string(manifest)?;
            let value: Value = serde_saphyr::from_str(&text).map_err(|error| {
                DshrError::config(format!("parse {}: {error}", manifest.display()))
            })?;
            value["signer"]
                .as_str()
                .ok_or_else(|| {
                    DshrError::invalid_input(
                        "manifest has no `signer`; pass --key with the public key",
                    )
                })?
                .to_owned()
        }
    };
    dshr_wasm::verify_manifest(manifest, &key)
        .map_err(|error| DshrError::invalid_input(error.to_string()))?;
    println!(
        "{}",
        current().plugin_verified(&manifest.display().to_string())
    );
    Ok(0)
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

fn cordis_mut(root: &mut Value) -> Result<&mut Vec<Value>> {
    if !root.is_object() {
        return Err(DshrError::config("configuration root must be a mapping"));
    }
    let plugins = root
        .as_object_mut()
        .expect("object")
        .entry("plugins")
        .or_insert_with(|| json!({"cordis": []}));
    let cordis = plugins
        .as_object_mut()
        .ok_or_else(|| DshrError::config("plugins must be a mapping"))?
        .entry("cordis")
        .or_insert_with(|| json!([]));
    cordis
        .as_array_mut()
        .ok_or_else(|| DshrError::config("plugins.cordis must be a list"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_npm_package_names() {
        assert_eq!(package_name("cordis").as_deref(), Some("cordis"));
        assert_eq!(package_name("cordis@2.0.0").as_deref(), Some("cordis"));
        assert_eq!(
            package_name("@scope/name@1.2.3").as_deref(),
            Some("@scope/name")
        );
        assert_eq!(package_name("@scope/name").as_deref(), Some("@scope/name"));
        assert!(package_name("file:../plugin").is_none());
        assert!(package_name("./plugin").is_none());
        assert!(package_name("https://example.test/plugin.tgz").is_none());
    }

    #[test]
    fn cordis_mut_creates_and_preserves_entries() {
        let mut root = json!({"version": 1, "mcp": {"servers": []}});
        let plugins = cordis_mut(&mut root).expect("cordis");
        plugins.push(json!({"name": "a", "entry": "/a", "enabled": true}));
        let plugins = cordis_mut(&mut root).expect("cordis");
        plugins.push(json!({"name": "b", "entry": "/b", "enabled": true}));
        assert_eq!(
            root["plugins"]["cordis"].as_array().expect("array").len(),
            2
        );
        assert_eq!(root["mcp"]["servers"].as_array().expect("array").len(), 0);
    }
}
