//! Provider settings endpoints: read the active route and write the user
//! configuration layer without touching stored credentials.

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
async fn reports_the_active_provider_and_routes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let (status, body) = call(&app, "GET", "/v1/config/model", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["model"], "test-model");
    // Nothing is persisted yet, so the editable route is empty while the live
    // process names its provider separately.
    assert_eq!(body["provider"], "");
    assert!(
        body["active_provider"]
            .as_str()
            .is_some_and(|name| !name.is_empty())
    );
    let routes = body["routes"].as_array().expect("routes");
    assert!(routes.iter().any(|route| route == "openai-compatible"));
    assert!(body["base_url"].is_null());
}

#[tokio::test]
async fn writes_the_user_layer_and_preserves_other_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "version: 1\nsession:\n  auto_title: true\ntools:\n  max_parallel: 3\n",
    )
    .expect("seed config");
    let app = router(state(dir.path()));

    let (status, body) = call(
        &app,
        "POST",
        "/v1/config/model",
        Some(json!({
            "provider": "openai-compatible",
            "model": "deepseek-v4-pro",
            "base_url": "https://router.example/v1",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["restart_required"], true);

    let written = std::fs::read_to_string(dir.path().join("config.yaml")).expect("config");
    let parsed: Value = serde_saphyr::from_str(&written).expect("yaml");
    assert_eq!(parsed["model"]["provider"], "openai-compatible");
    assert_eq!(parsed["model"]["name"], "deepseek-v4-pro");
    assert_eq!(parsed["model"]["base_url"], "https://router.example/v1");
    // Untouched keys survive the rewrite.
    assert_eq!(parsed["session"]["auto_title"], true);
    assert_eq!(parsed["tools"]["max_parallel"], 3);

    // The read endpoint surfaces the stored base URL.
    let (status, body) = call(&app, "GET", "/v1/config/model", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["base_url"], "https://router.example/v1");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("config.yaml"))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o600,
            "config with provider settings must stay private"
        );
    }
}

#[tokio::test]
async fn base_url_can_be_cleared() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "model:\n  base_url: https://old.example\n",
    )
    .expect("seed config");
    let app = router(state(dir.path()));
    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/model",
        Some(json!({"provider": "deepseek", "model": "deepseek-flash", "base_url": ""})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let written = std::fs::read_to_string(dir.path().join("config.yaml")).expect("config");
    let parsed: Value = serde_saphyr::from_str(&written).expect("yaml");
    assert!(parsed["model"]["base_url"].is_null(), "{parsed}");
}

#[tokio::test]
async fn rejects_unknown_routes_models_and_urls() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    for body in [
        json!({"provider": "not-a-route", "model": "x"}),
        json!({"provider": "deepseek", "model": ""}),
        json!({"provider": "deepseek", "model": "has space"}),
        json!({"provider": "deepseek", "model": "ok", "base_url": "ftp://example"}),
        json!({"provider": "deepseek", "model": "ok", "base_url": "https://exa mple"}),
    ] {
        let (status, _) = call(&app, "POST", "/v1/config/model", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    assert!(
        !dir.path().join("config.yaml").exists(),
        "a rejected request must not write the config"
    );
}
