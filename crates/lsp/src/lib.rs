#![forbid(unsafe_code)]
//! Minimal LSP client over stdio and coding tools built on it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, oneshot};
use url::Url;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct LspSpec {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    pub root: PathBuf,
}

#[derive(Debug, Error)]
pub enum LspError {
    #[error("spawn LSP: {0}")]
    Spawn(String),

    #[error("LSP I/O: {0}")]
    Io(String),

    #[error("LSP protocol: {0}")]
    Protocol(String),

    #[error("LSP request timed out")]
    Timeout,

    #[error("LSP error {code}: {message}")]
    Rpc { code: i64, message: String },

    #[error("no LSP server configured for this file")]
    Unsupported,
}

pub struct LspClient {
    _child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    diagnostics: Arc<Mutex<HashMap<String, Vec<Value>>>>,
    next_id: AtomicI64,
    name: String,
}

impl LspClient {
    pub async fn connect(spec: LspSpec) -> Result<Self, LspError> {
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .current_dir(&spec.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|error| LspError::Spawn(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::Spawn("missing stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::Spawn("missing stdout".into()))?;
        if let Some(stderr) = child.stderr.take() {
            let name = spec.name.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(server = %name, "lsp: {line}");
                }
            });
        }

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(Mutex::new(HashMap::new()));
        spawn_reader(stdout, Arc::clone(&pending), Arc::clone(&diagnostics));

        let client = Self {
            _child: child,
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            diagnostics,
            next_id: AtomicI64::new(1),
            name: spec.name,
        };
        let root_uri = Url::from_directory_path(&spec.root)
            .map(|url| url.to_string())
            .unwrap_or_default();
        client
            .request(
                "initialize",
                json!({
                    "processId": Value::Null,
                    "rootUri": root_uri,
                    "capabilities": {},
                    "workspaceFolders": Value::Null
                }),
            )
            .await?;
        client.notify("initialized", json!({})).await?;
        Ok(client)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn did_open(&self, path: &Path, text: &str) -> Result<(), LspError> {
        let uri = file_uri(path)?;
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id(path),
                    "version": 1,
                    "text": text
                }
            }),
        )
        .await
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, LspError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))
        .await?;
        let value = tokio::time::timeout(REQUEST_TIMEOUT, receiver)
            .await
            .map_err(|_| LspError::Timeout)?
            .map_err(|_| LspError::Protocol("server closed".into()))?;
        if let Some(error) = value.get("error") {
            return Err(LspError::Rpc {
                code: error["code"].as_i64().unwrap_or(-1),
                message: error["message"].as_str().unwrap_or("unknown").to_owned(),
            });
        }
        Ok(value["result"].clone())
    }

    pub async fn hover(&self, path: &Path, line: u64, column: u64) -> Result<Value, LspError> {
        self.position_request("textDocument/hover", path, line, column)
            .await
    }

    pub async fn definition(&self, path: &Path, line: u64, column: u64) -> Result<Value, LspError> {
        self.position_request("textDocument/definition", path, line, column)
            .await
    }

    pub async fn references(&self, path: &Path, line: u64, column: u64) -> Result<Value, LspError> {
        let uri = file_uri(path)?;
        self.request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": line.saturating_sub(1), "character": column.saturating_sub(1)},
                "context": {"includeDeclaration": true}
            }),
        )
        .await
    }

    pub async fn diagnostics(&self, path: &Path) -> Result<Vec<Value>, LspError> {
        let uri = file_uri(path)?;
        Ok(self
            .diagnostics
            .lock()
            .await
            .get(&uri)
            .cloned()
            .unwrap_or_default())
    }

    async fn position_request(
        &self,
        method: &str,
        path: &Path,
        line: u64,
        column: u64,
    ) -> Result<Value, LspError> {
        let uri = file_uri(path)?;
        self.request(
            method,
            json!({
                "textDocument": {"uri": uri},
                "position": {
                    "line": line.saturating_sub(1),
                    "character": column.saturating_sub(1)
                }
            }),
        )
        .await
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
        .await
    }

    async fn write_message(&self, message: &Value) -> Result<(), LspError> {
        let body =
            serde_json::to_vec(message).map_err(|error| LspError::Protocol(error.to_string()))?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(header.as_bytes())
            .await
            .map_err(|error| LspError::Io(error.to_string()))?;
        stdin
            .write_all(&body)
            .await
            .map_err(|error| LspError::Io(error.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|error| LspError::Io(error.to_string()))
    }
}

