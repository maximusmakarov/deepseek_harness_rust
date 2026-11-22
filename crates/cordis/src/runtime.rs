//! Runtime packs: pinned, hash-verified engines for plugin sidecars.
//!
//! A pack is an extracted archive plus a receipt, laid out as
//!
//! ```text
//! <home>/runtimes/<engine>/<version>/receipt.json
//! <home>/runtimes/<engine>/<version>/bin/node
//! <home>/runtimes/<engine>/current          # active version, plain text
//! ```
//!
//! Installing is fail-closed: the archive's sha256 must match the pinned value,
//! an optional Ed25519 signature must verify over the archive bytes, and
//! extraction refuses absolute paths, `..`, symlinks that leave the pack and
//! special files. Only `.tar.gz` archives are supported, which covers the Node
//! distributions used for Cordis sidecars on macOS and Linux.

use std::cmp::Ordering;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dshr_core::{DshrError, Result};
use serde::{Deserialize, Serialize};

/// Node.js release the sidecar provisions when no `DSHR_NODE` and no installed
/// pack exist. Matches the official desktop's bundled major (24.x), whose
/// `zstdDecompressSync` the session-superagent plugin needs.
pub const NODE_PIN_VERSION: &str = "24.21.0";

/// One downloadable Node distribution: URL, pinned sha256 and executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePackSpec {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub bin: String,
}

/// Pinned Node distribution for a platform, or `None` when unsupported (Windows
/// ships zip archives, so it keeps using a system `node`).
#[must_use]
pub fn node_pack_spec(os: &str, arch: &str) -> Option<NodePackSpec> {
    let (platform, sha256) = match (os, arch) {
        ("macos", "aarch64") => (
            "darwin-arm64",
            "bed7eea5325e1108f32ce5228ddd6a5f0f08a499ee42aa7442aea583702f6057",
        ),
        ("macos", "x86_64") => (
            "darwin-x64",
            "1462cb3b3046b815cf8ea436d3da450ec1a9f11dac7e5a46b0ada5305d7e8097",
        ),
        ("linux", "aarch64") => (
            "linux-arm64",
            "724282c3b43aec998aa9527380465b45d229e021b58035f5f4f63095eabfe5d5",
        ),
        ("linux", "x86_64") => (
            "linux-x64",
            "6e1db87ef58b8819e5d5402eff1536491b18edd8eb7bee5ef7897876e88dc5ff",
        ),
        _ => return None,
    };
    Some(NodePackSpec {
        version: NODE_PIN_VERSION.to_owned(),
        url: format!(
            "https://nodejs.org/dist/v{NODE_PIN_VERSION}/node-v{NODE_PIN_VERSION}-{platform}.tar.gz"
        ),
        sha256: sha256.to_owned(),
        bin: DEFAULT_BIN.to_owned(),
    })
}

/// Default executable inside a pack, relative to the pack root.
pub const DEFAULT_BIN: &str = "bin/node";

/// Engine name used by the Cordis sidecar when no `DSHR_NODE` override is set.
pub const NODE_ENGINE: &str = "node";

const RECEIPT: &str = "receipt.json";
const CURRENT: &str = "current";
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Signature recorded in the receipt of a signed pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureRecord {
    /// Public key (32-byte hex) the archive was verified against.
    pub public_key: String,
    /// Where the signature came from, for the audit trail.
    pub source: String,
}

/// Metadata stored next to an installed pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub engine: String,
    pub version: String,
    /// sha256 of the archive the pack was installed from.
    pub sha256: String,
    /// Executable path inside the pack.
    pub bin: String,
    /// Archive name or URL the pack came from.
    pub archive: String,
    /// Archive size in bytes.
    pub bytes: u64,
    pub installed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureRecord>,
}

/// One installed pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub receipt: Receipt,
    pub dir: PathBuf,
    /// Whether this version is the active one for its engine.
    pub current: bool,
}

impl Installed {
    /// Absolute path of the pack's executable.
    #[must_use]
    pub fn program(&self) -> PathBuf {
        self.dir.join(&self.receipt.bin)
    }
}

