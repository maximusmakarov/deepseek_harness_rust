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

mod models_api;

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use dshr_agent::{
    Agent, AgentConfig, AgentObserver, ApprovalPolicy, ApprovalRequest, Approver, PermissionMode,
    ToolCallInfo,
};
use dshr_cordis::client::{FrameMode, FrameSpec, PageRuntime};
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
    /// Allow plugin host routes (C1) when the server binds a non-loopback
    /// address. Off by default: those routes are unauthenticated (like DSH), so
    /// a LAN-exposed server would publish them to the network.
    pub allow_plugin_routes_lan: bool,
    /// Per-run secret for the C1.1 internal session-snapshot bridge. `None`
    /// disables the endpoint (tests/embeddings); `serve` generates one and
    /// publishes it to Cordis sidecars through `<home>/cordis/host.json`.
    pub internal_token: Option<String>,
    /// WASM plugin hot-reload manager.
    pub wasm: Option<Arc<dshr_wasm::WasmManager>>,
    /// UI language (`ru`/`en`) from `ui.language`, served so the web UI can
    /// pick its labels without a second round-trip.
    pub language: String,
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
    /// C5.3 live session feed: every session summary update is broadcast here,
    /// so plugin host halves keep `sessions` current without polling.
    feed: broadcast::Sender<Value>,
    /// Live agents of this server process, bridged to Cordis sidecars through
    /// `/v1/internal/agents` (round 121).
    agents: Arc<dshr_core::AgentRegistry>,
}

/// Time-limited read capability over a plugin's package files.
struct AssetGrant {
    plugin: String,
    expires: Instant,
}

impl AppState {
    /// Live agent registry of this server (bridged to plugin host halves).
    #[must_use]
    pub fn agents(&self) -> &Arc<dshr_core::AgentRegistry> {
        &self.agents
    }

    /// Tool registry serving this server (plugin commands are `cmd__*` tools).
    #[must_use]
    pub fn tools(&self) -> &Arc<ToolRegistry> {
        &self.tools
    }

