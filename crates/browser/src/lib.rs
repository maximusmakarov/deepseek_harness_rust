#![forbid(unsafe_code)]
//! Headless Chrome CUA tools.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::{Page, ScreenshotParams};
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use futures_util::StreamExt;
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("browser: {0}")]
    Failed(String),

    #[error("Chrome/Chromium not found; set CHROME or CHROME_PATH")]
    NotFound,
}

struct Session {
    browser: Browser,
    page: Page,
    handler: JoinHandle<()>,
}

pub struct BrowserManager {
    state: Mutex<Option<Session>>,
    headless: bool,
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
            headless: true,
        }
    }

    async fn page(&self) -> Result<Page, BrowserError> {
        let mut state = self.state.lock().await;
        if state.is_none() {
            let executable = find_chrome().ok_or(BrowserError::NotFound)?;
            let mut builder = BrowserConfig::builder()
                .no_sandbox()
                .chrome_executable(executable)
                .arg("--disable-gpu")
                .arg("--disable-dev-shm-usage");
            if self.headless {
                builder = builder.arg("--headless=new");
            }
            let config = builder.build().map_err(BrowserError::Failed)?;
            let (browser, mut handler) = Browser::launch(config)
                .await
                .map_err(|error| BrowserError::Failed(error.to_string()))?;
            let task = tokio::spawn(async move {
                while let Some(event) = handler.next().await {
                    if event.is_err() {
                        break;
                    }
                }
            });
            let page = browser
                .new_page("about:blank")
                .await
                .map_err(|error| BrowserError::Failed(error.to_string()))?;
            *state = Some(Session {
                browser,
                page: page.clone(),
                handler: task,
            });
        }
        Ok(state.as_ref().expect("session").page.clone())
    }

    pub async fn open(&self, url: &str) -> Result<Value, BrowserError> {
        validate_url(url)?;
        let page = self.page().await?;
        page.goto(url)
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(json!({
            "url": page.url().await.ok().flatten().unwrap_or_else(|| url.to_owned()),
            "title": page.get_title().await.ok().flatten().unwrap_or_default()
        }))
    }

    pub async fn screenshot(&self, path: &Path) -> Result<PathBuf, BrowserError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| BrowserError::Failed(error.to_string()))?;
        }
        let page = self.page().await?;
        page.save_screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .build(),
            path,
        )
        .await
        .map_err(|error| BrowserError::Failed(error.to_string()))?;
        Ok(path.to_path_buf())
    }

    pub async fn click(&self, selector: &str) -> Result<(), BrowserError> {
        let page = self.page().await?;
        page.find_element(selector)
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?
            .click()
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?;
        Ok(())
    }

    pub async fn type_text(&self, selector: &str, text: &str) -> Result<(), BrowserError> {
        let page = self.page().await?;
        page.find_element(selector)
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?
            .click()
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?
            .type_str(text)
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?;
        Ok(())
    }

    pub async fn evaluate(&self, script: &str) -> Result<Value, BrowserError> {
        let page = self.page().await?;
        page.evaluate(script)
            .await
            .map_err(|error| BrowserError::Failed(error.to_string()))?
            .into_value::<Value>()
            .map_err(|error| BrowserError::Failed(error.to_string()))
    }

    pub async fn close(&self) {
        let mut state = self.state.lock().await;
        if let Some(mut session) = state.take() {
            session.handler.abort();
            let _ = session.browser.close().await;
        }
    }
}

fn validate_url(url: &str) -> Result<(), BrowserError> {
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("data:")
        || url.starts_with("about:")
    {
        Ok(())
    } else {
        Err(BrowserError::Failed(
            "only http, https, data and about URLs are supported".into(),
        ))
    }
}

