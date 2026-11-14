#![forbid(unsafe_code)]
//! HTTP/SSE server and embedded web UI.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use dshr_agent::{
    Agent, AgentConfig, AgentObserver, ApprovalPolicy, ApprovalRequest, Approver, PermissionMode,
    ToolCallInfo,
};
use dshr_core::DshrError;
use dshr_llm::{LlmProvider, Usage};
use dshr_session_jsonl::SessionStore;
use dshr_tools::{ToolOutcome, ToolRegistry, UserPrompt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{Mutex, broadcast, oneshot};
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub token: String,
    pub workspace: PathBuf,
    pub mode: PermissionMode,
    pub system_prompt: String,
    pub max_steps: usize,
    pub max_tokens: u32,
    pub output_limit: usize,
    /// Requests per minute per bearer token; 0 disables the limiter.
    pub rate_limit_per_min: u32,
    /// Generate an LLM title for a session on its first turn.
    pub auto_title: bool,
    /// Home directory used by plugin install endpoints.
    pub home: PathBuf,
    /// Active profile used by plugin install endpoints.
    pub profile: String,
    /// Cordis plugins at startup (name, enabled).
    pub cordis_plugins: Vec<(String, bool)>,
    /// Live plugin supervisor; `None` in tests and one-shot embeddings.
    pub plugins: Option<Arc<dshr_cordis::CordisManager>>,
    /// WASM plugin hot-reload manager.
    pub wasm: Option<Arc<dshr_wasm::WasmManager>>,
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<ServerConfig>,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
    runtimes: Arc<Mutex<HashMap<String, Arc<Runtime>>>>,
    rate_limit: Arc<RateLimit>,
    /// Capabilities that let one sandboxed frame read one plugin package.
    assets: Arc<std::sync::Mutex<HashMap<String, AssetGrant>>>,
}

/// Time-limited read capability over a plugin's package files.
struct AssetGrant {
    plugin: String,
    expires: Instant,
}

