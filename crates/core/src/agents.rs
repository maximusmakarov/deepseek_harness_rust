//! Registry of live agents.
//!
//! Host processes publish the agents they are running here; the HTTP server
//! exposes the snapshot to Cordis sidecars so plugin host halves see the same
//! agent tree as the harness (`agents.list()`, `roots()`, `get(id)`, …).
//!
//! The registry holds only metadata: agent ids, the owning session, runtime
//! parentage and a start timestamp. Nothing about prompts or model output
//! travels through it.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

/// One live agent as seen by a plugin host half.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveAgent {
    /// Agent id; for a turn root this is the session id, as in DSH.
    pub id: String,
    pub session_id: String,
    /// Runtime creator (subagent parent), not the durable session lineage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Top-level agent of the tree this agent belongs to.
    pub root_id: String,
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// RFC3339 start timestamp.
    pub started_at: String,
}

impl LiveAgent {
    /// Top-level agent for `session_id`.
    #[must_use]
    pub fn root(session_id: impl Into<String>, started_at: impl Into<String>) -> Self {
        let session_id = session_id.into();
        Self {
            id: session_id.clone(),
            session_id: session_id.clone(),
            parent_id: None,
            root_id: session_id,
            depth: 0,
            title: None,
            model: None,
            started_at: started_at.into(),
        }
    }

    /// Child of `parent`, sharing its root.
    #[must_use]
    pub fn child(parent: &Self, id: impl Into<String>, started_at: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            session_id: parent.session_id.clone(),
            parent_id: Some(parent.id.clone()),
            root_id: parent.root_id.clone(),
            depth: parent.depth.saturating_add(1),
            title: None,
            model: None,
            started_at: started_at.into(),
        }
    }
}

#[derive(Debug, Default)]
struct Inner {
    agents: BTreeMap<String, LiveAgent>,
    revision: u64,
}

/// Shared, cheaply clonable registry of live agents.
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    inner: Arc<RwLock<Inner>>,
}

/// Removes an agent from the registry when dropped.
#[derive(Debug)]
pub struct AgentGuard {
    registry: AgentRegistry,
    id: String,
}

impl Drop for AgentGuard {
    fn drop(&mut self) {
        self.registry.unregister(&self.id);
    }
}

impl AgentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Publishes `agent` and returns a guard that removes it on drop.
    pub fn register(&self, agent: LiveAgent) -> AgentGuard {
        let id = agent.id.clone();
        if let Ok(mut inner) = self.inner.write() {
            inner.agents.insert(id.clone(), agent);
            inner.revision = inner.revision.saturating_add(1);
        }
        AgentGuard {
            registry: self.clone(),
            id,
        }
    }

    /// Removes one agent; returns whether it was present.
    pub fn unregister(&self, id: &str) -> bool {
        match self.inner.write() {
            Ok(mut inner) => {
                let removed = inner.agents.remove(id).is_some();
                if removed {
                    inner.revision = inner.revision.saturating_add(1);
                }
                removed
            }
            Err(_) => false,
        }
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<LiveAgent> {
        self.inner.read().ok()?.agents.get(id).cloned()
    }

    /// Every live agent, ordered by id.
    #[must_use]
    pub fn list(&self) -> Vec<LiveAgent> {
        self.inner
            .read()
            .map(|inner| inner.agents.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Live agents without a runtime parent.
    #[must_use]
    pub fn roots(&self) -> Vec<LiveAgent> {
        self.list()
            .into_iter()
            .filter(|agent| agent.parent_id.is_none())
            .collect()
    }

    /// Whether `id` was created by `owner_id`.
    #[must_use]
    pub fn is_owned_by(&self, id: &str, owner_id: &str) -> bool {
        self.get(id)
            .and_then(|agent| agent.parent_id)
            .is_some_and(|parent| parent == owner_id)
    }

    /// Root agent id of `id`, when it is live.
    #[must_use]
    pub fn root_of(&self, id: &str) -> Option<String> {
        self.get(id).map(|agent| agent.root_id)
    }

    /// Snapshot for the internal endpoint: agents plus a monotonic revision.
    #[must_use]
    pub fn snapshot(&self) -> (Vec<LiveAgent>, u64) {
        match self.inner.read() {
            Ok(inner) => (inner.agents.values().cloned().collect(), inner.revision),
            Err(_) => (Vec::new(), 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> LiveAgent {
        LiveAgent::root("s1", "2026-01-01T00:00:00Z")
    }

    #[test]
    fn registers_roots_and_children() {
        let registry = AgentRegistry::new();
        let _root_guard = registry.register(root());
        let child = LiveAgent::child(&root(), "s1#sub1", "2026-01-01T00:00:01Z");
        let guard = registry.register(child.clone());

        assert_eq!(registry.list().len(), 2);
        assert_eq!(registry.roots().len(), 1);
        assert_eq!(registry.get("s1#sub1"), Some(child.clone()));
        assert_eq!(child.depth, 1);
        assert_eq!(child.root_id, "s1");
        assert!(registry.is_owned_by("s1#sub1", "s1"));
        assert!(!registry.is_owned_by("s1", "s1#sub1"));
        assert_eq!(registry.root_of("s1#sub1").as_deref(), Some("s1"));

        drop(guard);
        assert_eq!(registry.list().len(), 1);
        assert!(!registry.is_owned_by("s1#sub1", "s1"));
    }

    #[test]
    fn revision_moves_with_actions() {
        let registry = AgentRegistry::new();
        let before = registry.snapshot().1;
        let guard = registry.register(root());
        assert!(registry.snapshot().1 > before);
        let after_register = registry.snapshot().1;
        drop(guard);
        assert!(registry.snapshot().1 > after_register);
        let (agents, _) = registry.snapshot();
        assert!(agents.is_empty());
    }
}
