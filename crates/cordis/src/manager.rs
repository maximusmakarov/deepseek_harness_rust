//! Cordis plugin supervisor: owns the sidecar processes and keeps the served
//! tool registry in sync with the desired plugin set.
//!
//! The supervisor is what turns "plugins are mounted once at startup" into
//! "plugins are a live set": starting a plugin registers its tools into the
//! shared [`ToolRegistry`], stopping it removes exactly those tools and drops
//! the sidecar (the backend owns the child with `kill_on_drop`), and reloading
//! performs both in one serialized operation. A watcher can drive reloads from
//! file timestamps, and a config refresh callback can add or remove plugins
//! without restarting the host.
//!
//! Concurrency: state lives in `std::sync::Mutex` maps that are only locked for
//! map operations — never across `.await`. All lifecycle transitions are
//! serialized by one async mutex so a reload cannot interleave with a sync.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dshr_tools::ToolRegistry;
use tokio::sync::broadcast;

use crate::{CordisConnection, CordisError, CordisPluginSpec, connect};

/// Tool name prefix for every Cordis plugin tool.
pub const TOOL_PREFIX: &str = "cordis__";

/// Entry stamp cap: a package larger than this is stamped from its root
/// metadata only, so watching stays cheap.
const MAX_WATCHED_FILES: usize = 4_000;

/// Runtime view of one configured plugin, returned by the status endpoint.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginStatus {
    pub name: String,
    /// The plugin is present in the desired set.
    pub enabled: bool,
    /// A sidecar is currently connected for it.
    pub running: bool,
    /// Supervisor generation of the running sidecar (0 when not running).
    pub generation: u64,
    /// Tool names currently served for this plugin.
    pub tools: Vec<String>,
    /// Unix seconds when the sidecar started.
    pub started_at: u64,
    /// Last start/reload error, cleared on the next successful start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Lifecycle notification published on the manager's broadcast channel.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginEvent {
    /// `started` | `stopped` | `reloaded` | `failed`
    pub action: String,
    pub plugin: String,
    pub generation: u64,
    pub tools: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Running {
    spec: CordisPluginSpec,
    /// Held so the sidecar process outlives the connection handshake; tools
    /// also hold backend clones, so an in-flight call keeps it alive.
    _connection: CordisConnection,
    tool_names: Vec<String>,
    generation: u64,
    started_at: u64,
}

/// Rebuilds the desired plugin list from disk; `Err` means the config could not
/// be read right now (mid-save, invalid), not that there are no plugins.
pub type RefreshFn = Arc<dyn Fn() -> Result<Vec<CordisPluginSpec>, String> + Send + Sync>;

/// Options for [`CordisManager::spawn_watch`].
pub struct WatchOptions {
    pub interval: Duration,
    /// Files whose change triggers a config refresh (the profile/user config).
    pub config_files: Vec<PathBuf>,
    /// Rebuilds the desired plugin list from disk. `None` disables config
    /// watching while entry watching stays active. `Err` keeps the current set:
    /// a half-written config must not stop every plugin.
    pub refresh: Option<RefreshFn>,
}

/// Allow/deny policy applied to plugin tools before they reach the registry.
///
/// The supervisor registers tools *after* the base registry was filtered for
/// serving, so it must apply the same lists itself; otherwise a plugin could
/// bypass `tools.allowed`.
#[derive(Debug, Clone, Default)]
pub struct ToolFilter {
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
}

impl ToolFilter {
    #[must_use]
    pub fn permits(&self, name: &str) -> bool {
        (self.allowed.is_empty() || self.allowed.iter().any(|candidate| candidate == name))
            && !self.denied.iter().any(|candidate| candidate == name)
    }
}

pub struct CordisManager {
    registry: Arc<ToolRegistry>,
    home: PathBuf,
    root: PathBuf,
    filter: ToolFilter,
    desired: Mutex<BTreeMap<String, CordisPluginSpec>>,
    running: Mutex<BTreeMap<String, Running>>,
    errors: Mutex<BTreeMap<String, String>>,
    generation: AtomicU64,
    ops: tokio::sync::Mutex<()>,
    events: broadcast::Sender<PluginEvent>,
}

