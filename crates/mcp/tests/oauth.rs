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