impl AppState {
    #[must_use]
    pub fn new(
        config: ServerConfig,
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        store: Arc<SessionStore>,
    ) -> Self {
        let rate_limit_per_min = config.rate_limit_per_min;
        Self {
            config: Arc::new(config),
            provider,
            tools,
            store,
            runtimes: Arc::new(Mutex::new(HashMap::new())),
            rate_limit: Arc::new(RateLimit::new(rate_limit_per_min)),
            assets: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    async fn runtime(&self, id: &str) -> Arc<Runtime> {
        let mut runtimes = self.runtimes.lock().await;
        Arc::clone(
            runtimes
                .entry(id.to_owned())
                .or_insert_with(|| Arc::new(Runtime::new())),
        )
    }
}

struct Runtime {
    tx: broadcast::Sender<ServerEvent>,
    cancel: Mutex<Option<CancellationToken>>,
    approvals: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    approval_counter: Arc<AtomicU64>,
    questions: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
    question_counter: Arc<AtomicU64>,
    busy: AtomicBool,
}

impl Runtime {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(512);
        Self {
            tx,
            cancel: Mutex::new(None),
            approvals: Arc::new(Mutex::new(HashMap::new())),
            approval_counter: Arc::new(AtomicU64::new(1)),
            questions: Arc::new(Mutex::new(HashMap::new())),
            question_counter: Arc::new(AtomicU64::new(1)),
            busy: AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Text {
        text: String,
    },
    ToolStart {
        name: String,
        summary: String,
        /// Tool input as JSON, so the UI can render a diff preview for
        /// `edit`/`write`/`apply_patch` without re-reading the session log.
        #[serde(default)]
        input: String,
    },
    ToolEnd {
        name: String,
        ok: bool,
        content: String,
    },
    Approval {
        id: String,
        tool: String,
        summary: String,
        reason: String,
    },
    Question {
        id: String,
        question: String,
        options: Vec<String>,
    },
    Done {
        session_id: String,
        text: String,
        usage: Usage,
        steps: usize,
        tool_calls: usize,
    },
    Cancelled {
        session_id: String,
    },
    Error {
        message: String,
    },
}

struct BroadcastObserver {
    tx: broadcast::Sender<ServerEvent>,
    questions: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
    counter: Arc<AtomicU64>,
}

struct HttpPrompt {
    tx: broadcast::Sender<ServerEvent>,
    questions: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
    counter: Arc<AtomicU64>,
}

#[async_trait]
impl dshr_tools::UserPrompt for HttpPrompt {
    async fn ask(&self, question: &str, options: &[String]) -> Option<String> {
        let id = format!("q_{}", self.counter.fetch_add(1, Ordering::SeqCst));
        let (respond, answer) = oneshot::channel();
        self.questions.lock().await.insert(id.clone(), respond);
        let _ = self.tx.send(ServerEvent::Question {
            id: id.clone(),
            question: question.to_owned(),
            options: options.to_vec(),
        });
        let answer = tokio::time::timeout(Duration::from_secs(300), answer)
            .await
            .ok()
            .and_then(Result::ok);
        self.questions.lock().await.remove(&id);
        answer
    }
}

#[async_trait]
impl AgentObserver for BroadcastObserver {
    async fn on_text(&self, text: &str) {
        let _ = self.tx.send(ServerEvent::Text {
            text: text.to_owned(),
        });
    }

    async fn on_tool_start(&self, call: &ToolCallInfo) {
        let summary = call.input.to_string();
        let _ = self.tx.send(ServerEvent::ToolStart {
            name: call.name.clone(),
            summary: summary.chars().take(160).collect(),
            input: summary.chars().take(4000).collect(),
        });
    }

    async fn on_tool_end(&self, call: &ToolCallInfo, outcome: &ToolOutcome) {
        let _ = self.tx.send(ServerEvent::ToolEnd {
            name: call.name.clone(),
            ok: !outcome.is_error,
            content: outcome.content.chars().take(2000).collect(),
        });
    }

    async fn on_question(&self, question: &str, options: &[String]) -> Option<String> {
        HttpPrompt {
            tx: self.tx.clone(),
            questions: Arc::clone(&self.questions),
            counter: Arc::clone(&self.counter),
        }
        .ask(question, options)
        .await
    }

    fn prompt(&self) -> Option<Arc<dyn dshr_tools::UserPrompt>> {
        Some(Arc::new(HttpPrompt {
            tx: self.tx.clone(),
            questions: Arc::clone(&self.questions),
            counter: Arc::clone(&self.counter),
        }))
    }
}

struct HttpApprover {
    tx: broadcast::Sender<ServerEvent>,
    approvals: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    counter: Arc<AtomicU64>,
}

#[async_trait]
impl Approver for HttpApprover {
    async fn approve(&self, request: ApprovalRequest) -> bool {
        let id = format!("appr_{}", self.counter.fetch_add(1, Ordering::SeqCst));
        let (respond, answer) = oneshot::channel();
        self.approvals.lock().await.insert(id.clone(), respond);
        let _ = self.tx.send(ServerEvent::Approval {
            id: id.clone(),
            tool: request.tool,
            summary: request.summary,
            reason: request.reason,
        });
        let approved = answer.await.unwrap_or(false);
        self.approvals.lock().await.remove(&id);
        approved
    }
}

struct RateLimit {
    limit: u32,
    buckets: std::sync::Mutex<HashMap<u64, Bucket>>,
}

struct Bucket {
    window: Instant,
    count: u32,
}

impl RateLimit {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            buckets: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: &str) -> bool {
        if self.limit == 0 {
            return true;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let key = hasher.finish();
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter poisoned");
        let bucket = buckets.entry(key).or_insert(Bucket {
            window: now,
            count: 0,
        });
        if now.duration_since(bucket.window) >= Duration::from_secs(60) {
            bucket.window = now;
            bucket.count = 0;
        }
        if bucket.count >= self.limit {
            return false;
        }
        bucket.count += 1;
        true
    }
}

async fn rate_limit(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if !path.starts_with("/v1/") || path == "/v1/health" || state.config.rate_limit_per_min == 0 {
        return next.run(request).await;
    }
    let key = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("anonymous");
    if state.rate_limit.allow(key) {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            Json(json!({"error": "rate limit exceeded"})),
        )
            .into_response()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/dshr-web-ui.js", get(web_js))
        .route("/dshr-web-ui_bg.wasm", get(web_wasm))
        .route("/style.css", get(style_css))
        .route("/openapi.json", get(openapi))
        .route("/manifest.webmanifest", get(manifest))
        .route("/icon-192.png", get(icon_192))
        .route("/icon-512.png", get(icon_512))
        .route("/v1/health", get(health))
        .route("/v1/tools", get(list_tools))
        .route(
            "/v1/config/model",
            get(get_model_config).post(set_model_config),
        )
        .route("/v1/plugins/search", get(search_plugins))
        .route("/v1/plugins/info", get(plugin_info))
        .route("/v1/plugins/catalog", get(plugin_catalog))
        .route("/v1/plugins/catalog/{name}", get(plugin_catalog_entry))
        .route("/v1/plugins/ratings", get(plugin_ratings))
        .route("/v1/plugins/rate", post(rate_plugin))
        .route("/v1/plugins/reviews", get(plugin_reviews))
        .route("/v1/plugins/review", post(add_review))
        .route("/v1/plugins/install", post(install_plugin))
        .route("/v1/plugins/installed", get(installed_plugins))
        .route("/v1/plugins/status", get(plugin_status))
        .route("/v1/plugins/reload", post(reload_plugins))
        .route("/v1/plugins/clients", get(plugin_clients))
        .route("/v1/plugins/{name}/client.js", get(plugin_client_bundle))
        .route("/v1/plugins/{name}/host", get(plugin_client_host))
        .route(
            "/v1/plugins/{name}/asset/{capability}/{*path}",
            get(plugin_client_asset),
        )
        .route("/v1/plugins/toggle", post(toggle_plugin))
        .route("/v1/plugins/remove", post(remove_plugin))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turns", post(start_turn))
        .route("/v1/sessions/{id}/events", get(session_events))
        .route("/v1/sessions/{id}/cancel", post(cancel_turn))
        .route(
            "/v1/sessions/{id}/approvals/{approval_id}",
            post(resolve_approval),
        )
        .route(
            "/v1/sessions/{id}/questions/{question_id}",
            post(resolve_question),
        )
        .route("/v1/rpc", post(rpc))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

pub async fn serve(
    config: ServerConfig,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
) -> Result<(), String> {
    let bind = config.bind;
    let token = config.token.clone();
    let state = AppState::new(config, provider, tools, store);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| format!("bind {bind}: {error}"))?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    println!("dshr server listening on http://{address}");
    println!("token: {token}");
    axum::serve(listener, router(state))
        .await
        .map_err(|error| error.to_string())
}

async fn list_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let tools: Vec<Value> = state
        .tools
        .definitions()
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema
            })
        })
        .collect();
    Ok(Json(json!({"tools": tools})))
}