/// What to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    pub engine: String,
    pub version: String,
    /// Local `.tar.gz` archive.
    pub archive: PathBuf,
    /// Pinned sha256 (64 hex characters).
    pub sha256: String,
    /// Executable inside the archive, relative to the pack root.
    pub bin: String,
    /// Leading path components to drop while unpacking.
    pub strip: usize,
    /// Optional `(public key hex, signature hex)` over the archive bytes.
    pub signature: Option<(String, String)>,
    /// Replace an already installed version.
    pub force: bool,
}

impl InstallRequest {
    #[must_use]
    pub fn new(
        engine: impl Into<String>,
        version: impl Into<String>,
        archive: impl Into<PathBuf>,
        sha256: impl Into<String>,
    ) -> Self {
        Self {
            engine: engine.into(),
            version: version.into(),
            archive: archive.into(),
            sha256: sha256.into(),
            bin: DEFAULT_BIN.to_owned(),
            strip: 1,
            signature: None,
            force: false,
        }
    }
}

/// Directory holding every installed engine pack.
#[must_use]
pub fn root(home: &Path) -> PathBuf {
    home.join("runtimes")
}

fn engine_dir(home: &Path, engine: &str) -> PathBuf {
    root(home).join(engine)
}

fn version_dir(home: &Path, engine: &str, version: &str) -> PathBuf {
    engine_dir(home, engine).join(version)
}

/// Lists every installed pack, ordered by engine and version.
pub fn list(home: &Path) -> Result<Vec<Installed>> {
    let mut out = Vec::new();
    let base = root(home);
    let engines = match fs::read_dir(&base) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(error) => return Err(error.into()),
    };
    let mut names: Vec<String> = engines
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    for engine in names {
        let active = current_version(home, &engine)?;
        let mut versions = list_versions(home, &engine)?;
        versions.sort_by(|left, right| version_cmp(left, right));
        for version in versions {
            if let Ok(receipt) = read_receipt(home, &engine, &version) {
                out.push(Installed {
                    current: active.as_deref() == Some(version.as_str()),
                    receipt,
                    dir: version_dir(home, &engine, &version),
                });
            }
        }
    }
    Ok(out)
}

/// Returns the active pack of `engine`, if any.
pub fn current(home: &Path, engine: &str) -> Result<Option<Installed>> {
    check_token("engine", engine)?;
    let Some(version) = current_version(home, engine)? else {
        return Ok(None);
    };
    match read_receipt(home, engine, &version) {
        Ok(receipt) => Ok(Some(Installed {
            dir: version_dir(home, engine, &version),
            receipt,
            current: true,
        })),
        Err(error) => Err(DshrError::config(format!(
            "runtime pack {engine}@{version} is marked active but unreadable: {error}"
        ))),
    }
}

