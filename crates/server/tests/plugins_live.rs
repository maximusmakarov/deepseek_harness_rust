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
    state_with(dir, "127.0.0.1:0", false).await
}

async fn state_with(
    dir: &Path,
    bind: &str,
    allow_plugin_routes_lan: bool,
) -> (AppState, Arc<ToolRegistry>, Arc<dshr_cordis::CordisManager>) {
    state_with_internal(dir, bind, allow_plugin_routes_lan, None).await
}

async fn state_with_internal(
    dir: &Path,
    bind: &str,
    allow_plugin_routes_lan: bool,
    internal_token: Option<&str>,
) -> (AppState, Arc<ToolRegistry>, Arc<dshr_cordis::CordisManager>) {
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
            bind: bind.parse().expect("addr"),
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
            allow_plugin_routes_lan,
            internal_token: internal_token.map(ToOwned::to_owned),
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

fn write_asset_client_half(dir: &Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"@acme/assets","dsh":{"client":{"platform":"web","entry":"client.js"}}}"#,
    )
    .expect("manifest");
    std::fs::write(
        dir.join("client.js"),
        r#"
import { render } from "./lib/panel.js";
export default function activate(ctx) {
  ctx.slots.register({ name: "panel", id: "assets", title: "Assets" }, render);
}
"#,
    )
    .expect("client");
    std::fs::create_dir_all(dir.join("lib")).expect("lib");
    std::fs::write(
        dir.join("lib/panel.js"),
        r#"export function render(element) { element.textContent = "panel from a chunk"; }"#,
    )
    .expect("chunk");
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
    // A plain host request is a registry frame: mode defaults and the slot-only
    // fields stay empty, so the v1.1 behaviour is unchanged.
    assert!(page.contains("window.__DSHR_FRAME__ = "), "{page}");
    assert!(page.contains("\"mode\":\"registry\""), "{page}");
    assert!(page.contains("\"plugin\":\"demo\""), "{page}");
    assert!(page.contains("\"slot\":\"\""), "{page}");
    assert!(page.contains("\"id\":\"\""), "{page}");
    assert!(page.contains("\"context\":\"\""), "{page}");
    // A half without `dsh.client.host_modules` keeps the network-free page.
    assert!(!page.contains("/v1/runtime/"), "{page}");
}

/// Percent-encodes a JSON context payload for a query string. The test payloads
/// are ASCII, so encoding every non-alphanumeric byte is enough.
fn encoded_context(context: &str) -> String {
    context
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_string()
            } else {
                format!("%{:02X}", ch as u32)
            }
        })
        .collect()
}

