use std::collections::BTreeMap;
use std::sync::Arc;

use dshr_core::SessionId;
use dshr_mcp::{McpServerSpec, connect_server};
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
    };
    let (_client, tools) = connect_server(spec).await.expect("connect");
    assert_eq!(tools.len(), 5);
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
    };
    let outcome = registry
        .execute("mcp__mock__echo", json!({"text": "hi"}), &ctx)
        .await
        .expect("call");
    assert_eq!(outcome.content, "echo: hi");
}
