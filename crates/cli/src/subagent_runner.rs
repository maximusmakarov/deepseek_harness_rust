use std::sync::Arc;

use async_trait::async_trait;
use dshr_agent::{Agent, AgentConfig, ApprovalPolicy, NullObserver, SubagentRunner};
use dshr_llm::LlmProvider;
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use tokio_util::sync::CancellationToken;

/// Runs a child agent with the same provider, policy, and tool set, but in a
/// fresh session. The child's registry intentionally excludes `subagent`, so
/// delegation cannot recurse indefinitely.
pub struct AgentSubagentRunner {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
    policy: ApprovalPolicy,
    config: AgentConfig,
}

impl AgentSubagentRunner {
    #[must_use]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        store: Arc<SessionStore>,
        policy: ApprovalPolicy,
        config: AgentConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            store,
            policy,
            config,
        }
    }
}

#[async_trait]
impl SubagentRunner for AgentSubagentRunner {
    async fn run(&self, prompt: String, cancel: CancellationToken) -> Result<String, String> {
        let cwd = self.policy.workspace.display().to_string();
        let session = self
            .store
            .create(self.provider.name(), self.provider.model(), &cwd)
            .map_err(|error| error.to_string())?;
        let mut agent = Agent::new(
            Arc::clone(&self.provider),
            Arc::clone(&self.tools),
            Arc::clone(&self.store),
            session,
            self.policy.clone(),
            self.config.clone(),
        );
        let report = agent
            .run_turn(&prompt, cancel, &NullObserver)
            .await
            .map_err(|error| error.to_string())?;
        Ok(report.text)
    }
}