fn spawn_reader(
    stdout: ChildStdout,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    diagnostics: Arc<Mutex<HashMap<String, Vec<Value>>>>,
) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            let Ok(body) = read_message(&mut reader).await else {
                break;
            };
            let Ok(value) = serde_json::from_slice::<Value>(&body) else {
                continue;
            };
            if let Some(id) = value["id"].as_i64() {
                if let Some(sender) = pending.lock().await.remove(&id) {
                    let _ = sender.send(value);
                }
                continue;
            }
            if value["method"].as_str() == Some("textDocument/publishDiagnostics") {
                let uri = value["params"]["uri"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let items = value["params"]["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                diagnostics.lock().await.insert(uri, items);
            }
        }
    });
}

async fn read_message(reader: &mut BufReader<ChildStdout>) -> Result<Vec<u8>, LspError> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .map_err(|error| LspError::Io(error.to_string()))?;
        if read == 0 {
            return Err(LspError::Protocol("server closed stdout".into()));
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            length = value.trim().parse().ok();
        }
    }
    let length = length.ok_or_else(|| LspError::Protocol("missing Content-Length".into()))?;
    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|error| LspError::Io(error.to_string()))?;
    Ok(body)
}

fn file_uri(path: &Path) -> Result<String, LspError> {
    Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| LspError::Protocol(format!("invalid path: {}", path.display())))
}

fn language_id(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => "rust",
        Some("ts" | "tsx") => "typescript",
        Some("js" | "jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("md") => "markdown",
        _ => "plaintext",
    }
}

pub struct LspManager {
    root: PathBuf,
    specs: Vec<LspSpec>,
    clients: Mutex<HashMap<String, Arc<LspClient>>>,
}

impl LspManager {
    #[must_use]
    pub fn new(root: PathBuf, specs: Vec<LspSpec>) -> Self {
        Self {
            root,
            specs,
            clients: Mutex::new(HashMap::new()),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    pub async fn client_for(&self, path: &Path) -> Result<Arc<LspClient>, LspError> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let spec = self
            .specs
            .iter()
            .find(|spec| {
                spec.extensions
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            })
            .cloned()
            .ok_or(LspError::Unsupported)?;
        let key = spec.name.clone();
        if let Some(client) = self.clients.lock().await.get(&key) {
            return Ok(Arc::clone(client));
        }
        let client = Arc::new(LspClient::connect(spec).await?);
        self.clients.lock().await.insert(key, Arc::clone(&client));
        Ok(client)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn path_arg(ctx: &ToolCtx, args: &Value) -> Result<PathBuf, ToolError> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
    Ok(resolve_path(&ctx.cwd, path))
}

async fn open(manager: &LspManager, path: &Path) -> Result<Arc<LspClient>, ToolError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| ToolError::Failed(format!("read {}: {error}", path.display())))?;
    let client = manager
        .client_for(path)
        .await
        .map_err(|error| ToolError::Failed(error.to_string()))?;
    client
        .did_open(path, &text)
        .await
        .map_err(|error| ToolError::Failed(error.to_string()))?;
    Ok(client)
}

pub struct LspHoverTool {
    manager: Arc<LspManager>,
}

impl LspHoverTool {
    #[must_use]
    pub fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for LspHoverTool {
    fn name(&self) -> &str {
        "lsp_hover"
    }

    fn description(&self) -> &str {
        "Show the language server hover information at a position."
    }

    fn schema(&self) -> Value {
        position_schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let line = args["line"].as_u64().unwrap_or(1);
        let column = args["column"].as_u64().unwrap_or(1);
        let client = open(&self.manager, &path).await?;
        let value = client
            .hover(&path, line, column)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(hover_text(&value)))
    }
}