#[tokio::test]
async fn slot_host_page_embeds_the_frame_config() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let uri = "/v1/plugins/demo/host?token=test-token&theme=light\
&frame=slot:demo:demo-panel:0&mode=slot&slot=panel&id=demo-panel\
&context=%7B%22index%22%3A0%7D";
    let (status, headers, bytes) = raw(&app, uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    assert_eq!(headers[header::X_FRAME_OPTIONS], "SAMEORIGIN");
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(page.contains("data-theme=\"light\""), "{page}");
    assert!(page.contains("\"mode\":\"slot\""), "{page}");
    assert!(
        page.contains("\"frame\":\"slot:demo:demo-panel:0\""),
        "{page}"
    );
    assert!(page.contains("\"slot\":\"panel\""), "{page}");
    assert!(page.contains("\"id\":\"demo-panel\""), "{page}");
    assert!(page.contains(r#""context":"{\"index\":0}""#), "{page}");
    assert!(page.contains("\"theme\":\"light\""), "{page}");

    // Context values are escaped into the config; `<` cannot close the script.
    let context = r#"{"html":"</script>"}"#;
    let uri = format!(
        "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=demo-panel&context={}",
        encoded_context(context)
    );
    let (status, _, bytes) = raw(&app, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(page.contains(r"\u003c/script"), "{page}");
    assert!(!page.contains(context), "{page}");
}

#[tokio::test]
async fn rejects_invalid_host_frame_queries() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let long_frame = "a".repeat(121);
    let long_slot = format!("{}.{}", "a".repeat(60), "b".repeat(21));
    let cases = vec![
        (
            "/v1/plugins/demo/host?token=test-token&mode=bogus".to_owned(),
            "invalid mode",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&id=demo-panel".to_owned(),
            "slot is required",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel".to_owned(),
            "id is required",
        ),
        (
            format!(
                "/v1/plugins/demo/host?token=test-token&mode=slot&slot={long_slot}&id=demo-panel"
            ),
            "invalid slot name",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&slot=bad%20slot&id=demo-panel"
                .to_owned(),
            "invalid slot name",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=bad%20id".to_owned(),
            "invalid registration id",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&frame=bad%20frame".to_owned(),
            "invalid frame token",
        ),
        (
            format!("/v1/plugins/demo/host?token=test-token&frame={long_frame}"),
            "invalid frame token",
        ),
        // Slot fields are validated whenever they are present, even though a
        // registry frame does not use them.
        (
            "/v1/plugins/demo/host?token=test-token&id=bad%20id".to_owned(),
            "invalid registration id",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=demo-panel&context=not-json"
                .to_owned(),
            "context must be a JSON object",
        ),
        (
            "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=demo-panel&context=%5B1%5D"
                .to_owned(),
            "context must be a JSON object",
        ),
    ];
    for (uri, expected) in cases {
        let (status, headers, bytes) = raw(&app, &uri, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(headers[header::CONTENT_TYPE], "application/json", "{uri}");
        let body: Value = serde_json::from_slice(&bytes).expect("json error body");
        let error = body["error"].as_str().expect("error field");
        assert!(error.contains(expected), "{uri}: {error}");
    }

    // The 4096-character context limit is inclusive.
    let base = r#"{"a":""#;
    let suffix = r#""}"#;
    let fits = format!("{base}{}{suffix}", "a".repeat(4088));
    assert_eq!(fits.chars().count(), 4096);
    let uri = format!(
        "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=demo-panel&context={}",
        encoded_context(&fits)
    );
    let (status, _, _) = raw(&app, &uri, None).await;
    assert_eq!(status, StatusCode::OK);

    let oversized = format!("{base}{}{suffix}", "a".repeat(4089));
    assert_eq!(oversized.chars().count(), 4097);
    let uri = format!(
        "/v1/plugins/demo/host?token=test-token&mode=slot&slot=panel&id=demo-panel&context={}",
        encoded_context(&oversized)
    );
    let (status, _, bytes) = raw(&app, &uri, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(&bytes).expect("json error body");
    assert!(
        body["error"].as_str().expect("error").contains("4096"),
        "{body}"
    );
}

fn write_runtime_client_half(dir: &Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"@acme/react","dsh":{"client":{"platform":"web","entry":"client.js","runtime":"runtime/modules.js"}}}"#,
    )
    .expect("manifest");
    std::fs::write(
        dir.join("client.js"),
        r#"
window.__ModuleLoader__.load({
  id: "@acme/react",
  factory(require) {
    const framework = require("framework");
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register({ name: "panel", id: "react-panel", title: "React" }, (element) => {
          framework.mount(element);
        });
      },
    };
  },
});
"#,
    )
    .expect("client");
    std::fs::create_dir_all(dir.join("runtime")).expect("runtime dir");
    std::fs::write(
        dir.join("runtime/modules.js"),
        r#"window.__DSHR_MODULES__ = { framework: { mount: function (element) { element.textContent = "runtime works"; } } };"#,
    )
    .expect("runtime");
}

