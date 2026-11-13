#![forbid(unsafe_code)]
//! Minimal Model Context Protocol client over stdio.
//!
//! Implements the subset the harness needs: `initialize`, `tools/list`, and
//! `tools/call`. Responses are newline-delimited JSON-RPC 2.0. Server requests
//! that we do not implement are answered with `-32601` instead of being
//! silently dropped.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use dshr_llm::{LlmProvider, Message, Request, StreamChunk, ToolDef};
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use futures_util::StreamExt;
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Error)]
pub enum McpError {
    #[error("spawn MCP server: {0}")]
    Spawn(String),

    #[error("MCP I/O: {0}")]
    Io(String),

    #[error("MCP protocol: {0}")]
    Protocol(String),

    #[error("MCP request timed out")]
    Timeout,

    #[error("MCP error {code}: {message}")]
    Rpc { code: i64, message: String },
}

#[derive(Debug, Clone)]
pub struct McpServerSpec {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    /// When set, use Streamable HTTP instead of stdio.
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct McpCallResult {
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone)]
pub struct McpPromptInfo {
    pub name: String,
    pub description: String,
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct McpPromptResult {
    pub description: String,
    pub messages: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
}

#[derive(Clone)]
pub enum McpBackend {
    Stdio(Arc<Mutex<McpClient>>),
    Http(Arc<McpHttpClient>),
}

impl McpBackend {
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpCallResult, McpError> {
        match self {
            Self::Stdio(client) => client.lock().await.call_tool(name, arguments).await,
            Self::Http(client) => client.call_tool(name, arguments).await,
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        match self {
            Self::Stdio(client) => client.lock().await.list_resources().await,
            Self::Http(client) => client.list_resources().await,
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError> {
        match self {
            Self::Stdio(client) => client.lock().await.read_resource(uri).await,
            Self::Http(client) => client.read_resource(uri).await,
        }
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPromptInfo>, McpError> {
        match self {
            Self::Stdio(client) => client.lock().await.list_prompts().await,
            Self::Http(client) => client.list_prompts().await,
        }
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpPromptResult, McpError> {
        match self {
            Self::Stdio(client) => client.lock().await.get_prompt(name, arguments).await,
            Self::Http(client) => client.get_prompt(name, arguments).await,
        }
    }
}

pub struct McpClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    name: String,
    sampling: Option<Arc<dyn LlmProvider>>,
}

impl McpClient {
    pub async fn connect(
        spec: McpServerSpec,
        sampling: Option<Arc<dyn LlmProvider>>,
    ) -> Result<Self, McpError> {
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .envs(&spec.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }

        let mut child = command
            .spawn()
            .map_err(|error| McpError::Spawn(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Spawn("missing stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Spawn("missing stdout".into()))?;
        if let Some(stderr) = child.stderr.take() {
            let name = spec.name.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(server = %name, "mcp: {line}");
                }
            });
        }

        let mut client = Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            name: spec.name,
            sampling,
        };
        client.initialize().await?;
        Ok(client)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&mut self) -> Result<(), McpError> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "dshr", "version": env!("CARGO_PKG_VERSION")}
            }),
        )
        .await?;
        self.notify("notifications/initialized", json!({})).await
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpToolInfo>, McpError> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = match &cursor {
                Some(cursor) => json!({ "cursor": cursor }),
                None => json!({}),
            };
            let result = self.request("tools/list", params).await?;
            if let Some(items) = result["tools"].as_array() {
                for item in items {
                    tools.push(McpToolInfo {
                        name: item["name"].as_str().unwrap_or_default().to_owned(),
                        description: item["description"].as_str().unwrap_or_default().to_owned(),
                        input_schema: item["inputSchema"].clone(),
                    });
                }
            }
            cursor = result["nextCursor"].as_str().map(ToOwned::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> Result<McpCallResult, McpError> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        let mut content = String::new();
        if let Some(blocks) = result["content"].as_array() {
            for block in blocks {
                if let Some(text) = block["text"].as_str() {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(text);
                } else if let Some(kind) = block["type"].as_str() {
                    content.push_str(&format!("[{kind}]"));
                }
            }
        }
        Ok(McpCallResult {
            content,
            is_error: result["isError"].as_bool().unwrap_or(false),
        })
    }

    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>, McpError> {
        let result = self.request("resources/list", json!({})).await?;
        Ok(parse_resources(&result))
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<String, McpError> {
        let result = self.request("resources/read", json!({"uri": uri})).await?;
        Ok(parse_resource_text(&result))
    }

    pub async fn list_prompts(&mut self) -> Result<Vec<McpPromptInfo>, McpError> {
        let result = self.request("prompts/list", json!({})).await?;
        Ok(parse_prompts(&result))
    }

    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> Result<McpPromptResult, McpError> {
        let result = self
            .request("prompts/get", json!({"name": name, "arguments": arguments}))
            .await?;
        Ok(parse_prompt_result(&result))
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), McpError> {
        let message = json!({"jsonrpc": "2.0", "method": method, "params": params});
        self.write_message(&message).await
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_message(&message).await?;

        let timeout = tokio::time::sleep(REQUEST_TIMEOUT);
        tokio::pin!(timeout);
        loop {
            let mut line = String::new();
            let read = tokio::select! {
                _ = &mut timeout => return Err(McpError::Timeout),
                read = self.stdout.read_line(&mut line) => read,
            }
            .map_err(|error| McpError::Io(error.to_string()))?;
            if read == 0 {
                return Err(McpError::Protocol("server closed stdout".into()));
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(line)
                .map_err(|error| McpError::Protocol(format!("invalid JSON: {error}")))?;

            if value["id"].as_u64() == Some(id) {
                if let Some(error) = value.get("error") {
                    return Err(McpError::Rpc {
                        code: error["code"].as_i64().unwrap_or(-1),
                        message: error["message"].as_str().unwrap_or("unknown").to_owned(),
                    });
                }
                return Ok(value["result"].clone());
            }

            if value.get("id").is_some() && value["method"].is_string() {
                let method = value["method"].as_str().unwrap_or_default();
                let response = if method == "sampling/createMessage" {
                    match &self.sampling {
                        Some(provider) => {
                            sampling_response(
                                provider,
                                value["id"].clone(),
                                value["params"].clone(),
                            )
                            .await
                        }
                        None => json!({
                            "jsonrpc": "2.0",
                            "id": value["id"].clone(),
                            "error": {"code": -32601, "message": "sampling is not enabled"}
                        }),
                    }
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": value["id"].clone(),
                        "error": {"code": -32601, "message": "method not supported by dshr"}
                    })
                };
                self.write_message(&response).await?;
            }
        }
    }