#[derive(Debug, Deserialize)]
struct PluginSearch {
    q: String,
}

async fn search_plugins(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PluginSearch>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let text = percent_encode(query.q.trim());
    if text.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let url = format!("https://registry.npmjs.org/-/v1/search?text={text}&size=20");
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "dshr-server")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let value: Value = response.json().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    let objects: Vec<Value> = value["objects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|object| {
            let package = &object["package"];
            Some(json!({
                "name": package["name"].as_str()?,
                "version": package["version"].as_str().unwrap_or(""),
                "description": package["description"].as_str().unwrap_or("")
            }))
        })
        .collect();
    Ok(Json(json!({"packages": objects})))
}

const PLUGIN_CATALOG: &str = include_str!("catalog.json");

async fn plugin_ratings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let ratings = load_ratings(&state.config.home);
    Ok(Json(json!({"ratings": ratings})))
}

#[derive(Debug, Deserialize)]
struct RatePlugin {
    name: String,
    rating: u8,
}

async fn rate_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RatePlugin>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_package_name(&body.name) || !(1..=5).contains(&body.rating) {
        return Err(StatusCode::BAD_REQUEST);
    }
    save_rating(&state.config.home, &body.name, body.rating)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Review {
    rating: u8,
    text: String,
    at: u64,
}

fn reviews_path(home: &std::path::Path) -> PathBuf {
    home.join("plugin-reviews.json")
}

fn load_reviews(home: &std::path::Path) -> BTreeMap<String, Vec<Review>> {
    std::fs::read_to_string(reviews_path(home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_review(
    home: &std::path::Path,
    name: &str,
    rating: u8,
    text: &str,
) -> std::io::Result<(f64, usize)> {
    let mut reviews = load_reviews(home);
    reviews.entry(name.to_owned()).or_default().push(Review {
        rating,
        text: text.chars().take(2000).collect(),
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    });
    let json = serde_json::to_string_pretty(&reviews)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(reviews_path(home), json)?;
    Ok(summarize(
        reviews.get(name).map(Vec::as_slice).unwrap_or_default(),
    ))
}

fn summarize(reviews: &[Review]) -> (f64, usize) {
    let count = reviews.len();
    if count == 0 {
        return (0.0, 0);
    }
    let total: u32 = reviews.iter().map(|review| u32::from(review.rating)).sum();
    (f64::from(total) / count as f64, count)
}

#[derive(Debug, Deserialize)]
struct ReviewsQuery {
    name: Option<String>,
}

async fn plugin_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ReviewsQuery>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let reviews = load_reviews(&state.config.home);
    if let Some(name) = query.name {
        if !valid_package_name(&name) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let list = reviews.get(&name).cloned().unwrap_or_default();
        let (average, count) = summarize(&list);
        return Ok(Json(json!({
            "average": average,
            "count": count,
            "reviews": list
        })));
    }
    let mut plugins = serde_json::Map::new();
    for (name, list) in reviews {
        let (average, count) = summarize(&list);
        plugins.insert(
            name,
            json!({"average": average, "count": count, "reviews": list}),
        );
    }
    Ok(Json(json!({"plugins": plugins})))
}

#[derive(Debug, Deserialize)]
struct AddReview {
    name: String,
    rating: u8,
    #[serde(default)]
    text: String,
}

async fn add_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AddReview>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_package_name(&body.name)
        || !(1..=5).contains(&body.rating)
        || body.text.chars().count() > 2000
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (average, count) = save_review(&state.config.home, &body.name, body.rating, &body.text)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({"ok": true, "average": average, "count": count}),
    ))
}

fn ratings_path(home: &std::path::Path) -> PathBuf {
    home.join("plugin-ratings.json")
}