#[tokio::test]
async fn serves_the_plugin_runtime_before_the_bundle() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_runtime_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(&app, "GET", "/v1/plugins/clients", Some(TOKEN), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["clients"][0]["runtime"], "runtime/modules.js",
        "{body}"
    );

    let (status, _, bytes) = raw(
        &app,
        "/v1/plugins/demo/host?token=test-token&theme=dark",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(page.contains("dshr plugin runtime"), "{page}");
    assert!(
        page.contains("window.__DSHR_MODULES__ = { framework"),
        "{page}"
    );
    assert!(
        page.contains("Object.prototype.hasOwnProperty.call(modules, specifier)"),
        "{page}"
    );
    let runtime_at = page.find("dshr plugin runtime").expect("runtime marker");
    let bundle_at = page.find("const url = ").expect("bundle loader");
    assert!(runtime_at < bundle_at, "runtime must run before the bundle");
}

#[tokio::test]
async fn rejects_a_runtime_that_closes_the_script_tag() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_runtime_client_half(dir.path());
    std::fs::write(
        dir.path().join("runtime/modules.js"),
        r#"window.__DSHR_MODULES__ = {}; </script><script>window.pwned = 1;</script>"#,
    )
    .expect("hostile runtime");
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(
        &app,
        "GET",
        "/v1/plugins/demo/host?token=test-token",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("unescaped </script>"),
        "{body}"
    );
}

fn write_host_modules_client_half(dir: &Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name":"@acme/hosted","dsh":{"client":{"platform":"web","entry":"client.js","host_modules":["react","react-dom/client"]}}}"#,
    )
    .expect("manifest");
    std::fs::write(
        dir.join("client.js"),
        r#"
window.__ModuleLoader__.load({
  id: "@acme/hosted",
  factory(require) {
    const React = require("react");
    const { createRoot } = require("react-dom/client");
    return {
      inject: ["slots"],
      apply(ctx) {
        ctx.slots.register({ name: "panel", id: "hosted-panel", title: "Hosted" }, (element) => {
          createRoot(element).render(React.createElement("h3", null, "React from the host"));
        });
      },
    };
  },
});
"#,
    )
    .expect("client");
}

/// The shared runtime is one immutable, versioned artifact: the frame references
/// it by URL instead of the plugin embedding ~140 KB in every host page.
#[tokio::test]
async fn serves_one_immutable_host_runtime() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, headers, bytes) = raw(&app, "/v1/runtime/react-18.3.1.js", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/javascript; charset=utf-8"
    );
    let cache = headers[header::CACHE_CONTROL].to_str().expect("cache");
    assert!(cache.contains("immutable"), "{cache}");
    assert!(cache.contains("max-age=31536000"), "{cache}");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    let body = String::from_utf8(bytes).expect("utf8");
    assert!(
        body.contains("@license React"),
        "upstream license header kept"
    );
    assert!(
        body.contains("window.__DSHR_MODULES__"),
        "module table glue"
    );
    assert!(body.contains(r#""react/jsx-runtime""#), "{body:.200}");
    assert!(body.len() > 100_000, "React + ReactDOM are concatenated");

    // Only the exact versioned name is served; anything else is a 404.
    for file in ["react-19.0.0.js", "react-18.3.1.js.map", "bootstrap.js"] {
        let (status, _) = {
            let uri = format!("/v1/runtime/{file}");
            let request = Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .expect("request");
            let response = app.clone().oneshot(request).await.expect("response");
            (response.status(), ())
        };
        assert_eq!(status, StatusCode::NOT_FOUND, "{file}");
    }
}

#[tokio::test]
async fn host_page_opts_into_the_shared_runtime() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_host_modules_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(&app, "GET", "/v1/plugins/clients", Some(TOKEN), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["clients"][0]["host_modules"],
        json!(["react", "react-dom/client"]),
        "{body}"
    );

    let (status, headers, bytes) = raw(
        &app,
        "/v1/plugins/demo/host?token=test-token&theme=dark",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(
        page.contains(
            r#"<script src="/v1/runtime/react-18.3.1.js" data-dshr-shared-runtime="1"></script>"#
        ),
        "{page}"
    );
    // The page references the artifact, it does not embed React: that is what
    // removes the per-frame duplication.
    assert!(
        !page.contains("@license React"),
        "React must not be inlined"
    );
    let csp = headers[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("csp")
        .to_owned();
    assert!(
        csp.contains("script-src 'unsafe-inline' blob: http://"),
        "{csp}"
    );
    // C1: a host-modules frame may reach its own plugin routes on the server
    // origin (no external network); a plain frame stays network-free.
    assert!(csp.contains("connect-src http://"), "{csp}");
    assert!(!csp.contains("connect-src 'none'"), "{csp}");
}

#[tokio::test]
async fn plain_frames_stay_network_free() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);
    let (status, headers, _) = raw(&app, "/v1/plugins/demo/host?token=test-token", None).await;
    assert_eq!(status, StatusCode::OK);
    let csp = headers[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("csp")
        .to_owned();
    assert!(csp.contains("connect-src 'none'"), "{csp}");
}

#[tokio::test]
async fn hostile_host_cannot_inject_the_shared_runtime_csp() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_host_modules_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/plugins/demo/host?token=test-token&theme=dark")
        .header("host", "evil.test; script-src *")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("csp")
        .to_owned();
    assert!(csp.contains("http://127.0.0.1"), "{csp}");
    assert!(!csp.contains("evil.test"), "{csp}");
    assert!(!csp.contains("script-src *"), "{csp}");
}