    #[must_use]
    pub fn new(
        config: ServerConfig,
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        store: Arc<SessionStore>,
    ) -> Self {
        let rate_limit_per_min = config.rate_limit_per_min;
        let agents = Arc::new(dshr_core::AgentRegistry::new());
        // The stored Brave key applies to the running server without a restart.
        let (search_key, search_base) = search_settings(&config.home);
        if search_key.is_some() {
            tools.register(dshr_tools::WebSearchTool::new(
                search_key,
                search_base
                    .unwrap_or_else(|| "https://api.search.brave.com/res/v1/web/search".to_owned()),
            ));
        }
        Self {
            agents,
            config: Arc::new(config),
            provider,
            tools,
            store,
            runtimes: Arc::new(Mutex::new(HashMap::new())),
            rate_limit: Arc::new(RateLimit::new(rate_limit_per_min)),
            assets: Arc::new(std::sync::Mutex::new(HashMap::new())),
            feed: broadcast::channel(256).0,
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
        .route(
            "/v1/config/search",
            get(get_search_config).post(set_search_config),
        )
        .route("/v1/config/providers", get(models_api::get_providers))
        .route(
            "/v1/config/providers/discover",
            post(models_api::discover_provider),
        )
        .route(
            "/v1/config/providers/welcome",
            post(models_api::acknowledge_welcome),
        )
        .route(
            "/v1/credentials/describe",
            post(models_api::describe_credentials),
        )
        .route(
            "/v1/credentials/{reference}",
            put(models_api::store_credential).delete(models_api::delete_credential),
        )
        .route(
            "/v1/config/providers/{id}",
            put(models_api::put_provider).delete(models_api::delete_provider),
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
        .route("/v1/runtime/{file}", get(shared_runtime))
        .route(
            "/v1/config-forms/{entry}",
            get(read_config_form).post(mutate_config_form),
        )
        .route(
            "/v1/internal/session-snapshot",
            get(internal_session_snapshot),
        )
        .route("/v1/internal/session-events", get(internal_session_events))
        .route("/v1/internal/agents", get(internal_agents))
        .route("/v1/commands", get(list_commands))
        .route("/v1/commands/{name}", post(run_command))
        .fallback(plugin_http_proxy)
        .route(
            "/v1/plugins/{name}/asset/{capability}/{*path}",
            get(plugin_client_asset),
        )
        .route("/v1/plugins/toggle", post(toggle_plugin))
        .route("/v1/plugins/remove", post(remove_plugin))
        .route("/v1/workspace/file", get(workspace_file))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turns", post(start_turn))
        .route("/v1/sessions/events", get(session_feed))
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
    mut config: ServerConfig,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
) -> Result<(), String> {
    let bind = config.bind;
    let token = config.token.clone();
    if config.internal_token.is_none() {
        config.internal_token = Some(uuid::Uuid::new_v4().to_string());
    }
    let internal_token = config.internal_token.clone().unwrap_or_default();
    let state = AppState::new(config, provider, tools, store);
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| format!("bind {bind}: {error}"))?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    write_host_bridge(&state.config.home, &address, &internal_token);
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

/// Plugin commands exposed by Cordis sidecars (`cmd__<plugin>__<name>` tools).
async fn list_commands(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let commands: Vec<Value> = state
        .tools
        .definitions()
        .into_iter()
        .filter(|tool| tool.name.starts_with("cmd__"))
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect();
    Ok(Json(json!({ "commands": commands })))
}

#[derive(Debug, Deserialize)]
struct CommandRequest {
    #[serde(default)]
    args: String,
    #[serde(default)]
    session_id: Option<String>,
}

/// Runs one plugin command; the sidecar executes the plugin handler.
async fn run_command(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommandRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    if !name.starts_with("cmd__") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let tool = state.tools.get(&name).ok_or(StatusCode::NOT_FOUND)?;
    let session_id = match body.session_id {
        Some(ref value) => value.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
        None => dshr_core::SessionId::new(),
    };
    let ctx = dshr_tools::ToolCtx {
        cwd: state.config.workspace.clone(),
        session_id,
        output_limit: state.config.output_limit,
        spill_dir: state.config.workspace.join(".dshr/spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = tool
        .execute(&ctx, json!({ "args": body.args }))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "output": outcome.content,
        "is_error": outcome.is_error,
        "files": outcome.files,
    })))
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

pub(crate) fn read_user_config(home: &std::path::Path) -> Value {
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

/// Rejects a level the selected model does not declare.
///
/// The profile is the only place that knows which levels a model offers, so a
/// model it does not describe can only be checked against the tool-wide level
/// list. Refusing here means an unsupported level never reaches a restart that
/// would fail to start the server.
fn validate_reasoning_level(
    document: &dshr_config::ModelsDocument,
    provider: &str,
    model: &str,
    level: &str,
) -> Result<(), StatusCode> {
    let Some(entry) = document
        .providers
        .get(provider)
        .and_then(|profile| profile.models.iter().find(|entry| entry.id == model))
    else {
        return if dshr_config::REASONING_LEVELS.contains(&level) {
            Ok(())
        } else {
            Err(StatusCode::BAD_REQUEST)
        };
    };
    entry
        .reasoning_effort_value(Some(level))
        .map(|_| ())
        .map_err(|_| StatusCode::BAD_REQUEST)
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
    // The stored level, not its wire value: `models.yaml` owns the mapping and
    // only `dshr serve` resolves it against the profile.
    let reasoning_effort = config["model"]["reasoning_effort"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    Ok(Json(json!({
        "provider": provider,
        "active_provider": state.provider.name(),
        "model": state.provider.model(),
        "base_url": base_url,
        "reasoning_effort": reasoning_effort,
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
    /// Selected reasoning level, or absent/empty for the provider's default.
    #[serde(default)]
    reasoning_effort: Option<String>,
}

async fn set_model_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ModelConfigRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let document = dshr_config::ModelsDocument::load(&state.config.home)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // A catalog route is always selectable; a hand-declared one (or any route
    // with a stored profile) is selectable because the runtime can resolve it.
    if (!PROVIDER_ROUTES.contains(&body.provider.as_str())
        && !document.providers.contains_key(&body.provider))
        || !valid_model(&body.model)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    // A non-empty model list is the provider's contract: the active model must
    // be one of its rows, so a removed model cannot be activated by a stale page.
    if let Some(profile) = document.providers.get(&body.provider)
        && !profile.models.is_empty()
        && !profile.models.iter().any(|model| model.id == body.model)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(url) = body.base_url.as_deref()
        && !url.is_empty()
        && !valid_base_url(url)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let reasoning_effort = body
        .reasoning_effort
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(level) = reasoning_effort.as_deref() {
        validate_reasoning_level(&document, &body.provider, &body.model, level)?;
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
    match reasoning_effort {
        Some(level) => model.insert("reasoning_effort".into(), json!(level)),
        None => model.remove("reasoning_effort"),
    };
    write_user_config(&state.config.home, &config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "ok": true,
        "restart_required": true,
        "provider": config["model"]["provider"],
        "model": config["model"]["name"],
        "reasoning_effort": config["model"]["reasoning_effort"],
    })))
}

/// Brave Search settings for the `web_search` tool. The environment wins over
/// the stored key, exactly like the provider from_env paths.
#[derive(Debug, Deserialize)]
struct SearchConfigRequest {
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
}

fn search_settings(home: &std::path::Path) -> (Option<String>, Option<String>) {
    let env_key = std::env::var("BRAVE_SEARCH_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let config = read_user_config(home);
    let stored_key = config["search"]["api_key"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    let stored_base = config["search"]["base_url"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    (env_key.or(stored_key), stored_base)
}

fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else if value.len() <= 8 {
        "•".repeat(value.len())
    } else {
        format!("{}…{}", &value[..4], &value[value.len() - 4..])
    }
}

async fn get_search_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let env_configured = std::env::var("BRAVE_SEARCH_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let config = read_user_config(&state.config.home);
    let stored = config["search"]["api_key"].as_str().unwrap_or_default();
    Ok(Json(json!({
        "configured": env_configured || !stored.trim().is_empty(),
        "source": if env_configured { "env" } else if !stored.trim().is_empty() { "config" } else { "" },
        "api_key_masked": mask_secret(stored),
        "base_url": config["search"]["base_url"].as_str().unwrap_or_default(),
        "env_override": env_configured,
    })))
}

async fn set_search_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SearchConfigRequest>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let key = body.api_key.unwrap_or_default().trim().to_owned();
    if key.len() > 512 || key.chars().any(char::is_whitespace) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let base = body.base_url.unwrap_or_default().trim().to_owned();
    if !base.is_empty() && !valid_base_url(&base) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut config = read_user_config(&state.config.home);
    if !config.is_object() {
        config = json!({});
    }
    let search = config
        .as_object_mut()
        .expect("object")
        .entry("search")
        .or_insert_with(|| json!({}));
    let search = search.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    if key.is_empty() {
        search.remove("api_key");
    } else {
        search.insert("api_key".into(), json!(key));
    }
    if base.is_empty() {
        search.remove("base_url");
    } else {
        search.insert("base_url".into(), json!(base));
    }
    write_user_config(&state.config.home, &config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (key, base) = search_settings(&state.config.home);
    state.tools().register(dshr_tools::WebSearchTool::new(
        key,
        base.unwrap_or_else(|| "https://api.search.brave.com/res/v1/web/search".to_owned()),
    ));
    Ok(Json(json!({"ok": true})))
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
    authorize_service(&state, &headers)?;
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
    authorize_service(&state, &headers)?;
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
    authorize_service(&state, &headers)?;
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
    authorize_service(&state, &headers)?;
    if !valid_plugin_name(&body.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    run_plugin_cli(&state, &["plugin", "npm", "remove", &body.name]).await
}

/// C5.6a: the settings store behind `configForms`.
///
/// One JSON document per entry under `<home>/cordis/config-forms/<entry>.json`
/// (the directory plugin host halves already use), served with a numeric
/// revision so a stale form cannot overwrite a newer one.
fn config_form_entry(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "._:-".contains(ch))
}

fn config_form_path(state: &AppState, entry: &str) -> Option<std::path::PathBuf> {
    if !config_form_entry(entry) {
        return None;
    }
    Some(
        state
            .config
            .home
            .join("cordis")
            .join("config-forms")
            .join(format!("{entry}.json")),
    )
}

fn read_config_form_document(path: &std::path::Path) -> (Value, u64) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (json!({}), 0);
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return (json!({}), 0);
    };
    // Host halves write `{value, revision}`; a bare object is a legacy form.
    if value.get("value").is_some() {
        let revision = value["revision"].as_u64().unwrap_or(0);
        (value["value"].clone(), revision)
    } else {
        (value, 0)
    }
}

fn config_form_snapshot(value: &Value, revision: u64) -> Value {
    json!({
        "status": "ready",
        "value": value,
        "base": {},
        "user": value,
        "revision": revision,
        "writable": true,
    })
}

fn write_config_form_document(
    path: &std::path::Path,
    value: &Value,
    revision: u64,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let document = json!({"value": value, "revision": revision});
    let temporary = path.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(&document).unwrap_or_default(),
    )?;
    std::fs::rename(temporary, path)
}

async fn read_config_form(
    State(state): State<AppState>,
    Path(entry): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    authorize_service(&state, &headers)?;
    let path = config_form_path(&state, &entry).ok_or(StatusCode::NOT_FOUND)?;
    let (value, revision) = read_config_form_document(&path);
    Ok(Json(config_form_snapshot(&value, revision)))
}

#[derive(Debug, Deserialize)]
struct ConfigFormMutate {
    #[serde(default)]
    ops: Vec<ConfigFormOp>,
    #[serde(default)]
    expected_revision: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ConfigFormOp {
    op: String,
    path: Value,
    #[serde(default)]
    value: Value,
}

fn config_form_segments(path: &Value) -> Option<Vec<String>> {
    let segments: Vec<String> = match path {
        Value::String(path) => path
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(str::to_owned)
            .collect(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.as_str().map(str::to_owned))
            .collect(),
        _ => return None,
    };
    (!segments.is_empty()).then_some(segments)
}

fn apply_config_form_ops(value: &mut Value, ops: &[ConfigFormOp]) -> bool {
    for operation in ops {
        let Some(segments) = config_form_segments(&operation.path) else {
            return false;
        };
        let mut cursor = &mut *value;
        for segment in &segments[..segments.len() - 1] {
            if !cursor.is_object() {
                *cursor = json!({});
            }
            let object = cursor.as_object_mut().expect("object");
            cursor = object.entry(segment.clone()).or_insert_with(|| json!({}));
        }
        let last = segments.last().expect("segment");
        match operation.op.as_str() {
            "set" => {
                if !cursor.is_object() {
                    *cursor = json!({});
                }
                cursor
                    .as_object_mut()
                    .expect("object")
                    .insert(last.clone(), operation.value.clone());
            }
            "unset" => {
                if let Some(object) = cursor.as_object_mut() {
                    object.remove(last);
                }
            }
            _ => return false,
        }
    }
    true
}

async fn mutate_config_form(
    State(state): State<AppState>,
    Path(entry): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ConfigFormMutate>,
) -> Result<Json<Value>, StatusCode> {
    authorize_service(&state, &headers)?;
    let path = config_form_path(&state, &entry).ok_or(StatusCode::NOT_FOUND)?;
    let (mut value, revision) = read_config_form_document(&path);
    if let Some(expected) = body.expected_revision
        && expected != revision
    {
        return Err(StatusCode::CONFLICT);
    }
    if !apply_config_form_ops(&mut value, &body.ops) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let next = revision.saturating_add(1);
    write_config_form_document(&path, &value, next)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(config_form_snapshot(&value, next)))
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
    /// Opaque shell frame token echoed in every bridge message.
    #[serde(default)]
    frame: Option<String>,
    /// `registry` (default) or `slot`.
    #[serde(default)]
    mode: Option<String>,
    /// Slot name (DSH slot key) rendered by a slot frame.
    #[serde(default)]
    slot: Option<String>,
    /// Registration id rendered by a slot frame.
    #[serde(default)]
    id: Option<String>,
    /// URL-encoded JSON object handed to the slot render props.
    #[serde(default)]
    context: Option<String>,
    /// URL-encoded JSON object of shell render-site owner props (reference
    /// `renderSlot` owner share), passed through to the frame config.
    #[serde(default, rename = "ownerProps")]
    owner_props: Option<String>,
}

/// Maximum length of the URL-encoded JSON `context` a slot frame accepts.
const FRAME_CONTEXT_MAX_CHARS: usize = 4096;
/// Maximum length of the opaque frame token the shell echoes back.
const FRAME_TOKEN_MAX_CHARS: usize = 120;
/// Maximum length of a slot name or registration id.
const FRAME_NAME_MAX_CHARS: usize = 80;

fn host_query_error(message: impl Into<String>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
}

/// Validates the host-page query and builds the bridge's v1.2 frame config.
///
/// A missing `mode` means `registry` for backwards compatibility; slot frames
/// additionally need the slot name and registration id they render. `frame`,
/// `slot`, `id` and `context` are valid charset-wise whenever they are present
/// — a malformed shell URL should fail loudly instead of silently shrinking to
/// a registry frame. Only slot mode actually forwards the slot fields.
fn frame_spec(query: &ClientHostQuery) -> Result<FrameSpec, (StatusCode, Json<Value>)> {
    let mode = match query.mode.as_deref() {
        None => FrameMode::Registry,
        Some("registry") => FrameMode::Registry,
        Some("slot") => FrameMode::Slot,
        Some(other) => {
            return Err(host_query_error(format!(
                "invalid mode '{other}' (expected 'registry' or 'slot')"
            )));
        }
    };

    let frame = query.frame.as_deref().unwrap_or_default();
    if frame.chars().count() > FRAME_TOKEN_MAX_CHARS
        || !frame
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "._:-".contains(ch))
    {
        return Err(host_query_error(format!("invalid frame token '{frame}'")));
    }

    let slot = query.slot.as_deref().unwrap_or_default();
    if !slot.is_empty()
        && (slot.chars().count() > FRAME_NAME_MAX_CHARS
            || !slot
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "._-".contains(ch)))
    {
        return Err(host_query_error(format!("invalid slot name '{slot}'")));
    }

    let id = query.id.as_deref().unwrap_or_default();
    if !id.is_empty()
        && (id.chars().count() > FRAME_NAME_MAX_CHARS
            || !id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "._:-".contains(ch)))
    {
        return Err(host_query_error(format!("invalid registration id '{id}'")));
    }

    let context = query.context.as_deref().unwrap_or_default();
    let owner_props = query.owner_props.as_deref().unwrap_or_default();
    if !owner_props.is_empty() {
        if owner_props.chars().count() > FRAME_CONTEXT_MAX_CHARS {
            return Err(host_query_error(format!(
                "ownerProps is larger than {FRAME_CONTEXT_MAX_CHARS} characters"
            )));
        }
        if !matches!(
            serde_json::from_str::<Value>(owner_props),
            Ok(Value::Object(_))
        ) {
            return Err(host_query_error("ownerProps must be a JSON object"));
        }
    }
    if !context.is_empty() {
        if context.chars().count() > FRAME_CONTEXT_MAX_CHARS {
            return Err(host_query_error(format!(
                "context is larger than {FRAME_CONTEXT_MAX_CHARS} characters"
            )));
        }
        if !matches!(serde_json::from_str::<Value>(context), Ok(Value::Object(_))) {
            return Err(host_query_error("context must be a JSON object"));
        }
    }

    if mode == FrameMode::Slot {
        if slot.is_empty() {
            return Err(host_query_error("slot is required in slot mode"));
        }
        if id.is_empty() {
            return Err(host_query_error("id is required in slot mode"));
        }
    }

    let (slot, id, context) = if mode == FrameMode::Slot {
        (slot.to_owned(), id.to_owned(), context.to_owned())
    } else {
        (String::new(), String::new(), String::new())
    };
    Ok(FrameSpec {
        frame: frame.to_owned(),
        mode,
        slot,
        id,
        context,
        owner_props: owner_props.to_owned(),
        theme: match query.theme.as_deref() {
            Some("light") => "light".to_owned(),
            _ => "dark".to_owned(),
        },
    })
}

/// Publishes the internal bridge endpoint to Cordis sidecars (C1.1): the URL of
/// this server plus the per-run internal token. Sidecars read it lazily, so a
/// stale file from a previous run is overwritten on every start.
fn write_host_bridge(home: &std::path::Path, address: &SocketAddr, internal_token: &str) {
    let directory = home.join("cordis");
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let payload = json!({
        "url": format!("http://{address}"),
        "token": internal_token,
    });
    let _ = std::fs::write(
        directory.join("host.json"),
        serde_json::to_string(&payload).unwrap_or_default(),
    );
}

/// Round 121: live agent tree for a plugin host half.
///
/// Only metadata is exposed (ids, parentage, session, model, start time); a
/// plugin learns that an agent exists, not what it is working on.
async fn internal_agents(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    internal_token_matches(&state, &headers)?;
    let (agents, revision) = state.agents.snapshot();
    Ok(Json(json!({ "agents": agents, "revision": revision })))
}

/// C1.1: one snapshot of dshr sessions for a plugin host half.
///
/// The sidecar builds `sessions`/`sessionProjections` services from this
/// payload; only recent sessions and usage fields are sent — prompts and
/// message content stay on the server.
async fn internal_session_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    internal_token_matches(&state, &headers)?;
    let summaries = state
        .store
        .list()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sessions: Vec<Value> = summaries
        .iter()
        .take(50)
        .filter_map(|summary| session_summary_entry(&state.store, summary))
        .collect();
    Ok(Json(json!({ "sessions": sessions })))
}

