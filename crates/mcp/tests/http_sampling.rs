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
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_mcp::{McpServerSpec, connect_server_with_sampling};
use futures_util::stream;
use serde_json::{Value, json};
use tokio::sync::Mutex;

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
        "notifications/initialized" => return StatusCode::ACCEPTED.into_response(),
        "tools/list" => json!({"tools": []}),
        _ => {
            return json_response(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "not found"}
            }));
        }
    };
    json_response(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

async fn sse() -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 77,
        "method": "sampling/createMessage",
        "params": {
            "messages": [{"role": "user", "content": {"type": "text", "text": "sample me"}}],
            "maxTokens": 16
        }
    });
    let events = stream::once(async move { Ok(Event::default().data(request.to_string())) });
    Sse::new(events).keep_alive(axum::response::sse::KeepAlive::default())
}

#[tokio::test]
async fn http_server_can_request_sampling() {
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

    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::TextDelta {
                text: "sampled response".into(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]],
    ));
    let spec = McpServerSpec {
        name: "mock".into(),
        command: String::new(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: Some(format!("http://{address}/mcp")),
        headers: BTreeMap::new(),
    };
    let _ = connect_server_with_sampling(spec, Some(provider))
        .await
        .expect("connect");

    for _ in 0..100 {
        if !state.responses.lock().await.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let responses = state.responses.lock().await;
    assert_eq!(responses.len(), 1, "sampling response missing");
    assert_eq!(
        responses[0]["result"]["content"]["text"],
        "sampled response"
    );
}