fn find_chrome() -> Option<PathBuf> {
    for key in ["CHROME", "CHROME_PATH"] {
        if let Some(value) = std::env::var_os(key) {
            let path = PathBuf::from(value);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    for key in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(key) {
            let root = PathBuf::from(root);
            for relative in [
                "Google/Chrome/Application/chrome.exe",
                "Microsoft/Edge/Application/msedge.exe",
            ] {
                let path = root.join(relative);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn screenshot_path(ctx: &ToolCtx, args: &Value) -> PathBuf {
    match args["path"].as_str() {
        Some(value) => dshr_tools::resolve_path(&ctx.cwd, value),
        None => ctx
            .cwd
            .join(".dshr/screenshots")
            .join(format!("screenshot-{}.png", uuid::Uuid::new_v4().simple())),
    }
}

pub struct BrowserOpenTool {
    manager: Arc<BrowserManager>,
}

impl BrowserOpenTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserOpenTool {
    fn name(&self) -> &str {
        "browser_open"
    }

    fn description(&self) -> &str {
        "Navigate headless Chrome to an http(s) URL."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "additionalProperties": false,
            "properties": {"url": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("url is required".into()))?;
        let value = self
            .manager
            .open(url)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(value.to_string()))
    }
}

pub struct BrowserScreenshotTool {
    manager: Arc<BrowserManager>,
}

impl BrowserScreenshotTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn name(&self) -> &str {
        "browser_screenshot"
    }

    fn description(&self) -> &str {
        "Capture a PNG screenshot of the current page."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {"path": {"type": "string"}}
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = screenshot_path(ctx, &args);
        let path = self
            .manager
            .screenshot(&path)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome {
            content: format!("screenshot saved to {}", path.display()),
            is_error: false,
            files: vec![path.display().to_string()],
        })
    }
}

pub struct BrowserClickTool {
    manager: Arc<BrowserManager>,
}

impl BrowserClickTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserClickTool {
    fn name(&self) -> &str {
        "browser_click"
    }

    fn description(&self) -> &str {
        "Click an element by CSS selector."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["selector"],
            "additionalProperties": false,
            "properties": {"selector": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let selector = args["selector"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("selector is required".into()))?;
        self.manager
            .click(selector)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("clicked {selector}")))
    }
}

pub struct BrowserTypeTool {
    manager: Arc<BrowserManager>,
}

impl BrowserTypeTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserTypeTool {
    fn name(&self) -> &str {
        "browser_type"
    }

    fn description(&self) -> &str {
        "Click an input by CSS selector and type text."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["selector", "text"],
            "additionalProperties": false,
            "properties": {
                "selector": {"type": "string", "minLength": 1},
                "text": {"type": "string"}
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let selector = args["selector"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("selector is required".into()))?;
        let text = args["text"].as_str().unwrap_or("");
        self.manager
            .type_text(selector, text)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("typed into {selector}")))
    }
}

pub struct BrowserEvalTool {
    manager: Arc<BrowserManager>,
}

impl BrowserEvalTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserEvalTool {
    fn name(&self) -> &str {
        "browser_eval"
    }

    fn description(&self) -> &str {
        "Evaluate JavaScript in the page and return its JSON value."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["script"],
            "additionalProperties": false,
            "properties": {"script": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let script = args["script"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("script is required".into()))?;
        let value = self
            .manager
            .evaluate(script)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(value.to_string()))
    }
}

pub struct BrowserCloseTool {
    manager: Arc<BrowserManager>,
}

impl BrowserCloseTool {
    #[must_use]
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for BrowserCloseTool {
    fn name(&self) -> &str {
        "browser_close"
    }

    fn description(&self) -> &str {
        "Close the headless browser."
    }

    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        self.manager.close().await;
        Ok(ToolOutcome::text("browser closed"))
    }
}

pub fn register_tools(registry: &mut dshr_tools::ToolRegistry, manager: Arc<BrowserManager>) {
    registry.register_arc(Arc::new(BrowserOpenTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(BrowserScreenshotTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(BrowserClickTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(BrowserTypeTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(BrowserEvalTool::new(Arc::clone(&manager))));
    registry.register_arc(Arc::new(BrowserCloseTool::new(manager)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_core::SessionId;
    use tokio_util::sync::CancellationToken;

    fn ctx(dir: &Path) -> ToolCtx {
        ToolCtx {
            cwd: dir.to_path_buf(),
            session_id: SessionId::new(),
            output_limit: 10_000,
            spill_dir: dir.join("spills"),
            cancel: CancellationToken::new(),
            prompt: None,
        }
    }

    #[test]
    fn validates_urls() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[tokio::test]
    #[ignore = "requires a local Chrome/Chromium"]
    async fn navigates_screenshots_and_evaluates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = Arc::new(BrowserManager::new());
        let open = BrowserOpenTool::new(Arc::clone(&manager));
        let outcome = open
            .execute(
                &ctx(dir.path()),
                json!({"url": "data:text/html,<h1>Hello</h1>"}),
            )
            .await
            .expect("open");
        assert!(outcome.content.contains("title"), "{}", outcome.content);

        let eval = BrowserEvalTool::new(Arc::clone(&manager));
        let value = eval
            .execute(
                &ctx(dir.path()),
                json!({"script": "document.querySelector('h1').textContent"}),
            )
            .await
            .expect("eval");
        assert_eq!(value.content, "\"Hello\"");

        let shot = BrowserScreenshotTool::new(Arc::clone(&manager));
        let outcome = shot
            .execute(&ctx(dir.path()), json!({}))
            .await
            .expect("shot");
        assert!(Path::new(&outcome.files[0]).exists());
        manager.close().await;
    }
}
