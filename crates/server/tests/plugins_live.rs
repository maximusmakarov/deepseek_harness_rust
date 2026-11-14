//! Server-level plugin hot reload and client-half endpoints.
//!
//! These start a real Cordis sidecar (node) and exercise the HTTP surface that
//! the desktop/web UI uses; they skip when node is unavailable.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dshr_agent::PermissionMode;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token";

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_ok()
}

fn plugin_source(version: &str) -> String {
    format!(
        r#"
export default {{
  apply(ctx) {{
    ctx.tool({{
      name: "echo",
      description: "echo {version}",
      inputSchema: {{ type: "object" }},
      handler: () => "{version}",
    }});
  }},
}};
"#
    )
}

fn write_plugin(dir: &Path, version: &str) {
    std::fs::write(dir.join("plugin.mjs"), plugin_source(version)).expect("plugin");
}

fn write_client_half(dir: &Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"@acme/demo","dsh":{"client":{"platform":"web","entry":"client.js"}}}"#,
    )
    .expect("manifest");
    std::fs::write(
        dir.join("client.js"),
        r#"
export default function activate(ctx) {
  ctx.slots.register({
    id: "demo-panel",
    title: "Demo panel",
    render(element) { element.textContent = "hello from the plugin"; },
  });
}
"#,
    )
    .expect("client");
}

async fn state(dir: &Path) -> (AppState, Arc<ToolRegistry>, Arc<dshr_cordis::CordisManager>) {
    let registry = Arc::new(ToolRegistry::new());
    let home = dir.join("home");
    std::fs::create_dir_all(&home).expect("home");
    let manager = dshr_cordis::CordisManager::new(
        Arc::clone(&registry),
        home,
        dir.to_path_buf(),
        dshr_cordis::manager::ToolFilter::default(),
    );
    manager
        .sync(vec![dshr_cordis::CordisPluginSpec {
            name: "demo".into(),
            entry: PathBuf::from("plugin.mjs"),
            root: dir.to_path_buf(),
            env: Default::default(),
            grants: dshr_cordis::CordisGrants::default(),
        }])
        .await;
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
    let state = AppState::new(
        ServerConfig {
            bind: "127.0.0.1:0".parse().expect("addr"),
            token: TOKEN.into(),
            workspace: dir.to_path_buf(),
            mode: PermissionMode::DangerFull,
            system_prompt: "test".into(),
            max_steps: 4,
            max_tokens: 64,
            output_limit: 10_000,
            rate_limit_per_min: 0,
            auto_title: false,
            home: dir.to_path_buf(),
            profile: "default".into(),
            cordis_plugins: vec![("demo".into(), true)],
            plugins: Some(Arc::clone(&manager)),
            wasm: None,
        },
        provider,
        Arc::clone(&registry),
        Arc::new(SessionStore::new(dir.join("sessions"))),
    );
    (state, registry, manager)
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let request = request
        .header("content-type", "application/json")
        .body(if body.is_null() {
            Body::empty()
        } else {
            Body::from(body.to_string())
        })
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("body");
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn raw(
    app: &axum::Router,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut request = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .expect("body")
        .to_vec();
    (status, headers, bytes)
}

#[tokio::test]
async fn reports_plugin_status_and_hot_reloads() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(&app, "GET", "/v1/plugins/status", Some(TOKEN), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let plugins = body["plugins"].as_array().expect("plugins");
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0]["name"], "demo");
    assert_eq!(plugins[0]["running"], true);
    assert!(
        plugins[0]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .any(|tool| tool == "cordis__demo__echo")
    );

    write_plugin(dir.path(), "v2");
    let (status, body) = call(
        &app,
        "POST",
        "/v1/plugins/reload",
        Some(TOKEN),
        json!({"name": "demo"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["errors"].as_array().expect("errors").len(), 0);
    assert_eq!(body["plugins"][0]["running"], true);

    // Unknown plugin names are reported as 404, not as a failed reload.
    let (status, _) = call(
        &app,
        "POST",
        "/v1/plugins/reload",
        Some(TOKEN),
        json!({"name": "nope"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The endpoint is authenticated like the rest of the API.
    let (status, _) = call(&app, "POST", "/v1/plugins/reload", None, json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn lists_and_serves_client_halves() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(&app, "GET", "/v1/plugins/clients", Some(TOKEN), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let clients = body["clients"].as_array().expect("clients");
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0]["name"], "demo");
    assert_eq!(clients[0]["entry"], "client.js");
    assert_eq!(clients[0]["package"], "@acme/demo");

    let (status, headers, bytes) =
        raw(&app, "/v1/plugins/demo/client.js?token=test-token", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/javascript; charset=utf-8"
    );
    let bundle = String::from_utf8(bytes).expect("utf8");
    assert!(bundle.contains("hello from the plugin"), "{bundle}");

    let (status, headers, bytes) = raw(
        &app,
        "/v1/plugins/demo/host?token=test-token&theme=light",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(page.contains("data-theme=\"light\""), "{page}");
    // The bundle is inlined (the frame is network-free), not linked.
    assert!(!page.contains("/client.js?"), "{page}");
    assert!(page.contains("dshr-client"), "{page}");
}