/// C5.3: one session entry in the internal snapshot shape.
fn session_summary_entry(
    store: &SessionStore,
    summary: &dshr_session::SessionSummary,
) -> Option<Value> {
    let session = store.open(summary.id.as_str()).ok()?;
    let mut usage_events = Vec::new();
    let mut llm_ms = 0_u64;
    let mut turns_detail: Vec<Value> = Vec::new();
    for event in &session.events {
        if let dshr_session::SessionEvent::AssistantMessage {
            seq,
            at,
            turn,
            usage,
            latency_ms,
            ..
        } = event
        {
            let latency = latency_ms.unwrap_or(0);
            llm_ms = llm_ms.saturating_add(latency);
            usage_events.push(json!({
                "seq": seq,
                "at": at,
                "turn": turn.as_str(),
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_creation_input_tokens": usage.cache_creation_input_tokens,
                "cache_read_input_tokens": usage.cache_read_input_tokens,
                "latency_ms": latency,
            }));
            // C5.4b: per-turn history, so projections do not lose the turn
            // boundaries when several steps belong to one turn.
            let key = turn.as_str();
            match turns_detail
                .iter_mut()
                .find(|entry| entry["turn"].as_str() == Some(key))
            {
                Some(entry) => {
                    entry["steps"] = json!(entry["steps"].as_u64().unwrap_or(0) + 1);
                    entry["llm_ms"] = json!(entry["llm_ms"].as_u64().unwrap_or(0) + latency);
                    entry["input_tokens"] =
                        json!(entry["input_tokens"].as_u64().unwrap_or(0) + usage.input_tokens);
                    entry["output_tokens"] =
                        json!(entry["output_tokens"].as_u64().unwrap_or(0) + usage.output_tokens);
                }
                None => turns_detail.push(json!({
                    "turn": key,
                    "steps": 1,
                    "llm_ms": latency,
                    "input_tokens": usage.input_tokens,
                    "output_tokens": usage.output_tokens,
                })),
            }
        }
    }
    Some(json!({
        "id": summary.id.as_str(),
        "title": session.header.title,
        "created_at": session.header.created_at,
        "model": session.header.model,
        "turns": session.turn_count(),
        "steps": usage_events.len(),
        "llm_ms": llm_ms,
        "turns_detail": turns_detail,
        "usage_events": usage_events,
    }))
}