/// Installs a pack and makes it active when the engine has no active version.
pub fn install(home: &Path, request: InstallRequest) -> Result<Installed> {
    check_token("engine", &request.engine)?;
    check_token("version", &request.version)?;
    let bin = relative_path(&request.bin)?;
    if request.strip > 1 {
        return Err(DshrError::invalid_input(
            "runtime pack: --strip must be 0 or 1".to_owned(),
        ));
    }
    let expected = request.sha256.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(DshrError::invalid_input(
            "runtime pack: --sha256 must be a 64-character hex digest".to_owned(),
        ));
    }
    let metadata = fs::metadata(&request.archive)?;
    if !metadata.is_file() {
        return Err(DshrError::invalid_input(format!(
            "runtime pack: {} is not a file",
            request.archive.display()
        )));
    }
    if metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(DshrError::invalid_input(format!(
            "runtime pack: archive is {} bytes, the limit is {MAX_ARCHIVE_BYTES}",
            metadata.len()
        )));
    }
    let (digest, bytes) = sha256_file(&request.archive)?;
    if digest != expected {
        return Err(DshrError::invalid_input(format!(
            "runtime pack: sha256 mismatch: expected {expected}, got {digest}"
        )));
    }
    let mut signature_record = None;
    if let Some((public_key, signature)) = &request.signature {
        let mut body = Vec::new();
        File::open(&request.archive)?.read_to_end(&mut body)?;
        dshr_sign::verify_bytes(public_key, &body, signature.trim()).map_err(|error| {
            DshrError::invalid_input(format!("runtime pack: signature does not verify: {error}"))
        })?;
        signature_record = Some(SignatureRecord {
            public_key: public_key.clone(),
            source: request.archive.display().to_string(),
        });
    }

    let dest = version_dir(home, &request.engine, &request.version);
    if dest.exists() && !request.force {
        return Err(DshrError::invalid_input(format!(
            "runtime pack: {}@{} is already installed (pass --force to replace it)",
            request.engine, request.version
        )));
    }
    let engine = engine_dir(home, &request.engine);
    fs::create_dir_all(&engine)?;
    let staging = engine.join(format!(".tmp-{}-{}", std::process::id(), nanos()));
    fs::create_dir(&staging)?;
    let mut guard = Cleanup::new(staging.clone());
    extract(&request.archive, &staging, request.strip)?;
    let program = staging.join(&bin);
    if !program.is_file() {
        return Err(DshrError::invalid_input(format!(
            "runtime pack: archive has no executable at {}",
            bin.display()
        )));
    }
    let receipt = Receipt {
        engine: request.engine.clone(),
        version: request.version.clone(),
        sha256: digest,
        bin: request.bin.clone(),
        archive: request.archive.display().to_string(),
        bytes,
        installed_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        signature: signature_record,
    };
    let encoded = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| DshrError::config(format!("runtime pack: receipt: {error}")))?;
    fs::write(staging.join(RECEIPT), encoded)?;
    // Replace atomically: keep the old pack until the new one is in place, and
    // put it back if the final rename fails.
    let backup = if dest.exists() {
        let backup = engine.join(format!(".old-{}-{}", std::process::id(), nanos()));
        fs::rename(&dest, &backup)?;
        Some(backup)
    } else {
        None
    };
    if let Err(error) = fs::rename(&staging, &dest) {
        if let Some(backup) = &backup {
            let _ = fs::rename(backup, &dest);
        }
        return Err(error.into());
    }
    guard.disarm();
    if let Some(backup) = backup {
        let _ = fs::remove_dir_all(backup);
    }
    if current_version(home, &request.engine)?.is_none() {
        write_current(home, &request.engine, &request.version)?;
    }
    let current = current_version(home, &request.engine)?;
    Ok(Installed {
        current: current.as_deref() == Some(request.version.as_str()),
        receipt,
        dir: dest,
    })
}

/// Removes one version; returns `false` when it was not installed.
pub fn remove(home: &Path, engine: &str, version: &str) -> Result<bool> {
    check_token("engine", engine)?;
    check_token("version", version)?;
    let dir = version_dir(home, engine, version);
    if !dir.is_dir() {
        return Ok(false);
    }
    let was_current = current_version(home, engine)?.as_deref() == Some(version);
    fs::remove_dir_all(&dir)?;
    if was_current {
        let mut remaining = list_versions(home, engine)?;
        remaining.sort_by(|left, right| version_cmp(left, right));
        match remaining.last() {
            Some(next) => write_current(home, engine, next)?,
            None => {
                let _ = fs::remove_file(engine_dir(home, engine).join(CURRENT));
            }
        }
    }
    Ok(true)
}

/// Makes an installed version the active one.
pub fn activate(home: &Path, engine: &str, version: &str) -> Result<Installed> {
    check_token("engine", engine)?;
    check_token("version", version)?;
    let receipt = read_receipt(home, engine, version).map_err(|error| {
        DshrError::not_found(format!(
            "runtime pack {engine}@{version} is not installed: {error}"
        ))
    })?;
    write_current(home, engine, version)?;
    Ok(Installed {
        receipt,
        dir: version_dir(home, engine, version),
        current: true,
    })
}

/// Absolute executable path of the active pack of `engine`.
#[must_use]
pub fn current_program(home: &Path, engine: &str) -> Option<PathBuf> {
    let version = current_version(home, engine).ok()??;
    let receipt = read_receipt(home, engine, &version).ok()?;
    let program = version_dir(home, engine, &version).join(&receipt.bin);
    program.is_file().then_some(program)
}

/// Program the Cordis sidecar runs: `DSHR_NODE`, the `node` pack, or `node`.
#[must_use]
pub fn node_program(home: &Path) -> String {
    if let Ok(value) = std::env::var("DSHR_NODE")
        && !value.trim().is_empty()
    {
        return value;
    }
    match ensure_node(home) {
        Some(program) => program.display().to_string(),
        None => NODE_ENGINE.to_owned(),
    }
}

