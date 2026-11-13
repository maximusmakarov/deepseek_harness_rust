use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dshr_agent::PermissionMode;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use serde_json::{Value, json};
use tower::ServiceExt;

fn state(dir: &std::path::Path) -> AppState {
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::TextDelta {
                text: "pong".into(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]],
    ));
    AppState::new(
        ServerConfig {
            bind: "127.0.0.1:0".parse().expect("addr"),
            token: "test-token".into(),
            workspace: PathBuf::from(dir),
            mode: PermissionMode::DangerFull,
            system_prompt: "test".into(),
            max_steps: 4,
            max_tokens: 64,
            output_limit: 10_000,
            rate_limit_per_min: 0,
        },
        provider,
        Arc::new(ToolRegistry::new()),
        Arc::new(SessionStore::new(dir.join("sessions"))),
    )
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Body,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let request = request
        .header("content-type", "application/json")
        .body(body)
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
async fn rejects_missing_token() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let (status, _) = call(&app, "GET", "/v1/sessions", None, Body::empty()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn creates_session_runs_turn_and_lists_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, created) = call(
        &app,
        "POST",
        "/v1/sessions",
        Some("test-token"),
        Body::from("{}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = created["session"]["id"].as_str().expect("id").to_owned();

    let (status, accepted) = call(
        &app,
        "POST",
        &format!("/v1/sessions/{id}/turns"),
        Some("test-token"),
        Body::from(json!({"prompt": "hi"}).to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    assert_eq!(accepted["accepted"], true);

    tokio::time::sleep(Duration::from_millis(300)).await;

    let (status, session) = call(
        &app,
        "GET",
        &format!("/v1/sessions/{id}"),
        Some("test-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let messages = session["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 2, "{session}");
    assert!(session.to_string().contains("pong"), "{session}");

    let (status, listed) = call(
        &app,
        "GET",
        "/v1/sessions",
        Some("test-token"),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["sessions"].as_array().expect("sessions").len(), 1);
}

#[tokio::test]
async fn serves_embedded_ui() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let request = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("dshr"), "{html}");
}

#[tokio::test]
async fn json_rpc_session_methods() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, created) = call(
        &app,
        "POST",
        "/v1/rpc",
        Some("test-token"),
        Body::from(
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "session.create",
                "params": {"title": "rpc"}
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["result"]["session"]["title"], "rpc");

    let (status, listed) = call(
        &app,
        "POST",
        "/v1/rpc",
        Some("test-token"),
        Body::from(
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "session.list", "params": {}
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        listed["result"]["sessions"]
            .as_array()
            .expect("sessions")
            .len(),
        1
    );
}

#[tokio::test]
async fn serves_pwa_assets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let manifest = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/manifest.webmanifest")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("manifest");
    assert_eq!(manifest.status(), StatusCode::OK);
    assert!(
        manifest
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .contains("manifest")
    );
    let body = axum::body::to_bytes(manifest.into_body(), 64 * 1024)
        .await
        .expect("body");
    assert!(String::from_utf8_lossy(&body).contains("\"name\": \"dshr\""));

    let index = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("index");
    let body = axum::body::to_bytes(index.into_body(), 64 * 1024)
        .await
        .expect("body");
    assert!(String::from_utf8_lossy(&body).contains("/manifest.webmanifest"));
}

#[tokio::test]
async fn rate_limits_api_requests() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = ServerConfig {
        bind: "127.0.0.1:0".parse().expect("addr"),
        token: "test-token".into(),
        workspace: PathBuf::from(dir.path()),
        mode: PermissionMode::DangerFull,
        system_prompt: "test".into(),
        max_steps: 4,
        max_tokens: 64,
        output_limit: 10_000,
        rate_limit_per_min: 1,
    };
    let app = router(AppState::new(
        config,
        Arc::new(ScriptedProvider::new("test-model", vec![])),
        Arc::new(ToolRegistry::new()),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
    ));

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("first");
    assert_eq!(first.status(), StatusCode::UNAUTHORIZED);

    let second = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("second");
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        second
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok()),
        Some("60")
    );
}