/// C5.3: pushes the current summary of one session to every listener.
///
/// Called after a session is created, renamed or finishes a turn, so the
/// internal feed carries the same shape as the snapshot and the sidecar can
/// apply it incrementally.
fn publish_session(state: &AppState, id: &str) {
    let Ok(summaries) = state.store.list() else {
        return;
    };
    let Some(summary) = summaries.iter().find(|summary| summary.id.as_str() == id) else {
        return;
    };
    let Some(entry) = session_summary_entry(&state.store, summary) else {
        return;
    };
    let _ = state
        .feed
        .send(json!({"type": "session", "session": entry}));
}

/// C5.5: service calls from a plugin host half carry the per-run internal token
/// instead of the user's bearer token.
fn authorize_service(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    if state.config.internal_token.is_some() && internal_token_matches(state, headers).is_ok() {
        return Ok(());
    }
    authorize(state, headers, None)
}

fn internal_token_matches(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let Some(expected) = state.config.internal_token.as_deref() else {
        return Err(StatusCode::NOT_FOUND);
    };
    let presented = headers
        .get("x-dshr-internal-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if presented.is_empty() || presented != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

/// C5.3: live internal session feed (SSE).
///
/// Same token as the snapshot; `resync` is emitted when a listener fell behind,
/// so it can hydrate again instead of losing updates silently.
async fn internal_session_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    internal_token_matches(&state, &headers)?;
    Ok(Sse::new(session_feed_stream(state.feed.subscribe())).keep_alive(KeepAlive::default()))
}

/// C5.4: the same feed for the shell, under the ordinary bearer token.
async fn session_feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    authorize(&state, &headers, query.get("token").map(String::as_str))?;
    Ok(Sse::new(session_feed_stream(state.feed.subscribe())).keep_alive(KeepAlive::default()))
}

