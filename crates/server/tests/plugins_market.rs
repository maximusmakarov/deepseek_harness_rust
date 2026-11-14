//! Plugin market endpoints: searchable catalog, entry details and the
//! server-side review aggregate (ROADMAP P1).

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dshr_agent::PermissionMode;
use dshr_llm::StreamChunk;
use dshr_llm_mock::ScriptedProvider;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "market-token";

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
            cordis_plugins: vec![("cordis".into(), true)],
            plugins: None,
            wasm: None,
        },
        provider,
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
async fn catalog_lists_entries_with_categories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let (status, body) = call(&app, "GET", "/v1/plugins/catalog", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let packages = body["packages"].as_array().expect("packages");
    assert_eq!(packages.len(), 3, "{body}");
    let cordis = packages
        .iter()
        .find(|package| package["name"] == "cordis")
        .expect("cordis");
    assert_eq!(cordis["category"], "framework");
    assert!(cordis["tags"].as_array().expect("tags").len() >= 2);
    let categories = body["categories"].as_array().expect("categories");
    assert!(
        categories
            .iter()
            .any(|category| category["name"] == "framework" && category["count"] == 1),
        "{body}"
    );
}

#[tokio::test]
async fn catalog_searches_and_filters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, body) = call(&app, "GET", "/v1/plugins/catalog?q=timer", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let packages = body["packages"].as_array().expect("packages");
    assert_eq!(packages.len(), 1, "{body}");
    assert_eq!(packages[0]["name"], "@deepseek-ai/cordis-plugin-timer");

    // Search covers descriptions and tags, not only names.
    let (_, body) = call(&app, "GET", "/v1/plugins/catalog?q=spatiotemporal", None).await;
    assert_eq!(body["packages"].as_array().expect("packages").len(), 1);

    let (_, body) = call(&app, "GET", "/v1/plugins/catalog?category=loader", None).await;
    let packages = body["packages"].as_array().expect("packages");
    assert_eq!(packages.len(), 1, "{body}");
    assert_eq!(packages[0]["category"], "loader");

    let (_, body) = call(&app, "GET", "/v1/plugins/catalog?limit=1", None).await;
    assert_eq!(body["packages"].as_array().expect("packages").len(), 1);

    let (_, body) = call(&app, "GET", "/v1/plugins/catalog?q=no-such-package", None).await;
    assert!(body["packages"].as_array().expect("packages").is_empty());
}

#[tokio::test]
async fn catalog_entry_reports_reviews_rating_and_install_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, body) = call(&app, "GET", "/v1/plugins/catalog/cordis", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["package"]["name"], "cordis");
    assert_eq!(body["installed"], true);
    assert_eq!(body["reviews"]["count"], 0);

    let (status, _) = call(
        &app,
        "POST",
        "/v1/plugins/rate",
        Some(json!({"name": "cordis", "rating": 5})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    for rating in [5, 3] {
        let (status, _) = call(
            &app,
            "POST",
            "/v1/plugins/review",
            Some(json!({"name": "cordis", "rating": rating, "text": "works"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = call(&app, "GET", "/v1/plugins/catalog/cordis", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["reviews"]["count"], 2);
    assert_eq!(body["reviews"]["average"], 4.0);
    assert_eq!(body["rating"], 5);
}

#[tokio::test]
async fn catalog_entry_rejects_unknown_and_invalid_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, _) = call(&app, "GET", "/v1/plugins/catalog/not-in-catalog", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Only curated entries are matched, so a traversal-shaped name can never
    // reach the filesystem; the router may answer 400 or 404 depending on how
    // the segment is decoded, but never 200.
    let (status, _) = call(&app, "GET", "/v1/plugins/catalog/..%2Fescape", None).await;
    assert!(
        matches!(status, StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND),
        "{status}"
    );
}
