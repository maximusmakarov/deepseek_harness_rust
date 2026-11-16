//! Models settings: provider directory, profiles and the revision fence.

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

const TOKEN: &str = "models-token";

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

async fn get(app: &axum::Router) -> Value {
    let (status, body) = call(app, "GET", "/v1/config/providers", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn lists_the_catalog_and_the_active_route() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "version: 1\nmodel:\n  provider: deepseek-account\n  name: deepseek-flash\n",
    )
    .expect("seed config");
    let app = router(state(dir.path()));

    let body = get(&app).await;
    assert_eq!(body["revision"], 0);
    assert_eq!(body["writable"], true);
    assert_eq!(body["active"]["provider"], "deepseek-account");
    assert_eq!(body["active"]["model"], "deepseek-flash");

    let catalog = body["catalog"].as_array().expect("catalog");
    let ids: Vec<&str> = catalog
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    assert!(ids.contains(&"deepseek-official"));
    assert!(ids.contains(&"openai-compatible"));
    assert!(ids.contains(&"routerai"));

    // The active route appears even without a stored profile, with the
    // catalog's models and a derived credential state.
    let providers = body["providers"].as_array().expect("providers");
    assert_eq!(providers.len(), 1);
    let row = &providers[0];
    assert_eq!(row["id"], "deepseek-account");
    assert_eq!(row["configured"], false);
    assert_eq!(row["removable"], false);
    assert_eq!(row["active"], true);
    assert_eq!(row["protocol"], "anthropic-messages");
    assert_eq!(row["models_from_profile"], false);
    assert_eq!(row["models"][0]["id"], "deepseek-flash");
    assert!(row["credential_configured"].is_boolean());
}

