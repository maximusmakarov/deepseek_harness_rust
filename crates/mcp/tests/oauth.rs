use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use dshr_mcp::{McpOAuthSpec, McpServerSpec, connect_server};
use serde_json::{Value, json};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    authorized: Arc<Mutex<bool>>,
}

async fn token() -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        Json(json!({"access_token": "secret-token", "token_type": "Bearer", "expires_in": 3600})),
    )
        .into_response()
}

async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if request["method"].as_str() == Some("notifications/initialized") {
        return StatusCode::ACCEPTED.into_response();
    }
    let expected = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some("Bearer secret-token");
    if expected {
        *state.authorized.lock().await = true;
    }
    let id = request["id"].clone();
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "oauth-mock", "version": "0.1.0"}
        }),
        "tools/list" => json!({"tools": []}),
        _ => json!({}),
    };
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::HeaderName::from_static("mcp-session-id"),
                "oauth-session",
            ),
        ],
        Json(json!({"jsonrpc": "2.0", "id": id, "result": result})),
    )
        .into_response()
}

#[tokio::test]
async fn fetches_client_credentials_token() {
    let state = AppState {
        authorized: Arc::new(Mutex::new(false)),
    };
    let app = axum::Router::new()
        .route("/token", post(token))
        .route("/mcp", post(handler))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let spec = McpServerSpec {
        name: "oauth".into(),
        command: String::new(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: Some(format!("http://{address}/mcp")),
        headers: BTreeMap::new(),
        oauth: Some(McpOAuthSpec {
            token_url: format!("http://{address}/token"),
            client_id: "client".into(),
            client_secret: "secret".into(),
            scope: Some("tools".into()),
        }),
    };
    let (_backend, _tools) = connect_server(spec).await.expect("connect");
    assert!(*state.authorized.lock().await);
}

#[derive(Clone)]
struct RefreshState {
    grants: Arc<Mutex<Vec<String>>>,
    unauthorized: Arc<Mutex<usize>>,
    initial_expires_in: u64,
}

async fn refresh_token(
    State(state): State<RefreshState>,
    axum::extract::Form(form): axum::extract::Form<std::collections::BTreeMap<String, String>>,
) -> Response {
    let grant = form
        .get("grant_type")
        .cloned()
        .unwrap_or_else(|| "missing".into());
    state.grants.lock().await.push(grant.clone());
    let body = match grant.as_str() {
        "client_credentials" => {
            json!({
                "access_token": "stale",
                "refresh_token": "refresh-1",
                "expires_in": state.initial_expires_in
            })
        }
        "refresh_token" if form.get("refresh_token").map(String::as_str) == Some("refresh-1") => {
            json!({"access_token": "fresh", "refresh_token": "refresh-2", "expires_in": 3600})
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_grant"})),
            )
                .into_response();
        }
    };
    ([(header::CONTENT_TYPE, "application/json")], Json(body)).into_response()
}

async fn refresh_handler(
    State(state): State<RefreshState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if request["method"].as_str() == Some("notifications/initialized") {
        return StatusCode::ACCEPTED.into_response();
    }
    if headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some("Bearer fresh")
    {
        *state.unauthorized.lock().await += 1;
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    let id = request["id"].clone();
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "refresh-mock", "version": "0.1.0"}
        }),
        "tools/list" => json!({"tools": []}),
        _ => json!({}),
    };
    (
        [(header::CONTENT_TYPE, "application/json")],
        Json(json!({"jsonrpc": "2.0", "id": id, "result": result})),
    )
        .into_response()
}

#[tokio::test]
async fn refreshes_token_on_unauthorized() {
    let state = RefreshState {
        grants: Arc::new(Mutex::new(Vec::new())),
        unauthorized: Arc::new(Mutex::new(0)),
        initial_expires_in: 3600,
    };
    let app = axum::Router::new()
        .route("/token", post(refresh_token))
        .route("/mcp", post(refresh_handler))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let spec = McpServerSpec {
        name: "oauth".into(),
        command: String::new(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: Some(format!("http://{address}/mcp")),
        headers: BTreeMap::new(),
        oauth: Some(McpOAuthSpec {
            token_url: format!("http://{address}/token"),
            client_id: "client".into(),
            client_secret: "secret".into(),
            scope: None,
        }),
    };
    let (_backend, _tools) = connect_server(spec).await.expect("connect");
    assert_eq!(*state.unauthorized.lock().await, 1);
    assert_eq!(
        *state.grants.lock().await,
        vec!["client_credentials", "refresh_token"]
    );
}

#[tokio::test]
async fn refreshes_expired_token_before_the_first_request() {
    let state = RefreshState {
        grants: Arc::new(Mutex::new(Vec::new())),
        unauthorized: Arc::new(Mutex::new(0)),
        // A zero lifetime forces a refresh before `initialize`.
        initial_expires_in: 0,
    };
    let app = axum::Router::new()
        .route("/token", post(refresh_token))
        .route("/mcp", post(refresh_handler))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let spec = McpServerSpec {
        name: "oauth".into(),
        command: String::new(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: Some(format!("http://{address}/mcp")),
        headers: BTreeMap::new(),
        oauth: Some(McpOAuthSpec {
            token_url: format!("http://{address}/token"),
            client_id: "client".into(),
            client_secret: "secret".into(),
            scope: None,
        }),
    };
    let (_backend, _tools) = connect_server(spec).await.expect("connect");
    assert_eq!(*state.unauthorized.lock().await, 0);
    assert_eq!(
        *state.grants.lock().await,
        vec!["client_credentials", "refresh_token"]
    );
}
