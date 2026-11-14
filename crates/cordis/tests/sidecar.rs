use dshr_cordis::{CordisPluginSpec, connect};
use dshr_core::SessionId;
use dshr_tools::ToolCtx;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn loads_js_plugin_tools() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("node is not installed; skipping Cordis sidecar test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("plugin.mjs"),
        r#"
export default {
  apply(ctx) {
    ctx.tool({
      name: "echo",
      description: "Echo from Cordis",
      inputSchema: { type: "object", properties: { text: { type: "string" } } },
      handler: (args) => `cordis:${args.text}`,
    });
  },
};
"#,
    )
    .expect("plugin");

    let tools = connect(
        CordisPluginSpec {
            name: "demo".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let echo = tools
        .iter()
        .find(|tool| tool.name() == "cordis__demo__echo")
        .expect("echo tool");
    assert!(echo.description().contains("Cordis"));

    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = echo
        .execute(&ctx, json!({"text": "hi"}))
        .await
        .expect("call");
    assert_eq!(outcome.content, "cordis:hi");
}

#[tokio::test]
async fn read_grants_extend_the_workspace() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("node is not installed; skipping Cordis sidecar test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let file = outside.path().join("secret.txt");
    std::fs::write(&file, "secret").expect("secret");
    std::fs::write(
        dir.path().join("plugin.mjs"),
        r#"
export default {
  apply(ctx) {
    ctx.tool({
      name: "read",
      description: "Read via ctx.fs",
      inputSchema: { type: "object", properties: { path: { type: "string" } } },
      handler: async (args) => ctx.fs.readFile(args.path),
    });
  },
};
"#,
    )
    .expect("plugin");

    let tools = connect(
        CordisPluginSpec {
            name: "demo".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants {
                read: vec![outside.path().to_path_buf()],
                write: Vec::new(),
            },
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let read = tools
        .iter()
        .find(|tool| tool.name() == "cordis__demo__read")
        .expect("read tool");
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = read
        .execute(&ctx, json!({"path": file}))
        .await
        .expect("call");
    assert_eq!(outcome.content, "secret");
    assert!(!outcome.is_error);
}