/// One stream shape for both feeds: `session` frames plus a `resync` marker when
/// a slow listener missed events, so it can hydrate again.
fn session_feed_stream(
    mut receiver: broadcast::Receiver<Value>,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(value) => {
                    yield Ok(Event::default().event("session").data(value.to_string()));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("resync").data("{\"reason\":\"lagged\"}"));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

/// Paths owned by the dshr HTTP API/embedded UI: a plugin route must never
/// shadow them, even if a sidecar somehow registered one.
fn reserved_plugin_path(path: &str) -> bool {
    const RESERVED: [&str; 8] = [
        "/v1",
        "/dshr",
        "/openapi",
        "/manifest.webmanifest",
        "/style.css",
        "/icon-192.png",
        "/icon-512.png",
        "/",
    ];
    path.is_empty()
        || RESERVED
            .iter()
            .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}

/// Rejects cross-site browser requests to plugin routes. Sandboxed frames send
/// `Origin: null`; same-origin pages send a loopback origin; non-browser clients
/// send no `Origin` at all. Anything else is a foreign page.
fn plugin_route_origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return true;
    };
    if origin == "null" {
        return true;
    }
    ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|prefix| origin == *prefix || origin.starts_with(&format!("{prefix}:")))
}

fn plugin_proxy_error(status: StatusCode, message: &str) -> Response {
    let body = Json(json!({ "ok": false, "error": message })).into_response();
    let mut response = body;
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_static("*"),
    );
    response
}

fn with_plugin_cors(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_static("*"),
    );
    response
}