#[tokio::test]
async fn writes_a_profile_and_fences_the_revision() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "version: 1\nmodel:\n  provider: deepseek-official\n  name: deepseek-v4-pro\nsession:\n  auto_title: true\n",
    )
    .expect("seed config");
    let app = router(state(dir.path()));

    let body = json!({
        "expected_revision": 0,
        "protocol": "anthropic-messages",
        "base_url": "https://gateway.example/anthropic",
        "api_key_env": "MY_DEEPSEEK_KEY",
        "retry": {
            "mode": "normal",
            "max_retries": 7,
            "retryable_codes": ["RATE_LIMIT", "SERVER"],
            "backoff": {
                "initial_delay_ms": 250,
                "max_delay_ms": 2000,
                "jitter_ratio": 0.25
            }
        },
        "models": [
            {
                "id": "deepseek-flash",
                "name": "Flash via gateway",
                "context_window": 1000000,
                "max_tokens": 64000,
                "input": ["text", "image"],
                "reasoning_efforts": {"off": "off", "high": "high"}
            }
        ]
    });
    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/config/providers/deepseek-official",
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["revision"], 1);

    let document = dir.path().join("models.yaml");
    let written = std::fs::read_to_string(&document).expect("models.yaml");
    assert!(written.contains("deepseek-official"), "{written}");
    assert!(written.contains("MY_DEEPSEEK_KEY"), "{written}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&document)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    let body = get(&app).await;
    assert_eq!(body["revision"], 1);
    let row = body["providers"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == "deepseek-official"))
        .expect("row");
    assert_eq!(row["configured"], true);
    assert_eq!(row["removable"], true);
    assert_eq!(row["active"], true);
    assert_eq!(row["base_url"], "https://gateway.example/anthropic");
    assert_eq!(row["api_key_env"], "MY_DEEPSEEK_KEY");
    assert_eq!(row["models_from_profile"], true);
    assert_eq!(row["models"][0]["name"], "Flash via gateway");
    assert_eq!(row["models"][0]["context_window"], 1_000_000);
    assert_eq!(row["models"][0]["reasoning_efforts"]["high"], "high");
    assert_eq!(row["retry"]["mode"], "normal");
    assert_eq!(row["retry"]["max_retries"], 7);
    assert_eq!(
        row["retry"]["retryable_codes"],
        json!(["RATE_LIMIT", "SERVER"])
    );
    assert_eq!(row["retry"]["backoff"]["jitter_ratio"], 0.25);

    // The active selection lives in config.yaml and this write leaves it alone.
    let config: Value = serde_saphyr::from_str(
        &std::fs::read_to_string(dir.path().join("config.yaml")).expect("config"),
    )
    .expect("yaml");
    assert_eq!(config["model"]["provider"], "deepseek-official");
    assert_eq!(config["model"]["name"], "deepseek-v4-pro");
    assert_eq!(config["session"]["auto_title"], true);

    // A stale revision is refused and nothing changes.
    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/config/providers/deepseek-official",
        Some(json!({
            "expected_revision": 0,
            "base_url": null,
            "api_key_env": null,
            "models": []
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(
        answer["error"]
            .as_str()
            .is_some_and(|text| text.contains("revision"))
    );
    assert_eq!(get(&app).await["revision"], 1);
}

#[tokio::test]
async fn deletes_a_profile_back_to_the_catalog() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "version: 1\nmodel:\n  provider: deepseek-account\n  name: deepseek-flash\n",
    )
    .expect("seed config");
    let app = router(state(dir.path()));
    let write = |revision: u64| {
        json!({
            "expected_revision": revision,
            "base_url": "https://gateway.example",
            "api_key_env": null,
            "models": [{"id": "custom-model"}]
        })
    };
    let (status, _) = call(
        &app,
        "PUT",
        "/v1/config/providers/openai-compatible",
        Some(write(0)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(get(&app).await["revision"], 1);

    let (status, answer) = call(
        &app,
        "DELETE",
        "/v1/config/providers/openai-compatible?expected_revision=0",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");

    let (status, answer) = call(
        &app,
        "DELETE",
        "/v1/config/providers/openai-compatible?expected_revision=1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["revision"], 2);
    let rows = get(&app).await["providers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "deepseek-account");
    assert_eq!(rows[0]["configured"], false);

    let (status, _) = call(
        &app,
        "DELETE",
        "/v1/config/providers/openai-compatible?expected_revision=2",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rejects_invalid_profiles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let cases = [
        json!({
            "expected_revision": 0,
            "protocol": "openai-chat",
            "base_url": null,
            "api_key_env": null,
            "models": []
        }),
        json!({
            "expected_revision": 0,
            "base_url": "ftp://example",
            "api_key_env": null,
            "models": []
        }),
        json!({
            "expected_revision": 0,
            "base_url": null,
            "api_key_env": null,
            "models": [{"id": "same"}, {"id": "same"}]
        }),
    ];
    for body in cases {
        let (status, answer) = call(
            &app,
            "PUT",
            "/v1/config/providers/deepseek-official",
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    }

    // A retry policy whose initial delay exceeds its ceiling is refused.
    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/config/providers/deepseek-official",
        Some(json!({
            "expected_revision": 0,
            "base_url": null,
            "api_key_env": null,
            "models": [],
            "retry": {
                "mode": "normal",
                "backoff": {"initial_delay_ms": 5000, "max_delay_ms": 1000}
            }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");

    // A route outside the catalog needs an explicit protocol.
    let (status, _) = call(
        &app,
        "PUT",
        "/v1/config/providers/my-gateway",
        Some(json!({
            "expected_revision": 0,
            "base_url": "https://gateway.example/v1",
            "api_key_env": null,
            "models": []
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // With one, it is a first-class provider row.
    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/config/providers/my-gateway",
        Some(json!({
            "expected_revision": 0,
            "protocol": "openai-chat",
            "base_url": "https://gateway.example/v1",
            "api_key_env": "GATEWAY_KEY",
            "models": [{"id": "gateway-model"}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let row = get(&app).await["providers"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == "my-gateway").cloned())
        .expect("custom row");
    assert_eq!(row["protocol"], "openai-chat");
    assert_eq!(row["effective_base_url"], "https://gateway.example/v1");
    assert_eq!(row["models"][0]["id"], "gateway-model");

    // The hand-declared route can become active, and its model list is the
    // contract the activation is checked against.
    let (status, answer) = call(
        &app,
        "POST",
        "/v1/config/model",
        Some(json!({"provider": "my-gateway", "model": "gateway-model"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/model",
        Some(json!({"provider": "my-gateway", "model": "not-listed"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn requires_the_bearer_token() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let request = Request::builder()
        .uri("/v1/config/providers")
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn discovers_models_from_an_openai_compatible_endpoint() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let address = listener.local_addr().expect("addr");
    let mock = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buffer = [0_u8; 4096];
        let _ = socket.read(&mut buffer).await;
        let body = r#"{"data":[{"id":"alpha","name":"Alpha"},{"id":"beta"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
    });

    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    let (status, body) = call(
        &app,
        "POST",
        "/v1/config/providers/discover",
        Some(json!({
            "protocol": "openai-chat",
            "base_url": format!("http://{address}/v1")
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["count"], 2);
    assert_eq!(body["models"][0]["id"], "alpha");
    assert_eq!(body["models"][0]["name"], "Alpha");
    assert_eq!(body["models"][1]["name"], "beta");
    mock.await.expect("mock");

    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/providers/discover",
        Some(json!({"protocol": "openai-chat", "base_url": "ftp://example"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/providers/discover",
        Some(json!({"protocol": "nope", "base_url": "https://example.test"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn acknowledges_the_welcome_notice_durably() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));
    assert_eq!(get(&app).await["welcome_version"], Value::Null);

    let (status, answer) = call(
        &app,
        "POST",
        "/v1/config/providers/welcome",
        Some(json!({"version": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["welcome_version"], 1);
    assert_eq!(get(&app).await["welcome_version"], 1);
    let written = std::fs::read_to_string(dir.path().join("models.yaml")).expect("models.yaml");
    assert!(written.contains("welcome_version: 1"), "{written}");

    let (status, _) = call(
        &app,
        "POST",
        "/v1/config/providers/welcome",
        Some(json!({"version": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stores_credentials_write_only_and_joins_them_into_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = router(state(dir.path()));

    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/credentials/MY_TEST_KEY",
        Some(json!({"value": "sk-1234567890abcdef", "expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["masked"], "sk-1…cdef");
    assert!(!answer.to_string().contains("1234567890"));

    let (status, answer) = call(
        &app,
        "POST",
        "/v1/credentials/describe",
        Some(json!({"references": ["MY_TEST_KEY", "MISSING_KEY"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["credentials"]["MY_TEST_KEY"]["configured"], true);
    assert_eq!(answer["credentials"]["MY_TEST_KEY"]["source"], "config");
    assert_eq!(answer["credentials"]["MISSING_KEY"]["configured"], false);
    assert!(!answer.to_string().contains("1234567890"));

    // A profile naming the reference joins the stored key into its row.
    let (status, answer) = call(
        &app,
        "PUT",
        "/v1/config/providers/deepseek-official",
        Some(json!({
            "expected_revision": 0,
            "base_url": null,
            "api_key_env": "MY_TEST_KEY",
            "models": [],
            "retry": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let row = get(&app).await["providers"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["id"] == "deepseek-official")
                .cloned()
        })
        .expect("row");
    assert_eq!(row["credential_configured"], true);
    assert_eq!(row["credential_source"], "config");
    assert_eq!(row["credential_masked"], "sk-1…cdef");

    let (status, _) = call(
        &app,
        "PUT",
        "/v1/credentials/MY_TEST_KEY",
        Some(json!({"value": "replacement", "expected_revision": 0})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = call(
        &app,
        "DELETE",
        "/v1/credentials/MY_TEST_KEY?expected_revision=1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, answer) = call(
        &app,
        "POST",
        "/v1/credentials/describe",
        Some(json!({"references": ["MY_TEST_KEY"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["credentials"]["MY_TEST_KEY"]["configured"], false);

    let (status, _) = call(
        &app,
        "PUT",
        "/v1/credentials/MY_TEST_KEY",
        Some(json!({"value": "has space"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