const WEB_ROUTE_PLUGIN: &str = r#"
export default {
  apply(ctx) {
    ctx.tool({
      name: "probe",
      description: "probe",
      inputSchema: { type: "object" },
      handler: () => "ok",
    });
    ctx.inject(["webServer"], (scope) => {
      scope.webServer.register({
        name: "probe-route",
        kind: "exact",
        path: "/probe-route",
        handler: async (req, res) => {
          let body = "";
          if (req.method === "POST") {
            for await (const chunk of req) body += chunk;
          }
          res.writeHead(200, { "content-type": "application/json" });
          res.end(JSON.stringify({ ok: true, url: req.url, method: req.method, body }));
        },
      });
      scope.webServer.register({
        name: "probe-prefix",
        kind: "prefix",
        path: "/probe-prefix",
        handler: async (req, res) => {
          res.writeHead(200, { "content-type": "application/json" });
          res.end(JSON.stringify({ ok: true, url: req.url }));
        },
      });
      try {
        // Reserved paths must be refused by the sidecar, never proxied.
        scope.webServer.register({
          name: "reserved",
          kind: "exact",
          path: "/v1/probe-route",
          handler: async (_req, res) => res.end("x"),
        });
      } catch (error) {
        void error;
      }
      let duplicateThrew = false;
      try {
        scope.webServer.register({
          name: "duplicate",
          kind: "exact",
          path: "/probe-route",
          handler: async (_req, res) => res.end("dup"),
        });
      } catch (error) {
        duplicateThrew = true;
      }
      let trailingRejected = false;
      try {
        scope.webServer.register({
          name: "trailing",
          kind: "exact",
          path: "/trailing/",
          handler: async (_req, res) => res.end("x"),
        });
      } catch (error) {
        trailingRejected = true;
      }
      scope.webServer.register({
        name: "probe-rules",
        kind: "exact",
        path: "/probe-rules",
        handler: async (_req, res) => {
          res.writeHead(200, { "content-type": "application/json" });
          res.end(JSON.stringify({ duplicateThrew, trailingRejected }));
        },
      });
    });
  },
};
"#;

