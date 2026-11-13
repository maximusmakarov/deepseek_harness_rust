use std::collections::BTreeMap;

use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use dshr_mcp::{McpServerSpec, connect_server};
use serde_json::{Value, json};

fn respond(value: Value) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::HeaderName::from_static("mcp-session-id"),
                "test-session",
            ),
        ],
        Json(value),
    )
        .into_response()
}

async fn handler(Json(request): Json<Value>) -> Response {
    let id = request["id"].clone();
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}, "resources": {}},
            "serverInfo": {"name": "mock-http", "version": "0.1.0"}
        }),
        "notifications/initialized" => return axum::http::StatusCode::ACCEPTED.into_response(),
        "tools/list" => json!({
            "tools": [{
                "name": "echo",
                "description": "Echo",
                "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}}
            }]
        }),
        "tools/call" => json!({
            "content": [{
                "type": "text",
                "text": format!("echo: {}", request["params"]["arguments"]["text"].as_str().unwrap_or(""))
            }],
            "isError": false
        }),
        "prompts/list" => json!({
            "prompts": [{"name": "mock", "description": "Mock prompt"}]
        }),
        "prompts/get" => json!({
            "description": "Mock prompt",
            "messages": [{"role": "user", "content": {"type": "text", "text": "prompt body"}}]
        }),
        "resources/list" => json!({
            "resources": [{
                "uri": "mock://resource",
                "name": "mock",
                "description": "Mock resource"
            }]
        }),
        "resources/read" => json!({
            "contents": [{"uri": "mock://resource", "text": "resource body"}]
        }),
        _ => {
            return respond(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "not found"}
            }));
        }
    };
    respond(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

#[tokio::test]
async fn http_client_initializes_calls_tools_and_reads_resources() {
    let app = axum::Router::new().route("/mcp", post(handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let spec = McpServerSpec {
        name: "mock".into(),
        command: String::new(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: Some(format!("http://{address}/mcp")),
        headers: BTreeMap::new(),
    };
    let (backend, tools) = connect_server(spec).await.expect("connect");
    assert_eq!(tools.len(), 5);

    let calls = backend
        .call_tool("echo", json!({"text": "hi"}))
        .await
        .expect("call");
    assert_eq!(calls.content, "echo: hi");

    let resources = backend.list_resources().await.expect("resources");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "mock://resource");

    let text = backend
        .read_resource("mock://resource")
        .await
        .expect("read");
    assert_eq!(text, "resource body");

    let prompts = backend.list_prompts().await.expect("prompts");
    assert_eq!(prompts.len(), 1);
    let prompt = backend.get_prompt("mock", json!({})).await.expect("prompt");
    assert_eq!(prompt.messages[0].1, "prompt body");
}
