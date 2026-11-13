use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use dshr_core::SessionId;
use dshr_llm::ToolDef;
use serde_json::Value;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::schema::validate;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("denied: {0}")]
    Denied(String),

    #[error("{0}")]
    Failed(String),

    #[error("cancelled")]
    Cancelled,

    #[error("timed out")]
    Timeout,
}

#[derive(Debug, Clone)]
pub struct ToolCtx {
    pub cwd: PathBuf,
    pub session_id: SessionId,
    pub output_limit: usize,
    pub spill_dir: PathBuf,
    pub cancel: CancellationToken,
}

#[derive(Debug, Clone, Default)]
pub struct ToolOutcome {
    pub content: String,
    pub is_error: bool,
    pub files: Vec<String>,
}

impl ToolOutcome {
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            files: Vec::new(),
        }
    }

    #[must_use]
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            files: Vec::new(),
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;
    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError>;
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_builtin() -> Self {
        let mut registry = Self::new();
        registry.register(crate::tools::ReadTool);
        registry.register(crate::tools::WriteTool);
        registry.register(crate::tools::EditTool);
        registry.register(crate::tools::ApplyPatchTool);
        registry.register(crate::tools::GlobTool);
        registry.register(crate::tools::GrepTool);
        registry.register(crate::tools::BashTool::default());
        registry.register(crate::tools::WebFetchTool::default());
        registry.register(crate::tools::WebSearchTool::from_env());
        registry.register(crate::tools::TodoTool::default());
        registry.register(crate::tools::WorktreeCreateTool);
        registry.register(crate::tools::WorktreeListTool);
        registry.register(crate::tools::WorktreeRemoveTool);
        registry.register(crate::tools::PresentTool);
        registry
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_owned(), Arc::new(tool));
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_owned(), tool);
    }

    /// Returns a registry filtered by an allowlist and a denylist.
    #[must_use]
    pub fn filtered(&self, allowed: &[String], denied: &[String]) -> Self {
        let mut filtered = Self::default();
        for (name, tool) in &self.tools {
            if !allowed.is_empty() && !allowed.iter().any(|candidate| candidate == name) {
                continue;
            }
            if denied.iter().any(|candidate| candidate == name) {
                continue;
            }
            filtered.tools.insert(name.clone(), Arc::clone(tool));
        }
        filtered
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|tool| ToolDef {
                name: tool.name().to_owned(),
                description: tool.description().to_owned(),
                input_schema: tool.schema(),
            })
            .collect()
    }

    #[tracing::instrument(name = "tool.execute", skip(self, ctx, args), fields(tool = %name))]
    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        ctx: &ToolCtx,
    ) -> Result<ToolOutcome, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::InvalidArgs(format!("unknown tool '{name}'")))?;

        validate(&tool.schema(), &args).map_err(ToolError::InvalidArgs)?;
        let outcome = tool.execute(ctx, args).await?;
        self.apply_output_limit(outcome, name, ctx).await
    }

    async fn apply_output_limit(
        &self,
        mut outcome: ToolOutcome,
        name: &str,
        ctx: &ToolCtx,
    ) -> Result<ToolOutcome, ToolError> {
        if outcome.content.len() <= ctx.output_limit {
            return Ok(outcome);
        }

        tokio::fs::create_dir_all(&ctx.spill_dir)
            .await
            .map_err(|error| ToolError::Failed(format!("create spill dir: {error}")))?;
        let file_name = format!("{name}-{}.txt", uuid::Uuid::now_v7());
        let path = ctx.spill_dir.join(file_name);
        tokio::fs::write(&path, &outcome.content)
            .await
            .map_err(|error| ToolError::Failed(format!("write spill file: {error}")))?;

        let mut end = ctx.output_limit.min(outcome.content.len());
        while !outcome.content.is_char_boundary(end) {
            end -= 1;
        }
        outcome.content = format!(
            "{}\n\n[output truncated at {} bytes; full output: {}]",
            &outcome.content[..end],
            ctx.output_limit,
            path.display()
        );
        Ok(outcome)
    }
}

pub fn resolve_path(cwd: &std::path::Path, path: &str) -> PathBuf {
    let candidate = std::path::Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        cwd.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_tools_by_allow_and_deny_lists() {
        let registry = ToolRegistry::with_builtin();
        let allowed = registry.filtered(&["read".into()], &[]);
        assert_eq!(allowed.names(), vec!["read".to_owned()]);

        let denied = registry.filtered(&[], &["bash".into()]);
        assert!(denied.names().iter().any(|name| name == "read"));
        assert!(!denied.names().iter().any(|name| name == "bash"));
    }
}
