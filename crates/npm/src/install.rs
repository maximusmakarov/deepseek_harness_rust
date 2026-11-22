//! Dependency resolution and materialisation of a `node_modules` tree.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Component, Path, PathBuf};

use dshr_core::{DshrError, Result};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::registry::{Packument, Registry, VersionMeta};
use crate::store::Store;
use crate::{PackageSpec, Requirement};

/// Name of the generated lock file.
pub const LOCK_FILE: &str = "dshr-npm-lock.json";

/// Safety valve against peer-dependency cycles: a real tree stays far below it.
const MAX_PACKAGES: usize = 1500;
/// Nesting cap; deeper placement means a peer loop, not a real dependency.
const MAX_DEPTH: usize = 64;

/// Knobs for one install run.
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Install `optionalDependencies` when they resolve (best effort).
    pub include_optional: bool,
    /// Read metadata only from the cache; tarballs still come from the store.
    pub offline: bool,
}

/// One package placed in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    /// Path relative to the install target, e.g. `node_modules/a`.
    pub path: String,
    pub resolved: String,
    pub integrity: String,
    pub dependencies: BTreeMap<String, String>,
}

/// Result of an install run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub packages: Vec<InstalledPackage>,
    pub lock_path: PathBuf,
}

impl InstallReport {
    #[must_use]
    pub fn version_of(&self, name: &str) -> Option<&str> {
        self.packages
            .iter()
            .find(|package| package.name == name)
            .map(|package| package.version.as_str())
    }

    #[must_use]
    pub fn path_of(&self, name: &str) -> Option<&str> {
        self.packages
            .iter()
            .find(|package| package.name == name)
            .map(|package| package.path.as_str())
    }
}

/// Resolves and installs packages into a target directory.
#[derive(Debug, Clone)]
pub struct Installer {
    registry: Registry,
    store: Store,
    client: reqwest::Client,
}

impl Installer {
    #[must_use]
    pub fn for_home(home: &Path) -> Self {
        Self {
            registry: Registry::for_home(home),
            store: Store::for_home(home),
            client: client(),
        }
    }

    /// Installer pointed at an explicit registry (tests, mirrors).
    #[must_use]
    pub fn with_registry(base: impl Into<String>, home: &Path) -> Self {
        Self {
            registry: Registry::new(base, home.join("npm").join("registry")),
            store: Store::for_home(home),
            client: client(),
        }
    }

