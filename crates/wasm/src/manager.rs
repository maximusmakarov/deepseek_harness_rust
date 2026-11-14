//! Hot reload for WASM plugin directories.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dshr_tools::ToolRegistry;

use crate::{SignaturePolicy, discover, register_plugin_tools};

/// Watches plugin directories and swaps `wasm__*` tools when files change.
pub struct WasmManager {
    registry: ToolRegistry,
    dirs: Vec<PathBuf>,
    policy: SignaturePolicy,
    fingerprint: Mutex<Option<u64>>,
    ops: tokio::sync::Mutex<()>,
}

impl std::fmt::Debug for WasmManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WasmManager")
            .field("dirs", &self.dirs)
            .finish_non_exhaustive()
    }
}

impl WasmManager {
    #[must_use]
    pub fn new(registry: ToolRegistry, dirs: Vec<PathBuf>, policy: SignaturePolicy) -> Arc<Self> {
        Arc::new(Self {
            registry,
            dirs,
            policy,
            fingerprint: Mutex::new(None),
            ops: tokio::sync::Mutex::new(()),
        })
    }

    /// Re-discovers plugins when the directory fingerprint changed.
    /// Returns the number of currently registered WASM tools.
    pub async fn sync(&self) -> usize {
        let _guard = self.ops.lock().await;
        let fingerprint = directory_fingerprint(&self.dirs);
        let unchanged = {
            let current = self
                .fingerprint
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *current == Some(fingerprint)
        };
        if !unchanged {
            self.registry.remove_prefix("wasm__");
            for plugin in discover(&self.dirs, &self.policy) {
                match plugin.tools() {
                    Ok(tools) => {
                        for definition in tools {
                            register_plugin_tools(&self.registry, Arc::clone(&plugin), definition);
                        }
                    }
                    Err(error) => {
                        tracing::warn!(plugin = plugin.name(), %error, "wasm tools skipped");
                    }
                }
            }
            *self
                .fingerprint
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(fingerprint);
        }
        self.registry
            .names()
            .into_iter()
            .filter(|name| name.starts_with("wasm__"))
            .count()
    }

    /// Spawns a periodic sync loop on the current runtime.
    pub fn spawn_watch(self: &Arc<Self>, interval: Duration) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                manager.sync().await;
            }
        });
    }
}

fn directory_fingerprint(dirs: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for dir in dirs {
        let mut files = Vec::new();
        collect_files(dir, &mut files);
        files.sort();
        for path in files {
            path.hash(&mut hasher);
            if let Ok(metadata) = std::fs::metadata(&path) {
                metadata.len().hash(&mut hasher);
                if let Ok(modified) = metadata.modified()
                    && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
                {
                    duration.as_nanos().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yaml" | "wasm")
        ) {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
    use serde_json::Value;

    struct Mock(&'static str);

    #[async_trait]
    impl Tool for Mock {
        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &'static str {
            "mock"
        }

        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
            Ok(ToolOutcome::text("ok"))
        }
    }

    #[tokio::test]
    async fn sync_removes_stale_wasm_tools() {
        let registry = ToolRegistry::new();
        registry.register(Mock("wasm__old__echo"));
        registry.register(Mock("read"));
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = WasmManager::new(
            registry.clone(),
            vec![dir.path().to_path_buf()],
            SignaturePolicy::default(),
        );
        assert_eq!(manager.sync().await, 0);
        assert_eq!(registry.names(), vec!["read".to_owned()]);
    }
}