fn load_ratings(home: &std::path::Path) -> BTreeMap<String, u8> {
    std::fs::read_to_string(ratings_path(home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_rating(home: &std::path::Path, name: &str, rating: u8) -> std::io::Result<()> {
    let mut ratings = load_ratings(home);
    ratings.insert(name.to_owned(), rating);
    let text = serde_json::to_string_pretty(&ratings)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(ratings_path(home), text)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CatalogEntry {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    verified: bool,
}

impl CatalogEntry {
    fn matches(&self, query: &str) -> bool {
        let query = query.to_ascii_lowercase();
        self.name.to_ascii_lowercase().contains(&query)
            || self.description.to_ascii_lowercase().contains(&query)
            || self.tags.iter().any(|tag| tag.contains(&query))
    }
}

fn catalog_entries() -> Vec<CatalogEntry> {
    serde_json::from_str(PLUGIN_CATALOG).unwrap_or_default()
}

fn catalog_categories(entries: &[CatalogEntry]) -> Vec<Value> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in entries {
        if !entry.category.is_empty() {
            *counts.entry(entry.category.as_str()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(category, count)| json!({"name": category, "count": count}))
        .collect()
}

#[derive(Debug, Deserialize)]
struct CatalogQuery {
    q: Option<String>,
    category: Option<String>,
    limit: Option<usize>,
}

async fn plugin_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let entries = catalog_entries();
    let categories = catalog_categories(&entries);
    let needle = query.q.unwrap_or_default();
    let category = query.category.unwrap_or_default();
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let mut packages: Vec<CatalogEntry> = entries
        .into_iter()
        .filter(|entry| needle.is_empty() || entry.matches(needle.trim()))
        .filter(|entry| category.is_empty() || entry.category == category)
        .collect();
    packages.truncate(limit);
    let total = packages.len();
    Ok(Json(json!({
        "packages": packages,
        "total": total,
        "categories": categories,
    })))
}

async fn plugin_catalog_entry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_package_name(&name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let Some(entry) = catalog_entries()
        .into_iter()
        .find(|entry| entry.name == name)
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    let reviews = load_reviews(&state.config.home);
    let list = reviews.get(&name).cloned().unwrap_or_default();
    let (average, count) = summarize(&list);
    let ratings = load_ratings(&state.config.home);
    let installed = state
        .config
        .cordis_plugins
        .iter()
        .any(|(plugin, enabled)| plugin == &name && *enabled);
    Ok(Json(json!({
        "package": entry,
        "rating": ratings.get(&name).copied().unwrap_or(0),
        "reviews": {"average": average, "count": count},
        "installed": installed,
    })))
}

/// Provider routes the CLI can build; the settings screen may only pick one.
const PROVIDER_ROUTES: [&str; 6] = [
    "deepseek",
    "deepseek-account",
    "deepseek-official",
    "openai",
    "openai-compatible",
    "routerai",
];

fn user_config_path(home: &std::path::Path) -> PathBuf {
    home.join("config.yaml")
}

fn read_user_config(home: &std::path::Path) -> Value {
    std::fs::read_to_string(user_config_path(home))
        .ok()
        .and_then(|text| serde_saphyr::from_str::<Value>(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn write_user_config(home: &std::path::Path, value: &Value) -> std::io::Result<()> {
    std::fs::create_dir_all(home)?;
    let path = user_config_path(home);
    let text =
        serde_saphyr::to_string(value).map_err(|error| std::io::Error::other(error.to_string()))?;
    // Write through a sibling temp file so a reader never sees a partial config.
    let temp = path.with_extension("yaml.tmp");
    std::fs::write(&temp, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&temp, &path)?;
    Ok(())
}

fn valid_model(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
}

fn valid_base_url(value: &str) -> bool {
    value.len() <= 2048
        && (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_whitespace)
}

async fn get_model_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let config = read_user_config(&state.config.home);
    let base_url = config["model"]["base_url"].as_str().map(str::to_owned);
    // `provider` is the route persisted in the user layer (editable); the live
    // process may run another one, e.g. the replay provider behind a cassette.
    let configured = config["model"]["provider"].as_str().unwrap_or_default();
    let provider = if PROVIDER_ROUTES.contains(&configured) {
        configured
    } else {
        ""
    };
    Ok(Json(json!({
        "provider": provider,
        "active_provider": state.provider.name(),
        "model": state.provider.model(),
        "base_url": base_url,
        "routes": PROVIDER_ROUTES,
        "note": "credentials are read from the environment; a route or model change needs a restart",
    })))
}

#[derive(Debug, Deserialize)]
struct ModelConfigRequest {
    provider: String,
    model: String,
    #[serde(default)]
    base_url: Option<String>,
}

async fn set_model_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ModelConfigRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !PROVIDER_ROUTES.contains(&body.provider.as_str()) || !valid_model(&body.model) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(url) = body.base_url.as_deref()
        && !url.is_empty()
        && !valid_base_url(url)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut config = read_user_config(&state.config.home);
    let model = config["model"].as_object_mut();
    let model = match model {
        Some(model) => model,
        None => {
            config["model"] = json!({});
            config["model"].as_object_mut().expect("just inserted")
        }
    };
    model.insert("provider".into(), json!(body.provider));
    model.insert("name".into(), json!(body.model));
    match body.base_url.filter(|url| !url.is_empty()) {
        Some(url) => model.insert("base_url".into(), json!(url)),
        None => model.remove("base_url"),
    };
    write_user_config(&state.config.home, &config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "ok": true,
        "restart_required": true,
        "provider": config["model"]["provider"],
        "model": config["model"]["name"],
    })))
}

#[derive(Debug, Deserialize)]
struct PluginInfoQuery {
    name: String,
}

async fn plugin_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PluginInfoQuery>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_package_name(&query.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let url = format!("https://registry.npmjs.org/{}", percent_encode(&query.name));
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "dshr-server")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(StatusCode::NOT_FOUND);
    }
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let value: Value = response.json().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    let latest = value["dist-tags"]["latest"].as_str().unwrap_or("");
    let entry = &value["versions"][latest];
    Ok(Json(json!({
        "name": query.name,
        "version": latest,
        "description": entry["description"].as_str().or_else(|| value["description"].as_str()).unwrap_or(""),
        "homepage": entry["homepage"].as_str().or_else(|| value["homepage"].as_str()).unwrap_or(""),
        "repository": entry["repository"]["url"].as_str().unwrap_or("")
    })))
}

#[derive(Debug, Deserialize)]
struct InstallPlugin {
    name: String,
}

async fn install_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<InstallPlugin>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_package_name(&body.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let executable = std::env::current_exe().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let command = tokio::process::Command::new(executable)
        .args([
            "--home",
            &state.config.home.display().to_string(),
            "--profile",
            &state.config.profile,
            "plugin",
            "npm",
            "add",
            &body.name,
        ])
        .output();
    let output = tokio::time::timeout(Duration::from_secs(180), command)
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(Json(json!({
        "ok": output.status.success(),
        "output": text.chars().take(4000).collect::<String>()
    })))
}