    async fn write_message(&mut self, message: &Value) -> Result<(), McpError> {
        let mut line = serde_json::to_vec(message)
            .map_err(|error| McpError::Protocol(format!("serialize: {error}")))?;
        line.push(b'\n');
        self.stdin
            .write_all(&line)
            .await
            .map_err(|error| McpError::Io(error.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| McpError::Io(error.to_string()))
    }
}

pub struct McpHttpClient {
    name: String,
    url: String,
    headers: BTreeMap<String, String>,
    client: reqwest::Client,
    session_id: Mutex<Option<String>>,
    next_id: AtomicU64,
    sampling: Option<Arc<dyn LlmProvider>>,
}

impl McpHttpClient {
    pub async fn connect(
        spec: McpServerSpec,
        sampling: Option<Arc<dyn LlmProvider>>,
    ) -> Result<Arc<Self>, McpError> {
        let url = spec
            .url
            .clone()
            .ok_or_else(|| McpError::Spawn("HTTP MCP server has no url".into()))?;
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| McpError::Io(error.to_string()))?;
        let this = Self {
            name: spec.name,
            url,
            headers: spec.headers,
            client,
            session_id: Mutex::new(None),
            next_id: AtomicU64::new(1),
            sampling,
        };
        let this = Arc::new(this);
        this.initialize().await?;
        if this.sampling.is_some() {
            tokio::spawn(start_http_sse_listener(Arc::clone(&this)));
        }
        Ok(this)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&self) -> Result<(), McpError> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "dshr", "version": env!("CARGO_PKG_VERSION")}
            }),
        )
        .await?;
        self.notify("notifications/initialized", json!({})).await
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>, McpError> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = match &cursor {
                Some(cursor) => json!({ "cursor": cursor }),
                None => json!({}),
            };
            let result = self.request("tools/list", params).await?;
            if let Some(items) = result["tools"].as_array() {
                for item in items {
                    tools.push(McpToolInfo {
                        name: item["name"].as_str().unwrap_or_default().to_owned(),
                        description: item["description"].as_str().unwrap_or_default().to_owned(),
                        input_schema: item["inputSchema"].clone(),
                    });
                }
            }
            cursor = result["nextCursor"].as_str().map(ToOwned::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpCallResult, McpError> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        Ok(parse_call_result(&result))
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        let result = self.request("resources/list", json!({})).await?;
        Ok(parse_resources(&result))
    }

    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError> {
        let result = self.request("resources/read", json!({"uri": uri})).await?;
        Ok(parse_resource_text(&result))
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPromptInfo>, McpError> {
        let result = self.request("prompts/list", json!({})).await?;
        Ok(parse_prompts(&result))
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpPromptResult, McpError> {
        let result = self
            .request("prompts/get", json!({"name": name, "arguments": arguments}))
            .await?;
        Ok(parse_prompt_result(&result))
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), McpError> {
        let body = json!({"jsonrpc": "2.0", "method": method, "params": params});
        let mut request = self
            .client
            .post(&self.url)
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        if let Some(session) = self.session_id.lock().await.as_deref() {
            request = request.header("Mcp-Session-Id", session);
        }
        let response = request
            .send()
            .await
            .map_err(|error| McpError::Io(error.to_string()))?;
        self.capture_session(&response).await;
        let status = response.status();
        if !status.is_success() {
            return Err(McpError::Io(format!("HTTP {status}")));
        }
        Ok(())
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let mut request = self
            .client
            .post(&self.url)
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        if let Some(session) = self.session_id.lock().await.as_deref() {
            request = request.header("Mcp-Session-Id", session);
        }
        let response = request
            .send()
            .await
            .map_err(|error| McpError::Io(error.to_string()))?;
        self.capture_session(&response).await;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let text = response
            .text()
            .await
            .map_err(|error| McpError::Io(error.to_string()))?;
        if !status.is_success() {
            return Err(McpError::Io(format!("HTTP {status}: {text}")));
        }
        if content_type.contains("text/event-stream") {
            parse_sse_response(&text, id)
        } else {
            let value: Value = serde_json::from_str(&text)
                .map_err(|error| McpError::Protocol(format!("invalid JSON: {error}")))?;
            extract_response(value, id)
        }
    }

    async fn post_json(&self, value: &Value) -> Result<(), McpError> {
        let mut request = self
            .client
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(value);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        if let Some(session) = self.session_id.lock().await.as_deref() {
            request = request.header("Mcp-Session-Id", session);
        }
        let response = request
            .send()
            .await
            .map_err(|error| McpError::Io(error.to_string()))?;
        if !response.status().is_success() {
            return Err(McpError::Io(format!("HTTP {}", response.status())));
        }
        Ok(())
    }

    async fn capture_session(&self, response: &reqwest::Response) {
        if let Some(session) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
        {
            *self.session_id.lock().await = Some(session.to_owned());
        }
    }
}

async fn start_http_sse_listener(client: Arc<McpHttpClient>) {
    let mut request = client
        .client
        .get(&client.url)
        .header(reqwest::header::ACCEPT, "text/event-stream");
    for (key, value) in &client.headers {
        request = request.header(key, value);
    }
    if let Some(session) = client.session_id.lock().await.as_deref() {
        request = request.header("Mcp-Session-Id", session);
    }
    let Ok(response) = request.send().await else {
        return;
    };
    if !response.status().is_success() {
        return;
    }
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            break;
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(position) = buffer.find("\n\n") {
            let event = buffer[..position].to_owned();
            buffer.drain(..position + 2);
            handle_sse_event(&client, &event).await;
        }
    }
}

