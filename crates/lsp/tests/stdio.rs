use std::sync::Arc;

use dshr_core::SessionId;
use dshr_lsp::{LspDiagnosticsTool, LspManager, LspSpec};
use dshr_tools::{Tool, ToolCtx};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn opens_document_and_reads_diagnostics_and_hover() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("main.rs");
    std::fs::write(&path, "fn main() {}\n").expect("write");
    let manager = LspManager::new(
        dir.path().to_path_buf(),
        vec![LspSpec {
            name: "mock".into(),
            command: env!("CARGO_BIN_EXE_mock_lsp").into(),
            args: Vec::new(),
            extensions: vec!["rs".into()],
            root: dir.path().to_path_buf(),
        }],
    );
    let client = manager.client_for(&path).await.expect("client");
    client
        .did_open(&path, "fn main() {}\n")
        .await
        .expect("open");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let diagnostics = client.diagnostics(&path).await.expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["message"], "mock diagnostic");
    let hover = client.hover(&path, 1, 1).await.expect("hover");
    assert_eq!(hover["contents"]["value"], "mock hover");

    let tool = LspDiagnosticsTool::new(Arc::new(manager));
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = tool
        .execute(&ctx, json!({"path": "main.rs"}))
        .await
        .expect("tool");
    assert!(
        outcome.content.contains("mock diagnostic"),
        "{}",
        outcome.content
    );
}
