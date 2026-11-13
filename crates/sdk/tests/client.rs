use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dshr_agent::PermissionMode;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_sdk::Client;
use dshr_server::{AppState, ServerConfig, router};
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use futures_util::StreamExt;

#[tokio::test]
async fn sdk_creates_and_runs_a_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = AppState::new(
        ServerConfig {
            bind: "127.0.0.1:0".parse().expect("addr"),
            token: "sdk-token".into(),
            workspace: PathBuf::from(dir.path()),
            mode: PermissionMode::DangerFull,
            system_prompt: "test".into(),
            max_steps: 4,
            max_tokens: 64,
            output_limit: 10_000,
            rate_limit_per_min: 0,
        },
        Arc::new(ScriptedProvider::new(
            "test-model",
            vec![vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::TextDelta {
                    text: "sdk-pong".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]],
        )),
        Arc::new(ToolRegistry::new()),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router(state)).await;
    });

    let client = Client::new(format!("http://{address}"), "sdk-token");
    let health = client.health().await.expect("health");
    assert_eq!(health["status"], "ok");
    assert!(client.list_sessions().await.expect("list").is_empty());

    let session = client.create_session().await.expect("create");
    let mut events = Box::pin(client.events(session.id.clone()));
    let first_event = tokio::spawn(async move { events.next().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let accepted = client.start_turn(&session.id, "ping").await.expect("turn");
    assert!(accepted.accepted);

    let mut seen = false;
    for _ in 0..40 {
        let view = client.session(&session.id).await.expect("session");
        if view.messages.iter().any(|message| {
            message["role"] == "assistant" && message["content"].to_string().contains("sdk-pong")
        }) {
            seen = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(seen, "assistant answer missing");
    let event = tokio::time::timeout(Duration::from_secs(3), first_event)
        .await
        .expect("sse timeout")
        .expect("sse task")
        .expect("sse event")
        .expect("sse json");
    assert!(
        event["type"] == "text" || event["type"] == "done",
        "unexpected SSE event: {event}"
    );
}