/// Tunnels one plugin upgrade (WebSocket or any other protocol) to its sidecar.
///
/// The upgraded socket is opaque to dshr: the sidecar's `registerUpgrade` handler
/// owns negotiation, so the proxy forwards the request head, replays the
/// upstream handshake response, then copies bytes in both directions. Allowlist
/// and loopback rules mirror the HTTP proxy; upgrades are exact-path only, like
/// the reference table.
async fn plugin_upgrade_proxy(state: AppState, mut request: Request) -> Response {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let path = request.uri().path().to_owned();
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    if reserved_plugin_path(&path) {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    }
    if !state.config.bind.ip().is_loopback() && !state.config.allow_plugin_routes_lan {
        return plugin_proxy_error(
            StatusCode::FORBIDDEN,
            "plugin routes are loopback-only; serve on 127.0.0.1 or pass --allow-lan-plugin-routes",
        );
    }
    let Some(manager) = state.config.plugins.as_ref() else {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    };
    let Some(target) = manager.http_upgrade(&path) else {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    };
    let Ok(mut upstream) = tokio::net::TcpStream::connect(("127.0.0.1", target.port)).await else {
        return plugin_proxy_error(StatusCode::BAD_GATEWAY, "sidecar unavailable");
    };

    // Forward the browser's request head; the sidecar token is the proxy's own
    // credential, never one the browser sent.
    let mut head = format!("{} {}{} HTTP/1.1\r\n", request.method(), path, query);
    for (name, value) in request.headers() {
        if name == header::HOST || name.as_str() == "x-dshr-sidecar-token" {
            continue;
        }
        let Ok(value) = value.to_str() else {
            continue;
        };
        head.push_str(name.as_str());
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("x-dshr-sidecar-token: ");
    head.push_str(&target.token);
    head.push_str("\r\n\r\n");
    if upstream.write_all(head.as_bytes()).await.is_err() {
        return plugin_proxy_error(StatusCode::BAD_GATEWAY, "sidecar write failed");
    }

    // Read the upstream handshake response head before upgrading the browser
    // side: the accept headers must reach the browser unchanged.
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let head_end = loop {
        if let Some(end) = find_head_end(&buffer) {
            break end;
        }
        if buffer.len() > 16 * 1024 {
            return plugin_proxy_error(StatusCode::BAD_GATEWAY, "sidecar handshake too large");
        }
        match tokio::time::timeout_at(deadline, upstream.read(&mut chunk)).await {
            Ok(Ok(0)) => return plugin_proxy_error(StatusCode::BAD_GATEWAY, "sidecar closed"),
            Ok(Ok(read)) => buffer.extend_from_slice(&chunk[..read]),
            Ok(Err(_)) | Err(_) => {
                return plugin_proxy_error(StatusCode::BAD_GATEWAY, "sidecar handshake failed");
            }
        }
    };
    let Some((status, headers)) = parse_upgrade_head(&buffer[..head_end]) else {
        return plugin_proxy_error(StatusCode::BAD_GATEWAY, "invalid sidecar handshake");
    };
    if status != StatusCode::SWITCHING_PROTOCOLS.as_u16() {
        return plugin_proxy_error(
            StatusCode::BAD_GATEWAY,
            &format!("plugin refused the upgrade with status {status}"),
        );
    }
    let trailing = buffer[head_end..].to_vec();
    let on_upgrade = hyper::upgrade::on(&mut request);
    tokio::spawn(async move {
        let Ok(upgraded) = on_upgrade.await else {
            return;
        };
        let mut upgraded = hyper_util::rt::TokioIo::new(upgraded);
        if !trailing.is_empty() {
            let _ = upgraded.write_all(&trailing).await;
        }
        let _ = tokio::io::copy_bidirectional(&mut upgraded, &mut upstream).await;
    });

    let mut response = Response::new(axum::body::Body::empty());
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    for (name, value) in headers {
        response.headers_mut().append(name, value);
    }
    if !response.headers().contains_key(header::CONNECTION) {
        response.headers_mut().insert(
            header::CONNECTION,
            header::HeaderValue::from_static("upgrade"),
        );
    }
    if !response.headers().contains_key(header::UPGRADE) {
        response.headers_mut().insert(
            header::UPGRADE,
            header::HeaderValue::from_static("websocket"),
        );
    }
    response
}

fn find_head_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn parse_upgrade_head(
    head: &[u8],
) -> Option<(u16, Vec<(header::HeaderName, header::HeaderValue)>)> {
    let text = std::str::from_utf8(head).ok()?;
    let mut lines = text.split("\r\n");
    let code: u16 = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let headers: Vec<(header::HeaderName, header::HeaderValue)> = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            let name = header::HeaderName::from_bytes(name.trim().as_bytes()).ok()?;
            let value = header::HeaderValue::from_str(value.trim()).ok()?;
            Some((name, value))
        })
        .collect();
    Some((code, headers))
}