async fn installed_plugins(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    // Prefer the live supervisor: it also knows plugins added or toggled while
    // the server was running (the config watcher reconciles them).
    let plugins: Vec<Value> = match state.config.plugins.as_ref() {
        Some(manager) => manager
            .statuses()
            .into_iter()
            .map(|plugin| json!({"name": plugin.name, "enabled": plugin.enabled}))
            .collect(),
        None => state
            .config
            .cordis_plugins
            .iter()
            .map(|(name, enabled)| json!({"name": name, "enabled": enabled}))
            .collect(),
    };
    Ok(Json(json!({"plugins": plugins})))
}

#[derive(Debug, Deserialize)]
struct TogglePlugin {
    name: String,
    enabled: bool,
}

async fn toggle_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TogglePlugin>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_plugin_name(&body.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let action = if body.enabled { "enable" } else { "disable" };
    run_plugin_cli(&state, &["plugin", "npm", action, &body.name]).await
}

#[derive(Debug, Deserialize)]
struct RemovePlugin {
    name: String,
}

async fn remove_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RemovePlugin>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !valid_plugin_name(&body.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    run_plugin_cli(&state, &["plugin", "npm", "remove", &body.name]).await
}

/// Cordis plugin control: live status and hot reload.
///
/// Reloading restarts one sidecar (or all of them) and swaps its tools in the
/// serving registry; agent sessions that are already running pick up the new
/// generation on their next tool call, because they hold the same registry.
async fn plugin_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let plugins = state
        .config
        .plugins
        .as_ref()
        .map(|manager| manager.statuses())
        .unwrap_or_default();
    Ok(Json(json!({"plugins": plugins})))
}

#[derive(Debug, Deserialize, Default)]
struct ReloadPlugins {
    #[serde(default)]
    name: Option<String>,
}

async fn reload_plugins(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ReloadPlugins>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authorize(&state, &headers, None)
        .map_err(|status| (status, Json(json!({"error": "unauthorized"}))))?;
    if state.config.plugins.is_none() && state.config.wasm.is_none() {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"error": "no plugin supervisor is attached to this server"})),
        ));
    }
    let errors = if let Some(manager) = state.config.plugins.as_ref() {
        match body.name.clone() {
            Some(name) => {
                if !valid_plugin_name(&name) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": "invalid plugin name"})),
                    ));
                }
                match manager.reload(&name).await {
                    Ok(()) => Vec::new(),
                    Err(dshr_cordis::CordisError::NotConfigured(_)) => {
                        return Err((
                            StatusCode::NOT_FOUND,
                            Json(json!({"error": format!("plugin '{name}' is not configured")})),
                        ));
                    }
                    Err(error) => vec![json!({"plugin": name, "error": error.to_string()})],
                }
            }
            None => manager
                .reload_all()
                .await
                .into_iter()
                .map(|(plugin, error)| json!({"plugin": plugin, "error": error}))
                .collect(),
        }
    } else {
        Vec::new()
    };
    let wasm_tools = match state.config.wasm.as_ref() {
        Some(wasm) => wasm.sync().await,
        None => 0,
    };
    let status = if errors.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::BAD_GATEWAY
    };
    let statuses = state
        .config
        .plugins
        .as_ref()
        .map(|manager| manager.statuses())
        .unwrap_or_default();
    let payload = json!({"plugins": statuses, "errors": errors, "wasm_tools": wasm_tools});
    if status == StatusCode::OK {
        Ok(Json(payload))
    } else {
        Err((status, Json(payload)))
    }
}

/// Client halves (`dsh.client`) of the configured plugins.
async fn plugin_clients(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let (clients, errors) = state
        .config
        .plugins
        .as_ref()
        .map(|manager| manager.client_halves())
        .unwrap_or_default();
    Ok(Json(json!({
        "clients": clients,
        "errors": errors.into_iter().map(|(plugin, error)| json!({"plugin": plugin, "error": error})).collect::<Vec<_>>()
    })))
}

/// Serves the plugin bundle for debugging; the host page inlines it instead.
async fn plugin_client_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    authorize(&state, &headers, query.get("token").map(String::as_str))?;
    let manager = state.config.plugins.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let half = manager
        .client_half(&name)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let bytes = tokio::fs::read(&half.path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Debug, Deserialize, Default)]
struct ClientHostQuery {
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    token: Option<String>,
}

/// Serves the sandboxed host page that runs one client half.
///
/// Single-file bundles are inlined base64 into a network-free page. Bundles
/// with relative imports get a short-lived read capability and load their
/// module graph from the asset route instead.
async fn plugin_client_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<ClientHostQuery>,
) -> Result<Response, StatusCode> {
    authorize(&state, &headers, query.token.as_deref())?;
    let manager = state.config.plugins.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let half = manager
        .client_half(&name)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let theme = query.theme.as_deref().unwrap_or("dark");
    let (page, csp) = match half.bundle {
        dshr_cordis::client::BundleMode::Inline => {
            let bundle = tokio::fs::read(&half.path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            (
                dshr_cordis::client::host_page(&half, &bundle, theme),
                CLIENT_HOST_CSP.to_owned(),
            )
        }
        dshr_cordis::client::BundleMode::Assets => {
            let capability = grant_assets(&state, &half.name);
            let base = format!(
                "/v1/plugins/{}/asset/{capability}",
                percent_encode(&half.name)
            );
            let origin = request_origin(&headers);
            (
                dshr_cordis::client::host_page_assets(&half, &base, theme),
                asset_host_csp(&origin),
            )
        }
    };
    Ok((
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_SECURITY_POLICY, csp.as_str()),
            (
                header::HeaderName::from_static("x-frame-options"),
                "SAMEORIGIN",
            ),
        ],
        page,
    )
        .into_response())
}

