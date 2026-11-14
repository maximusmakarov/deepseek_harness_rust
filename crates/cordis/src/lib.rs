#![forbid(unsafe_code)]
//! Cordis/npm plugin sidecar.
//!
//! A JavaScript plugin runs in a separate `node` process. The bootstrap
//! exposes the plugin's `ctx.tool(...)` registrations as an MCP stdio server,
//! so the harness reuses the existing MCP tool pipeline. Capabilities are
//! explicit: `ctx.fs` is confined to the workspace root, `ctx.exec` and
//! `ctx.net` throw. `cordis` itself is optional at runtime; when the package is
//! installed next to the plugin the bootstrap loads it, otherwise it calls the
//! plugin's `apply(ctx)` directly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dshr_mcp::{McpBackend, McpServerSpec, connect_server};
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::Value;
use thiserror::Error;

pub mod client;
pub mod manager;

pub use client::{ClientHalf, ClientHalfError, discover_client_half};
pub use dshr_config::CordisGrants;
pub use manager::{CordisManager, PluginEvent, PluginStatus, RefreshFn, WatchOptions};

const BOOTSTRAP: &str = include_str!("bootstrap.mjs");

#[derive(Debug, Error)]
pub enum CordisError {
    #[error("cordis plugin I/O: {0}")]
    Io(String),

    #[error("cordis sidecar: {0}")]
    Sidecar(String),

    #[error("plugin '{0}' is not configured")]
    NotConfigured(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordisPluginSpec {
    pub name: String,
    pub entry: PathBuf,
    pub root: PathBuf,
    pub env: BTreeMap<String, String>,
    pub grants: dshr_config::CordisGrants,
}

impl CordisPluginSpec {
    /// Absolute plugin package directory: the entry itself when it points at a
    /// package directory, otherwise the entry's parent directory.
    #[must_use]
    pub fn package_dir(&self) -> PathBuf {
        let entry = if self.entry.is_absolute() {
            self.entry.clone()
        } else {
            self.root.join(&self.entry)
        };
        if entry.is_dir() {
            entry
        } else {
            entry.parent().map(Path::to_path_buf).unwrap_or(entry)
        }
    }

    /// Entry path resolved against the workspace root.
    #[must_use]
    pub fn resolved_entry(&self) -> PathBuf {
        if self.entry.is_absolute() {
            self.entry.clone()
        } else {
            self.root.join(&self.entry)
        }
    }
}

/// A live Cordis sidecar together with the tools it registered.
///
/// Keeping the connection alive keeps the node process alive: the backend owns
/// the child with `kill_on_drop`. Hot reload drops the connection (after
/// removing its tools) and connects a fresh process.
pub struct CordisConnection {
    pub backend: McpBackend,
    pub tools: Vec<Arc<dyn Tool>>,
}

impl std::fmt::Debug for CordisConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CordisConnection")
            .field("tools", &self.tools.len())
            .finish_non_exhaustive()
    }
}

/// Connects a Cordis plugin sidecar and returns its tools, named
/// `cordis__<plugin>__<tool>`.
pub async fn connect(spec: CordisPluginSpec, home: &Path) -> Result<CordisConnection, CordisError> {
    let bootstrap = bootstrap_path(home);
    if let Some(parent) = bootstrap.parent() {
        std::fs::create_dir_all(parent).map_err(|error| CordisError::Io(error.to_string()))?;
    }
    let current = std::fs::read_to_string(&bootstrap).unwrap_or_default();
    if current != BOOTSTRAP {
        std::fs::write(&bootstrap, BOOTSTRAP)
            .map_err(|error| CordisError::Io(error.to_string()))?;
    }
    let node = std::env::var("DSHR_NODE").unwrap_or_else(|_| "node".to_owned());
    let entry = spec.resolved_entry();
    let mut env = spec.env.clone();
    env.insert("DSHR_CORDIS_ROOT".into(), spec.root.display().to_string());
    env.insert(
        "DSHR_CORDIS_GRANTS".into(),
        serde_json::to_string(&spec.grants).unwrap_or_else(|_| "{}".to_owned()),
    );
    let server = McpServerSpec {
        name: format!("cordis-{}", spec.name),
        command: node,
        args: vec![bootstrap.display().to_string(), entry.display().to_string()],
        env,
        cwd: Some(spec.root.clone()),
        url: None,
        headers: BTreeMap::new(),
        oauth: None,
    };
    let (backend, tools) = connect_server(server)
        .await
        .map_err(|error| CordisError::Sidecar(error.to_string()))?;
    let tools = tools
        .into_iter()
        .map(|tool| {
            let prefix = format!("mcp__cordis-{}__", spec.name);
            let short = tool
                .name()
                .strip_prefix(&prefix)
                .unwrap_or(tool.name())
                .to_owned();
            Arc::new(CordisTool {
                name: format!("cordis__{}__{short}", spec.name),
                inner: tool,
                home: home.to_path_buf(),
            }) as Arc<dyn Tool>
        })
        .collect();
    Ok(CordisConnection { backend, tools })
}

#[must_use]
pub fn bootstrap_path(home: &Path) -> PathBuf {
    home.join("cordis").join("bootstrap.mjs")
}

struct CordisTool {
    name: String,
    inner: Arc<dyn Tool>,
    home: PathBuf,
}

impl CordisTool {
    fn audit(&self, result: &Result<ToolOutcome, ToolError>) {
        let directory = self.home.join("cordis");
        if std::fs::create_dir_all(&directory).is_err() {
            return;
        }
        let entry = serde_json::json!({
            "time": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0),
            "plugin": self.name.split("__").nth(1).unwrap_or(""),
            "tool": self.inner.name(),
            "ok": result.is_ok(),
            "error": result.as_ref().err().map(ToString::to_string)
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("audit.jsonl"))
        {
            use std::io::Write;
            let _ = writeln!(file, "{entry}");
        }
    }
}

#[async_trait]
impl Tool for CordisTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn schema(&self) -> Value {
        self.inner.schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let result = self.inner.execute(ctx, args).await;
        self.audit(&result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "echo"
        }

        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
            Ok(ToolOutcome {
                content: "ok".into(),
                is_error: false,
                files: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn writes_an_audit_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home");
        let tool = CordisTool {
            name: "cordis__demo__echo".into(),
            inner: Arc::new(MockTool),
            home: home.clone(),
        };
        let ctx = dshr_tools::ToolCtx {
            cwd: dir.path().to_path_buf(),
            session_id: dshr_core::SessionId::new(),
            output_limit: 10_000,
            spill_dir: dir.path().join("spills"),
            cancel: tokio_util::sync::CancellationToken::new(),
            prompt: None,
        };
        tool.execute(&ctx, serde_json::json!({}))
            .await
            .expect("tool");
        let audit = std::fs::read_to_string(home.join("cordis/audit.jsonl")).expect("audit");
        assert!(audit.contains("\"plugin\":\"demo\""), "{audit}");
        assert!(audit.contains("\"tool\":\"echo\""), "{audit}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