/// Resolves the Node executable for the Cordis sidecar: `DSHR_NODE`, then an
/// installed runtime pack, then a pinned pack downloaded once (the official
/// desktop bundles Node, so plugins must not depend on the user's toolchain).
/// Set `DSHR_RUNTIME_AUTO=0` to keep the old PATH-only behaviour; a failed
/// download falls back to `node` from PATH with a warning.
#[must_use]
pub fn ensure_node(home: &Path) -> Option<PathBuf> {
    if let Some(program) = current_program(home, NODE_ENGINE) {
        return Some(program);
    }
    let auto = std::env::var("DSHR_RUNTIME_AUTO")
        .map(|value| !(value == "0" || value.eq_ignore_ascii_case("false")))
        .unwrap_or(true);
    if !auto {
        return None;
    }
    let spec = node_pack_spec(std::env::consts::OS, std::env::consts::ARCH)?;
    match download_pack(home, &spec) {
        Ok(installed) => {
            eprintln!(
                "dshr: installed node {} runtime pack at {}",
                spec.version,
                installed.dir.display()
            );
            current_program(home, NODE_ENGINE)
        }
        Err(error) => {
            eprintln!("dshr: node runtime pack unavailable ({error}); using node from PATH");
            None
        }
    }
}

/// Downloads a pack to a temporary file, verifies its pinned sha256 by
/// installing it, and removes the download in every outcome.
fn download_pack(home: &Path, spec: &NodePackSpec) -> Result<Installed> {
    let dir = home.join("runtimes");
    fs::create_dir_all(&dir)?;
    let archive = dir.join(format!(
        ".node-download-{}-{}.tar.gz",
        std::process::id(),
        nanos()
    ));
    let download = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "600", "-o"])
        .arg(&archive)
        .arg(&spec.url)
        .output();
    let outcome = match download {
        Ok(output) if output.status.success() => {
            let request = InstallRequest::new(
                NODE_ENGINE,
                spec.version.clone(),
                &archive,
                spec.sha256.clone(),
            );
            install(home, request)
        }
        Ok(output) => Err(DshrError::invalid_input(format!(
            "node pack download failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
        Err(error) => Err(DshrError::invalid_input(format!(
            "node pack download failed: {error}"
        ))),
    };
    let _ = fs::remove_file(&archive);
    outcome
}

fn current_version(home: &Path, engine: &str) -> Result<Option<String>> {
    let path = engine_dir(home, engine).join(CURRENT);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let version = text.trim();
            Ok((!version.is_empty()).then(|| version.to_owned()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_current(home: &Path, engine: &str, version: &str) -> Result<()> {
    let dir = engine_dir(home, engine);
    fs::create_dir_all(&dir)?;
    let staging = dir.join(format!(".current-{}-{}", std::process::id(), nanos()));
    fs::write(&staging, format!("{version}\n"))?;
    let target = dir.join(CURRENT);
    if fs::rename(&staging, &target).is_err() {
        // Windows cannot replace a file by rename; the target is a one-line
        // pointer, so recreating it is safe.
        let _ = fs::remove_file(&target);
        fs::rename(&staging, &target)?;
    }
    Ok(())
}

fn read_receipt(home: &Path, engine: &str, version: &str) -> Result<Receipt> {
    let path = version_dir(home, engine, version).join(RECEIPT);
    let body = fs::read_to_string(&path)?;
    serde_json::from_str(&body)
        .map_err(|error| DshrError::config(format!("runtime pack: {}: {error}", path.display())))
}

fn list_versions(home: &Path, engine: &str) -> Result<Vec<String>> {
    let dir = engine_dir(home, engine);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect())
}

fn check_token(kind: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('.')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(DshrError::invalid_input(format!(
            "runtime pack: {kind} '{value}' must be 1-64 characters of [A-Za-z0-9._-] and must not start with '.'"
        )))
    }
}

fn relative_path(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => {
                return Err(DshrError::invalid_input(format!(
                    "runtime pack: bin path '{value}' must stay inside the pack"
                )));
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(DshrError::invalid_input(
            "runtime pack: bin path must not be empty".to_owned(),
        ));
    }
    Ok(out)
}

fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        context.update(&buffer[..read]);
    }
    Ok((dshr_sign::hex_encode(context.finish().as_ref()), total))
}

fn version_cmp(left: &str, right: &str) -> Ordering {
    let mut left_parts = left.split(['.', '-', '_']);
    let mut right_parts = right.split(['.', '-', '_']);
    loop {
        match (left_parts.next(), right_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left), Some(right)) => {
                let ordering = match (left.parse::<u64>(), right.parse::<u64>()) {
                    (Ok(left), Ok(right)) => left.cmp(&right),
                    _ => left.cmp(right),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

/// Removes a staging directory unless the install completed.
struct Cleanup {
    path: PathBuf,
    armed: bool,
}

impl Cleanup {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Drops the leading `strip` components and rejects unsafe paths.
fn strip_root(path: &Path, strip: usize) -> Result<Option<PathBuf>> {
    let components: Vec<Component<'_>> = path.components().collect();
    for component in &components {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(DshrError::invalid_input(format!(
                "runtime pack: refusing unsafe path '{}'",
                path.display()
            )));
        }
    }
    if components.len() <= strip {
        return Ok(None);
    }
    let mut out = PathBuf::new();
    for component in components.iter().skip(strip) {
        if let Component::Normal(part) = component {
            out.push(part);
        }
    }
    Ok((!out.as_os_str().is_empty()).then_some(out))
}

fn ensure_directory(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(DshrError::invalid_input(format!(
                "runtime pack: refusing unsafe directory '{}'",
                relative.display()
            )));
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(DshrError::invalid_input(format!(
                    "runtime pack: '{}' is not a plain directory",
                    current.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

/// A symlink may point anywhere inside the pack: its target is resolved from
/// the link's own directory, so `../lib/x` from `bin/npm` is fine while
/// `../../etc/passwd` is not.
fn link_target_is_safe(relative: &Path, link: &Path) -> bool {
    if link.is_absolute() {
        return false;
    }
    let mut depth = relative
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter(|component| matches!(component, Component::Normal(_)))
                .count() as i64
        })
        .unwrap_or(0);
    for component in link.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::CurDir => {}
            _ => return false,
        }
    }
    true
}

fn extract(archive: &Path, dest: &Path, strip: usize) -> Result<()> {
    let file = File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut tar = tar::Archive::new(decoder);
    let mut unpacked = 0u64;
    let entries = tar.entries().map_err(|error| {
        DshrError::invalid_input(format!("runtime pack: cannot read archive: {error}"))
    })?;
    for entry in entries {
        let mut entry = entry.map_err(|error| {
            DshrError::invalid_input(format!("runtime pack: bad archive entry: {error}"))
        })?;
        let path = entry
            .path()
            .map_err(|error| {
                DshrError::invalid_input(format!("runtime pack: bad entry path: {error}"))
            })?
            .into_owned();
        let Some(relative) = strip_root(&path, strip)? else {
            continue;
        };
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            ensure_directory(dest, &relative)?;
            continue;
        }
        if kind.is_symlink() {
            let link = entry
                .link_name()
                .map_err(|error| {
                    DshrError::invalid_input(format!("runtime pack: bad symlink: {error}"))
                })?
                .map(|link| link.into_owned())
                .ok_or_else(|| {
                    DshrError::invalid_input(format!(
                        "runtime pack: symlink '{}' has no target",
                        relative.display()
                    ))
                })?;
            if !link_target_is_safe(&relative, &link) {
                return Err(DshrError::invalid_input(format!(
                    "runtime pack: symlink '{}' -> '{}' leaves the pack",
                    relative.display(),
                    link.display()
                )));
            }
            create_symlink(dest, &relative, &link)?;
            continue;
        }
        if !kind.is_file() {
            return Err(DshrError::invalid_input(format!(
                "runtime pack: unsupported entry type for '{}'",
                relative.display()
            )));
        }
        unpacked = unpacked.saturating_add(entry.size());
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(DshrError::invalid_input(format!(
                "runtime pack: unpacked size exceeds {MAX_UNPACKED_BYTES} bytes"
            )));
        }
        let target = ensure_directory(dest, relative.parent().unwrap_or_else(|| Path::new("")))?
            .join(relative.file_name().ok_or_else(|| {
                DshrError::invalid_input(format!(
                    "runtime pack: entry '{}' has no file name",
                    relative.display()
                ))
            })?);
        let mode = entry.header().mode().unwrap_or(0o644);
        // Duplicate entries are legal in tar; overwrite rather than fail.
        let mut handle = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(handle) => handle,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                fs::remove_file(&target)?;
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)?
            }
            Err(error) => return Err(error.into()),
        };
        io::copy(&mut entry, &mut handle)?;
        drop(handle);
        set_mode(&target, mode)?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(root: &Path, relative: &Path, link: &Path) -> Result<()> {
    let target = ensure_directory(root, relative.parent().unwrap_or_else(|| Path::new("")))?.join(
        relative.file_name().ok_or_else(|| {
            DshrError::invalid_input("runtime pack: symlink has no name".to_owned())
        })?,
    );
    if fs::symlink_metadata(&target).is_ok() {
        let _ = fs::remove_file(&target);
    }
    std::os::unix::fs::symlink(link, &target)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_root: &Path, relative: &Path, _link: &Path) -> Result<()> {
    Err(DshrError::invalid_input(format!(
        "runtime pack: symlinks are not supported on this platform ('{}')",
        relative.display()
    )))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if mode & 0o111 != 0 { 0o755 } else { 0o644 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    fn sha256_of(path: &Path) -> String {
        sha256_file(path).expect("hash").0
    }

    fn pack_archive(dir: &Path, root: &str) -> PathBuf {
        pack_archive_with(dir, root, false)
    }

    #[cfg(unix)]
    fn pack_archive_symlinked(dir: &Path, root: &str) -> PathBuf {
        pack_archive_with(dir, root, true)
    }

    fn pack_archive_with(dir: &Path, root: &str, symlink: bool) -> PathBuf {
        let path = dir.join(format!(
            "{root}{}.tar.gz",
            if symlink { "-symlinked" } else { "" }
        ));
        let file = File::create(&path).expect("create");
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);

        let script = b"#!/bin/sh\necho node\n";
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(script.len() as u64);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("{root}/bin/node"), &script[..])
            .expect("append node");

        let lib = b"module.exports = {};\n";
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(lib.len() as u64);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("{root}/lib/npm-cli.js"), &lib[..])
            .expect("append lib");

        if symlink {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header
                .set_link_name("../lib/npm-cli.js")
                .expect("link name");
            header.set_cksum();
            builder
                .append_data(&mut header, format!("{root}/bin/npm"), io::empty())
                .expect("append symlink");
        }

        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");
        path
    }

    /// `file://` URL for a local path. `Path::display` yields backslashes on
    /// Windows, which curl cannot parse as a URL.
    fn local_url(path: &Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/");
        if text.starts_with('/') {
            format!("file://{text}")
        } else {
            format!("file:///{text}")
        }
    }

    fn request(home: &TempDir, archive: &Path, version: &str) -> InstallRequest {
        let _ = home;
        InstallRequest::new("node", version, archive, sha256_of(archive))
    }

    #[test]
    fn install_unpacks_pack_and_activates_it() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        let installed = install(home.path(), request(&home, &archive, "22.11.0")).expect("install");

        assert_eq!(installed.receipt.engine, "node");
        assert_eq!(installed.receipt.version, "22.11.0");
        assert_eq!(installed.receipt.bin, DEFAULT_BIN);
        assert_eq!(installed.receipt.sha256, sha256_of(&archive));
        assert!(installed.current, "the first version becomes active");
        assert!(installed.program().is_file());
        assert_eq!(
            fs::read_to_string(engine_dir(home.path(), "node").join(CURRENT)).expect("current"),
            "22.11.0\n"
        );
        assert_eq!(
            current_program(home.path(), "node"),
            Some(installed.program())
        );
        assert!(current(home.path(), "node").expect("current").is_some());
        assert_eq!(list(home.path()).expect("list").len(), 1);
    }

    // Symlink entries are rejected off unix, so the pack builder can only
    // exercise link reproduction on unix hosts. Keep the gated body thin.
    #[cfg(unix)]
    #[test]
    fn install_reproduces_archive_symlinks() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive_symlinked(work.path(), "node-v22.11.0");
        let installed = install(home.path(), request(&home, &archive, "22.11.0")).expect("install");
        let npm = installed.dir.join("bin/npm");
        assert!(npm.is_symlink(), "archive symlink is reproduced");
        assert_eq!(
            fs::read_to_string(&npm).expect("read through the symlink"),
            "module.exports = {};\n"
        );
    }

    #[test]
    fn pinned_node_covers_desktop_platforms() {
        for (os, arch) in [
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("linux", "aarch64"),
            ("linux", "x86_64"),
        ] {
            let spec = node_pack_spec(os, arch).expect("supported");
            assert_eq!(spec.version, NODE_PIN_VERSION);
            assert_eq!(spec.sha256.len(), 64);
            assert!(spec.url.ends_with(".tar.gz"));
            assert_eq!(spec.bin, DEFAULT_BIN);
        }
        assert!(node_pack_spec("windows", "x86_64").is_none());
        assert!(node_pack_spec("freebsd", "x86_64").is_none());
    }

    #[test]
    fn download_pack_installs_a_verified_archive_from_a_local_url() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v24.21.0");
        let sha = sha256_of(&archive);
        let spec = NodePackSpec {
            version: "24.21.0".to_owned(),
            url: local_url(&archive),
            sha256: sha,
            bin: DEFAULT_BIN.to_owned(),
        };
        let installed = download_pack(home.path(), &spec).expect("installed");
        assert!(installed.program().is_file());
        assert_eq!(
            current_version(home.path(), NODE_ENGINE)
                .expect("version")
                .as_deref(),
            Some("24.21.0")
        );
        assert!(
            !home
                .path()
                .join("runtimes")
                .read_dir()
                .expect("dir")
                .any(|entry| entry
                    .map(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".node-download-"))
                    .unwrap_or(false)),
            "temporary download must be removed"
        );
    }

    #[test]
    fn download_pack_rejects_a_tampered_archive() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v24.21.0");
        let spec = NodePackSpec {
            version: "24.21.0".to_owned(),
            url: local_url(&archive),
            sha256: "0".repeat(64),
            bin: DEFAULT_BIN.to_owned(),
        };
        assert!(download_pack(home.path(), &spec).is_err());
        assert!(
            current_version(home.path(), NODE_ENGINE)
                .expect("version")
                .is_none()
        );
    }

    #[test]
    fn sha256_mismatch_is_rejected_and_leaves_nothing_behind() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        let mut spec = InstallRequest::new("node", "22.11.0", &archive, "0".repeat(64));
        spec.force = false;
        let error = install(home.path(), spec).expect_err("must fail");
        assert!(
            error.to_string().contains("sha256 mismatch"),
            "unexpected error: {error}"
        );
        assert!(!version_dir(home.path(), "node", "22.11.0").exists());
        let leftovers: Vec<_> = fs::read_dir(engine_dir(home.path(), "node"))
            .map(|entries| entries.filter_map(|entry| entry.ok()).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "staging directories are cleaned up");
    }

    #[test]
    fn signature_is_verified_when_given() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        let (public, secret) = dshr_sign::keygen().expect("keygen");
        let bytes = fs::read(&archive).expect("read archive");
        let signature = dshr_sign::sign_bytes(&secret, &bytes).expect("sign");

        let mut spec = request(&home, &archive, "22.11.0");
        spec.signature = Some((public.clone(), signature.clone()));
        let installed = install(home.path(), spec).expect("install");
        let record = installed.receipt.signature.expect("signature recorded");
        assert_eq!(record.public_key, public);

        let mut forged = request(&home, &archive, "22.11.1");
        forged.signature = Some((public, "0".repeat(128)));
        let error = install(home.path(), forged).expect_err("must fail");
        assert!(
            error.to_string().contains("signature does not verify"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn existing_version_needs_force() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        install(home.path(), request(&home, &archive, "22.11.0")).expect("first install");
        let error =
            install(home.path(), request(&home, &archive, "22.11.0")).expect_err("duplicate");
        assert!(
            error.to_string().contains("already installed"),
            "unexpected error: {error}"
        );
        let mut forced = request(&home, &archive, "22.11.0");
        forced.force = true;
        install(home.path(), forced).expect("forced install");
        assert_eq!(list(home.path()).expect("list").len(), 1);
    }

    #[test]
    fn failed_force_install_keeps_the_previous_version() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        install(home.path(), request(&home, &archive, "22.11.0")).expect("install");

        let evil = work.path().join("evil.tar.gz");
        let file = File::create(&evil).expect("create");
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_link_name("../../etc/passwd").expect("link");
        header.set_cksum();
        builder
            .append_data(&mut header, "node/bin/evil", io::empty())
            .expect("append");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");

        let mut forced = request(&home, &evil, "22.11.0");
        forced.force = true;
        install(home.path(), forced).expect_err("must fail");

        let installed = current(home.path(), "node")
            .expect("current")
            .expect("some");
        assert_eq!(installed.receipt.version, "22.11.0");
        assert!(installed.program().is_file(), "previous pack still works");
    }

    #[test]
    fn remove_falls_back_to_the_remaining_version() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let old = pack_archive(work.path(), "node-v20.1.0");
        let new = pack_archive(work.path(), "node-v22.11.0");
        install(home.path(), request(&home, &old, "20.1.0")).expect("install old");
        install(home.path(), request(&home, &new, "22.11.0")).expect("install new");
        assert_eq!(
            current(home.path(), "node")
                .expect("current")
                .expect("some")
                .receipt
                .version,
            "20.1.0",
            "a second install does not steal the active slot"
        );
        activate(home.path(), "node", "22.11.0").expect("activate");
        assert_eq!(
            current(home.path(), "node")
                .expect("current")
                .expect("some")
                .receipt
                .version,
            "22.11.0"
        );
        assert!(remove(home.path(), "node", "22.11.0").expect("remove"));
        assert!(!remove(home.path(), "node", "22.11.0").expect("second remove"));
        assert_eq!(
            current(home.path(), "node")
                .expect("current")
                .expect("some")
                .receipt
                .version,
            "20.1.0"
        );
        activate(home.path(), "node", "20.1.0").expect("activate old");
        remove(home.path(), "node", "20.1.0").expect("remove old");
        assert!(current(home.path(), "node").expect("current").is_none());
        assert!(list(home.path()).expect("list").is_empty());
    }

    #[test]
    fn unsafe_entries_are_refused() {
        assert!(strip_root(Path::new("../evil"), 1).is_err());
        assert!(strip_root(Path::new("/etc/passwd"), 1).is_err());
        assert!(relative_path("../node").is_err());
        assert!(!link_target_is_safe(
            Path::new("bin/npm"),
            Path::new("../../etc/passwd")
        ));
        assert!(!link_target_is_safe(
            Path::new("bin/npm"),
            Path::new("/etc/passwd")
        ));
        assert!(!link_target_is_safe(
            Path::new("npm"),
            Path::new("../lib/npm-cli.js")
        ));
        assert!(link_target_is_safe(
            Path::new("bin/npm"),
            Path::new("../lib/npm-cli.js")
        ));
        assert!(check_token("engine", "../node").is_err());
        assert!(check_token("version", ".hidden").is_err());
    }

    #[test]
    fn escaping_symlink_fails_the_install() {
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = work.path().join("evil.tar.gz");
        let file = File::create(&archive).expect("create");
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header
            .set_link_name("../../../etc/passwd")
            .expect("link name");
        header.set_cksum();
        builder
            .append_data(&mut header, "node/bin/evil", io::empty())
            .expect("append symlink");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");

        let error = install(home.path(), request(&home, &archive, "1.0")).expect_err("must fail");
        assert!(
            error.to_string().contains("leaves the pack"),
            "unexpected error: {error}"
        );
        assert!(!version_dir(home.path(), "node", "1.0").exists());
    }

    #[test]
    fn node_program_prefers_the_pack() {
        if std::env::var_os("DSHR_NODE").is_some() {
            return;
        }
        assert_eq!(node_program(Path::new("/nonexistent-home")), "node");
        let home = TempDir::new().expect("home");
        let work = TempDir::new().expect("work");
        let archive = pack_archive(work.path(), "node-v22.11.0");
        let installed = install(home.path(), request(&home, &archive, "22.11.0")).expect("install");
        assert_eq!(
            node_program(home.path()),
            installed.program().display().to_string()
        );
    }
}
