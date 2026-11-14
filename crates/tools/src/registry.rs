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

/// Asks the active interface a question; `None` means the user declined or the
/// interface cannot answer.
#[async_trait]
pub trait UserPrompt: Send + Sync {
    async fn ask(&self, question: &str, options: &[String]) -> Option<String>;
}

impl std::fmt::Debug for ToolCtx {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ToolCtx")
            .field("cwd", &self.cwd)
            .field("session_id", &self.session_id)
            .field("output_limit", &self.output_limit)
            .field("spill_dir", &self.spill_dir)
            .field("prompt", &self.prompt.is_some())
            .finish()
    }
}

#[derive(Clone)]
pub struct ToolCtx {
    pub cwd: PathBuf,
    pub session_id: SessionId,
    pub output_limit: usize,
    pub spill_dir: PathBuf,
    pub cancel: CancellationToken,
    pub prompt: Option<Arc<dyn UserPrompt>>,
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

/// A tool table that can be mutated while it is being served.
///
/// The registry is a shared handle: `Clone` returns another handle to the same
/// table, and every mutation is visible to all holders. This is what makes
/// plugin hot reload possible — a supervisor swaps a plugin's tools underneath
/// a running agent loop without rebuilding the registry or restarting sessions.
/// Mutations take a write lock only for the duration of a map operation; tool
/// calls clone the `Arc<dyn Tool>` and run without holding a lock.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<std::sync::RwLock<BTreeMap<String, Arc<dyn Tool>>>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn map(&self) -> std::sync::RwLockReadGuard<'_, BTreeMap<String, Arc<dyn Tool>>> {
        self.tools.read().unwrap_or_else(|error| error.into_inner())
    }

    fn map_mut(&self) -> std::sync::RwLockWriteGuard<'_, BTreeMap<String, Arc<dyn Tool>>> {
        self.tools
            .write()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// Monotonic counter bumped by every mutation; useful for diagnostics and
    /// for tools that cache `definitions()`.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn bump(&self) {
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    #[must_use]
    pub fn with_builtin() -> Self {
        let registry = Self::new();
        registry.register(crate::tools::ReadTool);
        registry.register(crate::tools::WriteTool);
        registry.register(crate::tools::EditTool);
        registry.register(crate::tools::ApplyPatchTool);
        registry.register(crate::tools::AskUserTool);
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

    pub fn register<T: Tool + 'static>(&self, tool: T) {
        self.register_arc(Arc::new(tool));
    }

    pub fn register_arc(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_owned();
        self.map_mut().insert(name, tool);
        self.bump();
    }

    /// Registers every tool, returning the names that were inserted.
    pub fn register_all(&self, tools: impl IntoIterator<Item = Arc<dyn Tool>>) -> Vec<String> {
        let mut names = Vec::new();
        {
            let mut map = self.map_mut();
            for tool in tools {
                names.push(tool.name().to_owned());
                map.insert(tool.name().to_owned(), tool);
            }
        }
        if !names.is_empty() {
            self.bump();
        }
        names
    }

    /// Removes one tool; returns whether it was present.
    pub fn unregister(&self, name: &str) -> bool {
        let removed = self.map_mut().remove(name).is_some();
        if removed {
            self.bump();
        }
        removed
    }

    /// Removes every tool whose name starts with `prefix` and returns the
    /// removed names. Plugins use this to swap a whole generation at once.
    pub fn remove_prefix(&self, prefix: &str) -> Vec<String> {
        let mut removed = Vec::new();
        {
            let mut map = self.map_mut();
            let names: Vec<String> = map
                .keys()
                .filter(|name| name.starts_with(prefix))
                .cloned()
                .collect();
            for name in names {
                map.remove(&name);
                removed.push(name);
            }
        }
        if !removed.is_empty() {
            self.bump();
        }
        removed
    }

    /// Returns a registry filtered by an allowlist and a denylist.
    ///
    /// The result is a new, independent table: later mutations of `self` do not
    /// affect it, so a filtered serving registry must be the one a plugin
    /// supervisor holds.
    #[must_use]
    pub fn filtered(&self, allowed: &[String], denied: &[String]) -> Self {
        let filtered = Self::default();
        {
            let mut target = filtered.map_mut();
            for (name, tool) in self.map().iter() {
                if !allowed.is_empty() && !allowed.iter().any(|candidate| candidate == name) {
                    continue;
                }
                if denied.iter().any(|candidate| candidate == name) {
                    continue;
                }
                target.insert(name.clone(), Arc::clone(tool));
            }
        }
        filtered
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.map().get(name).cloned()
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.map().contains_key(name)
    }

    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.map().keys().cloned().collect()
    }

    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.map()
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

    struct NamedTool(&'static str);

    #[async_trait]
    impl Tool for NamedTool {
        fn name(&self) -> &str {
            self.0
        }

        fn description(&self) -> &'static str {
            "test tool"
        }

        fn schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
            Ok(ToolOutcome::text(self.0))
        }
    }

    #[test]
    fn swaps_a_plugin_generation_while_shared() {
        let registry = ToolRegistry::new();
        let handle = registry.clone();
        assert_eq!(registry.generation(), 0);

        let inserted = registry.register_all([
            Arc::new(NamedTool("cordis__demo__echo")) as Arc<dyn Tool>,
            Arc::new(NamedTool("cordis__demo__ping")),
        ]);
        assert_eq!(inserted.len(), 2);
        assert!(handle.contains("cordis__demo__echo"));
        assert_eq!(handle.generation(), 1);

        let removed = registry.remove_prefix("cordis__demo__");
        assert_eq!(
            removed,
            vec![
                "cordis__demo__echo".to_owned(),
                "cordis__demo__ping".to_owned()
            ]
        );
        assert!(!handle.contains("cordis__demo__echo"));
        assert_eq!(handle.generation(), 2);

        assert!(!registry.unregister("cordis__demo__echo"));
        assert_eq!(registry.generation(), 2);
    }

    #[test]
    fn filtered_is_independent_from_later_mutations() {
        let registry = ToolRegistry::new();
        registry.register(NamedTool("cordis__demo__echo"));
        let serving = registry.filtered(&[], &[]);
        registry.register(NamedTool("cordis__demo__later"));
        assert!(!serving.contains("cordis__demo__later"));
        assert!(registry.contains("cordis__demo__later"));
    }
}
