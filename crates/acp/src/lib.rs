#![forbid(unsafe_code)]
//! Minimal Agent Client Protocol v1 server over stdio.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::oneshot;

mod fs;
pub use fs::{AcpFsClient, AcpRpc, ClientRpc};
mod fs_tools;
pub use fs_tools::{AcpReadTool, AcpWriteTool};
mod terminal;
pub use terminal::AcpTerminalClient;
mod terminal_tool;
pub use terminal_tool::AcpBashTool;

use async_trait::async_trait;
use dshr_agent::{
    Agent, AgentConfig, AgentError, AgentObserver, ApprovalPolicy, ApprovalRequest, Approver,
    PermissionMode, ToolCallInfo,
};
use dshr_llm::LlmProvider;
use dshr_session_jsonl::SessionStore;
use dshr_tools::{ToolOutcome, ToolRegistry};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("acp I/O: {0}")]
    Io(String),

    #[error("acp protocol: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone)]
pub struct AcpConfig {
    pub system_prompt: String,
    pub mode: PermissionMode,
    pub max_steps: usize,
    pub max_tokens: u32,
    pub output_limit: usize,
}

#[derive(Clone)]
pub struct AcpServer {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
    config: AcpConfig,
    fs: Option<Arc<AcpFsClient>>,
    terminal: Option<Arc<AcpTerminalClient>>,
}

