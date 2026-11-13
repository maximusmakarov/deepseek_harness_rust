#![forbid(unsafe_code)]
//! Agent loop: stream a model response, execute tool calls, repeat.
//!
//! The loop is provider-neutral and UI-neutral. Observers receive streamed text
//! and tool lifecycle events; approvals are delegated to an [`Approver`].

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use dshr_core::{DshrError, TurnId};
use dshr_llm::{BlockAssembler, ContentBlock, LlmError, LlmProvider, Request, StreamChunk, Usage};
use dshr_session::{Session, SessionEvent, TurnEndReason, now_rfc3339};
use dshr_session_jsonl::SessionStore;
use dshr_tools::{ToolCtx, ToolOutcome, ToolRegistry, resolve_path};
use futures_util::StreamExt;
use serde_json::Value;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("cancelled")]
    Cancelled,

    #[error("denied: {0}")]
    Denied(String),

    #[error("maximum steps reached")]
    MaxSteps,

    #[error("token budget exhausted")]
    BudgetExceeded,

    #[error("llm: {0}")]
    Llm(String),

    #[error("session: {0}")]
    Session(String),

    #[error("tool: {0}")]
    Tool(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFull,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub tool: String,
    pub summary: String,
    pub input: Value,
    pub reason: String,
}

#[async_trait]
pub trait Approver: Send + Sync {
    async fn approve(&self, request: ApprovalRequest) -> bool;
}

pub struct AutoApprover;

#[async_trait]
impl Approver for AutoApprover {
    async fn approve(&self, _request: ApprovalRequest) -> bool {
        true
    }
}

pub struct DenyApprover;

#[async_trait]
impl Approver for DenyApprover {
    async fn approve(&self, _request: ApprovalRequest) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct ApprovalPolicy {
    pub mode: PermissionMode,
    pub workspace: PathBuf,
    pub approver: Arc<dyn Approver>,
}

impl ApprovalPolicy {
    async fn check(&self, tool: &str, input: &Value) -> Result<(), AgentError> {
        match self.decide(tool, input) {
            Decision::Allow => Ok(()),
            Decision::Deny(reason) => Err(AgentError::Denied(reason)),
            Decision::Ask(reason) => {
                let request = ApprovalRequest {
                    tool: tool.to_owned(),
                    summary: summarize(tool, input),
                    input: input.clone(),
                    reason,
                };
                if self.approver.approve(request).await {
                    Ok(())
                } else {
                    Err(AgentError::Denied("user denied the request".into()))
                }
            }
        }
    }

    fn inside_workspace(&self, path: &str) -> bool {
        let path = normalize(&resolve_path(&self.workspace, path));
        path.starts_with(normalize(&self.workspace))
    }

    fn decide(&self, tool: &str, input: &Value) -> Decision {
        match tool {
            "read" => {
                let Some(path) = input["path"].as_str() else {
                    return Decision::Deny("path is required".into());
                };
                if self.inside_workspace(path) {
                    Decision::Allow
                } else {
                    Decision::Ask("path is outside the workspace".into())
                }
            }
            "glob" | "grep" => match input["path"].as_str() {
                Some(path) if !self.inside_workspace(path) => {
                    Decision::Ask("search path is outside the workspace".into())
                }
                _ => Decision::Allow,
            },
            "todo" | "present" | "ask_user" => Decision::Allow,
            "apply_patch" => {
                if self.mode == PermissionMode::ReadOnly {
                    Decision::Ask("file modifications are disabled".into())
                } else {
                    Decision::Allow
                }
            }
            "write" | "edit" => {
                let Some(path) = input["path"].as_str() else {
                    return Decision::Deny("path is required".into());
                };
                let path = normalize(&resolve_path(&self.workspace, path));
                if self.mode == PermissionMode::ReadOnly {
                    return Decision::Ask("file modifications are disabled".into());
                }
                if path.starts_with(&self.workspace) {
                    Decision::Allow
                } else {
                    Decision::Ask("path is outside the workspace".into())
                }
            }
            "bash" => {
                if self.mode == PermissionMode::DangerFull {
                    Decision::Allow
                } else {
                    Decision::Ask("shell commands require approval".into())
                }
            }
            _ => Decision::Ask("unknown tool".into()),
        }
    }
}

enum Decision {
    Allow,
    Ask(String),
    Deny(String),
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn summarize(tool: &str, input: &Value) -> String {
    let text = input.to_string();
    let mut end = text.len().min(200);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{tool} {}{}",
        &text[..end],
        if text.len() > 200 { "…" } else { "" }
    )
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[async_trait]
pub trait AgentObserver: Send + Sync {
    async fn on_text(&self, _text: &str) {}