/// The inline host page runs with an opaque origin and must stay network-free:
/// the bundle arrives base64-inlined and executes from a blob URL.
const CLIENT_HOST_CSP: &str = "default-src 'none'; script-src 'unsafe-inline' blob:; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'";

/// Capability TTL: long enough to open a panel, short enough to be useless if
/// it leaks into a log.
const ASSET_TTL: Duration = Duration::from_secs(3600);
const ASSET_MAX_BYTES: usize = 8 * 1024 * 1024;

fn grant_assets(state: &AppState, plugin: &str) -> String {
    let capability = uuid::Uuid::new_v4().simple().to_string();
    let mut grants = state
        .assets
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let now = Instant::now();
    grants.retain(|_, grant| grant.expires > now);
    grants.insert(
        capability.clone(),
        AssetGrant {
            plugin: plugin.to_owned(),
            expires: now + ASSET_TTL,
        },
    );
    capability
}

/// `http://<host>` of the request, used for the asset-mode CSP so the frame can
/// load its module graph from this server only.
fn request_origin(headers: &HeaderMap) -> String {
    let scheme = match headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
    {
        Some("https") => "https",
        _ => "http",
    };
    // The Host header is attacker-controlled and ends up inside the CSP, so a
    // strict charset is enforced; anything else falls back to loopback.
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let valid = !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | ':' | '[' | ']'));
    if valid {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://127.0.0.1")
    }
}

fn asset_host_csp(origin: &str) -> String {
    format!(
        "default-src 'none'; script-src 'unsafe-inline' {origin}; style-src 'unsafe-inline' {origin}; \
         img-src {origin} data: blob:; font-src {origin}; connect-src {origin}; \
         media-src {origin}; base-uri 'none'; form-action 'none'; frame-ancestors 'self'"
    )
}

fn capability_granted(state: &AppState, plugin: &str, capability: &str) -> bool {
    if capability.len() != 32 || !capability.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return false;
    }
    let mut grants = state
        .assets
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let now = Instant::now();
    grants.retain(|_, grant| grant.expires > now);
    grants
        .get(capability)
        .is_some_and(|grant| grant.plugin == plugin)
}

fn asset_content_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "js" | "mjs" | "cjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Serves one file of a plugin package under a capability derived from the host
/// page request. The iframe is sandboxed, so this is the only way for a
/// multi-file bundle to resolve `./chunk.js` without exposing the shell token.
async fn plugin_client_asset(
    State(state): State<AppState>,
    Path((name, capability, path)): Path<(String, String, String)>,
) -> Result<Response, StatusCode> {
    if !capability_granted(&state, &name, &capability) {
        return Err(StatusCode::FORBIDDEN);
    }
    let manager = state.config.plugins.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let half = manager
        .client_half(&name)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let file =
        dshr_cordis::client::resolve_asset(&half, &path).map_err(|_| StatusCode::NOT_FOUND)?;
    let metadata = tokio::fs::metadata(&file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if metadata.len() as usize > ASSET_MAX_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let bytes = tokio::fs::read(&file)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        [
            (header::CONTENT_TYPE, asset_content_type(&file)),
            (header::CACHE_CONTROL, "no-store"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::HeaderName::from_static("cross-origin-resource-policy"),
                "cross-origin",
            ),
        ],
        bytes,
    )
        .into_response())
}

async fn run_plugin_cli(state: &AppState, arguments: &[&str]) -> Result<Json<Value>, StatusCode> {
    let executable = std::env::current_exe().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut command = tokio::process::Command::new(executable);
    command.args([
        "--home",
        &state.config.home.display().to_string(),
        "--profile",
        &state.config.profile,
    ]);
    command.args(arguments);
    let output = tokio::time::timeout(Duration::from_secs(180), command.output())
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(Json(json!({
        "ok": output.status.success(),
        "output": text.chars().take(4000).collect::<String>()
    })))
}

fn valid_plugin_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "@/_.-".contains(ch))
}