/// C1: proxy a plugin host-half HTTP route to its sidecar.
///
/// A Cordis host half registers plain Node `(req, res)` routes; the sidecar
/// serves them on loopback with a per-process token. dshr exposes only paths
/// present in the live allowlist (no wildcard), keeps the request/response size
/// bounded and adds permissive CORS because plugin frames have an opaque origin.
async fn plugin_http_proxy(State(state): State<AppState>, request: Request) -> Response {
    // `webServer.registerUpgrade`: a socket protocol owner. dshr tunnels the
    // upgraded connection to the sidecar, which owns negotiation.
    if request
        .headers()
        .get(header::UPGRADE)
        .is_some_and(|value| !value.is_empty())
    {
        return plugin_upgrade_proxy(state, request).await;
    }
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    if reserved_plugin_path(&path) {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    }
    // Plugin frames are sandboxed without `allow-same-origin`, so they cannot
    // carry the bearer token or a cookie; the boundary is the browser origin:
    // only an opaque origin (`Origin: null`, a sandboxed frame) or a loopback
    // page may drive plugin routes. Another website's page is rejected.
    if !plugin_route_origin_allowed(request.headers()) {
        return plugin_proxy_error(StatusCode::FORBIDDEN, "cross-origin plugin request");
    }
    // A non-loopback bind still needs the explicit opt-in: the cookie is sent by
    // the frame, but the route reaches the plugin's HTTP surface.
    if !state.config.bind.ip().is_loopback() && !state.config.allow_plugin_routes_lan {
        return plugin_proxy_error(
            StatusCode::FORBIDDEN,
            "plugin routes are loopback-only; serve on 127.0.0.1 or pass --allow-lan-plugin-routes",
        );
    }
    let Some(manager) = state.config.plugins.as_ref() else {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    };
    // The reference webServer has one fallback seat; a plugin may claim it for
    // every path dshr does not own (SPA-like routes). Reserved dshr paths never
    // reach the fallback because they are rejected above.
    let Some(target) = manager
        .http_route(&path)
        .or_else(|| manager.http_fallback())
    else {
        return plugin_proxy_error(StatusCode::NOT_FOUND, "not found");
    };
    // Browser CORS preflight for plugin routes that send JSON bodies.
    if method == axum::http::Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            header::HeaderValue::from_static("*"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            header::HeaderValue::from_static("GET, POST, PUT, PATCH, DELETE, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            header::HeaderValue::from_static("content-type, accept"),
        );
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            header::HeaderValue::from_static("600"),
        );
        return response;
    }
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let content_type = request.headers().get(header::CONTENT_TYPE).cloned();
    let accept = request.headers().get(header::ACCEPT).cloned();
    let body = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(body) => body,
        Err(_) => {
            return plugin_proxy_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "plugin request body too large",
            );
        }
    };
    // Server-sent events must not be cut by the buffered-response timeout.
    let wants_stream = accept
        .as_ref()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/event-stream"));
    let mut builder = reqwest::Client::builder().no_proxy();
    if !wants_stream {
        builder = builder.timeout(Duration::from_secs(15));
    }
    let client = match builder.build() {
        Ok(client) => client,
        Err(_) => return plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin proxy unavailable"),
    };
    let Ok(reqwest_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
        return plugin_proxy_error(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
    };
    let url = format!("http://127.0.0.1:{}{path}{query}", target.port);
    let mut outgoing = client
        .request(reqwest_method, &url)
        .header("x-dshr-sidecar-token", &target.token);
    if let Some(value) = content_type {
        outgoing = outgoing.header("content-type", value.as_bytes());
    }
    if let Some(value) = accept {
        outgoing = outgoing.header("accept", value.as_bytes());
    }
    if !body.is_empty() {
        outgoing = outgoing.body(body);
    }
    let response = match outgoing.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(plugin = %target.plugin, path = %path, %error, "plugin route proxy failed");
            return plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin route failed");
        }
    };
    let status =
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let cache_control = response
        .headers()
        .get(reqwest::header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    // SSE (or any chunked body) is forwarded incrementally, so a plugin can push
    // events; ordinary responses keep the buffered path and its 8 MiB guard.
    let streamable = wants_stream
        || response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"))
        || response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .is_none();
    if streamable {
        let stream =
            futures_util::TryStreamExt::map_err(response.bytes_stream(), std::io::Error::other);
        let mut builder = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .header(header::CACHE_CONTROL, "no-store");
        if let Some(cache_control) = cache_control {
            builder = builder.header(header::CACHE_CONTROL, cache_control);
        }
        return builder
            .body(Body::from_stream(stream))
            .map(with_plugin_cors)
            .unwrap_or_else(|_| {
                plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin route failed")
            });
    }
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin route failed"),
    };
    if bytes.len() > 8 * 1024 * 1024 {
        return plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin response too large");
    }
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if let Some(cache_control) = cache_control {
        builder = builder.header(header::CACHE_CONTROL, cache_control);
    }
    builder
        .body(Body::from(bytes))
        .map(with_plugin_cors)
        .unwrap_or_else(|_| plugin_proxy_error(StatusCode::BAD_GATEWAY, "plugin route failed"))
}

/// Versioned module-table artifact shared by every opt-in client frame of a page
/// (tier B PoC). Public like a static asset: it contains no secrets, and a
/// sandboxed opaque-origin frame could not authenticate anyway.
async fn shared_runtime(Path(file): Path<String>) -> Result<Response, StatusCode> {
    let expected = format!("react-{}.js", dshr_cordis::client::HOST_RUNTIME_VERSION);
    if file != expected {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::HeaderName::from_static("cross-origin-resource-policy"),
                "cross-origin",
            ),
        ],
        shared_runtime_body(),
    )
        .into_response())
}

/// The concatenated host runtime (React UMD + ReactDOM UMD + our module-table
/// glue). Built once; the route serves the same bytes forever per version.
fn shared_runtime_body() -> &'static [u8] {
    static BODY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    BODY.get_or_init(|| {
        let mut body = Vec::with_capacity(160 * 1024);
        body.extend_from_slice(include_bytes!(
            "../assets/runtime/react-18.3.1.production.min.js"
        ));
        body.push(b'\n');
        body.extend_from_slice(include_bytes!(
            "../assets/runtime/react-dom-18.3.1.production.min.js"
        ));
        body.push(b'\n');
        body.extend_from_slice(include_bytes!("../assets/runtime/bootstrap.js"));
        body.push(b'\n');
        // Partial, independently implemented design-system shim (C-lite).
        body.extend_from_slice(include_bytes!("../assets/runtime/primitives.js"));
        body.push(b'\n');
        body.extend_from_slice(include_bytes!("../assets/runtime/markdown.js"));
        body
    })
}