pub struct LspDefinitionTool {
    manager: Arc<LspManager>,
}

impl LspDefinitionTool {
    #[must_use]
    pub fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for LspDefinitionTool {
    fn name(&self) -> &str {
        "lsp_definition"
    }

    fn description(&self) -> &str {
        "Find the definition of a symbol through the language server."
    }

    fn schema(&self) -> Value {
        position_schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let line = args["line"].as_u64().unwrap_or(1);
        let column = args["column"].as_u64().unwrap_or(1);
        let client = open(&self.manager, &path).await?;
        let value = client
            .definition(&path, line, column)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(locations_text(&value)))
    }
}

pub struct LspReferencesTool {
    manager: Arc<LspManager>,
}

impl LspReferencesTool {
    #[must_use]
    pub fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for LspReferencesTool {
    fn name(&self) -> &str {
        "lsp_references"
    }

    fn description(&self) -> &str {
        "Find references of a symbol through the language server."
    }

    fn schema(&self) -> Value {
        position_schema()
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let line = args["line"].as_u64().unwrap_or(1);
        let column = args["column"].as_u64().unwrap_or(1);
        let client = open(&self.manager, &path).await?;
        let value = client
            .references(&path, line, column)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(locations_text(&value)))
    }
}

pub struct LspDiagnosticsTool {
    manager: Arc<LspManager>,
}

impl LspDiagnosticsTool {
    #[must_use]
    pub fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for LspDiagnosticsTool {
    fn name(&self) -> &str {
        "lsp_diagnostics"
    }

    fn description(&self) -> &str {
        "Return language-server diagnostics for a file."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {"path": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let client = open(&self.manager, &path).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        let diagnostics = client
            .diagnostics(&path)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        if diagnostics.is_empty() {
            return Ok(ToolOutcome::text("(no diagnostics)"));
        }
        let lines = diagnostics
            .iter()
            .map(|diagnostic| {
                let severity = match diagnostic["severity"].as_u64() {
                    Some(1) => "error",
                    Some(2) => "warning",
                    Some(3) => "info",
                    _ => "hint",
                };
                let line = diagnostic["range"]["start"]["line"].as_u64().unwrap_or(0) + 1;
                format!(
                    "{severity} {}:{} {}",
                    path.display(),
                    line,
                    diagnostic["message"].as_str().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutcome::text(lines))
    }
}

pub fn register_tools(registry: &mut dshr_tools::ToolRegistry, manager: Arc<LspManager>) {
    registry.register_arc(Arc::new(LspHoverTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(LspDefinitionTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(LspReferencesTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(LspDiagnosticsTool::new(manager)));
}

fn position_schema() -> Value {
    json!({
        "type": "object",
        "required": ["path"],
        "additionalProperties": false,
        "properties": {
            "path": {"type": "string", "minLength": 1},
            "line": {"type": "integer", "minimum": 1},
            "column": {"type": "integer", "minimum": 1}
        }
    })
}

fn hover_text(value: &Value) -> String {
    let contents = &value["contents"];
    if let Some(text) = contents.as_str() {
        return text.to_owned();
    }
    if let Some(text) = contents["value"].as_str() {
        return text.to_owned();
    }
    if contents.is_null() {
        "(no hover)".to_owned()
    } else {
        contents.to_string()
    }
}

fn locations_text(value: &Value) -> String {
    let Some(items) = value.as_array() else {
        return value.to_string();
    };
    if items.is_empty() {
        return "(no locations)".to_owned();
    }
    items
        .iter()
        .map(|item| {
            let uri = item["uri"].as_str().unwrap_or_default();
            let line = item["range"]["start"]["line"].as_u64().unwrap_or(0) + 1;
            let character = item["range"]["start"]["character"].as_u64().unwrap_or(0) + 1;
            format!("{uri}:{line}:{character}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