impl CordisManager {
    /// Creates a supervisor that registers tools into `registry`.
    ///
    /// `registry` must be the serving registry (already filtered by the
    /// allow/deny lists); the supervisor never rebuilds it.
    #[must_use]
    pub fn new(
        registry: Arc<ToolRegistry>,
        home: PathBuf,
        root: PathBuf,
        filter: ToolFilter,
    ) -> Arc<Self> {
        let (events, _) = broadcast::channel(128);
        Arc::new(Self {
            registry,
            home,
            root,
            filter,
            desired: Mutex::new(BTreeMap::new()),
            running: Mutex::new(BTreeMap::new()),
            errors: Mutex::new(BTreeMap::new()),
            generation: AtomicU64::new(0),
            ops: tokio::sync::Mutex::new(()),
            events,
        })
    }

    #[must_use]
    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<PluginEvent> {
        self.events.subscribe()
    }

    /// Desired plugins, in name order.
    #[must_use]
    pub fn specs(&self) -> Vec<CordisPluginSpec> {
        lock(&self.desired).values().cloned().collect()
    }

    #[must_use]
    pub fn is_running(&self, name: &str) -> bool {
        lock(&self.running).contains_key(name)
    }

    /// Configuration and runtime state for every known plugin.
    #[must_use]
    pub fn statuses(&self) -> Vec<PluginStatus> {
        let desired = lock(&self.desired);
        let running = lock(&self.running);
        let errors = lock(&self.errors);
        let mut names = BTreeSet::new();
        names.extend(desired.keys().cloned());
        names.extend(running.keys().cloned());
        names
            .into_iter()
            .map(|name| {
                let live = running.get(&name);
                PluginStatus {
                    enabled: desired.contains_key(&name),
                    running: live.is_some(),
                    generation: live.map(|live| live.generation).unwrap_or(0),
                    tools: live.map(|live| live.tool_names.clone()).unwrap_or_default(),
                    started_at: live.map(|live| live.started_at).unwrap_or(0),
                    error: errors.get(&name).cloned(),
                    name,
                }
            })
            .collect()
    }

    /// Starts (or restarts) one plugin. On failure the desired spec is kept so
    /// a later reload or file change can retry.
    pub async fn start(&self, spec: CordisPluginSpec) -> Result<(), CordisError> {
        let _guard = self.ops.lock().await;
        lock(&self.desired).insert(spec.name.clone(), spec.clone());
        self.start_locked(spec).await
    }

    /// Stops one plugin and removes its tools. Returns whether it was running.
    /// The desired spec is kept: stopping is temporary, unlike a config change.
    pub async fn stop(&self, name: &str) -> bool {
        let _guard = self.ops.lock().await;
        self.stop_locked(name)
    }

    /// Restarts one plugin from the desired spec, picking up edited entry code,
    /// changed env or grants.
    pub async fn reload(&self, name: &str) -> Result<(), CordisError> {
        let _guard = self.ops.lock().await;
        let spec = lock(&self.desired)
            .get(name)
            .cloned()
            .ok_or_else(|| CordisError::NotConfigured(name.to_owned()))?;
        self.start_locked(spec).await?;
        let running = lock(&self.running);
        let (generation, tools) = running
            .get(name)
            .map(|running| (running.generation, running.tool_names.len()))
            .unwrap_or((0, 0));
        drop(running);
        let _ = self.events.send(PluginEvent {
            action: "reloaded".into(),
            plugin: name.to_owned(),
            generation,
            tools,
            error: None,
        });
        Ok(())
    }

    /// Reloads every desired plugin, returning per-plugin errors.
    pub async fn reload_all(&self) -> Vec<(String, String)> {
        let names: Vec<String> = lock(&self.desired).keys().cloned().collect();
        let mut failures = Vec::new();
        for name in names {
            if let Err(error) = self.reload(&name).await {
                failures.push((name, error.to_string()));
            }
        }
        failures
    }

