#![forbid(unsafe_code)]
//! HTTP/SSE server and embedded web UI.

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
use dshr_tools::{ToolOutcome, ToolRegistry};
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
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<ServerConfig>,
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    store: Arc<SessionStore>,
    runtimes: Arc<Mutex<HashMap<String, Arc<Runtime>>>>,
    rate_limit: Arc<RateLimit>,
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
    },
    ToolEnd {
        name: String,
        ok: bool,
    },
    Approval {
        id: String,
        tool: String,
        summary: String,
        reason: String,
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
        });
    }

    async fn on_tool_end(&self, call: &ToolCallInfo, outcome: &ToolOutcome) {
        let _ = self.tx.send(ServerEvent::ToolEnd {
            name: call.name.clone(),
            ok: !outcome.is_error,
        });
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
        .route("/app.js", get(app_js))
        .route("/style.css", get(style_css))
        .route("/openapi.json", get(openapi))
        .route("/manifest.webmanifest", get(manifest))
        .route("/icon-192.png", get(icon_192))
        .route("/icon-512.png", get(icon_512))
        .route("/v1/health", get(health))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turns", post(start_turn))
        .route("/v1/sessions/{id}/events", get(session_events))
        .route("/v1/sessions/{id}/cancel", post(cancel_turn))
        .route(
            "/v1/sessions/{id}/approvals/{approval_id}",
            post(resolve_approval),
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
        let observer = BroadcastObserver { tx: tx.clone() };
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
    Html(include_str!("../ui/index.html"))
}

async fn app_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../ui/app.js"),
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
        include_str!("../ui/style.css"),
    )
        .into_response()
}
