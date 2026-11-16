//! `dshr runtime`: pinned, hash-verified runtime packs for plugin sidecars.

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use dshr_config::ConfigLoader;
use dshr_cordis::runtime::{self, InstallRequest, Installed};
use dshr_core::{DshrError, Result};

#[derive(Debug, Args)]
pub struct RuntimeArgs {
    #[command(subcommand)]
    pub action: RuntimeAction,
}

#[derive(Debug, Subcommand)]
pub enum RuntimeAction {
    /// List installed runtime packs
    List {
        #[arg(long)]
        json: bool,
    },
    /// Install a runtime pack from a local archive or an https URL
    Install {
        /// Pack spec: <engine>@<version>, for example node@22.11.0
        spec: String,
        /// Local .tar.gz archive or https:// URL
        #[arg(long)]
        from: String,
        /// Pinned sha256 of the archive (64 hex characters)
        #[arg(long)]
        sha256: String,
        /// Executable inside the archive, relative to the pack root
        #[arg(long, default_value = runtime::DEFAULT_BIN)]
        bin: String,
        /// Leading path components to drop while unpacking (0 or 1)
        #[arg(long, default_value_t = 1)]
        strip: usize,
        /// Signature file with a hex Ed25519 signature over the archive
        #[arg(long, requires = "public_key")]
        signature: Option<PathBuf>,
        /// Public key (hex) matching --signature
        #[arg(long)]
        public_key: Option<String>,
        /// Replace an already installed version
        #[arg(long)]
        force: bool,
    },
    /// Make an installed pack active
    Use {
        /// Pack spec: <engine>@<version>
        spec: String,
    },
    /// Remove an installed pack
    Remove {
        /// Pack spec: <engine>@<version>
        spec: String,
    },
}

pub async fn execute(loader: &ConfigLoader, args: &RuntimeArgs) -> Result<u8> {
    let home = loader.home();
    match &args.action {
        RuntimeAction::List { json } => list(home, *json),
        RuntimeAction::Install {
            spec,
            from,
            sha256,
            bin,
            strip,
            signature,
            public_key,
            force,
        } => {
            let (engine, version) = split_spec(spec)?;
            let archive = fetch_archive(home, from).await?;
            let signature = match (signature, public_key) {
                (Some(path), Some(key)) => Some((
                    key.trim().to_owned(),
                    std::fs::read_to_string(path)
                        .map_err(|error| {
                            DshrError::invalid_input(format!(
                                "runtime: cannot read {}: {error}",
                                path.display()
                            ))
                        })?
                        .trim()
                        .to_owned(),
                )),
                _ => None,
            };
            let mut request = InstallRequest::new(engine, version, &archive, sha256);
            request.bin.clone_from(bin);
            request.strip = *strip;
            request.signature = signature;
            request.force = *force;
            let installed = runtime::install(home, request)?;
            print_installed(&installed);
            Ok(0)
        }
        RuntimeAction::Use { spec } => {
            let (engine, version) = split_spec(spec)?;
            let installed = runtime::activate(home, engine, version)?;
            println!(
                "{} {} -> {}",
                installed.receipt.engine,
                installed.receipt.version,
                installed.program().display()
            );
            Ok(0)
        }
        RuntimeAction::Remove { spec } => {
            let (engine, version) = split_spec(spec)?;
            if runtime::remove(home, engine, version)? {
                println!("removed {engine}@{version}");
            } else {
                println!("{engine}@{version} is not installed");
            }
            Ok(0)
        }
    }
}

fn list(home: &Path, json: bool) -> Result<u8> {
    let installed = runtime::list(home)?;
    if json {
        let rows: Vec<_> = installed
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "engine": entry.receipt.engine,
                    "version": entry.receipt.version,
                    "current": entry.current,
                    "program": entry.program().display().to_string(),
                    "sha256": entry.receipt.sha256,
                    "installed_at": entry.receipt.installed_at,
                    "signature": entry.receipt.signature.is_some(),
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "packs": rows }));
        return Ok(0);
    }
    if installed.is_empty() {
        println!("no runtime packs installed");
        println!(
            "install one with: dshr runtime install node@<version> --from <archive> --sha256 <hex>"
        );
        return Ok(0);
    }
    println!("{:<8} {:<12} {:<8} PROGRAM", "ENGINE", "VERSION", "ACTIVE");
    for entry in &installed {
        println!(
            "{:<8} {:<12} {:<8} {}",
            entry.receipt.engine,
            entry.receipt.version,
            if entry.current { "yes" } else { "no" },
            entry.program().display()
        );
    }
    Ok(0)
}

fn print_installed(installed: &Installed) {
    println!(
        "installed {}@{} ({}{})",
        installed.receipt.engine,
        installed.receipt.version,
        installed.receipt.sha256,
        if installed.current { ", active" } else { "" }
    );
    println!("  program: {}", installed.program().display());
    if let Some(signature) = &installed.receipt.signature {
        println!("  signature: verified with {}", signature.public_key);
    }
}

fn split_spec(spec: &str) -> Result<(&str, &str)> {
    let (engine, version) = spec.split_once('@').ok_or_else(|| {
        DshrError::invalid_input(format!("runtime: '{spec}' must be <engine>@<version>"))
    })?;
    if engine.is_empty() || version.is_empty() {
        return Err(DshrError::invalid_input(format!(
            "runtime: '{spec}' must be <engine>@<version>"
        )));
    }
    Ok((engine, version))
}

/// Local archives are used as-is; `http(s)` URLs are downloaded once into
/// `<home>/cache/runtime` so an offline reinstall can reuse them.
async fn fetch_archive(home: &Path, source: &str) -> Result<PathBuf> {
    if !source.starts_with("http://") && !source.starts_with("https://") {
        return Ok(PathBuf::from(source));
    }
    let name = source
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("runtime-pack.tar.gz");
    let dir = home.join("cache").join("runtime");
    std::fs::create_dir_all(&dir)?;
    let target = dir.join(name);
    if target.is_file() {
        println!("using cached {}", target.display());
        return Ok(target);
    }
    let response = reqwest::get(source)
        .await
        .map_err(|error| DshrError::invalid_input(format!("runtime: download failed: {error}")))?;
    if !response.status().is_success() {
        return Err(DshrError::invalid_input(format!(
            "runtime: download failed with HTTP {}",
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| DshrError::invalid_input(format!("runtime: download failed: {error}")))?;
    let staging = dir.join(format!(".{name}.part"));
    let mut file = std::fs::File::create(&staging)?;
    file.write_all(&bytes)?;
    drop(file);
    std::fs::rename(&staging, &target)?;
    Ok(target)
}
