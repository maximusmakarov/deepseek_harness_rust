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

    let connection = connect(
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
    let tools = connection.tools;
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

    let connection = connect(
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
    let tools = connection.tools;
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

/// The reference ctx surface real DSH plugins call: effects, provided services,
/// events, prompt sections, sandbox confining, tool registration with the host
/// `parameters`/`execute` shape, and schema-resolved plugin config.
#[tokio::test]
async fn exposes_the_reference_plugin_surface() {
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
export const Config = Object.assign((value) => ({ ...value, resolved: true }));

export default {
  apply(ctx, config) {
    ctx.effect(() => () => {}, "demo.effect");
    ctx.on("demo:event", () => {});
    ctx.provide("demoService", { ping: () => "pong" });
    ctx.systemPrompt.section({ title: "demo" });
    const confined = ctx.sandbox.confine("/workspace/x");
    const viaGet = ctx.get("demoService").ping();
    ctx.tools.register({
      name: "host_shape",
      description: `config=${JSON.stringify(config)} confined=${confined} get=${viaGet}`,
      parameters: { type: "object", properties: {} },
      async execute() {
        return "ok";
      },
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "surface".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__surface__host_shape")
        .expect("host-shaped tool registered through ctx.tools.register");
    let description = tool.description();
    assert!(description.contains("\"resolved\":true"), "{description}");
    assert!(
        description.contains("confined=/workspace/x"),
        "{description}"
    );
    assert!(description.contains("get=pong"), "{description}");
}

/// `subagents` is a provider registry in the sidecar (the agent tree lives in
/// the host); config forms are created lazily instead of returning `undefined`,
/// because real plugins read `.writable` without a null check.
#[tokio::test]
async fn exposes_subagent_registry_and_safe_config_forms() {
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
    const dispose = ctx.subagents.registerProvider({
      name: "probe",
      start: async () => ({ id: "child" }),
    });
    const providers = ctx.subagents.list().join(",");
    const found = ctx.subagents.getProvider("probe")?.name ?? "none";
    const missing = ctx.get("configForms").get("missing-form");
    ctx.tool({
      name: "probe",
      description: `providers=${providers} found=${found} writable=${missing.writable}`,
      handler: () => {
        dispose();
        return `after=${ctx.subagents.list().length}`;
      },
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "subs".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__subs__probe")
        .expect("probe tool");
    let description = tool.description();
    assert!(description.contains("providers=probe"), "{description}");
    assert!(description.contains("found=probe"), "{description}");
    assert!(
        description.contains("writable=true"),
        "config forms are created lazily: {description}"
    );
}

/// Minimal HTTP stub serving `/v1/internal/agents` for the host bridge.
fn agents_stub(
    payload: serde_json::Value,
    token: &'static str,
) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let body = payload.to_string();
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming().take(16) {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0u8; 2048];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]);
            let authorized = request
                .lines()
                .any(|line| line.eq_ignore_ascii_case(&format!("x-dshr-internal-token: {token}")));
            let (status, body) = if authorized && request.starts_with("GET /v1/internal/agents") {
                ("200 OK", body.as_str())
            } else {
                ("401 Unauthorized", "{}")
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (format!("http://{address}"), handle)
}

/// The `agents` service is hydrated from the host bridge: the sidecar polls
/// `/v1/internal/agents` and exposes the tree to plugins.
#[tokio::test]
async fn agents_service_is_bridged_from_the_host() {
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("node is not installed; skipping Cordis sidecar test");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let (url, stub) = agents_stub(
        json!({
            "revision": 7,
            "agents": [
                {"id": "s1", "session_id": "s1", "root_id": "s1", "depth": 0,
                 "title": "root", "started_at": "2026-01-01T00:00:00Z"},
                {"id": "s1#sub1", "session_id": "s1", "parent_id": "s1", "root_id": "s1",
                 "depth": 1, "started_at": "2026-01-01T00:00:01Z"}
            ]
        }),
        "bridge-token",
    );
    std::fs::create_dir_all(dir.path().join("cordis")).expect("cordis dir");
    std::fs::write(
        dir.path().join("cordis/host.json"),
        json!({"url": url, "token": "bridge-token"}).to_string(),
    )
    .expect("host bridge");
    std::fs::write(
        dir.path().join("plugin.mjs"),
        r#"
export default {
  apply(ctx) {
    ctx.tool({
      name: "agents",
      description: "agents bridge",
      handler: () => {
        const agents = ctx.get("agents");
        return JSON.stringify({
          count: agents.list().length,
          roots: agents.roots().map((agent) => agent.id),
          rootOf: agents.rootOf("s1#sub1"),
          owned: agents.isOwnedBy("s1#sub1", "s1"),
          parentId: agents.get("s1#sub1")?.parentId,
        });
      },
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "bridge".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    // The bridge poll runs every 2 s; give the first one time to land.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__bridge__agents")
        .expect("agents tool");
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = tool.execute(&ctx, json!({})).await.expect("call");
    let content = outcome.content;
    assert!(content.contains("\"count\":2"), "{content}");
    assert!(content.contains("\"roots\":[\"s1\"]"), "{content}");
    assert!(content.contains("\"rootOf\":\"s1\""), "{content}");
    assert!(content.contains("\"owned\":true"), "{content}");
    assert!(content.contains("\"parentId\":\"s1\""), "{content}");
    drop(stub);
}

/// The `settings` service resolves schema defaults over a composition base and
/// a persisted user layer, notifies watchers and reports revisions.
#[tokio::test]
async fn settings_service_resolves_and_persists() {
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
    const settings = ctx.get("settings");
    const scope = settings.register(
      "probe-settings",
      (value) => ({ mode: "auto", size: 5, ...value }),
      { base: { size: 9 }, applies: "live" },
    );
    ctx.tool({
      name: "settings_probe",
      description: "settings",
      handler: async () => {
        const before = scope.get();
        let seen = null;
        const dispose = scope.watch((next) => { seen = next; });
        await scope.update({ mode: "manual" });
        dispose();
        const after = scope.get();
        const described = settings
          .describe()
          .map((item) => ({ ns: item.ns, revision: item.revision, applies: item.applies }));
        return JSON.stringify({
          before,
          after,
          seen,
          described,
          writable: settings.writable === true,
          configured: typeof settings.configure === "function",
        });
      },
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "settings".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__settings__settings_probe")
        .expect("settings tool");
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let content = tool.execute(&ctx, json!({})).await.expect("call").content;
    let value: serde_json::Value = serde_json::from_str(&content).expect("json");
    assert_eq!(value["before"]["mode"], json!("auto"));
    assert_eq!(value["before"]["size"], json!(9));
    assert_eq!(value["after"]["mode"], json!("manual"));
    assert_eq!(value["after"]["size"], json!(9));
    assert_eq!(value["seen"]["mode"], json!("manual"));
    assert_eq!(value["described"][0]["ns"], json!("probe-settings"));
    assert_eq!(value["described"][0]["applies"], json!("live"));
    assert_eq!(value["writable"], json!(true));
    assert_eq!(value["configured"], json!(true));

    let stored =
        std::fs::read_to_string(dir.path().join("cordis/settings.json")).expect("settings");
    assert!(stored.contains("probe-settings"), "{stored}");
    assert!(stored.contains("manual"), "{stored}");
}

/// Plugin commands are exposed to the host as `cmd__<plugin>__<name>` tools and
/// run through the sidecar command registry.
#[tokio::test]
async fn plugin_commands_become_tools() {
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
    ctx.commands.register({
      name: "hello",
      description: "Say hello",
      input: { hint: "<name>" },
      handler: (invocation) => `hello:${invocation.args}:${invocation.agent?.sessionId ?? "none"}`,
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "cmds".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cmd__cmds__hello")
        .expect("command tool");
    assert!(tool.description().contains("Say hello"));

    let session = SessionId::new();
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: session.clone(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let outcome = tool
        .execute(&ctx, json!({"args": "world"}))
        .await
        .expect("call");
    assert_eq!(outcome.content, format!("hello:world:{}", session.as_str()));
}

/// Plugins that log with `console.log` must not corrupt the JSON-RPC channel.
#[tokio::test]
async fn plugin_stdout_logging_does_not_break_mcp() {
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
console.log("[plugin] loading");
export default {
  apply(ctx) {
    console.log("[plugin] applying");
    ctx.tool({ name: "quiet", description: "still works", handler: () => "ok" });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "noisy".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect despite console.log");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__noisy__quiet")
        .expect("tool registered");
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    assert_eq!(
        tool.execute(&ctx, json!({})).await.expect("call").content,
        "ok"
    );
}

/// Compatibility services answer honestly instead of leaving plugins with
/// `undefined` (empty lists, fail-closed approval, ready loader).
#[tokio::test]
async fn compatibility_services_answer_honestly() {
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
      name: "services",
      description: "compat services",
      handler: async () => {
        const loader = ctx.get("loader");
        await loader.await();
        const approval = await ctx.get("approval").request({ tool: "bash" });
        return JSON.stringify({
          loader: loader.loaded === true,
          profile: ctx.get("profileContext").profile,
          workspaces: ctx.get("workspaceRegistry").list().length,
          presets: ctx.get("agentPresets").list(),
          credentials: await ctx.get("credentials").list(),
          jobs: ctx.get("jobs").list(),
          search: await ctx.get("sessionQuery").search("x"),
          packages: ctx.get("pluginPackages").list(),
          approved: approval.approved,
          decision: approval.decision,
          defaultModel: ctx.get("agentDefaultModel").model,
        });
      },
    });
  },
};
"#,
    )
    .expect("plugin");

    let connection = connect(
        CordisPluginSpec {
            name: "compat".into(),
            entry: "plugin.mjs".into(),
            root: dir.path().to_path_buf(),
            env: Default::default(),
            grants: dshr_config::CordisGrants::default(),
        },
        dir.path(),
    )
    .await
    .expect("connect");
    let tool = connection
        .tools
        .iter()
        .find(|tool| tool.name() == "cordis__compat__services")
        .expect("services tool");
    let ctx = ToolCtx {
        cwd: dir.path().to_path_buf(),
        session_id: SessionId::new(),
        output_limit: 10_000,
        spill_dir: dir.path().join("spills"),
        cancel: CancellationToken::new(),
        prompt: None,
    };
    let content = tool.execute(&ctx, json!({})).await.expect("call").content;
    let value: serde_json::Value = serde_json::from_str(&content).expect("json");
    assert_eq!(value["loader"], json!(true));
    assert_eq!(value["workspaces"], json!(1));
    assert_eq!(value["presets"], json!([]));
    assert_eq!(value["credentials"], json!([]));
    assert_eq!(value["jobs"], json!([]));
    assert_eq!(value["search"], json!([]));
    assert_eq!(value["packages"], json!([]));
    assert_eq!(value["approved"], json!(false));
    assert_eq!(value["decision"], json!("deny"));
    assert_eq!(value["defaultModel"], json!(null));
}