async fn handle_sse_event(client: &Arc<McpHttpClient>, event: &str) {
    let Some(data) = parse_sse_data(event) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&data) else {
        return;
    };
    if value["method"].as_str() != Some("sampling/createMessage") || value.get("id").is_none() {
        return;
    }
    let Some(provider) = &client.sampling else {
        return;
    };
    let response = sampling_response(provider, value["id"].clone(), value["params"].clone()).await;
    let _ = client.post_json(&response).await;
}

fn parse_sse_data(event: &str) -> Option<String> {
    let data = event
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("data:"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");
    (!data.is_empty()).then_some(data)
}

fn extract_response(value: Value, id: u64) -> Result<Value, McpError> {
    if let Some(error) = value.get("error") {
        return Err(McpError::Rpc {
            code: error["code"].as_i64().unwrap_or(-1),
            message: error["message"].as_str().unwrap_or("unknown").to_owned(),
        });
    }
    if value["id"].as_u64().is_some_and(|actual| actual != id) {
        return Err(McpError::Protocol(format!(
            "unexpected response id: {}",
            value["id"]
        )));
    }
    Ok(value["result"].clone())
}

fn parse_sse_response(text: &str, id: u64) -> Result<Value, McpError> {
    let mut data = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            data.push_str(rest.trim_start());
            data.push('\n');
        } else if line.trim().is_empty() && !data.is_empty() {
            let value: Value = serde_json::from_str(data.trim())
                .map_err(|error| McpError::Protocol(format!("invalid SSE JSON: {error}")))?;
            data.clear();
            if value["id"].as_u64() == Some(id) {
                return extract_response(value, id);
            }
        }
    }
    if !data.trim().is_empty() {
        let value: Value = serde_json::from_str(data.trim())
            .map_err(|error| McpError::Protocol(format!("invalid SSE JSON: {error}")))?;
        if value["id"].as_u64() == Some(id) {
            return extract_response(value, id);
        }
    }
    Err(McpError::Protocol(
        "SSE response without matching id".into(),
    ))
}