#[tokio::test]
async fn proxies_plugin_host_routes() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("plugin.mjs"), WEB_ROUTE_PLUGIN).expect("plugin");
    let (state, _registry, manager) = state(dir.path()).await;
    let app = router(state);

    // Exact route, query string and status are proxied from the sidecar.
    let request = Request::builder()
        .method("GET")
        .uri("/probe-route?x=1")
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body");
    let value: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(value["ok"], true, "{value}");
    assert_eq!(value["url"], "/probe-route?x=1", "{value}");
    assert_eq!(value["method"], "GET", "{value}");

    // Request bodies are forwarded for plugin POST routes.
    let request = Request::builder()
        .method("POST")
        .uri("/probe-route")
        .header("content-type", "text/plain")
        .body(Body::from("hello"))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body");
    let value: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(value["method"], "POST", "{value}");
    assert_eq!(value["body"], "hello", "{value}");

    // CORS preflight is answered without touching the sidecar.
    let request = Request::builder()
        .method("OPTIONS")
        .uri("/probe-route")
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    // The global permissive CorsLayer answers browser preflights before the
    // fallback (200), the handler's own 204 branch is the defensive fallback.
    assert!(
        response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::OK,
        "{}",
        response.status()
    );
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");

    // Prefix routes match themselves and their children.
    for uri in ["/probe-prefix", "/probe-prefix/a/b"] {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }

    // Reference route rules: duplicate (kind, path) throws, trailing slash rejected.
    let request = Request::builder()
        .method("GET")
        .uri("/probe-rules")
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body");
    let rules: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(rules["duplicateThrew"], true, "{rules}");
    assert_eq!(rules["trailingRejected"], true, "{rules}");

    // Status exposes the live route table and hides MCP pseudo-tools.
    let request = Request::builder()
        .method("GET")
        .uri("/v1/plugins/status")
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body");
    let status: Value = serde_json::from_slice(&body).expect("json");
    let plugin = &status["plugins"][0];
    assert!(
        plugin["routes"]
            .as_array()
            .is_some_and(|routes| routes.iter().any(|route| route == "/probe-route")),
        "{status}"
    );
    let tools = plugin["tools"].as_array().expect("tools");
    assert!(
        tools.iter().any(|tool| tool == "cordis__demo__probe"),
        "{status}"
    );
    assert!(
        !tools.iter().any(|tool| matches!(
            tool.as_str(),
            Some("list_resources" | "read_resource" | "list_prompts" | "get_prompt")
        )),
        "MCP pseudo-tools leaked into the registry: {status}"
    );

    // Unregistered and reserved paths keep the normal 404.
    for uri in ["/probe-missing", "/v1/probe-route"] {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }

    // Stopping the plugin drops its routes (live allowlist, not a cached copy).
    manager.sync(Vec::new()).await;
    let request = Request::builder()
        .method("GET")
        .uri("/probe-route")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_session_snapshot_feeds_the_host_bridge() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dshr_session_jsonl::SessionStore::new(dir.path().join("sessions"));
    let mut session = store
        .create("mock", "deepseek-flash", dir.path().to_str().expect("path"))
        .expect("session");
    let seq = session.next_seq();
    store
        .append(
            &mut session,
            dshr_session::SessionEvent::AssistantMessage {
                seq,
                at: "2026-01-01T00:00:00Z".into(),
                turn: dshr_core::TurnId::new(),
                content: Vec::new(),
                usage: dshr_llm::Usage {
                    input_tokens: 10,
                    output_tokens: 4,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 3,
                },
                stop_reason: None,
            },
        )
        .expect("append");
    std::fs::write(dir.path().join("plugin.mjs"), WEB_ROUTE_PLUGIN).expect("plugin");
    let (app_state, _registry, _manager) =
        state_with_internal(dir.path(), "127.0.0.1:0", false, Some("test-internal")).await;
    let app = router(app_state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/internal/session-snapshot")
        .header("x-dshr-internal-token", "test-internal")
        .body(Body::empty())
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body");
    let snapshot: Value = serde_json::from_slice(&body).expect("json");
    let sessions = snapshot["sessions"].as_array().expect("sessions");
    assert_eq!(sessions.len(), 1, "{snapshot}");
    assert_eq!(sessions[0]["model"], "deepseek-flash", "{snapshot}");
    assert_eq!(
        sessions[0]["usage_events"][0]["input_tokens"], 10,
        "{snapshot}"
    );
    assert_eq!(
        sessions[0]["usage_events"][0]["cache_read_input_tokens"], 3,
        "{snapshot}"
    );

    let request = Request::builder()
        .method("GET")
        .uri("/v1/internal/session-snapshot")
        .header("x-dshr-internal-token", "wrong")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Without a configured internal token the endpoint stays invisible.
    let other = tempfile::tempdir().expect("tempdir");
    std::fs::write(other.path().join("plugin.mjs"), WEB_ROUTE_PLUGIN).expect("plugin");
    let (plain_state, _registry, _manager) = state(other.path()).await;
    let app = router(plain_state);
    let request = Request::builder()
        .method("GET")
        .uri("/v1/internal/session-snapshot")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn plugin_routes_are_loopback_only_unless_opted_in() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("plugin.mjs"), WEB_ROUTE_PLUGIN).expect("plugin");
    let (state, _registry, _manager) = state_with(dir.path(), "0.0.0.0:0", false).await;
    let app = router(state);
    let request = Request::builder()
        .method("GET")
        .uri("/probe-route")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let (state, _registry, _manager) = state_with(dir.path(), "0.0.0.0:0", true).await;
    let app = router(state);
    let request = Request::builder()
        .method("GET")
        .uri("/probe-route")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_an_oversized_plugin_runtime() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_runtime_client_half(dir.path());
    std::fs::write(
        dir.path().join("runtime/modules.js"),
        vec![b'x'; 8 * 1024 * 1024 + 1],
    )
    .expect("oversized runtime");
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(
        &app,
        "GET",
        "/v1/plugins/demo/host?token=test-token",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("larger than 8 MiB"),
        "{body}"
    );
}