impl AcpServer {
    #[must_use]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        store: Arc<SessionStore>,
        config: AcpConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            store,
            config,
            fs: None,
            terminal: None,
        }
    }

    #[must_use]
    pub fn with_fs(mut self, fs: Arc<AcpFsClient>) -> Self {
        self.fs = Some(fs);
        self
    }

    #[must_use]
    pub fn with_terminal(mut self, terminal: Arc<AcpTerminalClient>) -> Self {
        self.terminal = Some(terminal);
        self
    }

    pub async fn serve<R, W>(self, reader: R, writer: W) -> Result<(), AcpError>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let writer = Arc::new(Mutex::new(BufWriter::new(writer)));
        let active: Arc<Mutex<HashMap<String, CancellationToken>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let permission_id = Arc::new(AtomicU64::new(1));
        let rpc: Arc<dyn ClientRpc> = Arc::new(AcpRpc {
            writer: Arc::clone(&writer),
            pending: Arc::clone(&pending),
            next_id: Arc::clone(&permission_id),
        });
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| AcpError::Io(error.to_string()))?
        {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let method = message["method"].as_str().unwrap_or_default();
            let id = message.get("id").cloned();
            let params = message["params"].clone();

            if method.is_empty() {
                if let Some(id) = id.as_ref().and_then(Value::as_u64)
                    && let Some(sender) = pending.lock().await.remove(&id)
                {
                    let _ = sender.send(message.clone());
                }
                continue;
            }

            match method {
                "initialize" => {
                    if let Some(fs) = &self.fs {
                        let read = params["clientCapabilities"]["fs"]["readTextFile"]
                            .as_bool()
                            .unwrap_or(false);
                        let write = params["clientCapabilities"]["fs"]["writeTextFile"]
                            .as_bool()
                            .unwrap_or(false);
                        fs.attach(Arc::clone(&rpc), read, write).await;
                    }
                    if let Some(terminal) = &self.terminal {
                        let enabled = params["clientCapabilities"]["terminal"]
                            .as_bool()
                            .unwrap_or(false);
                        terminal.attach(Arc::clone(&rpc), enabled).await;
                    }
                    if let Some(id) = id {
                        send(
                            &writer,
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "protocolVersion": 1,
                                    "agentCapabilities": {
                                        "loadSession": true,
                                        "promptCapabilities": {
                                            "image": false,
                                            "audio": false,
                                            "embeddedContext": false
                                        }
                                    },
                                    "agentInfo": {
                                        "name": "dshr",
                                        "version": env!("CARGO_PKG_VERSION")
                                    }
                                }
                            }),
                        )
                        .await?;
                    }
                }
                "session/new" => {
                    let Some(id) = id else { continue };
                    let cwd = params["cwd"]
                        .as_str()
                        .filter(|cwd| !cwd.is_empty())
                        .unwrap_or(".");
                    let cwd = if PathBuf::from(cwd).is_absolute() {
                        cwd.to_owned()
                    } else {
                        std::env::current_dir()
                            .map(|dir| dir.join(cwd).display().to_string())
                            .unwrap_or_else(|_| cwd.to_owned())
                    };
                    match self
                        .store
                        .create(self.provider.name(), self.provider.model(), &cwd)
                    {
                        Ok(session) => {
                            send(
                                &writer,
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {"sessionId": session.header.id.as_str()}
                                }),
                            )
                            .await?;
                        }
                        Err(error) => {
                            send_error(&writer, id, -32000, &error.to_string()).await?;
                        }
                    }
                }
                "session/load" => {
                    let Some(id) = id else { continue };
                    let Some(session_id) = params["sessionId"].as_str() else {
                        send_error(&writer, id, -32602, "sessionId is required").await?;
                        continue;
                    };
                    match self.store.open(session_id) {
                        Ok(session) => {
                            if let Some(cwd) = params["cwd"].as_str()
                                && cwd != session.header.cwd
                            {
                                send_error(&writer, id, -32602, "cwd does not match session")
                                    .await?;
                                continue;
                            }
                            for event in &session.events {
                                let (update, content) = match event {
                                    dshr_session::SessionEvent::UserMessage { content, .. } => {
                                        ("user_message_chunk", content)
                                    }
                                    dshr_session::SessionEvent::AssistantMessage {
                                        content,
                                        ..
                                    } => ("agent_message_chunk", content),
                                    _ => continue,
                                };
                                for block in content {
                                    if let dshr_llm::ContentBlock::Text { text } = block {
                                        send(
                                            &writer,
                                            json!({
                                                "jsonrpc": "2.0",
                                                "method": "session/update",
                                                "params": {
                                                    "sessionId": session_id,
                                                    "update": {
                                                        "sessionUpdate": update,
                                                        "content": {"type": "text", "text": text}
                                                    }
                                                }
                                            }),
                                        )
                                        .await?;
                                    }
                                }
                            }
                            send(&writer, json!({"jsonrpc": "2.0", "id": id, "result": {}}))
                                .await?;
                        }
                        Err(error) => {
                            send_error(&writer, id, -32002, &error.to_string()).await?;
                        }
                    }
                }
                "session/prompt" => {
                    let Some(id) = id else { continue };
                    let Some(session_id) = params["sessionId"].as_str() else {
                        send_error(&writer, id, -32602, "sessionId is required").await?;
                        continue;
                    };
                    let prompt = prompt_text(&params["prompt"]);
                    if prompt.trim().is_empty() {
                        send_error(&writer, id, -32602, "prompt is empty").await?;
                        continue;
                    }
                    let cancel = CancellationToken::new();
                    {
                        let mut active = active.lock().await;
                        if active.contains_key(session_id) {
                            send_error(&writer, id, -32000, "session is busy").await?;
                            continue;
                        }
                        active.insert(session_id.to_owned(), cancel.clone());
                    }
                    let server = self.clone();
                    let writer = Arc::clone(&writer);
                    let active = Arc::clone(&active);
                    let prompt_rpc = Arc::clone(&rpc);
                    let session_id = session_id.to_owned();
                    tokio::spawn(async move {
                        let result = server
                            .run_prompt(
                                &session_id,
                                &prompt,
                                cancel,
                                Arc::clone(&writer),
                                prompt_rpc,
                            )
                            .await;
                        match result {
                            Ok(()) => {
                                send(
                                    &writer,
                                    json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {"stopReason": "end_turn"}
                                    }),
                                )
                                .await
                                .ok();
                            }
                            Err(PromptError::Cancelled) => {
                                send(
                                    &writer,
                                    json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {"stopReason": "cancelled"}
                                    }),
                                )
                                .await
                                .ok();
                            }
                            Err(PromptError::Failed(message)) => {
                                send_error(&writer, id, -32000, &message).await.ok();
                            }
                        }
                        active.lock().await.remove(&session_id);
                    });
                }
                "session/cancel" => {
                    if let Some(session_id) = params["sessionId"].as_str()
                        && let Some(cancel) = active.lock().await.get(session_id)
                    {
                        cancel.cancel();
                    }
                }
                _ => {
                    if let Some(id) = id {
                        send_error(&writer, id, -32601, "method not found").await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn run_prompt(
        &self,
        session_id: &str,
        prompt: &str,
        cancel: CancellationToken,
        writer: Arc<Mutex<BufWriter<impl tokio::io::AsyncWrite + Unpin + Send + 'static>>>,
        rpc: Arc<dyn ClientRpc>,
    ) -> Result<(), PromptError> {
        let session = self
            .store
            .open(session_id)
            .map_err(|error| PromptError::Failed(error.to_string()))?;
        let observer = AcpObserver {
            session_id: session_id.to_owned(),
            writer: Arc::clone(&writer),
        };
        let policy = ApprovalPolicy {
            mode: self.config.mode,
            workspace: PathBuf::from(&session.header.cwd),
            approver: Arc::new(AcpApprover {
                session_id: session_id.to_owned(),
                rpc,
            }),
        };
        let config = AgentConfig {
            system_prompt: self.config.system_prompt.clone(),
            max_steps: self.config.max_steps,
            max_tokens: self.config.max_tokens,
            output_limit: self.config.output_limit,
            compaction: Some(dshr_compaction::CompactionConfig::default()),
            budget: None,
        };
        let mut agent = Agent::new(
            Arc::clone(&self.provider),
            Arc::clone(&self.tools),
            Arc::clone(&self.store),
            session,
            policy,
            config,
        );
        match agent.run_turn(prompt, cancel, &observer).await {
            Ok(_) => Ok(()),
            Err(AgentError::Cancelled) => Err(PromptError::Cancelled),
            Err(error) => Err(PromptError::Failed(error.to_string())),
        }
    }
}

struct AcpApprover {
    session_id: String,
    rpc: Arc<dyn ClientRpc>,
}

#[async_trait]
impl Approver for AcpApprover {
    async fn approve(&self, request: ApprovalRequest) -> bool {
        match self
            .rpc
            .request(
                "session/request_permission",
                json!({
                    "sessionId": self.session_id,
                    "toolCall": {
                        "toolCallId": format!("permission-{}", request.tool),
                        "title": request.tool,
                        "kind": "other"
                    },
                    "options": [
                        {"optionId": "allow-once", "name": "Allow once", "kind": "allow_once"},
                        {"optionId": "reject-once", "name": "Reject", "kind": "reject_once"}
                    ]
                }),
            )
            .await
        {
            Ok(response) => {
                response["outcome"]["outcome"].as_str() == Some("selected")
                    && response["outcome"]["optionId"].as_str() == Some("allow-once")
            }
            Err(_) => false,
        }
    }
}

enum PromptError {
    Cancelled,
    Failed(String),
}

struct AcpObserver<W> {
    session_id: String,
    writer: Arc<Mutex<BufWriter<W>>>,
}

#[async_trait]
impl<W> AgentObserver for AcpObserver<W>
where
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    async fn on_text(&self, text: &str) {
        let _ = send(
            &self.writer,
            json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": self.session_id,
                    "update": {
                        "sessionUpdate": "agent_message_chunk",
                        "content": {"type": "text", "text": text}
                    }
                }
            }),
        )
        .await;
    }

    async fn on_tool_start(&self, call: &ToolCallInfo) {
        let _ = send(
            &self.writer,
            json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": self.session_id,
                    "update": {
                        "sessionUpdate": "tool_call",
                        "toolCallId": call.id,
                        "title": call.name,
                        "kind": "other",
                        "status": "pending"
                    }
                }
            }),
        )
        .await;
    }

    async fn on_tool_end(&self, call: &ToolCallInfo, outcome: &ToolOutcome) {
        let _ = send(
            &self.writer,
            json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": self.session_id,
                    "update": {
                        "sessionUpdate": "tool_call_update",
                        "toolCallId": call.id,
                        "status": if outcome.is_error { "failed" } else { "completed" }
                    }
                }
            }),
        )
        .await;
    }
}

pub(crate) async fn send<W>(writer: &Arc<Mutex<BufWriter<W>>>, value: Value) -> Result<(), AcpError>
where
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    let mut writer = writer.lock().await;
    let text =
        serde_json::to_string(&value).map_err(|error| AcpError::Protocol(error.to_string()))?;
    writer
        .write_all(text.as_bytes())
        .await
        .map_err(|error| AcpError::Io(error.to_string()))?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|error| AcpError::Io(error.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|error| AcpError::Io(error.to_string()))
}

async fn send_error<W>(
    writer: &Arc<Mutex<BufWriter<W>>>,
    id: Value,
    code: i64,
    message: &str,
) -> Result<(), AcpError>
where
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    send(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message}
        }),
    )
    .await
}

fn prompt_text(value: &Value) -> String {
    value
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| {
                    if block["type"].as_str() == Some("text") {
                        block["text"].as_str()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