fn valid_package_name(name: &str) -> bool {
    let name = name.strip_prefix('@').unwrap_or(name);
    let (scope, package) = name.split_once('/').unwrap_or(("", name));
    let valid = |value: &str| {
        !value.is_empty()
            && value.len() <= 100
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    };
    if name.contains('/') {
        valid(scope) && valid(package)
    } else {
        valid(package)
    }
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

async fn health() -> impl IntoResponse {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

#[derive(Debug, Deserialize, Default)]
struct CreateSessionRequest {
    title: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let cwd = state.config.workspace.display().to_string();
    let mut session = state
        .store
        .create(state.provider.name(), state.provider.model(), &cwd)
        .map_err(not_found)?;
    if let Some(title) = &body.title {
        session = state
            .store
            .set_title(session.header.id.as_str(), title)
            .map_err(not_found)?;
    }
    let _ = state.runtime(session.header.id.as_str()).await;
    Ok(Json(json!({"session": session.header})))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let sessions = state.store.list().map_err(not_found)?;
    Ok(Json(json!({"sessions": sessions})))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let session = state.store.open(&id).map_err(not_found)?;
    Ok(Json(json!({
        "session": session.header,
        "messages": session.derive_messages(),
    })))
}

#[derive(Debug, Deserialize)]
struct TurnRequest {
    prompt: String,
}

async fn start_turn(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<TurnRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if body.prompt.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let session = state.store.open(&id).map_err(not_found)?;
    let runtime = state.runtime(&id).await;
    if runtime
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(StatusCode::CONFLICT);
    }

    if state.config.auto_title
        && session.header.title.is_none()
        && !matches!(state.provider.name(), "replay" | "scripted" | "mock")
    {
        let provider = Arc::clone(&state.provider);
        let store = Arc::clone(&state.store);
        let title_session = id.clone();
        let title_prompt = body.prompt.clone();
        tokio::spawn(async move {
            if let Ok(Some(title)) =
                dshr_agent::generate_title(provider.as_ref(), &title_prompt).await
            {
                let _ = store.set_title(&title_session, &title);
            }
        });
    }

    let cancel = CancellationToken::new();
    *runtime.cancel.lock().await = Some(cancel.clone());
    let approver: Arc<dyn Approver> = Arc::new(HttpApprover {
        tx: runtime.tx.clone(),
        approvals: Arc::clone(&runtime.approvals),
        counter: Arc::clone(&runtime.approval_counter),
    });
    let policy = ApprovalPolicy {
        mode: state.config.mode,
        workspace: state.config.workspace.clone(),
        approver,
    };
    let agent_config = AgentConfig {
        system_prompt: state.config.system_prompt.clone(),
        max_steps: state.config.max_steps,
        max_tokens: state.config.max_tokens,
        output_limit: state.config.output_limit,
        compaction: Some(dshr_compaction::CompactionConfig::default()),
        budget: None,
    };
    let mut agent = Agent::new(
        Arc::clone(&state.provider),
        Arc::clone(&state.tools),
        Arc::clone(&state.store),
        session,
        policy,
        agent_config,
    );

    let session_id = id.clone();
    let tx = runtime.tx.clone();
    let runtime = Arc::clone(&runtime);
    tokio::spawn(async move {
        let observer = BroadcastObserver {
            tx: tx.clone(),
            questions: Arc::clone(&runtime.questions),
            counter: Arc::clone(&runtime.question_counter),
        };
        let result = agent.run_turn(&body.prompt, cancel, &observer).await;
        match result {
            Ok(report) => {
                let _ = tx.send(ServerEvent::Done {
                    session_id: session_id.clone(),
                    text: report.text,
                    usage: report.usage,
                    steps: report.steps,
                    tool_calls: report.tool_calls,
                });
            }
            Err(dshr_agent::AgentError::Cancelled) => {
                let _ = tx.send(ServerEvent::Cancelled { session_id });
            }
            Err(error) => {
                let _ = tx.send(ServerEvent::Error {
                    message: error.to_string(),
                });
            }
        }
        runtime.busy.store(false, Ordering::SeqCst);
        *runtime.cancel.lock().await = None;
    });

    Ok(Json(json!({"accepted": true, "session_id": id})))
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

async fn rpc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RpcRequest>,
) -> Response {
    if authorize(&state, &headers, None).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let result = match request.method.as_str() {
        "session.list" => state
            .store
            .list()
            .map(|sessions| json!({"sessions": sessions})),
        "session.get" => {
            let Some(id) = request.params["id"].as_str() else {
                return rpc_error(request.id, -32602, "params.id is required");
            };
            state.store.open(id).map(|session| {
                json!({
                    "session": session.header,
                    "messages": session.derive_messages(),
                })
            })
        }
        "session.create" => {
            let title = request.params["title"].as_str();
            let cwd = state.config.workspace.display().to_string();
            state
                .store
                .create(state.provider.name(), state.provider.model(), &cwd)
                .and_then(|session| match title {
                    Some(title) => state.store.set_title(session.header.id.as_str(), title),
                    None => Ok(session),
                })
                .map(|session| json!({"session": session.header}))
        }
        _ => {
            return rpc_error(
                request.id,
                -32601,
                &format!("method '{}' not found", request.method),
            );
        }
    };
    match result {
        Ok(value) => {
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": value})).into_response()
        }
        Err(error) => rpc_error(request.id, -32000, &error.to_string()),
    }
}

fn rpc_error(id: Value, code: i64, message: &str) -> Response {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    }))
    .into_response()
}