    /// Reconciles the running set with `specs`: starts new plugins, reloads
    /// changed ones (entry, env or grants) and stops plugins that disappeared
    /// or were removed from the config.
    pub async fn sync(&self, specs: Vec<CordisPluginSpec>) {
        let _guard = self.ops.lock().await;
        let desired: BTreeMap<String, CordisPluginSpec> = specs
            .into_iter()
            .map(|spec| (spec.name.clone(), spec))
            .collect();
        let running_names: Vec<String> = lock(&self.running).keys().cloned().collect();
        for name in running_names {
            if !desired.contains_key(&name) {
                self.stop_locked(&name);
                lock(&self.errors).remove(&name);
            }
        }
        lock(&self.desired).clone_from(&desired);
        let changes: Vec<CordisPluginSpec> = desired
            .values()
            .filter(|spec| {
                lock(&self.running)
                    .get(&spec.name)
                    .is_none_or(|running| running.spec != **spec)
            })
            .cloned()
            .collect();
        for spec in changes {
            if let Err(error) = self.start_locked(spec).await {
                tracing::warn!(%error, "cordis plugin sync failed");
            }
        }
    }

    /// Starts every desired plugin that is not running, e.g. after startup.
    pub async fn start_all(&self) -> Vec<(String, String)> {
        let specs = self.specs();
        let mut failures = Vec::new();
        for spec in specs {
            if self.is_running(&spec.name) {
                continue;
            }
            if let Err(error) = self.start(spec.clone()).await {
                failures.push((spec.name, error.to_string()));
            }
        }
        failures
    }

    async fn start_locked(&self, spec: CordisPluginSpec) -> Result<(), CordisError> {
        // Detach the previous generation but keep its tools registered until the
        // new connection is up: `replace_prefix` swaps both in one write-lock
        // section, so a reader never sees the plugin's tools disappear mid-reload.
        let previous = lock(&self.running).remove(&spec.name);
        match connect(spec.clone(), &self.home).await {
            Ok(connection) => {
                let tools: Vec<_> = connection
                    .tools
                    .iter()
                    .filter(|tool| self.filter.permits(tool.name()))
                    .cloned()
                    .collect();
                let tool_names: Vec<String> =
                    tools.iter().map(|tool| tool.name().to_owned()).collect();
                let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
                let started_at = unix_now();
                lock(&self.running).insert(
                    spec.name.clone(),
                    Running {
                        spec: spec.clone(),
                        _connection: connection,
                        tool_names: tool_names.clone(),
                        generation,
                        started_at,
                    },
                );
                lock(&self.errors).remove(&spec.name);
                let prefix = format!("{TOOL_PREFIX}{}__", spec.name);
                let (_removed, registered) = self.registry.replace_prefix(&prefix, tools);
                debug_assert_eq!(registered, tool_names);
                // The new sidecar owns the tools now; dropping the old
                // connection terminates the previous child process.
                drop(previous);
                let _ = self.events.send(PluginEvent {
                    action: "started".into(),
                    plugin: spec.name.clone(),
                    generation,
                    tools: tool_names.len(),
                    error: None,
                });
                Ok(())
            }
            Err(error) => {
                // A failed reload must not take a working plugin offline: put the
                // previous generation back (its tools were never removed).
                if let Some(previous) = previous {
                    lock(&self.running).insert(spec.name.clone(), previous);
                }
                lock(&self.errors).insert(spec.name.clone(), error.to_string());
                let generation = self.generation.load(Ordering::SeqCst);
                let _ = self.events.send(PluginEvent {
                    action: "failed".into(),
                    plugin: spec.name.clone(),
                    generation,
                    tools: 0,
                    error: Some(error.to_string()),
                });
                Err(error)
            }
        }
    }

    fn stop_locked(&self, name: &str) -> bool {
        let Some(running) = lock(&self.running).remove(name) else {
            return false;
        };
        let removed = self
            .registry
            .remove_prefix(&format!("{TOOL_PREFIX}{name}__"));
        debug_assert_eq!(removed.len(), running.tool_names.len());
        // Dropping the connection terminates the child once in-flight calls
        // release their backend clones.
        drop(running);
        let _ = self.events.send(PluginEvent {
            action: "stopped".into(),
            plugin: name.to_owned(),
            generation: 0,
            tools: 0,
            error: None,
        });
        true
    }

