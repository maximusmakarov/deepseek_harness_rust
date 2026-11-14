use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use dshr_core::SessionId;
use dshr_mcp::{McpServerSpec, connect_server};
use dshr_tools::ToolCtx;
use futures_util::stream;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct AppState {
    responses: Arc<Mutex<Vec<Value>>>,
}

fn json_response(value: Value) -> Response {
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

async fn post(State(state): State<AppState>, Json(request): Json<Value>) -> Response {
    let id = request["id"].clone();
    if request.get("method").is_none() {
        state.responses.lock().await.push(request);
        return StatusCode::ACCEPTED.into_response();
    }
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "mock-http", "version": "0.1.0"}
        }),
        "tools/list" => json!({"tools": [{
            "name": "wait",
            "description": "Wait",
            "inputSchema": {"type": "object", "properties": {}}
        }]}),
        "tools/call" => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            json!({"content": [{"type": "text", "text": "done"}], "isError": false})
        }
        _ => json!({}),
    };
    json_response(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

async fn sse() -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 77,
        "method": "elicitation/create",
        "params": {
            "message": "Answer from the background stream",
            "requestedSchema": {
                "type": "object",
                "properties": {"answer": {"type": "string", "title": "Answer"}},
                "required": ["answer"]
            }
        }
    });
    let events = stream::once(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        Ok(Event::default().data(request.to_string()))
    });
    Sse::new(events).keep_alive(axum::response::sse::KeepAlive::default())
}

struct YesPrompt;

#[async_trait::async_trait]
impl dshr_tools::UserPrompt for YesPrompt {
    async fn ask(&self, question: &str, _options: &[String]) -> Option<String> {
        assert!(question.contains("background stream"), "{question}");
        Some("yes".to_owned())
    }
}

#[tokio::test]
async fn answers_background_get_sse_elicitation() {
    let state = AppState {
        responses: Arc::new(Mutex::new(Vec::new())),
    };
    let app = axum::Router::new()
        .route("/mcp", get(sse).post(post))
        .with_state(state.clone());
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
        oauth: None,
    };
    let (_backend, tools) = connect_server(spec).await.expect("connect");
    let wait = tools
        .iter()
        .find(|tool| tool.name() == "mcp__mock__wait")
        .expect("wait tool");
    let ctx = ToolCtx {
        cwd: std::env::current_dir().expect("cwd"),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: std::env::temp_dir().join("dshr-mcp-spills"),
        cancel: CancellationToken::new(),
        prompt: Some(Arc::new(YesPrompt)),
    };
    let outcome = wait.execute(&ctx, json!({})).await.expect("call");
    assert_eq!(outcome.content, "done");

    for _ in 0..100 {
        if !state.responses.lock().await.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let responses = state.responses.lock().await;
    assert_eq!(responses.len(), 1, "elicitation response missing");
    assert_eq!(responses[0]["result"]["action"], "accept");
    assert_eq!(responses[0]["result"]["content"]["answer"], "yes");
}