#[tokio::test]
async fn multi_file_client_bundle_uses_capability_assets() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_asset_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let (status, body) = call(&app, "GET", "/v1/plugins/clients", Some(TOKEN), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["clients"][0]["bundle"], "assets", "{body}");

    let (status, headers, bytes) = raw(
        &app,
        "/v1/plugins/demo/host?token=test-token&theme=dark",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let csp = headers[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("csp")
        .to_owned();
    assert!(csp.contains("script-src 'unsafe-inline' http://"), "{csp}");
    let page = String::from_utf8(bytes).expect("utf8");
    assert!(page.contains("encodeURI(\"client.js\")"), "{page}");
    let start =
        page.find("/v1/plugins/demo/asset/").expect("asset base") + "/v1/plugins/demo/asset/".len();
    let capability: String = page[start..].chars().take(32).collect();
    assert_eq!(capability.len(), 32, "{capability}");

    let (status, headers, bytes) = raw(
        &app,
        &format!("/v1/plugins/demo/asset/{capability}/client.js"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/javascript; charset=utf-8"
    );
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert!(String::from_utf8_lossy(&bytes).contains("./lib/panel.js"));

    let (status, _, bytes) = raw(
        &app,
        &format!("/v1/plugins/demo/asset/{capability}/lib/panel.js"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&bytes).contains("panel from a chunk"));

    // A capability is scoped to one plugin and only to package files.
    let (status, _, _) = raw(
        &app,
        "/v1/plugins/demo/asset/00000000000000000000000000000000/client.js",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = raw(
        &app,
        &format!("/v1/plugins/demo/asset/{capability}/missing.js"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = raw(
        &app,
        &format!("/v1/plugins/demo/asset/{capability}/%2e%2e/%2e%2e/secret.txt"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn hostile_host_header_cannot_inject_the_asset_csp() {
    if !node_available() {
        eprintln!("node is not installed; skipping server plugin test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    write_plugin(dir.path(), "v1");
    write_asset_client_half(dir.path());
    let (state, _registry, _manager) = state(dir.path()).await;
    let app = router(state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/plugins/demo/host?token=test-token&theme=dark")
        .header("host", "evil.test; script-src *")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .expect("csp")
        .to_owned();
    assert!(csp.contains("http://127.0.0.1"), "{csp}");
    assert!(!csp.contains("script-src *"), "{csp}");
}