    /// Spawns the polling watcher.
    ///
    /// It compares entry stamps (recursive mtimes for package directories) and
    /// config stamps, then reloads the changed plugins and syncs the desired
    /// set. Polling keeps the dependency surface small and is exact enough for
    /// a dev loop; the interval is the worst-case reload latency.
    pub fn spawn_watch(self: &Arc<Self>, options: WatchOptions) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(options.interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut configs = config_stamp(&options.config_files);
            let mut stamps: BTreeMap<String, u64> = manager
                .specs()
                .into_iter()
                .map(|spec| (spec.name.clone(), entry_stamp(&spec)))
                .collect();
            loop {
                ticker.tick().await;
                if let Some(refresh) = &options.refresh {
                    let next = config_stamp(&options.config_files);
                    if next != configs {
                        match refresh() {
                            Ok(specs) => {
                                configs = next;
                                tracing::info!("cordis config changed; reconciling plugins");
                                manager.sync(specs).await;
                                stamps = manager
                                    .specs()
                                    .into_iter()
                                    .map(|spec| (spec.name.clone(), entry_stamp(&spec)))
                                    .collect();
                            }
                            Err(error) => {
                                // Retry on the next tick: the file is probably
                                // mid-save or temporarily invalid.
                                tracing::warn!(%error, "cordis config reload skipped");
                            }
                        }
                    }
                }
                let specs = manager.specs();
                let names: BTreeSet<&str> = specs.iter().map(|spec| spec.name.as_str()).collect();
                stamps.retain(|name, _| names.contains(name.as_str()));
                for spec in specs {
                    let stamp = entry_stamp(&spec);
                    let changed = stamps
                        .get(&spec.name)
                        .is_some_and(|previous| *previous != stamp);
                    stamps.insert(spec.name.clone(), stamp);
                    if !changed {
                        continue;
                    }
                    tracing::info!(plugin = %spec.name, "cordis plugin changed; reloading");
                    if let Err(error) = manager.reload(&spec.name).await {
                        tracing::warn!(plugin = %spec.name, %error, "cordis hot reload failed");
                    }
                }
            }
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Browser halves of the desired plugins, plus per-plugin discovery errors.
    #[must_use]
    pub fn client_halves(&self) -> (Vec<crate::ClientHalf>, Vec<(String, String)>) {
        let mut halves = Vec::new();
        let mut errors = Vec::new();
        for spec in self.specs() {
            match crate::discover_client_half(&spec) {
                Ok(half) => halves.push(half),
                Err(crate::ClientHalfError::NoClientHalf(_)) => {}
                Err(error) => errors.push((spec.name, error.to_string())),
            }
        }
        (halves, errors)
    }

    pub fn client_half(&self, name: &str) -> Result<crate::ClientHalf, crate::ClientHalfError> {
        let spec = lock(&self.desired)
            .get(name)
            .cloned()
            .ok_or_else(|| crate::ClientHalfError::NoClientHalf(name.to_owned()))?;
        crate::discover_client_half(&spec)
    }
}

impl std::fmt::Debug for CordisManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CordisManager")
            .field("root", &self.root)
            .field("plugins", &self.specs().len())
            .field("running", &lock(&self.running).len())
            .finish_non_exhaustive()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Stable stamp of a config file set: mtime + length of each file.
fn config_stamp(files: &[PathBuf]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in files {
        file.hash(&mut hasher);
        match std::fs::metadata(file) {
            Ok(metadata) => {
                modified(&metadata).hash(&mut hasher);
                metadata.len().hash(&mut hasher);
            }
            Err(_) => 0u8.hash(&mut hasher),
        }
    }
    hasher.finish()
}

/// Stable stamp of a plugin entry: one file, or a bounded recursive walk for a
/// package directory. `node_modules` and VCS directories inside the package are
/// skipped — editing a dependency should not hot reload the plugin itself.
fn entry_stamp(spec: &CordisPluginSpec) -> u64 {
    let entry = spec.resolved_entry();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut budget = MAX_WATCHED_FILES;
    stamp_path(&entry, &mut hasher, &mut budget);
    hasher.finish()
}

fn stamp_path(path: &Path, hasher: &mut impl Hasher, budget: &mut usize) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        path.hash(hasher);
        0u8.hash(hasher);
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    path.hash(hasher);
    modified(&metadata).hash(hasher);
    metadata.len().hash(hasher);
    if !metadata.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    children.sort();
    for child in children {
        let name = child
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name == "node_modules" || name == ".git" || name == "target" {
            continue;
        }
        stamp_path(&child, hasher, budget);
    }
}

fn modified(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