async fn cancel_turn(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let runtime = state.runtime(&id).await;
    let cancelled = {
        let slot = runtime.cancel.lock().await;
        match slot.as_ref() {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    };
    Ok(Json(json!({"cancelled": cancelled})))
}

#[derive(Debug, Deserialize)]
struct ApprovalDecision {
    approved: bool,
}

async fn resolve_approval(
    State(state): State<AppState>,
    Path((id, approval_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<ApprovalDecision>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let runtime = state.runtime(&id).await;
    let sender = runtime.approvals.lock().await.remove(&approval_id);
    match sender {
        Some(sender) => {
            let _ = sender.send(body.approved);
            Ok(Json(json!({"resolved": approval_id})))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Debug, Deserialize)]
struct QuestionAnswer {
    answer: String,
}

async fn resolve_question(
    State(state): State<AppState>,
    Path((id, question_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<QuestionAnswer>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let runtime = state.runtime(&id).await;
    let sender = runtime.questions.lock().await.remove(&question_id);
    match sender {
        Some(sender) => {
            let _ = sender.send(body.answer);
            Ok(Json(json!({"resolved": question_id})))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn session_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    authorize(&state, &headers, query.get("token").map(String::as_str))?;
    let runtime = state.runtime(&id).await;
    let mut receiver = runtime.tx.subscribe();
    let stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
                    yield Ok(Event::default().event("message").data(data));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<(), StatusCode> {
    let header_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if header_token == Some(state.config.token.as_str())
        || query_token == Some(state.config.token.as_str())
    {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn not_found(error: DshrError) -> StatusCode {
    match error.kind() {
        dshr_core::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        dshr_core::ErrorKind::InvalidInput | dshr_core::ErrorKind::Config => {
            StatusCode::BAD_REQUEST
        }
        dshr_core::ErrorKind::Io => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../web/dist/index.html"))
}

async fn web_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_bytes!("../../web/dist/dshr-web-ui.js").as_slice(),
    )
        .into_response()
}

async fn web_wasm() -> Response {
    (
        [(header::CONTENT_TYPE, "application/wasm")],
        include_bytes!("../../web/dist/dshr-web-ui_bg.wasm").as_slice(),
    )
        .into_response()
}

async fn openapi() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        include_str!("../ui/openapi.json"),
    )
        .into_response()
}

async fn manifest() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/manifest+json; charset=utf-8",
        )],
        include_str!("../ui/manifest.webmanifest"),
    )
        .into_response()
}

async fn icon_192() -> Response {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../ui/icon-192.png").as_slice(),
    )
        .into_response()
}

async fn icon_512() -> Response {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../ui/icon-512.png").as_slice(),
    )
        .into_response()
}

async fn style_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../web/dist/style.css"),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_plugin_ratings() {
        let dir = tempfile::tempdir().expect("tempdir");
        save_rating(dir.path(), "cordis", 4).expect("save");
        assert_eq!(load_ratings(dir.path()).get("cordis"), Some(&4));
    }

    #[test]
    fn tool_end_event_carries_content() {
        let value = serde_json::to_value(ServerEvent::ToolEnd {
            name: "read".into(),
            ok: true,
            content: "hello".into(),
        })
        .expect("json");
        assert_eq!(value["type"], "tool_end");
        assert_eq!(value["name"], "read");
        assert_eq!(value["content"], "hello");
    }

    #[tokio::test]
    async fn question_observer_waits_for_the_answer() {
        let (tx, mut receiver) = broadcast::channel(8);
        let questions = Arc::new(Mutex::new(HashMap::new()));
        let counter = Arc::new(AtomicU64::new(1));
        let observer = BroadcastObserver {
            tx,
            questions: Arc::clone(&questions),
            counter,
        };
        let pending = tokio::spawn(async move {
            observer
                .on_question("Which file?", &["a.rs".to_owned(), "b.rs".to_owned()])
                .await
        });
        let event = receiver.recv().await.expect("event");
        let ServerEvent::Question {
            id,
            question,
            options,
        } = event
        else {
            panic!("expected a question event");
        };
        assert_eq!(question, "Which file?");
        assert_eq!(options, vec!["a.rs", "b.rs"]);
        questions
            .lock()
            .await
            .remove(&id)
            .expect("question")
            .send("a.rs".to_owned())
            .expect("answer");
        assert_eq!(pending.await.expect("task"), Some("a.rs".to_owned()));
    }
    fn capability_state() -> AppState {
        let dir =
            std::env::temp_dir().join(format!("dshr-server-cap-{}", uuid::Uuid::new_v4().simple()));
        let provider = Arc::new(dshr_llm_mock::ScriptedProvider::new("test", Vec::new()));
        AppState::new(
            ServerConfig {
                bind: "127.0.0.1:0".parse().expect("addr"),
                token: "token".into(),
                workspace: dir.clone(),
                mode: PermissionMode::WorkspaceWrite,
                system_prompt: String::new(),
                max_steps: 1,
                max_tokens: 16,
                output_limit: 1024,
                rate_limit_per_min: 0,
                auto_title: false,
                home: dir.clone(),
                profile: "default".into(),
                cordis_plugins: Vec::new(),
                plugins: None,
                wasm: None,
            },
            provider,
            Arc::new(ToolRegistry::new()),
            Arc::new(SessionStore::new(dir.join("sessions"))),
        )
    }

    #[test]
    fn capability_is_scoped_to_one_plugin_and_expires() {
        let state = capability_state();
        let capability = grant_assets(&state, "demo");

        assert!(capability_granted(&state, "demo", &capability));
        // Scope: another plugin cannot use the capability.
        assert!(!capability_granted(&state, "other", &capability));
        // Malformed and unknown capabilities fail closed.
        assert!(!capability_granted(&state, "demo", "short"));
        assert!(!capability_granted(&state, "demo", &"0".repeat(32)));

        // Expiry: a grant in the past is rejected and pruned.
        {
            let mut grants = state.assets.lock().expect("assets");
            grants.get_mut(&capability).expect("grant").expires =
                Instant::now() - Duration::from_secs(1);
        }
        assert!(!capability_granted(&state, "demo", &capability));
        assert!(state.assets.lock().expect("assets").is_empty());
    }

    #[test]
    fn host_header_cannot_inject_into_the_asset_csp() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HOST,
            axum::http::HeaderValue::from_static("evil.test; script-src *"),
        );
        let origin = request_origin(&headers);
        assert_eq!(origin, "http://127.0.0.1");
        let csp = asset_host_csp(&origin);
        assert!(!csp.contains("script-src *"), "{csp}");
        assert!(
            csp.contains("script-src 'unsafe-inline' http://127.0.0.1"),
            "{csp}"
        );

        // A normal host is kept, and only http/https schemes are honoured.
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HOST,
            axum::http::HeaderValue::from_static("127.0.0.1:8080"),
        );
        headers.insert(
            "x-forwarded-proto",
            axum::http::HeaderValue::from_static("javascript"),
        );
        assert_eq!(request_origin(&headers), "http://127.0.0.1:8080");
    }
}
