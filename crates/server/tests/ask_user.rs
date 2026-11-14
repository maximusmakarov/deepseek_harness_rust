use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dshr_agent::PermissionMode;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use futures_util::StreamExt;
use serde_json::{Value, json};

fn state(dir: &std::path::Path) -> AppState {
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![
            vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::ToolUseStart {
                    index: 0,
                    id: "call-question".into(),
                    name: "ask_user".into(),
                },
                StreamChunk::ToolInputDelta {
                    index: 0,
                    json: r#"{"question":"Color?","options":["red","blue"]}"#.into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::TextDelta {
                    text: "answered".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
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
            auto_title: false,
            home: PathBuf::from("."),
            profile: "default".into(),
        },
        provider,
        Arc::new(ToolRegistry::with_builtin()),
        Arc::new(SessionStore::new(dir.join("sessions"))),
    )
}

async fn sse_until(
    stream: &mut (impl futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin),
    needle: &str,
) -> String {
    let mut buffer = String::new();
    let deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => panic!("timeout waiting for {needle}: {buffer}"),
            chunk = stream.next() => {
                let Some(chunk) = chunk else { panic!("stream ended before {needle}: {buffer}") };
                let chunk = chunk.expect("chunk");
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(index) = buffer.find('\n') {
                    let line = buffer[..index].to_owned();
                    buffer.drain(..=index);
                    if let Some(data) = line.strip_prefix("data: ")
                        && data.contains(needle)
                    {
                        return data.to_owned();
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn answers_an_ask_user_question_over_http() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = state(dir.path());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    let app = router(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let client = reqwest::Client::new();
    let auth = "Bearer test-token";
    let created: Value = client
        .post(format!("http://{address}/v1/sessions"))
        .header("authorization", auth)
        .json(&json!({}))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("json");
    let session = created["session"]["id"].as_str().expect("id").to_owned();

    let response = client
        .get(format!(
            "http://{address}/v1/sessions/{session}/events?token=test-token"
        ))
        .send()
        .await
        .expect("events");
    let mut stream = response.bytes_stream();

    client
        .post(format!("http://{address}/v1/sessions/{session}/turns"))
        .header("authorization", auth)
        .json(&json!({"prompt": "ask"}))
        .send()
        .await
        .expect("turn");

    let event = sse_until(&mut stream, "\"type\":\"question\"").await;
    let event: Value = serde_json::from_str(&event).expect("question json");
    let question_id = event["id"].as_str().expect("question id");
    let response = client
        .post(format!(
            "http://{address}/v1/sessions/{session}/questions/{question_id}"
        ))
        .header("authorization", auth)
        .json(&json!({"answer": "blue"}))
        .send()
        .await
        .expect("answer");
    assert!(response.status().is_success());

    let done = sse_until(&mut stream, "\"type\":\"done\"").await;
    assert!(done.contains("answered"), "{done}");
}
