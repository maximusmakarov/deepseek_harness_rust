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
use std::time::Duration;

use async_trait::async_trait;
use dshr_llm::ToolDef;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

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

pub struct McpClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    name: String,
}

impl McpClient {
    pub async fn connect(spec: McpServerSpec) -> Result<Self, McpError> {
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

            // A server request we do not implement: answer instead of hanging.
            if value.get("id").is_some() && value["method"].is_string() {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": value["id"].clone(),
                    "error": {"code": -32601, "message": "method not supported by dshr"}
                });
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

/// Registers one MCP tool under `mcp__<server>__<tool>`.
pub struct McpTool {
    client: Arc<Mutex<McpClient>>,
    server: String,
    remote_name: String,
    namespaced: String,
    description: String,
    schema: Value,
}

impl McpTool {
    #[must_use]
    pub fn new(client: Arc<Mutex<McpClient>>, server: &str, info: &McpToolInfo) -> Self {
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
        let mut client = self.client.lock().await;
        let result = client
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
) -> Result<(Arc<Mutex<McpClient>>, Vec<Arc<McpTool>>), McpError> {
    let name = spec.name.clone();
    let client = Arc::new(Mutex::new(McpClient::connect(spec).await?));
    let tools = {
        let mut guard = client.lock().await;
        guard.list_tools().await?
    };
    let adapters = tools
        .iter()
        .map(|tool| Arc::new(McpTool::new(Arc::clone(&client), &name, tool)))
        .collect();
    Ok((client, adapters))
}