fn parse_call_result(result: &Value) -> McpCallResult {
    let mut content = String::new();
    if let Some(blocks) = result["content"].as_array() {
        for block in blocks {
            if let Some(text) = block["text"].as_str() {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(text);
            } else if let Some(kind) = block["type"].as_str() {
                content.push_str(&format!("[{kind}]"));
            }
        }
    }
    McpCallResult {
        content,
        is_error: result["isError"].as_bool().unwrap_or(false),
    }
}

fn parse_prompts(result: &Value) -> Vec<McpPromptInfo> {
    result["prompts"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| McpPromptInfo {
                    name: item["name"].as_str().unwrap_or_default().to_owned(),
                    description: item["description"].as_str().unwrap_or_default().to_owned(),
                    arguments: item["arguments"]
                        .as_array()
                        .map(|arguments| {
                            arguments
                                .iter()
                                .map(|argument| McpPromptArgument {
                                    name: argument["name"].as_str().unwrap_or_default().to_owned(),
                                    description: argument["description"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_owned(),
                                    required: argument["required"].as_bool().unwrap_or(false),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_prompt_result(result: &Value) -> McpPromptResult {
    McpPromptResult {
        description: result["description"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        messages: result["messages"]
            .as_array()
            .map(|messages| {
                messages
                    .iter()
                    .map(|message| {
                        (
                            message["role"].as_str().unwrap_or("user").to_owned(),
                            message["content"]["text"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn parse_resources(result: &Value) -> Vec<McpResource> {
    result["resources"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| McpResource {
                    uri: item["uri"].as_str().unwrap_or_default().to_owned(),
                    name: item["name"].as_str().unwrap_or_default().to_owned(),
                    description: item["description"].as_str().unwrap_or_default().to_owned(),
                    mime_type: item["mimeType"].as_str().map(ToOwned::to_owned),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_resource_text(result: &Value) -> String {
    let mut text = String::new();
    if let Some(contents) = result["contents"].as_array() {
        for item in contents {
            if let Some(value) = item["text"].as_str() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(value);
            } else if item["blob"].is_string() {
                text.push_str("[binary resource]");
            }
        }
    }
    text
}

/// Registers one MCP tool under `mcp__<server>__<tool>`.
pub struct McpTool {
    client: McpBackend,
    server: String,
    remote_name: String,
    namespaced: String,
    description: String,
    schema: Value,
}

impl McpTool {
    #[must_use]
    pub fn new(client: McpBackend, server: &str, info: &McpToolInfo) -> Self {
        Self {
            client,
            server: server.to_owned(),
            remote_name: info.name.clone(),
            namespaced: format!("mcp__{server}__{}", info.name),
            description: format!("[MCP:{server}] {}", info.description),
            schema: normalize_schema(&info.input_schema),
        }
    }

    #[must_use]
    pub fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.namespaced.clone(),
            description: self.description.clone(),
            input_schema: self.schema.clone(),
        }
    }

    #[must_use]
    pub fn server(&self) -> &str {
        &self.server
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let result = self
            .client
            .call_tool(&self.remote_name, args)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome {
            content: result.content,
            is_error: result.is_error,
            files: Vec::new(),
        })
    }
}

pub struct McpListPromptsTool {
    client: McpBackend,
    namespaced: String,
    description: String,
}

impl McpListPromptsTool {
    #[must_use]
    pub fn new(client: McpBackend, server: &str) -> Self {
        Self {
            client,
            namespaced: format!("mcp__{server}__list_prompts"),
            description: format!("[MCP:{server}] List prompts"),
        }
    }
}

#[async_trait]
impl Tool for McpListPromptsTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        let prompts = self
            .client
            .list_prompts()
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        if prompts.is_empty() {
            return Ok(ToolOutcome::text("(no prompts)"));
        }
        let text = prompts
            .iter()
            .map(|prompt| {
                let args = prompt
                    .arguments
                    .iter()
                    .map(|argument| argument.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}\t{}\t[{args}]", prompt.name, prompt.description)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutcome::text(text))
    }
}

pub struct McpGetPromptTool {
    client: McpBackend,
    namespaced: String,
    description: String,
}

impl McpGetPromptTool {
    #[must_use]
    pub fn new(client: McpBackend, server: &str) -> Self {
        Self {
            client,
            namespaced: format!("mcp__{server}__get_prompt"),
            description: format!("[MCP:{server}] Get a prompt by name"),
        }
    }
}

#[async_trait]
impl Tool for McpGetPromptTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["name"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "minLength": 1},
                "arguments": {"type": "object"}
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name is required".into()))?;
        let arguments = args.get("arguments").cloned().unwrap_or_else(|| json!({}));
        let prompt = self
            .client
            .get_prompt(name, arguments)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        let mut text = prompt.description;
        for (role, content) in prompt.messages {
            text.push_str(&format!("\n[{role}]\n{content}"));
        }
        Ok(ToolOutcome::text(text))
    }
}

pub struct McpListResourcesTool {
    client: McpBackend,
    namespaced: String,
    description: String,
}

impl McpListResourcesTool {
    #[must_use]
    pub fn new(client: McpBackend, server: &str) -> Self {
        Self {
            client,
            namespaced: format!("mcp__{server}__list_resources"),
            description: format!("[MCP:{server}] List resources"),
        }
    }
}

#[async_trait]
impl Tool for McpListResourcesTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        let resources = self
            .client
            .list_resources()
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        if resources.is_empty() {
            return Ok(ToolOutcome::text("(no resources)"));
        }
        let text = resources
            .iter()
            .map(|resource| {
                format!(
                    "{}\t{}\t{}",
                    resource.uri, resource.name, resource.description
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutcome::text(text))
    }
}

pub struct McpReadResourceTool {
    client: McpBackend,
    namespaced: String,
    description: String,
}

impl McpReadResourceTool {
    #[must_use]
    pub fn new(client: McpBackend, server: &str) -> Self {
        Self {
            client,
            namespaced: format!("mcp__{server}__read_resource"),
            description: format!("[MCP:{server}] Read a resource by URI"),
        }
    }
}

#[async_trait]
impl Tool for McpReadResourceTool {
    fn name(&self) -> &str {
        &self.namespaced
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["uri"],
            "additionalProperties": false,
            "properties": {"uri": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let uri = args["uri"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("uri is required".into()))?;
        let text = self
            .client
            .read_resource(uri)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(if text.is_empty() {
            "(empty resource)".to_owned()
        } else {
            text
        }))
    }
}

async fn sampling_response(provider: &Arc<dyn LlmProvider>, id: Value, params: Value) -> Value {
    let mut messages = Vec::new();
    if let Some(items) = params["messages"].as_array() {
        for item in items {
            let text = if let Some(text) = item["content"]["text"].as_str() {
                text.to_owned()
            } else {
                item["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter_map(|block| block["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default()
            };
            if item["role"].as_str() == Some("assistant") {
                messages.push(Message::assistant(text));
            } else {
                messages.push(Message::user(text));
            }
        }
    }
    if messages.is_empty() {
        return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32602, "message": "sampling request has no messages"}
        });
    }
    let request = Request {
        model: provider.model().to_owned(),
        system: params["systemPrompt"].as_str().map(ToOwned::to_owned),
        messages,
        max_tokens: params["maxTokens"]
            .as_u64()
            .unwrap_or(1024)
            .clamp(1, u32::MAX as u64) as u32,
        ..Request::default()
    };
    let mut stream = match provider.stream(request, CancellationToken::new()).await {
        Ok(stream) => stream,
        Err(error) => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32000, "message": error.to_string()}
            });
        }
    };
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(StreamChunk::TextDelta { text: delta }) => text.push_str(&delta),
            Ok(_) => {}
            Err(error) => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32000, "message": error.to_string()}
                });
            }
        }
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "role": "assistant",
            "content": {"type": "text", "text": text},
            "model": provider.model(),
            "stopReason": "endTurn"
        }
    })
}

