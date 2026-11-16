//! Web search settings: the Brave key lives in the user config layer and the
//! running server re-registers `web_search` immediately after a save.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dshr_agent::PermissionMode;
use dshr_llm::{LlmProvider, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "settings-token";

fn state(dir: &std::path::Path) -> AppState {
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![vec![StreamChunk::Done {
            stop_reason: dshr_llm::StopReason::EndTurn,
        }]],
    ));
    AppState::new(
        ServerConfig {
            bind: "127.0.0.1:0".parse().expect("addr"),
            token: TOKEN.into(),
            workspace: PathBuf::from(dir),
            mode: PermissionMode::DangerFull,
            system_prompt: "test".into(),
            max_steps: 2,
            max_tokens: 64,
            output_limit: 10_000,
            rate_limit_per_min: 0,
            auto_title: false,
            home: dir.to_path_buf(),
            profile: "default".into(),
            cordis_plugins: Vec::new(),
            plugins: None,
            allow_plugin_routes_lan: false,
            internal_token: None,
            wasm: None,
        },
        provider.clone() as Arc<dyn LlmProvider>,
        Arc::new(ToolRegistry::with_builtin()),
        Arc::new(SessionStore::new(dir.join("sessions"))),
    )
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let value = if bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({}))
    };
    (status, value)
}

#[tokio::test]
async fn reads_and_writes_the_brave_key() {
    let dir = tempfile::tempdir().expect("dir");
    let state = state(dir.path());
    let app = router(state);

    let (status, value) = call(&app, "GET", "/v1/config/search", None).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["configured"], false);
    assert_eq!(value["source"], "");

    let (status, value) = call(
        &app,
        "POST",
        "/v1/config/search",
        Some(json!({"api_key": "brave-secret-key", "base_url": "https://search.example/v1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");

    let (status, value) = call(&app, "GET", "/v1/config/search", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["configured"], true);
    assert_eq!(value["source"], "config");
    assert_eq!(value["api_key_masked"], "brav…-key");
    assert_eq!(value["base_url"], "https://search.example/v1");
}

#[tokio::test]
async fn rejects_bad_search_settings() {
    let dir = tempfile::tempdir().expect("dir");
    let app = router(state(dir.path()));
    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/search",
        Some(json!({"api_key": "k", "base_url": "ftp://nope"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/search",
        Some(json!({"api_key": "with space"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
