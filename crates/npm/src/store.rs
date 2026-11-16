//! Content-addressed package store: one verified copy of every tarball, one
//! unpacked directory per package version.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Write};
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use dshr_core::{DshrError, Result};
use ring::digest;

use crate::registry::VersionMeta;

const MAX_TARBALL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

/// `<home>/npm` storage: `cas/<digest>/` for unpacked packages and
/// `cache/tarballs/<digest>.tgz` for the archives themselves.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    #[must_use]
    pub fn for_home(home: &Path) -> Self {
        Self {
            root: home.join("npm"),
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory holding the unpacked contents of one package version.
    #[must_use]
    pub fn package_dir(&self, key: &str) -> PathBuf {
        self.root.join("cas").join(key)
    }

    /// Makes sure the tarball described by `meta` is downloaded, verified and
    /// unpacked; returns the unpacked directory.
    pub async fn ensure(
        &self,
        client: &reqwest::Client,
        name: &str,
        meta: &VersionMeta,
    ) -> Result<PathBuf> {
        let digest = expected_digest(meta)?;
        let key = digest.key();
        let package_dir = self.package_dir(&key);
        // A killed install can leave a half-unpacked CAS entry: only a directory
        // with a package.json counts as a completed copy.
        if package_dir.is_dir() {
            if package_dir.join("package.json").is_file() {
                return Ok(package_dir);
            }
            fs::remove_dir_all(&package_dir).map_err(|error| {
                DshrError::config(format!(
                    "npm: cannot clear the incomplete store entry {}: {error}",
                    package_dir.display()
                ))
            })?;
        }
        let tarball = self
            .root
            .join("cache")
            .join("tarballs")
            .join(format!("{key}.tgz"));
        if !tarball.is_file() {
            let bytes = download(client, meta, name).await?;
            digest.verify(&bytes, name, &meta.version.to_string())?;
            if let Some(parent) = tarball.parent() {
                fs::create_dir_all(parent)?;
            }
            let staging = tarball.with_extension("tgz.part");
            let mut file = File::create(&staging)?;
            file.write_all(&bytes)?;
            drop(file);
            fs::rename(&staging, &tarball)?;
        }
        let bytes = fs::read(&tarball)?;
        digest.verify(&bytes, name, &meta.version.to_string())?;

        let cas = self.root.join("cas");
        fs::create_dir_all(&cas)?;
        let staging = cas.join(format!(".tmp-{}-{}", std::process::id(), key));
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir(&staging).map_err(|error| {
            DshrError::config(format!(
                "npm: cannot create the staging directory {}: {error}",
                staging.display()
            ))
        })?;
        let guard = Cleanup(staging.clone());
        let _ = &guard;
        extract(&tarball, &staging)?;
        if !staging.join("package.json").is_file() {
            return Err(DshrError::invalid_input(format!(
                "npm: {name}@{} has no package.json",
                meta.version
            )));
        }
        match fs::rename(&staging, &package_dir) {
            Ok(()) => {}
            // Another process won the race; keep its copy.
            Err(_) if package_dir.is_dir() => {}
            Err(error) => return Err(error.into()),
        }
        drop(guard);
        Ok(package_dir)
    }
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

async fn download(client: &reqwest::Client, meta: &VersionMeta, name: &str) -> Result<Vec<u8>> {
    let response = client.get(&meta.tarball).send().await.map_err(|error| {
        DshrError::config(format!(
            "npm: cannot download {}@{}: {error}",
            name, meta.version
        ))
    })?;
    if !response.status().is_success() {
        return Err(DshrError::config(format!(
            "npm: {}@{} tarball returned HTTP {}",
            name,
            meta.version,
            response.status()
        )));
    }
    if let Some(length) = response.content_length()
        && length > MAX_TARBALL_BYTES
    {
        return Err(DshrError::invalid_input(format!(
            "npm: {name}@{} tarball is {length} bytes, the limit is {MAX_TARBALL_BYTES}",
            meta.version
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| DshrError::config(format!("npm: download failed: {error}")))?;
    if bytes.len() as u64 > MAX_TARBALL_BYTES {
        return Err(DshrError::invalid_input(format!(
            "npm: {name}@{} tarball exceeds {MAX_TARBALL_BYTES} bytes",
            meta.version
        )));
    }
    Ok(bytes.to_vec())
}

#[derive(Debug, Clone)]
enum Digest {
    Sha512(Vec<u8>),
    Sha1(Vec<u8>),
}

impl Digest {
    fn key(&self) -> String {
        match self {
            Self::Sha512(bytes) => format!("sha512-{}", hex(bytes)),
            Self::Sha1(bytes) => format!("sha1-{}", hex(bytes)),
        }
    }

    fn verify(&self, bytes: &[u8], name: &str, version: &str) -> Result<()> {
        let (algorithm, expected) = match self {
            Self::Sha512(expected) => (&digest::SHA512, expected),
            Self::Sha1(expected) => (&digest::SHA1_FOR_LEGACY_USE_ONLY, expected),
        };
        let actual = digest::digest(algorithm, bytes);
        if actual.as_ref() != expected.as_slice() {
            return Err(DshrError::invalid_input(format!(
                "npm: integrity check failed for {name}@{version} (expected {})",
                self.key()
            )));
        }
        Ok(())
    }
}

fn expected_digest(meta: &VersionMeta) -> Result<Digest> {
    if let Some(integrity) = &meta.integrity {
        for part in integrity.split_whitespace() {
            let value = part.split('?').next().unwrap_or(part);
            if let Some(encoded) = value.strip_prefix("sha512-") {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|error| {
                        DshrError::invalid_input(format!("npm: bad sha512 integrity: {error}"))
                    })?;
                return Ok(Digest::Sha512(bytes));
            }
            if let Some(encoded) = value.strip_prefix("sha1-") {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|error| {
                        DshrError::invalid_input(format!("npm: bad sha1 integrity: {error}"))
                    })?;
                return Ok(Digest::Sha1(bytes));
            }
        }
    }
    if let Some(shasum) = &meta.shasum {
        let bytes = decode_hex(shasum).ok_or_else(|| {
            DshrError::invalid_input(format!(
                "npm: bad shasum '{shasum}' in the registry metadata"
            ))
        })?;
        return Ok(Digest::Sha1(bytes));
    }
    Err(DshrError::invalid_input(
        "npm: registry metadata has no integrity or shasum to verify".to_owned(),
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

/// Unpacks a `.tar.gz` npm tarball (one leading `package/` component) while
/// refusing absolute paths, `..`, optional links that leave the package and
/// special files.
fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut tar = tar::Archive::new(decoder);
    tar.set_preserve_permissions(false);
    let mut unpacked = 0u64;
    let entries = tar
        .entries()
        .map_err(|error| DshrError::invalid_input(format!("npm: cannot read tarball: {error}")))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| DshrError::invalid_input(format!("npm: bad entry: {error}")))?;
        let path = entry
            .path()
            .map_err(|error| DshrError::invalid_input(format!("npm: bad path: {error}")))?
            .into_owned();
        let Some(relative) = strip_first(&path)? else {
            continue;
        };
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            ensure_dir(dest, &relative)?;
            continue;
        }
        if kind.is_symlink() || kind.is_hard_link() {
            let link = entry
                .link_name()
                .map_err(|error| DshrError::invalid_input(format!("npm: bad link: {error}")))?
                .map(|link| link.into_owned())
                .ok_or_else(|| {
                    DshrError::invalid_input(format!(
                        "npm: link '{}' has no target",
                        relative.display()
                    ))
                })?;
            if !link_is_safe(&relative, &link) {
                return Err(DshrError::invalid_input(format!(
                    "npm: link '{}' -> '{}' leaves the package",
                    relative.display(),
                    link.display()
                )));
            }
            create_link(dest, &relative, &link)?;
            continue;
        }
        if !kind.is_file() {
            return Err(DshrError::invalid_input(format!(
                "npm: unsupported entry '{}'",
                relative.display()
            )));
        }
        unpacked = unpacked.saturating_add(entry.size());
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(DshrError::invalid_input(format!(
                "npm: package unpacks to more than {MAX_UNPACKED_BYTES} bytes"
            )));
        }
        let parent = ensure_dir(dest, relative.parent().unwrap_or_else(|| Path::new("")))?;
        let target =
            parent.join(relative.file_name().ok_or_else(|| {
                DshrError::invalid_input("npm: entry has no file name".to_owned())
            })?);
        let mode = entry.header().mode().unwrap_or(0o644);
        // Archives occasionally repeat an entry; tar semantics overwrite, so a
        // duplicate is truncated instead of failing with EEXIST.
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

fn strip_first(path: &Path) -> Result<Option<PathBuf>> {
    let components: Vec<Component<'_>> = path.components().collect();
    for component in &components {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(DshrError::invalid_input(format!(
                "npm: refusing unsafe path '{}'",
                path.display()
            )));
        }
    }
    if components.len() <= 1 {
        return Ok(None);
    }
    let mut out = PathBuf::new();
    for component in &components[1..] {
        if let Component::Normal(part) = component {
            out.push(part);
        }
    }
    Ok((!out.as_os_str().is_empty()).then_some(out))
}

fn ensure_dir(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(DshrError::invalid_input(format!(
                "npm: refusing unsafe directory '{}'",
                relative.display()
            )));
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(DshrError::invalid_input(format!(
                    "npm: '{}' is not a plain directory",
                    current.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

fn link_is_safe(relative: &Path, link: &Path) -> bool {
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

#[cfg(unix)]
fn create_link(root: &Path, relative: &Path, link: &Path) -> Result<()> {
    let parent = ensure_dir(root, relative.parent().unwrap_or_else(|| Path::new("")))?;
    let target = parent.join(
        relative
            .file_name()
            .ok_or_else(|| DshrError::invalid_input("npm: link has no name".to_owned()))?,
    );
    if fs::symlink_metadata(&target).is_ok() {
        let _ = fs::remove_file(&target);
    }
    std::os::unix::fs::symlink(link, &target)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_link(_root: &Path, relative: &Path, _link: &Path) -> Result<()> {
    Err(DshrError::invalid_input(format!(
        "npm: links are not supported on this platform ('{}')",
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
    use super::*;

    #[test]
    fn parses_integrity_strings() {
        let bytes = vec![1u8, 2, 3, 4];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let meta = VersionMeta {
            version: semver::Version::parse("1.0.0").expect("v"),
            tarball: "https://x/y.tgz".to_owned(),
            integrity: Some(format!("sha512-{encoded}")),
            shasum: None,
            dependencies: Default::default(),
            optional_dependencies: Default::default(),
            peer_dependencies: Default::default(),
            peer_optional: Default::default(),
        };
        assert!(matches!(expected_digest(&meta), Ok(Digest::Sha512(_))));

        let meta = VersionMeta {
            integrity: Some("md5-nope".to_owned()),
            shasum: Some("00".repeat(20)),
            ..meta
        };
        assert!(matches!(expected_digest(&meta), Ok(Digest::Sha1(_))));

        let meta = VersionMeta {
            integrity: None,
            shasum: None,
            ..meta
        };
        assert!(expected_digest(&meta).is_err());
    }

    #[test]
    fn rejects_unsafe_paths_and_links() {
        assert!(strip_first(Path::new("/etc/passwd")).is_err());
        assert!(strip_first(Path::new("../x")).is_err());
        assert!(!link_is_safe(Path::new("a/b"), Path::new("../../x")));
        assert!(link_is_safe(Path::new("a/b"), Path::new("../c")));
    }
}