    #[must_use]
    pub fn offline(mut self) -> Self {
        self.registry = self.registry.offline();
        self
    }

    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Installs `specs` and their dependencies under `target/node_modules`.
    pub async fn install(
        &self,
        target: &Path,
        specs: &[PackageSpec],
        options: &InstallOptions,
    ) -> Result<InstallReport> {
        let resolvers = Resolvers {
            registry: &self.registry,
            store: &self.store,
            client: &self.client,
        };
        let tree = build_tree(&resolvers, specs, options).await?;
        std::fs::create_dir_all(target)?;
        let node_modules = target.join("node_modules");
        if node_modules.exists() {
            std::fs::remove_dir_all(&node_modules)?;
        }
        std::fs::create_dir_all(&node_modules)?;

        let mut packages = Vec::new();
        for (slot, entries) in &tree.slots {
            for (name, index) in entries {
                let node = &tree.nodes[*index];
                let destination = target.join(slot).join(name);
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                materialise(&node.dir, &destination)?;
                packages.push(InstalledPackage {
                    name: node.name.clone(),
                    version: node.version.to_string(),
                    path: slot.join(name).display().to_string(),
                    resolved: node.resolved.clone(),
                    integrity: node.integrity.clone(),
                    dependencies: node.dependencies.clone(),
                });
            }
        }
        packages.sort_by(|left, right| left.path.cmp(&right.path));

        let lock = LockFile {
            lockfile_version: 1,
            generated_by: "dshr-npm".to_owned(),
            roots: specs.iter().map(PackageSpec::display).collect(),
            packages: packages
                .iter()
                .map(|package| {
                    (
                        format!("{}@{}", package.name, package.version),
                        LockedPackage {
                            resolved: package.resolved.clone(),
                            integrity: package.integrity.clone(),
                            path: package.path.clone(),
                            dependencies: package.dependencies.clone(),
                        },
                    )
                })
                .collect(),
        };
        let lock_path = target.join(LOCK_FILE);
        let encoded = serde_json::to_vec_pretty(&lock)
            .map_err(|error| DshrError::config(format!("npm: lock file: {error}")))?;
        std::fs::write(&lock_path, encoded)?;
        Ok(InstallReport {
            packages,
            lock_path,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LockFile {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: u32,
    #[serde(rename = "generatedBy")]
    generated_by: String,
    roots: Vec<String>,
    packages: BTreeMap<String, LockedPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LockedPackage {
    resolved: String,
    integrity: String,
    path: String,
    dependencies: BTreeMap<String, String>,
}

struct Node {
    name: String,
    version: Version,
    resolved: String,
    integrity: String,
    dependencies: BTreeMap<String, String>,
    /// Unpacked package directory in the content-addressed store.
    dir: PathBuf,
}

struct Tree {
    nodes: Vec<Node>,
    /// `node_modules` directories (relative to the target) and what they hold.
    slots: BTreeMap<PathBuf, BTreeMap<String, usize>>,
}

struct Resolvers<'a> {
    registry: &'a Registry,
    store: &'a Store,
    client: &'a reqwest::Client,
}

struct Request {
    from_dir: PathBuf,
    name: String,
    requirement: Requirement,
    optional: bool,
    /// Peer dependencies are host-level singletons: they are placed at the
    /// root and never nested.
    peer: bool,
}

async fn build_tree(
    resolvers: &Resolvers<'_>,
    specs: &[PackageSpec],
    options: &InstallOptions,
) -> Result<Tree> {
    let mut tree = Tree {
        nodes: Vec::new(),
        slots: BTreeMap::new(),
    };
    let mut packuments: BTreeMap<String, Packument> = BTreeMap::new();
    let mut peer_seen: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();
    let mut queue: VecDeque<Request> = VecDeque::new();
    for spec in specs {
        queue.push_back(Request {
            from_dir: PathBuf::new(),
            name: spec.name.clone(),
            requirement: spec.requirement(),
            optional: false,
            peer: false,
        });
    }

    while let Some(request) = queue.pop_front() {
        if tree.nodes.len() >= MAX_PACKAGES {
            return Err(DshrError::invalid_input(format!(
                "npm: dependency graph exceeds {MAX_PACKAGES} packages (peer-dependency cycle?)"
            )));
        }
        if request.from_dir.components().count() > MAX_DEPTH {
            return Err(DshrError::invalid_input(format!(
                "npm: {} nests deeper than {MAX_DEPTH} packages (peer-dependency cycle?)",
                request.name
            )));
        }
        let candidates = candidate_slots(&request.from_dir);
        if visible(&tree, &candidates, &request).is_some() {
            continue;
        }
        let (version, meta) = match resolve(resolvers, &mut packuments, &request).await {
            Ok(found) => found,
            Err(error) if request.optional => {
                tracing::warn!(name = %request.name, %error, "npm: optional dependency skipped");
                continue;
            }
            Err(error) => return Err(error),
        };
        let dir = resolvers
            .store
            .ensure(resolvers.client, &request.name, &meta)
            .await?;
        let slot = if request.peer {
            let root = PathBuf::from("node_modules");
            if tree
                .slots
                .get(&root)
                .is_some_and(|entries| entries.contains_key(&request.name))
            {
                tracing::warn!(
                    name = %request.name,
                    "npm: peer dependency conflicts with the hoisted copy; skipped"
                );
                continue;
            }
            root
        } else {
            candidates
                .iter()
                .find(|slot| {
                    !tree
                        .slots
                        .get(*slot)
                        .is_some_and(|entries| entries.contains_key(&request.name))
                })
                .cloned()
                .unwrap_or_else(|| request.from_dir.join("node_modules"))
        };
        let index = tree.nodes.len();
        tree.nodes.push(Node {
            name: request.name.clone(),
            version,
            resolved: meta.tarball.clone(),
            integrity: integrity_label(&meta),
            dependencies: meta.dependencies.clone(),
            dir,
        });
        tree.slots
            .entry(slot.clone())
            .or_default()
            .insert(request.name.clone(), index);
        let from_dir = slot.join(&request.name);
        for (name, requirement) in &meta.dependencies {
            queue.push_back(Request {
                from_dir: from_dir.clone(),
                name: name.clone(),
                requirement: Requirement::parse(requirement),
                optional: false,
                peer: false,
            });
        }
        if options.include_optional {
            for (name, requirement) in &meta.optional_dependencies {
                queue.push_back(Request {
                    from_dir: from_dir.clone(),
                    name: name.clone(),
                    requirement: Requirement::parse(requirement),
                    optional: true,
                    peer: false,
                });
            }
        }
        // npm >= 7 installs peer dependencies as well; peers resolved from a
        // higher level are reused by `visible()`. They stay best effort even
        // when not marked optional in `peerDependenciesMeta`: DSH plugins pin
        // peers to host versions that may be unpublished, and the sidecar
        // provides those services itself.
        for (name, requirement) in &meta.peer_dependencies {
            let requirement = Requirement::parse(requirement);
            if !peer_seen.insert((name.clone(), requirement.key())) {
                continue;
            }
            queue.push_back(Request {
                from_dir: PathBuf::new(),
                name: name.clone(),
                requirement,
                optional: true,
                peer: true,
            });
        }
    }
    Ok(tree)
}

/// `node_modules` directories visible from `from_dir`, shallowest first.
fn candidate_slots(from_dir: &Path) -> Vec<PathBuf> {
    let mut slots = Vec::new();
    let mut dir = from_dir.to_path_buf();
    loop {
        if !dir.as_os_str().is_empty() {
            slots.push(dir.join("node_modules"));
        }
        match strip_package_dir(&dir) {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    slots.push(PathBuf::from("node_modules"));
    slots.reverse();
    slots.dedup();
    slots
}

/// Drops a trailing `node_modules/<name>` pair.
fn strip_package_dir(dir: &Path) -> Option<PathBuf> {
    let mut components: Vec<Component<'_>> = dir.components().collect();
    if components.len() < 2 {
        return None;
    }
    let name = components.pop()?;
    let marker = components.pop()?;
    if marker.as_os_str() != "node_modules" || !matches!(name, Component::Normal(_)) {
        return None;
    }
    let mut out = PathBuf::new();
    for component in components {
        out.push(component.as_os_str());
    }
    Some(out)
}

fn visible(tree: &Tree, candidates: &[PathBuf], request: &Request) -> Option<usize> {
    for slot in candidates {
        if let Some(index) = tree
            .slots
            .get(slot)
            .and_then(|entries| entries.get(&request.name))
            && request.requirement.matches(&tree.nodes[*index].version)
        {
            return Some(*index);
        }
    }
    None
}

async fn resolve(
    resolvers: &Resolvers<'_>,
    packuments: &mut BTreeMap<String, Packument>,
    request: &Request,
) -> Result<(Version, VersionMeta)> {
    if !packuments.contains_key(&request.name) {
        let packument = resolvers.registry.packument(&request.name).await?;
        packuments.insert(request.name.clone(), packument);
    }
    let meta = packuments[&request.name].resolve(&request.requirement)?;
    Ok((meta.version.clone(), meta.clone()))
}

fn integrity_label(meta: &VersionMeta) -> String {
    meta.integrity
        .clone()
        .or_else(|| meta.shasum.as_ref().map(|value| format!("sha1-{value}")))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Populates `destination` from the store.
///
/// Packages are hard-linked instead of symlinked: Node resolves a symlinked
/// package to its real path, so imports from inside the package would walk up
/// the store directory and miss the `node_modules` tree (the problem pnpm
/// solves with per-package `node_modules` links). Directories are real
/// directories, regular files are hard links (one copy of the bytes), and links
/// inside a package are recreated as links.
fn materialise(source: &Path, destination: &Path) -> Result<()> {
    materialise_tree(source, destination)
}

fn materialise_tree(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            let link = std::fs::read_link(&from)?;
            create_symlink(&link, &to)?;
            continue;
        }
        if kind.is_dir() {
            materialise_tree(&from, &to)?;
            continue;
        }
        if std::fs::hard_link(&from, &to).is_err() {
            // Different filesystem or an unsupported target: a copy is correct,
            // just less cheap.
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(link: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(link, target)?;
    Ok(())
}

/// Copies the link's contents where symlinks are unavailable.
///
/// Lives outside the `cfg(not(unix))` block on purpose: a host build then
/// compiles and type-checks it, instead of leaving it for a Windows-only CI leg
/// to discover.
#[cfg_attr(unix, allow(dead_code))]
fn copy_instead_of_link(link: &Path, target: &Path) -> Result<()> {
    std::fs::copy(link, target).map_err(|error| {
        DshrError::config(format!(
            "npm: cannot recreate symlink {} -> {}: {error}",
            target.display(),
            link.display()
        ))
    })?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(link: &Path, target: &Path) -> Result<()> {
    copy_instead_of_link(link, target)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("dshr/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_slots_are_ordered_shallow_first() {
        assert_eq!(
            candidate_slots(Path::new("")),
            vec![PathBuf::from("node_modules")]
        );
        assert_eq!(
            candidate_slots(Path::new("node_modules/a")),
            vec![
                PathBuf::from("node_modules"),
                PathBuf::from("node_modules/a/node_modules")
            ]
        );
        assert_eq!(
            candidate_slots(Path::new("node_modules/a/node_modules/b")),
            vec![
                PathBuf::from("node_modules"),
                PathBuf::from("node_modules/a/node_modules"),
                PathBuf::from("node_modules/a/node_modules/b/node_modules"),
            ]
        );
    }

    #[test]
    fn strips_package_directories() {
        assert_eq!(
            strip_package_dir(Path::new("node_modules/a")),
            Some(PathBuf::new())
        );
        assert_eq!(
            strip_package_dir(Path::new("node_modules/a/node_modules/b")),
            Some(PathBuf::from("node_modules/a"))
        );
        assert_eq!(strip_package_dir(Path::new("")), None);
        assert_eq!(strip_package_dir(Path::new("node_modules")), None);
        assert_eq!(
            strip_package_dir(Path::new("node_modules/node_modules")),
            Some(PathBuf::new())
        );
    }

    #[test]
    fn copy_fallback_copies_the_contents() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let source = dir.path().join("source.js");
        let target = dir.path().join("target.js");
        std::fs::write(&source, b"export default 1;").expect("write");
        copy_instead_of_link(&source, &target).expect("copy");
        assert_eq!(
            std::fs::read(&target).expect("read back"),
            b"export default 1;"
        );
    }

    #[test]
    fn copy_fallback_reports_a_missing_source() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let error =
            copy_instead_of_link(&dir.path().join("absent.js"), &dir.path().join("target.js"))
                .expect_err("a missing source must fail");
        let message = error.to_string();
        assert!(
            message.contains("cannot recreate symlink"),
            "unexpected message: {message}"
        );
    }
}
