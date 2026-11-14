#![forbid(unsafe_code)]
//! Minimal Agent Client Protocol v1 server over stdio.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::oneshot;

mod fs;
pub use fs::{AcpFsClient, AcpRpc, ClientRpc};
pub use question::{AcpPrompt, AcpQuestionClient};
mod fs_tools;
mod question;
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
use dshr_mcp::{McpServerSpec, connect_server};
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
    questions: Arc<AcpQuestionClient>,
    session_mcp: Arc<Mutex<HashMap<String, Value>>>,
    session_modes: Arc<Mutex<HashMap<String, String>>>,
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
            questions: Arc::new(AcpQuestionClient::new()),
            session_mcp: Arc::new(Mutex::new(HashMap::new())),
            session_modes: Arc::new(Mutex::new(HashMap::new())),
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
                    let elicitation = &params["clientCapabilities"]["elicitation"];
                    let elicitation_enabled = elicitation
                        .as_bool()
                        .or_else(|| elicitation["form"].as_bool())
                        .unwrap_or(false);
                    self.questions
                        .attach(Arc::clone(&rpc), elicitation_enabled)
                        .await;
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
                                        "sessionCapabilities": {
                                            "resume": {},
                                            "close": {}
                                        },
                                        "promptCapabilities": {
                                            "image": true,
                                            "audio": self.provider.capabilities().audio,
                                            "embeddedContext": true
                                        },
                                        "mcpCapabilities": {
                                            "http": true,
                                            "sse": false
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
                "session/list" => {
                    let sessions = self
                        .store
                        .list()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|summary| {
                            let cwd = self
                                .store
                                .open(summary.id.as_str())
                                .map(|session| session.header.cwd)
                                .unwrap_or_default();
                            json!({
                                "sessionId": summary.id.as_str(),
                                "cwd": cwd,
                                "title": summary.title,
                                "updatedAt": summary.created_at,
                                "provider": summary.provider,
                                "model": summary.model,
                                "turns": summary.turns
                            })
                        })
                        .collect::<Vec<_>>();
                    if let Some(id) = id {
                        send(
                            &writer,
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {"sessions": sessions}
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
                            self.session_mcp.lock().await.insert(
                                session.header.id.to_string(),
                                params["mcpServers"].clone(),
                            );
                            send(
                                &writer,
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "sessionId": session.header.id.as_str(),
                                        "modes": {
                                            "currentModeId": "default",
                                            "availableModes": [
                                                {"id": "default", "name": "Default"},
                                                {"id": "read-only", "name": "Read only"},
                                                {"id": "workspace-write", "name": "Workspace write"},
                                                {"id": "danger-full", "name": "Danger full access"}
                                            ]
                                        }
                                    }
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
                "session/resume" => {
                    let Some(id) = id else { continue };
                    let Some(session_id) = params["sessionId"].as_str() else {
                        send_error(&writer, id, -32602, "sessionId is required").await?;
                        continue;
                    };
                    match self.store.open(session_id) {
                        Ok(_) => {
                            send(&writer, json!({"jsonrpc": "2.0", "id": id, "result": {}}))
                                .await?;
                        }
                        Err(error) => {
                            send_error(&writer, id, -32002, &error.to_string()).await?;
                        }
                    }
                }
                "session/close" => {
                    let Some(id) = id else { continue };
                    let Some(session_id) = params["sessionId"].as_str() else {
                        send_error(&writer, id, -32602, "sessionId is required").await?;
                        continue;
                    };
                    if let Some(cancel) = active.lock().await.remove(session_id) {
                        cancel.cancel();
                    }
                    self.session_mcp.lock().await.remove(session_id);
                    send(&writer, json!({"jsonrpc": "2.0", "id": id, "result": {}})).await?;
                }
                "session/prompt" => {
                    let Some(id) = id else { continue };
                    let Some(session_id) = params["sessionId"].as_str() else {
                        send_error(&writer, id, -32602, "sessionId is required").await?;
                        continue;
                    };
                    let prompt =
                        match prompt_content(&params["prompt"], self.provider.capabilities().audio)
                        {
                            Ok(prompt) => prompt,
                            Err(message) => {
                                send_error(&writer, id, -32602, &message).await?;
                                continue;
                            }
                        };
                    if prompt.is_empty() {
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
                                prompt,
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
                "session/set_mode" => {
                    let session_id = params["sessionId"].as_str().unwrap_or_default();
                    let requested = params["modeId"].as_str().unwrap_or_default();
                    let mode = match requested {
                        "default" => None,
                        "read-only" | "plan" => Some(PermissionMode::ReadOnly),
                        "workspace-write" | "acceptEdits" => Some(PermissionMode::WorkspaceWrite),
                        "danger-full" | "bypassPermissions" => Some(PermissionMode::DangerFull),
                        _ => {
                            if let Some(id) = id {
                                send_error(&writer, id, -32602, "unknown mode").await?;
                            }
                            continue;
                        }
                    };
                    if let Some(mode) = mode {
                        self.session_modes
                            .lock()
                            .await
                            .insert(session_id.to_owned(), format!("{mode:?}"));
                    } else {
                        self.session_modes.lock().await.remove(session_id);
                    }
                    if let Some(id) = id {
                        send(&writer, json!({"jsonrpc": "2.0", "id": id, "result": {}})).await?;
                    }
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
        prompt: Vec<dshr_llm::ContentBlock>,
        cancel: CancellationToken,
        writer: Arc<Mutex<BufWriter<impl tokio::io::AsyncWrite + Unpin + Send + 'static>>>,
        rpc: Arc<dyn ClientRpc>,
    ) -> Result<(), PromptError> {
        let session = self
            .store
            .open(session_id)
            .map_err(|error| PromptError::Failed(error.to_string()))?;
        let mcp_servers = self
            .session_mcp
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or(Value::Null);
        let mut tools = (*self.tools).clone();
        if let Some(servers) = mcp_servers.as_array() {
            for server in servers {
                let adapters = connect_acp_mcp(server).await.map_err(PromptError::Failed)?;
                for adapter in adapters {
                    tools.register_arc(adapter);
                }
            }
        }
        let tools = Arc::new(tools);
        let observer = AcpObserver {
            session_id: session_id.to_owned(),
            writer: Arc::clone(&writer),
            questions: Arc::clone(&self.questions),
        };
        let mode = self
            .session_modes
            .lock()
            .await
            .get(session_id)
            .and_then(|value| match value.as_str() {
                "ReadOnly" => Some(PermissionMode::ReadOnly),
                "WorkspaceWrite" => Some(PermissionMode::WorkspaceWrite),
                "DangerFull" => Some(PermissionMode::DangerFull),
                _ => None,
            })
            .unwrap_or(self.config.mode);
        let policy = ApprovalPolicy {
            mode,
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
            tools,
            Arc::clone(&self.store),
            session,
            policy,
            config,
        );
        match agent.run_turn_with_content(prompt, cancel, &observer).await {
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
    questions: Arc<AcpQuestionClient>,
}

#[async_trait]
impl<W> AgentObserver for AcpObserver<W>
where
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    fn prompt(&self) -> Option<Arc<dyn dshr_tools::UserPrompt>> {
        Some(Arc::new(AcpPrompt {
            client: Arc::clone(&self.questions),
            session_id: self.session_id.clone(),
        }))
    }

    async fn on_question(&self, question: &str, options: &[String]) -> Option<String> {
        self.questions
            .ask(&self.session_id, question, options)
            .await
    }

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

async fn connect_acp_mcp(server: &Value) -> Result<Vec<Arc<dyn dshr_tools::Tool>>, String> {
    let name = server["name"]
        .as_str()
        .ok_or_else(|| "mcp server name is required".to_owned())?;
    let transport = server["type"].as_str().unwrap_or_default();
    let spec = if transport == "http" || (transport.is_empty() && server["url"].is_string()) {
        let Some(url) = server["url"].as_str().filter(|value| !value.is_empty()) else {
            return Err(format!("MCP server {name}: url is required for HTTP"));
        };
        McpServerSpec {
            name: name.to_owned(),
            command: String::new(),
            args: Vec::new(),
            env: std::collections::BTreeMap::new(),
            cwd: None,
            url: Some(url.to_owned()),
            headers: env_pairs(&server["headers"]),
            oauth: None,
        }
    } else if transport == "sse" {
        return Err(format!(
            "MCP server {name}: SSE transport is not supported; use http or stdio"
        ));
    } else {
        let Some(command) = server["command"].as_str().filter(|value| !value.is_empty()) else {
            return Err(format!(
                "MCP server {name}: command is required for stdio transport"
            ));
        };
        McpServerSpec {
            name: name.to_owned(),
            command: command.to_owned(),
            args: server["args"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect(),
            env: env_pairs(&server["env"]),
            cwd: None,
            url: None,
            headers: std::collections::BTreeMap::new(),
            oauth: None,
        }
    };
    let (_backend, adapters) = connect_server(spec)
        .await
        .map_err(|error| format!("MCP server {name}: {error}"))?;
    Ok(adapters)
}

fn env_pairs(value: &Value) -> std::collections::BTreeMap<String, String> {
    let mut pairs = std::collections::BTreeMap::new();
    if let Some(values) = value.as_array() {
        for pair in values {
            if let (Some(name), Some(value)) = (pair["name"].as_str(), pair["value"].as_str()) {
                pairs.insert(name.to_owned(), value.to_owned());
            }
        }
    }
    pairs
}

fn prompt_content(
    value: &Value,
    audio_supported: bool,
) -> Result<Vec<dshr_llm::ContentBlock>, String> {
    let mut blocks = Vec::new();
    for block in value.as_array().into_iter().flatten() {
        match block["type"].as_str() {
            Some("text") => {
                if let Some(text) = block["text"].as_str() {
                    blocks.push(dshr_llm::ContentBlock::text(text));
                }
            }
            Some("image") => {
                let media_type = block["mimeType"]
                    .as_str()
                    .ok_or_else(|| "image block requires mimeType".to_owned())?;
                let data = block["data"]
                    .as_str()
                    .ok_or_else(|| "image block requires data".to_owned())?;
                blocks.push(dshr_llm::ContentBlock::Image {
                    media_type: media_type.to_owned(),
                    data: data.to_owned(),
                });
            }
            Some("resource") => {
                let resource = &block["resource"];
                let uri = resource["uri"].as_str().unwrap_or("embedded");
                if let Some(text) = resource["text"].as_str() {
                    blocks.push(dshr_llm::ContentBlock::text(text));
                } else if let Some(blob) = resource["blob"].as_str() {
                    let media_type = resource["mimeType"]
                        .as_str()
                        .ok_or_else(|| format!("resource {uri}: blob requires mimeType"))?;
                    if media_type.starts_with("image/") {
                        blocks.push(dshr_llm::ContentBlock::Image {
                            media_type: media_type.to_owned(),
                            data: blob.to_owned(),
                        });
                    } else if media_type.starts_with("audio/") {
                        if !audio_supported {
                            return Err(
                                "the configured provider does not support audio prompts".to_owned()
                            );
                        }
                        blocks.push(dshr_llm::ContentBlock::Audio {
                            media_type: media_type.to_owned(),
                            data: blob.to_owned(),
                        });
                    } else {
                        return Err(format!(
                            "resource {uri}: unsupported embedded mime type {media_type}"
                        ));
                    }
                } else {
                    return Err(format!("resource {uri} has neither text nor blob"));
                }
            }
            Some("resource_link") => {
                let uri = block["uri"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "resource_link block requires uri".to_owned())?;
                let name = block["name"].as_str().unwrap_or(uri);
                let mut text = format!("[resource_link] {name} <{uri}>");
                if let Some(mime) = block["mimeType"].as_str() {
                    text.push_str(&format!(" ({mime})"));
                }
                if let Some(description) = block["description"].as_str() {
                    text.push_str(&format!(" — {description}"));
                }
                blocks.push(dshr_llm::ContentBlock::text(text));
            }
            Some("audio") => {
                if !audio_supported {
                    return Err("the configured provider does not support audio prompts".to_owned());
                }
                let media_type = block["mimeType"]
                    .as_str()
                    .ok_or_else(|| "audio block requires mimeType".to_owned())?;
                let data = block["data"]
                    .as_str()
                    .ok_or_else(|| "audio block requires data".to_owned())?;
                blocks.push(dshr_llm::ContentBlock::Audio {
                    media_type: media_type.to_owned(),
                    data: data.to_owned(),
                });
            }
            _ => {}
        }
    }
    Ok(blocks)
}