    async fn on_tool_start(&self, _call: &ToolCallInfo) {}

    async fn on_tool_end(&self, _call: &ToolCallInfo, _outcome: &ToolOutcome) {}

    /// Asks the active interface a question. `None` means the interface cannot
    /// answer; `ask_user` then fails closed.
    async fn on_question(&self, _question: &str, _options: &[String]) -> Option<String> {
        None
    }
}

pub struct NullObserver;

/// Runs one delegated subagent turn and returns its final text.
#[async_trait]
pub trait SubagentRunner: Send + Sync {
    async fn run(&self, prompt: String, cancel: CancellationToken) -> Result<String, String>;
}

/// Tool that delegates a prompt to a fresh child agent.
pub struct SubagentTool {
    runner: Arc<dyn SubagentRunner>,
}

impl SubagentTool {
    #[must_use]
    pub fn new(runner: Arc<dyn SubagentRunner>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl dshr_tools::Tool for SubagentTool {
    fn name(&self) -> &str {
        "subagent"
    }

    fn description(&self) -> &str {
        "Delegate a self-contained task to a child agent and return its result."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["prompt"],
            "additionalProperties": false,
            "properties": {
                "prompt": {"type": "string", "minLength": 1}
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dshr_tools::ToolCtx,
        args: serde_json::Value,
    ) -> Result<dshr_tools::ToolOutcome, dshr_tools::ToolError> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| dshr_tools::ToolError::InvalidArgs("prompt is required".into()))?;
        match self
            .runner
            .run(prompt.to_owned(), _ctx.cancel.clone())
            .await
        {
            Ok(text) => Ok(dshr_tools::ToolOutcome::text(text)),
            Err(error) => Ok(dshr_tools::ToolOutcome::error(format!(
                "subagent failed: {error}"
            ))),
        }
    }
}

#[async_trait]
impl AgentObserver for NullObserver {}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub max_steps: usize,
    pub max_tokens: u32,
    pub output_limit: usize,
    pub compaction: Option<dshr_compaction::CompactionConfig>,
    pub budget: Option<Arc<Budget>>,
}

/// Shared token budget for an agent tree (parent + subagents + workers).
#[derive(Debug)]
pub struct Budget {
    max_tokens: u64,
    used_tokens: AtomicU64,
}

impl Budget {
    #[must_use]
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max_tokens,
            used_tokens: AtomicU64::new(0),
        }
    }

    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.max_tokens
            .saturating_sub(self.used_tokens.load(Ordering::Relaxed))
    }

    /// Reserves tokens; returns false and saturates when the budget is exceeded.
    pub fn try_consume(&self, tokens: u64) -> bool {
        let mut current = self.used_tokens.load(Ordering::Relaxed);
        loop {
            if current >= self.max_tokens {
                return false;
            }
            let next = current.saturating_add(tokens);
            if next > self.max_tokens {
                self.used_tokens.store(self.max_tokens, Ordering::Relaxed);
                return false;
            }
            match self.used_tokens.compare_exchange_weak(
                current,
                next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnReport {
    pub text: String,
    pub usage: Usage,
    pub steps: usize,
    pub tool_calls: usize,
    pub reason: TurnEndReason,
}

pub struct Agent {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
    session: Session,
    policy: ApprovalPolicy,
    config: AgentConfig,
}

impl Agent {
    #[must_use]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        store: Arc<SessionStore>,
        session: Session,
        policy: ApprovalPolicy,
        config: AgentConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            store,
            session,
            policy,
            config,
        }
    }

    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    #[must_use]
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    #[tracing::instrument(
        name = "agent.turn",
        skip_all,
        fields(session_id = %self.session.header.id, provider = %self.provider.name())
    )]
    pub async fn run_turn(
        &mut self,
        input: &str,
        cancel: CancellationToken,
        observer: &dyn AgentObserver,
    ) -> Result<TurnReport, AgentError> {
        self.run_turn_with_content(vec![ContentBlock::text(input)], cancel, observer)
            .await
    }

    /// Runs a turn with explicit user content blocks (text, images, ...).
    pub async fn run_turn_with_content(
        &mut self,
        content: Vec<ContentBlock>,
        cancel: CancellationToken,
        observer: &dyn AgentObserver,
    ) -> Result<TurnReport, AgentError> {
        let turn = TurnId::new();
        self.append(SessionEvent::TurnStart {
            seq: self.session.next_seq(),
            at: now_rfc3339(),
            turn: turn.clone(),
        })?;
        self.append(SessionEvent::UserMessage {
            seq: self.session.next_seq(),
            at: now_rfc3339(),
            turn: turn.clone(),
            content,
        })?;

        let mut usage = Usage::default();
        let mut text = String::new();
        let mut tool_calls = 0;

        for step in 1..=self.config.max_steps {
            if cancel.is_cancelled() {
                self.end_turn(&turn, TurnEndReason::Cancelled)?;
                return Err(AgentError::Cancelled);
            }

            if let Some(compaction) = self.config.compaction.clone() {
                dshr_compaction::maybe_compact(
                    self.provider.as_ref(),
                    &mut self.session,
                    &self.store,
                    &compaction,
                    cancel.clone(),
                )
                .await
                .map_err(|error| AgentError::Llm(error.to_string()))?;
            }

            if self
                .config
                .budget
                .as_ref()
                .is_some_and(|budget| budget.remaining() == 0)
            {
                self.end_turn(&turn, TurnEndReason::Error)?;
                return Err(AgentError::BudgetExceeded);
            }

            let request = Request {
                model: self.provider.model().to_owned(),
                system: Some(self.config.system_prompt.clone()),
                messages: self.session.derive_messages(),
                max_tokens: self.config.max_tokens,
                tools: self.tools.definitions(),
                ..Request::default()
            };

            let mut stream = match self.provider.stream(request, cancel.clone()).await {
                Ok(stream) => stream,
                Err(LlmError::Cancelled) => {
                    self.end_turn(&turn, TurnEndReason::Cancelled)?;
                    return Err(AgentError::Cancelled);
                }
                Err(error) => {
                    self.append_attempt(&turn, &error.to_string())?;
                    self.end_turn(&turn, TurnEndReason::Error)?;
                    return Err(AgentError::Llm(error.to_string()));
                }
            };

            let mut assembler = BlockAssembler::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(StreamChunk::TextDelta { text: delta }) => {
                        observer.on_text(&delta).await;
                        assembler.push(StreamChunk::TextDelta {
                            text: delta.clone(),
                        });
                        text.push_str(&delta);
                    }
                    Ok(chunk) => assembler.push(chunk),
                    Err(LlmError::Cancelled) => {
                        self.end_turn(&turn, TurnEndReason::Cancelled)?;
                        return Err(AgentError::Cancelled);
                    }
                    Err(error) => {
                        self.append_attempt(&turn, &error.to_string())?;
                        self.end_turn(&turn, TurnEndReason::Error)?;
                        return Err(AgentError::Llm(error.to_string()));
                    }
                }
            }

            let output = match assembler.finish() {
                Ok(output) => output,
                Err(error) => {
                    self.append_attempt(&turn, &error.to_string())?;
                    self.end_turn(&turn, TurnEndReason::Error)?;
                    return Err(AgentError::Llm(error.to_string()));
                }
            };
            usage.merge(output.usage);

            let calls = tool_calls_of(&output.content);
            self.append(SessionEvent::AssistantMessage {
                seq: self.session.next_seq(),
                at: now_rfc3339(),
                turn: turn.clone(),
                content: output.content,
                usage: output.usage,
                stop_reason: output.stop_reason,
            })?;

            if let Some(budget) = &self.config.budget {
                let spent = output
                    .usage
                    .input_tokens
                    .saturating_add(output.usage.output_tokens);
                if !budget.try_consume(spent) {
                    self.end_turn(&turn, TurnEndReason::Error)?;
                    return Err(AgentError::BudgetExceeded);
                }
            }

            if calls.is_empty() {
                self.end_turn(&turn, TurnEndReason::Completed)?;
                return Ok(TurnReport {
                    text,
                    usage,
                    steps: step,
                    tool_calls,
                    reason: TurnEndReason::Completed,
                });
            }

            for call in calls {
                tool_calls += 1;
                if cancel.is_cancelled() {
                    self.end_turn(&turn, TurnEndReason::Cancelled)?;
                    return Err(AgentError::Cancelled);
                }

                let outcome = match self.policy.check(&call.name, &call.input).await {
                    Ok(()) => {
                        observer.on_tool_start(&call).await;
                        let ctx = ToolCtx {
                            cwd: self.policy.workspace.clone(),
                            session_id: self.session.header.id.clone(),
                            output_limit: self.config.output_limit,
                            spill_dir: self.store.root().join("spills"),
                            cancel: cancel.clone(),
                        };
                        if call.name == "ask_user" {
                            let question = call.input["question"]
                                .as_str()
                                .unwrap_or("Question from the agent");
                            let options: Vec<String> = call.input["options"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                                .collect();
                            match observer.on_question(question, &options).await {
                                Some(answer) => ToolOutcome::text(answer),
                                None => {
                                    ToolOutcome::error("ask_user is unavailable in this interface")
                                }
                            }
                        } else {
                            match self
                                .tools
                                .execute(&call.name, call.input.clone(), &ctx)
                                .await
                            {
                                Ok(outcome) => outcome,
                                Err(dshr_tools::ToolError::Cancelled) => {
                                    self.end_turn(&turn, TurnEndReason::Cancelled)?;
                                    return Err(AgentError::Cancelled);
                                }
                                Err(error) => ToolOutcome::error(error.to_string()),
                            }
                        }
                    }
                    Err(AgentError::Denied(reason)) => {
                        ToolOutcome::error(format!("denied: {reason}"))
                    }
                    Err(error) => return Err(error),
                };

                self.append(SessionEvent::ToolResult {
                    seq: self.session.next_seq(),
                    at: now_rfc3339(),
                    turn: turn.clone(),
                    tool_call_id: call.id.clone(),
                    name: call.name.clone(),
                    content: outcome.content.clone(),
                    is_error: outcome.is_error,
                })?;
                observer.on_tool_end(&call, &outcome).await;
            }
        }

        self.end_turn(&turn, TurnEndReason::MaxSteps)?;
        Err(AgentError::MaxSteps)
    }

    fn append(&mut self, event: SessionEvent) -> Result<(), AgentError> {
        self.store
            .append(&mut self.session, event)
            .map_err(|error: DshrError| AgentError::Session(error.to_string()))
    }

    fn append_attempt(&mut self, turn: &TurnId, error: &str) -> Result<(), AgentError> {
        self.append(SessionEvent::AssistantAttempt {
            seq: self.session.next_seq(),
            at: now_rfc3339(),
            turn: turn.clone(),
            error: error.to_owned(),
        })
    }

    fn end_turn(&mut self, turn: &TurnId, reason: TurnEndReason) -> Result<(), AgentError> {
        self.append(SessionEvent::TurnEnd {
            seq: self.session.next_seq(),
            at: now_rfc3339(),
            turn: turn.clone(),
            reason,
        })
    }
}

