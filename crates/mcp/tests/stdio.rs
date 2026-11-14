use std::collections::BTreeMap;
use std::sync::Arc;

use dshr_core::SessionId;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_mcp::{McpServerSpec, connect_server, connect_server_with_sampling};
use dshr_tools::{Tool, ToolCtx, ToolRegistry};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn connects_lists_and_calls_tools() {
    let spec = McpServerSpec {
        name: "mock".into(),
        command: env!("CARGO_BIN_EXE_mock_mcp").into(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: None,
        headers: BTreeMap::new(),
        oauth: None,
    };
    let (_client, tools) = connect_server(spec).await.expect("connect");
    assert_eq!(tools.len(), 6);
    assert_eq!(tools[0].name(), "mcp__mock__echo");

    let mut registry = ToolRegistry::new();
    let tool: Arc<dyn Tool> = tools[0].clone();
    registry.register_arc(tool);
    let ctx = ToolCtx {
        cwd: std::env::current_dir().expect("cwd"),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: std::env::temp_dir().join("dshr-mcp-spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = registry
        .execute("mcp__mock__echo", json!({"text": "hi"}), &ctx)
        .await
        .expect("call");
    assert_eq!(outcome.content, "echo: hi");

    let provider = Arc::new(ScriptedProvider::new(
        "mock-model",
        vec![vec![
            StreamChunk::Start {
                model: "mock-model".into(),
            },
            StreamChunk::TextDelta {
                text: "sampled answer".into(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]],
    ));
    let spec = McpServerSpec {
        name: "mock".into(),
        command: env!("CARGO_BIN_EXE_mock_mcp").into(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: None,
        headers: BTreeMap::new(),
        oauth: None,
    };
    let (backend, _tools) = connect_server_with_sampling(spec, Some(provider))
        .await
        .expect("connect with sampling");
    let sampled = backend
        .call_tool("echo", json!({"text": "sample"}))
        .await
        .expect("sample");
    assert!(
        sampled.content.contains("sampled answer"),
        "{}",
        sampled.content
    );
}

struct YesPrompt;

#[async_trait::async_trait]
impl dshr_tools::UserPrompt for YesPrompt {
    async fn ask(&self, question: &str, _options: &[String]) -> Option<String> {
        assert!(question.contains("Pick a value"), "{question}");
        Some("yes".to_owned())
    }
}

#[tokio::test]
async fn answers_stdio_elicitation_through_the_prompt() {
    let spec = McpServerSpec {
        name: "mock".into(),
        command: env!("CARGO_BIN_EXE_mock_mcp").into(),
        args: Vec::new(),
        env: BTreeMap::new(),
        cwd: None,
        url: None,
        headers: BTreeMap::new(),
        oauth: None,
    };
    let (_client, tools) = connect_server(spec).await.expect("connect");
    let elicit = tools
        .iter()
        .find(|tool| tool.name() == "mcp__mock__elicit")
        .expect("elicit");
    let ctx = ToolCtx {
        cwd: std::env::current_dir().expect("cwd"),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: std::env::temp_dir().join("dshr-mcp-spills"),
        cancel: CancellationToken::new(),
        prompt: Some(Arc::new(YesPrompt)),
    };
    let outcome = elicit.execute(&ctx, json!({})).await.expect("elicit");
    assert_eq!(outcome.content, "accept:yes");

    let mut declined = ctx.clone();
    declined.prompt = None;
    let outcome = elicit.execute(&declined, json!({})).await.expect("decline");
    assert_eq!(outcome.content, "decline:");
}
