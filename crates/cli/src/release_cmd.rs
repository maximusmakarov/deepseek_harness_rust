use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use dshr_core::{DshrError, Result};
use dshr_i18n::current;

#[derive(Debug, Args)]
pub struct ReleaseArgs {
    #[command(subcommand)]
    pub action: ReleaseAction,
}

#[derive(Debug, Subcommand)]
pub enum ReleaseAction {
    /// Generate an Ed25519 key pair for release artifacts
    Keygen {
        /// Where to write the secret key (hex). The public key is printed.
        #[arg(long)]
        out: PathBuf,
    },
    /// Sign a release artifact
    Sign {
        /// File to sign
        file: PathBuf,
        /// Secret key (hex); defaults to $DSHR_RELEASE_KEY
        #[arg(long, env = "DSHR_RELEASE_KEY", hide_env_values = true)]
        key: String,
        /// Signature output (default: <file>.sig)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Verify a release artifact signature
    Verify {
        /// Signed file
        file: PathBuf,
        /// Detached signature file
        #[arg(long)]
        signature: PathBuf,
        /// Public key (hex)
        #[arg(long)]
        key: String,
    },
    /// Print the public key for a secret key
    PublicKey {
        /// Secret key (hex); defaults to $DSHR_RELEASE_KEY
        #[arg(long, env = "DSHR_RELEASE_KEY", hide_env_values = true)]
        key: String,
    },
}

pub fn execute(args: &ReleaseArgs) -> Result<u8> {
    match &args.action {
        ReleaseAction::Keygen { out } => keygen(out),
        ReleaseAction::Sign { file, key, out } => sign(file, key, out.as_deref()),
        ReleaseAction::Verify {
            file,
            signature,
            key,
        } => verify(file, signature, key),
        ReleaseAction::PublicKey { key } => {
            let public = dshr_sign::public_key(key)
                .map_err(|error| DshrError::invalid_input(error.to_string()))?;
            // Raw hex so scripts can capture it.
            println!("{public}");
            Ok(0)
        }
    }
}

fn keygen(out: &Path) -> Result<u8> {
    let (public, secret) =
        dshr_sign::keygen().map_err(|error| DshrError::invalid_input(error.to_string()))?;
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

fn sign(file: &Path, key: &str, out: Option<&Path>) -> Result<u8> {
    let bytes = std::fs::read(file)?;
    let signature = dshr_sign::sign_bytes(key, &bytes)
        .map_err(|error| DshrError::invalid_input(error.to_string()))?;
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(format!("{}.sig", file.display())));
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, format!("{signature}\n"))?;
    let public =
        dshr_sign::public_key(key).map_err(|error| DshrError::invalid_input(error.to_string()))?;
    println!("{}", current().release_signed(&out.display().to_string()));
    println!("{}", current().plugin_public_key(&public));
    Ok(0)
}

fn verify(file: &Path, signature: &Path, key: &str) -> Result<u8> {
    let bytes = std::fs::read(file)?;
    let signature = std::fs::read_to_string(signature)?;
    dshr_sign::verify_bytes(key, &bytes, signature.trim())
        .map_err(|error| DshrError::invalid_input(error.to_string()))?;
    println!(
        "{}",
        current().release_verified(&file.display().to_string())
    );
    Ok(0)
}