/// Serves the sandboxed host page that runs one client half.
///
/// Single-file bundles are inlined base64 into a network-free page. Bundles
/// with relative imports get a short-lived read capability and load their
/// module graph from the asset route instead. The query also carries the frame
/// config: a registry frame by default, or a slot frame bound to one
/// registration and its occurrence context.
async fn plugin_client_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<ClientHostQuery>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    authorize(&state, &headers, query.token.as_deref())
        .map_err(|status| (status, Json(json!({ "error": "unauthorized" }))))?;
    let spec = frame_spec(&query)?;
    let manager = state.config.plugins.as_ref().ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no plugin supervisor is attached to this server" })),
        )
    })?;
    let half = manager.client_half(&name).map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("plugin '{name}' has no client half") })),
        )
    })?;
    // Optional plugin-provided runtime script (`dsh.client.runtime`): raw JS that
    // runs before the bundle and may publish `window.__DSHR_MODULES__`. The host
    // page injects it as a classic script, so a `</script>` in it would escape the
    // element and execute outside the sandboxed frame contract.
    // One versioned artifact per page: a half that declares
    // `dsh.client.host_modules` gets the shared module table instead of shipping
    // its own copy. The URL is same-origin, so the frame CSP below opens exactly
    // the server origin for scripts and nothing else.
    let shared_runtime = if half.host_modules.is_empty() {
        None
    } else {
        Some(dshr_cordis::client::host_runtime_url())
    };
    let runtime = match half.runtime_path.as_ref() {
        Some(path) => {
            let metadata = tokio::fs::metadata(path).await.map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "failed to read the plugin runtime" })),
                )
            })?;
            if metadata.len() as usize > ASSET_MAX_BYTES {
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(json!({ "error": "the plugin runtime is larger than 8 MiB" })),
                ));
            }
            let bytes = tokio::fs::read(path).await.map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "failed to read the plugin runtime" })),
                )
            })?;
            if contains_script_close(&bytes) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "the plugin runtime contains an unescaped </script>"
                    })),
                ));
            }
            Some(bytes)
        }
        None => None,
    };
    let (page, csp) = match half.bundle {
        dshr_cordis::client::BundleMode::Inline => {
            let bundle = tokio::fs::read(&half.path).await.map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "failed to read the plugin bundle" })),
                )
            })?;
            (
                dshr_cordis::client::host_page(
                    &half,
                    &bundle,
                    &spec,
                    PageRuntime {
                        shared: shared_runtime.as_deref(),
                        plugin: runtime.as_deref(),
                    },
                ),
                client_host_csp(
                    &request_origin(&headers),
                    shared_runtime.is_some(),
                    !half.host_modules.is_empty(),
                ),
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
                dshr_cordis::client::host_page_assets(
                    &half,
                    &base,
                    &spec,
                    PageRuntime {
                        shared: shared_runtime.as_deref(),
                        plugin: runtime.as_deref(),
                    },
                ),
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

/// CSP for an inline host page that opts into the shared host module table. The
/// only new permission is loading classic scripts from this server origin (the
/// versioned `/v1/runtime/react-<ver>.js` artifact); connect/img/style stay as
/// locked down as the network-free page, so no token or plugin data can leave.
fn client_host_csp(origin: &str, shared_runtime: bool, allow_connect: bool) -> String {
    if !shared_runtime && !allow_connect {
        return CLIENT_HOST_CSP.to_owned();
    }
    let script = if shared_runtime {
        format!("'unsafe-inline' blob: {origin}")
    } else {
        "'unsafe-inline' blob:".to_owned()
    };
    // Real plugins fetch their own host-half routes (`ctx.inject(['webServer'])`).
    // Opening connect-src to the server origin is the C1 trade-off: no external
    // network, but the frame may reach same-origin plugin routes (like DSH does).
    let connect = if allow_connect { origin } else { "'none'" };
    format!(
        "default-src 'none'; script-src {script}; style-src 'unsafe-inline'; \
         img-src data: blob:; connect-src {connect}; base-uri 'none'; form-action 'none'; frame-ancestors 'self'"
    )
}

/// Case-insensitive search for a closing script tag in plugin-provided JS. The
/// runtime is injected as a raw classic script, so it must not be able to close
/// the element and escape into the rest of the host page.
fn contains_script_close(bytes: &[u8]) -> bool {
    const NEEDLE: &[u8] = b"</script";
    bytes.windows(NEEDLE.len()).any(|window| {
        window
            .iter()
            .zip(NEEDLE)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

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

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "language": &state.config.language,
    }))
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
    publish_session(&state, session.header.id.as_str());
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

/// One workspace file for the shell's file inspector (`openFile` owner callback).
///
/// Read-only, workspace-confined (after canonicalization), capped at 64 KiB so a
/// plugin cannot use `openFile` to pull an unbounded payload into the UI.
async fn workspace_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    authorize(&state, &headers, None)?;
    let requested = query
        .get("path")
        .map(String::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if requested.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let candidate = std::path::Path::new(requested);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        state.config.workspace.join(candidate)
    };
    let root = tokio::fs::canonicalize(&state.config.workspace)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let path = tokio::fs::canonicalize(&joined)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !path.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !metadata.is_file() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let limit = 64 * 1024usize;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let truncated = bytes.len() > limit;
    let content = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]).to_string();
    let relative = path.strip_prefix(&root).unwrap_or(&path);
    Ok(Json(json!({
        "path": relative.to_string_lossy(),
        "content": content,
        "truncated": truncated,
    })))
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
        let title_state = state.clone();
        let title_session = id.clone();
        let title_prompt = body.prompt.clone();
        tokio::spawn(async move {
            if let Ok(Some(title)) =
                dshr_agent::generate_title(provider.as_ref(), &title_prompt).await
            {
                let _ = store.set_title(&title_session, &title);
                publish_session(&title_state, &title_session);
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
    let turn_state = state.clone();
    let runtime = Arc::clone(&runtime);
    let agents = Arc::clone(&state.agents);
    tokio::spawn(async move {
        // Publish the running agent so plugin host halves see the tree grow and
        // shrink; the guard removes it when the turn ends.
        let mut live = dshr_core::LiveAgent::root(
            session_id.clone(),
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        );
        live.title = turn_state
            .store
            .open(&session_id)
            .ok()
            .and_then(|session| session.header.title.clone());
        live.model = Some(turn_state.provider.name().to_owned());
        let _agent_guard = agents.register(live);
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
                let _ = tx.send(ServerEvent::Cancelled {
                    session_id: session_id.clone(),
                });
            }
            Err(error) => {
                let _ = tx.send(ServerEvent::Error {
                    message: error.to_string(),
                });
            }
        }
        runtime.busy.store(false, Ordering::SeqCst);
        *runtime.cancel.lock().await = None;
        publish_session(&turn_state, &session_id);
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

pub(crate) fn authorize(
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
                allow_plugin_routes_lan: false,
                internal_token: None,
                wasm: None,
                language: "ru".into(),
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