fn tool_calls_of(content: &[ContentBlock]) -> Vec<ToolCallInfo> {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(ToolCallInfo {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_llm::StopReason;
    use dshr_llm_mock::ScriptedProvider;
    use dshr_session_jsonl::SessionStore;
    use serde_json::json;
    use tempfile::TempDir;

    fn session(store: &SessionStore) -> Session {
        store
            .create("scripted", "test-model", "/tmp")
            .expect("session")
    }

    fn text_response(text: &str) -> Vec<StreamChunk> {
        vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::TextDelta { text: text.into() },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]
    }

    fn tool_response(name: &str, input: Value) -> Vec<StreamChunk> {
        vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::ToolUseStart {
                index: 0,
                id: "call_1".into(),
                name: name.into(),
            },
            StreamChunk::ToolInputDelta {
                index: 0,
                json: input.to_string(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::ToolUse,
            },
        ]
    }

    fn agent(
        dir: &TempDir,
        responses: Vec<Vec<StreamChunk>>,
        policy: ApprovalPolicy,
        max_steps: usize,
    ) -> Agent {
        let store = Arc::new(SessionStore::new(dir.path().join("sessions")));
        let session = session(&store);
        let provider = Arc::new(ScriptedProvider::new("test-model", responses));
        Agent::new(
            provider,
            Arc::new(ToolRegistry::with_builtin()),
            store,
            session,
            policy,
            AgentConfig {
                system_prompt: "test".into(),
                max_steps,
                max_tokens: 64,
                output_limit: 30_000,
                compaction: None,
                budget: None,
            },
        )
    }

    fn policy(dir: &TempDir, mode: PermissionMode, approver: Arc<dyn Approver>) -> ApprovalPolicy {
        ApprovalPolicy {
            mode,
            workspace: dir.path().to_path_buf(),
            approver,
        }
    }

    #[tokio::test]
    async fn executes_tool_and_finishes() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello").expect("write");
        let mut agent = agent(
            &dir,
            vec![
                tool_response("read", json!({"path": "a.txt"})),
                text_response("done"),
            ],
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            4,
        );

        let report = agent
            .run_turn("read the file", CancellationToken::new(), &NullObserver)
            .await
            .expect("turn");
        assert_eq!(report.text, "done");
        assert_eq!(report.tool_calls, 1);
        assert_eq!(report.steps, 2);

        let events = &agent.session().events;
        let result = events
            .iter()
            .find_map(|event| match event {
                SessionEvent::ToolResult { content, .. } => Some(content),
                _ => None,
            })
            .expect("tool result");
        assert!(result.contains("hello"), "{result}");
        assert!(agent.session().derive_messages().iter().any(|message| {
            message
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::ToolResult { .. }))
        }));
    }

    struct AnsweringObserver;

    #[async_trait]
    impl AgentObserver for AnsweringObserver {
        async fn on_question(&self, question: &str, options: &[String]) -> Option<String> {
            assert_eq!(question, "Color?");
            assert_eq!(options, vec!["red".to_owned(), "blue".to_owned()]);
            Some("blue".to_owned())
        }
    }

    #[tokio::test]
    async fn ask_user_is_answered_by_the_observer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut agent = agent(
            &dir,
            vec![
                tool_response(
                    "ask_user",
                    json!({"question": "Color?", "options": ["red", "blue"]}),
                ),
                text_response("picked"),
            ],
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            4,
        );
        let report = agent
            .run_turn("ask", CancellationToken::new(), &AnsweringObserver)
            .await
            .expect("turn");
        assert_eq!(report.text, "picked");
        let result = agent
            .session()
            .events
            .iter()
            .find_map(|event| match event {
                SessionEvent::ToolResult { content, .. } => Some(content.clone()),
                _ => None,
            })
            .expect("tool result");
        assert_eq!(result, "blue");
    }

    #[tokio::test]
    async fn ask_user_fails_closed_without_an_interface() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut agent = agent(
            &dir,
            vec![
                tool_response("ask_user", json!({"question": "Color?"})),
                text_response("no answer"),
            ],
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            4,
        );
        agent
            .run_turn("ask", CancellationToken::new(), &NullObserver)
            .await
            .expect("turn");
        let failed = agent.session().events.iter().any(|event| {
            matches!(
                event,
                SessionEvent::ToolResult { content, is_error: true, .. }
                    if content.contains("unavailable")
            )
        });
        assert!(failed);
    }

    #[tokio::test]
    async fn denied_bash_becomes_error_result() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut agent = agent(
            &dir,
            vec![
                tool_response("bash", json!({"command": "echo hi"})),
                text_response("cannot run"),
            ],
            policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover)),
            4,
        );

        let report = agent
            .run_turn("run", CancellationToken::new(), &NullObserver)
            .await
            .expect("turn");
        assert_eq!(report.text, "cannot run");
        let denied = agent.session().events.iter().any(|event| matches!(
            event,
            SessionEvent::ToolResult { is_error: true, content, .. } if content.contains("denied")
        ));
        assert!(denied);
    }

    #[tokio::test]
    async fn stops_at_max_steps() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello").expect("write");
        let mut agent = agent(
            &dir,
            vec![
                tool_response("read", json!({"path": "a.txt"})),
                tool_response("read", json!({"path": "a.txt"})),
            ],
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            1,
        );

        let error = agent
            .run_turn("loop", CancellationToken::new(), &NullObserver)
            .await
            .expect_err("max steps");
        assert!(matches!(error, AgentError::MaxSteps));
        assert!(agent.session().events.iter().any(|event| matches!(
            event,
            SessionEvent::TurnEnd {
                reason: TurnEndReason::MaxSteps,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn read_is_allowed_in_every_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        for mode in [
            PermissionMode::ReadOnly,
            PermissionMode::WorkspaceWrite,
            PermissionMode::DangerFull,
        ] {
            let policy = policy(&dir, mode, Arc::new(DenyApprover));
            policy
                .check("read", &json!({"path": "a.txt"}))
                .await
                .expect("read");
        }
    }

    #[tokio::test]
    async fn read_outside_workspace_requires_approval() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::ReadOnly, Arc::new(DenyApprover));
        let error = policy
            .check("read", &json!({"path": "/etc/hosts"}))
            .await
            .expect_err("outside read");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn search_tools_are_allowed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::ReadOnly, Arc::new(DenyApprover));
        for tool in ["glob", "grep", "todo", "present"] {
            policy
                .check(tool, &json!({"pattern": "x", "items": [], "paths": ["a"]}))
                .await
                .expect(tool);
        }
    }

    #[tokio::test]
    async fn write_inside_workspace_is_allowed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover));
        policy
            .check("write", &json!({"path": "src/main.rs", "content": "x"}))
            .await
            .expect("write");
    }

    #[tokio::test]
    async fn write_outside_workspace_requires_approval() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover));
        let error = policy
            .check(
                "write",
                &json!({"path": "/tmp/outside.txt", "content": "x"}),
            )
            .await
            .expect_err("outside");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn write_outside_workspace_can_be_approved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(AutoApprover));
        policy
            .check(
                "write",
                &json!({"path": "/tmp/outside.txt", "content": "x"}),
            )
            .await
            .expect("approved");
    }

    #[tokio::test]
    async fn read_only_mode_escalates_writes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::ReadOnly, Arc::new(DenyApprover));
        let error = policy
            .check(
                "edit",
                &json!({"path": "a.txt", "old_string": "a", "new_string": "b"}),
            )
            .await
            .expect_err("read only");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn bash_requires_approval_outside_danger_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        for mode in [PermissionMode::ReadOnly, PermissionMode::WorkspaceWrite] {
            let policy = policy(&dir, mode, Arc::new(DenyApprover));
            let error = policy
                .check("bash", &json!({"command": "echo hi"}))
                .await
                .expect_err("bash");
            assert!(matches!(error, AgentError::Denied(_)));
        }
    }

    #[tokio::test]
    async fn bash_is_allowed_in_danger_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::DangerFull, Arc::new(DenyApprover));
        policy
            .check("bash", &json!({"command": "echo hi"}))
            .await
            .expect("bash");
    }

    #[tokio::test]
    async fn ask_user_is_always_allowed() {
        let dir = tempfile::tempdir().expect("tempdir");
        for mode in [
            PermissionMode::ReadOnly,
            PermissionMode::WorkspaceWrite,
            PermissionMode::DangerFull,
        ] {
            let policy = policy(&dir, mode, Arc::new(DenyApprover));
            policy
                .check("ask_user", &json!({"question": "Color?"}))
                .await
                .expect("ask_user");
        }
    }

    #[tokio::test]
    async fn apply_patch_asks_in_read_only_mode() {
        let dir = tempfile::tempdir().expect("tempdir");
        let read_only = policy(&dir, PermissionMode::ReadOnly, Arc::new(DenyApprover));
        let error = read_only
            .check(
                "apply_patch",
                &json!({"patch": "*** Begin Patch\n*** End Patch"}),
            )
            .await
            .expect_err("read only");
        assert!(matches!(error, AgentError::Denied(_)));
        let workspace_write = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover));
        workspace_write
            .check(
                "apply_patch",
                &json!({"patch": "*** Begin Patch\n*** End Patch"}),
            )
            .await
            .expect("workspace write");
    }

    #[tokio::test]
    async fn unknown_tool_escalates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::DangerFull, Arc::new(DenyApprover));
        let error = policy
            .check("unknown", &json!({}))
            .await
            .expect_err("unknown");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn traversal_outside_workspace_escalates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover));
        let error = policy
            .check("write", &json!({"path": "../escape.txt", "content": "x"}))
            .await
            .expect_err("escape");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn missing_path_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::DangerFull, Arc::new(DenyApprover));
        let error = policy.check("write", &json!({})).await.expect_err("path");
        assert!(matches!(error, AgentError::Denied(_)));
    }

    #[tokio::test]
    async fn approval_summary_names_the_tool() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = policy(&dir, PermissionMode::WorkspaceWrite, Arc::new(DenyApprover));
        let error = policy
            .check("bash", &json!({"command": "echo hi"}))
            .await
            .expect_err("denied");
        let AgentError::Denied(reason) = error else {
            panic!("expected denial");
        };
        assert!(reason.contains("user denied"), "{reason}");
    }

    struct MockRunner;

    #[async_trait]
    impl SubagentRunner for MockRunner {
        async fn run(&self, prompt: String, _cancel: CancellationToken) -> Result<String, String> {
            Ok(format!("child:{prompt}"))
        }
    }

    #[tokio::test]
    async fn subagent_tool_returns_child_text() {
        use dshr_tools::Tool;
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = SubagentTool::new(Arc::new(MockRunner));
        let ctx = dshr_tools::ToolCtx {
            cwd: dir.path().to_path_buf(),
            session_id: dshr_core::SessionId::new(),
            output_limit: 10_000,
            spill_dir: dir.path().join("spills"),
            cancel: CancellationToken::new(),
        };
        let outcome = tool
            .execute(&ctx, json!({"prompt": "task"}))
            .await
            .expect("subagent");
        assert_eq!(outcome.content, "child:task");
    }

    #[tokio::test]
    async fn cancellation_ends_turn() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut agent = agent(
            &dir,
            vec![text_response("never")],
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            2,
        );
        let cancel = CancellationToken::new();
        cancel.cancel();

        let error = agent
            .run_turn("stop", cancel, &NullObserver)
            .await
            .expect_err("cancelled");
        assert!(matches!(error, AgentError::Cancelled));
        assert!(agent.session().events.iter().any(|event| matches!(
            event,
            SessionEvent::TurnEnd {
                reason: TurnEndReason::Cancelled,
                ..
            }
        )));
    }
    #[test]
    fn budget_saturates_and_refuses() {
        let budget = Budget::new(10);
        assert!(budget.try_consume(4));
        assert!(budget.try_consume(6));
        assert!(!budget.try_consume(1));
        assert_eq!(budget.remaining(), 0);
    }

    #[tokio::test]
    async fn budget_exhaustion_stops_turn() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(SessionStore::new(dir.path().join("sessions")));
        let session = session(&store);
        let provider = Arc::new(ScriptedProvider::new(
            "test-model",
            vec![vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::Usage {
                    usage: Usage {
                        input_tokens: 4,
                        output_tokens: 4,
                        ..Usage::default()
                    },
                },
                StreamChunk::TextDelta {
                    text: "hello".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]],
        ));
        let mut agent = Agent::new(
            provider,
            Arc::new(ToolRegistry::new()),
            store,
            session,
            policy(&dir, PermissionMode::DangerFull, Arc::new(AutoApprover)),
            AgentConfig {
                system_prompt: "test".into(),
                max_steps: 2,
                max_tokens: 64,
                output_limit: 10_000,
                compaction: None,
                budget: Some(Arc::new(Budget::new(5))),
            },
        );
        let error = agent
            .run_turn("hi", CancellationToken::new(), &NullObserver)
            .await
            .expect_err("budget");
        assert!(matches!(error, AgentError::BudgetExceeded));
    }
}