fn normalize_schema(schema: &Value) -> Value {
    if schema.is_object() {
        schema.clone()
    } else {
        json!({"type": "object", "properties": {}})
    }
}

/// Connects to a server and returns its tool adapters.
pub async fn connect_server(
    spec: McpServerSpec,
) -> Result<(McpBackend, Vec<Arc<dyn dshr_tools::Tool>>), McpError> {
    connect_server_with_sampling(spec, None).await
}

/// Connects a server with a sampling provider (stdio and HTTP).
pub async fn connect_server_with_sampling(
    spec: McpServerSpec,
    sampling: Option<Arc<dyn LlmProvider>>,
) -> Result<(McpBackend, Vec<Arc<dyn dshr_tools::Tool>>), McpError> {
    let name = spec.name.clone();
    let (backend, tools) = if spec.url.is_some() {
        let client = McpHttpClient::connect(spec, sampling).await?;
        let tools = client.list_tools().await?;
        (McpBackend::Http(client), tools)
    } else {
        let client = Arc::new(Mutex::new(McpClient::connect(spec, sampling).await?));
        let tools = client.lock().await.list_tools().await?;
        (McpBackend::Stdio(client), tools)
    };

    let mut adapters: Vec<Arc<dyn dshr_tools::Tool>> = tools
        .iter()
        .map(|tool| {
            Arc::new(McpTool::new(backend.clone(), &name, tool)) as Arc<dyn dshr_tools::Tool>
        })
        .collect();
    adapters.push(Arc::new(McpListResourcesTool::new(backend.clone(), &name)));
    adapters.push(Arc::new(McpReadResourceTool::new(backend.clone(), &name)));
    adapters.push(Arc::new(McpListPromptsTool::new(backend.clone(), &name)));
    adapters.push(Arc::new(McpGetPromptTool::new(backend.clone(), &name)));
    Ok((backend, adapters))
}

#[cfg(test)]
mod tests {
    use super::parse_sse_data;

    #[test]
    fn joins_multiline_sse_data() {
        assert_eq!(
            parse_sse_data("event: message\ndata: {\"a\":1}\ndata: {\"b\":2}"),
            Some("{\"a\":1}\n{\"b\":2}".to_owned())
        );
        assert_eq!(parse_sse_data("event: ping"), None);
    }
}
