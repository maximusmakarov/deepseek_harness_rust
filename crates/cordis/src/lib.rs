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
use dshr_mcp::{McpServerSpec, connect_server};
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::Value;
use thiserror::Error;

const BOOTSTRAP: &str = include_str!("bootstrap.mjs");

#[derive(Debug, Error)]
pub enum CordisError {
    #[error("cordis plugin I/O: {0}")]
    Io(String),

    #[error("cordis sidecar: {0}")]
    Sidecar(String),
}

#[derive(Debug, Clone)]
pub struct CordisPluginSpec {
    pub name: String,
    pub entry: PathBuf,
    pub root: PathBuf,
    pub env: BTreeMap<String, String>,
}

/// Connects a Cordis plugin sidecar and returns tools named
/// `cordis__<plugin>__<tool>`.
pub async fn connect(
    spec: CordisPluginSpec,
    home: &Path,
) -> Result<Vec<Arc<dyn Tool>>, CordisError> {
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
    let entry = if spec.entry.is_absolute() {
        spec.entry.clone()
    } else {
        spec.root.join(&spec.entry)
    };
    let mut env = spec.env;
    env.insert("DSHR_CORDIS_ROOT".into(), spec.root.display().to_string());
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
    let (_backend, tools) = connect_server(server)
        .await
        .map_err(|error| CordisError::Sidecar(error.to_string()))?;
    Ok(tools
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
            }) as Arc<dyn Tool>
        })
        .collect())
}

#[must_use]
pub fn bootstrap_path(home: &Path) -> PathBuf {
    home.join("cordis").join("bootstrap.mjs")
}

struct CordisTool {
    name: String,
    inner: Arc<dyn Tool>,
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
        self.inner.execute(ctx, args).await
    }
}
