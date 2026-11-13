use std::sync::Arc;

use dshr_acp::{AcpConfig, AcpServer};
use dshr_agent::PermissionMode;
use dshr_llm::{StopReason, StreamChunk};
use dshr_llm_mock::ScriptedProvider;
use dshr_session_jsonl::SessionStore;
use dshr_tools::ToolRegistry;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex, split};

async fn read_json(reader: &mut BufReader<impl tokio::io::AsyncRead + Unpin>) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).await.expect("line");
    serde_json::from_str(&line).expect("json")
}

#[tokio::test]
async fn initialize_new_and_prompt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::TextDelta {
                text: "hello from acp".into(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]],
    ));
    let server = AcpServer::new(
        provider,
        Arc::new(ToolRegistry::new()),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 2,
            max_tokens: 64,
            output_limit: 10_000,
        },
    );
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}})
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let mut reader = BufReader::new(client_read);
    let initialized = read_json(&mut reader).await;
    assert_eq!(initialized["result"]["protocolVersion"], 1);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":dir.path(),"mcpServers":[]}})
            )
            .as_bytes(),
        )
        .await
        .expect("new");
    let created = read_json(&mut reader).await;
    let session_id = created["result"]["sessionId"].as_str().expect("session id");

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":3,"method":"session/prompt",
                    "params": {
                        "sessionId": session_id,
                        "prompt": [{"type":"text","text":"hi"}]
                    }
                })
            )
            .as_bytes(),
        )
        .await
        .expect("prompt");

    let mut text = String::new();
    let mut stop = None;
    while stop.is_none() {
        let message = read_json(&mut reader).await;
        if message["method"] == "session/update"
            && message["params"]["update"]["sessionUpdate"] == "agent_message_chunk"
        {
            text.push_str(
                message["params"]["update"]["content"]["text"]
                    .as_str()
                    .unwrap_or_default(),
            );
        }
        if message["id"] == 3 {
            stop = message["result"]["stopReason"]
                .as_str()
                .map(ToOwned::to_owned);
        }
    }
    assert!(text.contains("hello from acp"), "{text}");
    assert_eq!(stop.as_deref(), Some("end_turn"));
}

#[derive(Clone)]
struct NoopBash;

#[async_trait::async_trait]
impl dshr_tools::Tool for NoopBash {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "noop"
    }

    fn schema(&self) -> Value {
        json!({"type": "object"})
    }

    async fn execute(
        &self,
        _ctx: &dshr_tools::ToolCtx,
        _args: Value,
    ) -> Result<dshr_tools::ToolOutcome, dshr_tools::ToolError> {
        Ok(dshr_tools::ToolOutcome::text("tool ok"))
    }
}

#[tokio::test]
async fn asks_the_client_for_permission() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![
            vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::ToolUseStart {
                    index: 0,
                    id: "call-1".into(),
                    name: "bash".into(),
                },
                StreamChunk::ToolInputDelta {
                    index: 0,
                    json: r#"{"command":"echo hi"}"#.into(),
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
                    text: "after tool".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
    ));
    let mut tools = ToolRegistry::new();
    tools.register(NoopBash);
    let server = AcpServer::new(
        provider,
        Arc::new(tools),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::WorkspaceWrite,
            max_steps: 3,
            max_tokens: 64,
            output_limit: 10_000,
        },
    );
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}})
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let _ = read_json(&mut reader).await;
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":dir.path()}})
            )
            .as_bytes(),
        )
        .await
        .expect("new");
    let created = read_json(&mut reader).await;
    let session_id = created["result"]["sessionId"].as_str().expect("session");

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":3,"method":"session/prompt",
                    "params":{"sessionId":session_id,"prompt":[{"type":"text","text":"run"}]}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("prompt");

    let mut text = String::new();
    let mut stop = None;
    while stop.is_none() {
        let message = read_json(&mut reader).await;
        if message["method"] == "session/request_permission" {
            let permission_id = message["id"].clone();
            client_write
                .write_all(
                    format!(
                        "{}\n",
                        json!({
                            "jsonrpc":"2.0","id":permission_id,
                            "result":{"outcome":{"outcome":"selected","optionId":"allow-once"}}
                        })
                    )
                    .as_bytes(),
                )
                .await
                .expect("permission");
        }
        if message["method"] == "session/update"
            && message["params"]["update"]["sessionUpdate"] == "agent_message_chunk"
        {
            text.push_str(
                message["params"]["update"]["content"]["text"]
                    .as_str()
                    .unwrap_or_default(),
            );
        }
        if message["id"] == 3 {
            stop = message["result"]["stopReason"]
                .as_str()
                .map(ToOwned::to_owned);
        }
    }
    assert!(text.contains("after tool"), "{text}");
    assert_eq!(stop.as_deref(), Some("end_turn"));
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn reads_files_through_the_client() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![
            vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::ToolUseStart {
                    index: 0,
                    id: "call-read".into(),
                    name: "read".into(),
                },
                StreamChunk::ToolInputDelta {
                    index: 0,
                    json: format!(
                        r#"{{"path":"{}"}}"#,
                        dir.path().join("client.txt").display()
                    ),
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
                    text: "used client file".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
    ));
    let fs = Arc::new(dshr_acp::AcpFsClient::new());
    let mut tools = ToolRegistry::new();
    tools.register(dshr_acp::AcpReadTool::new(Arc::clone(&fs)));
    let server = AcpServer::new(
        provider,
        Arc::new(tools),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 3,
            max_tokens: 64,
            output_limit: 10_000,
        },
    )
    .with_fs(fs);
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":1,"method":"initialize",
                    "params":{"protocolVersion":1,"clientCapabilities":{"fs":{"readTextFile":true}}}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let _ = read_json(&mut reader).await;
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":dir.path()}})
            )
            .as_bytes(),
        )
        .await
        .expect("new");
    let created = read_json(&mut reader).await;
    let session_id = created["result"]["sessionId"].as_str().expect("session");
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":3,"method":"session/prompt",
                    "params":{"sessionId":session_id,"prompt":[{"type":"text","text":"read"}]}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("prompt");

    let mut text = String::new();
    let mut stop = None;
    while stop.is_none() {
        let message = read_json(&mut reader).await;
        if message["method"] == "fs/read_text_file" {
            let request_id = message["id"].clone();
            client_write
                .write_all(
                    format!(
                        "{}\n",
                        json!({"jsonrpc":"2.0","id":request_id,"result":{"content":"client text"}})
                    )
                    .as_bytes(),
                )
                .await
                .expect("fs reply");
        }
        if message["method"] == "session/update"
            && message["params"]["update"]["sessionUpdate"] == "agent_message_chunk"
        {
            text.push_str(
                message["params"]["update"]["content"]["text"]
                    .as_str()
                    .unwrap_or_default(),
            );
        }
        if message["id"] == 3 {
            stop = message["result"]["stopReason"]
                .as_str()
                .map(ToOwned::to_owned);
        }
    }
    assert!(text.contains("used client file"), "{text}");
    assert_eq!(stop.as_deref(), Some("end_turn"));
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn uses_client_terminal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![
            vec![
                StreamChunk::Start {
                    model: "test-model".into(),
                },
                StreamChunk::ToolUseStart {
                    index: 0,
                    id: "call-bash".into(),
                    name: "bash".into(),
                },
                StreamChunk::ToolInputDelta {
                    index: 0,
                    json: r#"{"command":"echo hi"}"#.into(),
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
                    text: "terminal done".into(),
                },
                StreamChunk::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ],
    ));
    let terminal = Arc::new(dshr_acp::AcpTerminalClient::new());
    let mut tools = ToolRegistry::new();
    tools.register(dshr_acp::AcpBashTool::new(Arc::clone(&terminal)));
    let server = AcpServer::new(
        provider,
        Arc::new(tools),
        Arc::new(SessionStore::new(dir.path().join("sessions"))),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 3,
            max_tokens: 64,
            output_limit: 10_000,
        },
    )
    .with_terminal(terminal);
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":1,"method":"initialize",
                    "params":{"protocolVersion":1,"clientCapabilities":{"terminal":true}}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let _ = read_json(&mut reader).await;
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":dir.path()}})
            )
            .as_bytes(),
        )
        .await
        .expect("new");
    let created = read_json(&mut reader).await;
    let session_id = created["result"]["sessionId"].as_str().expect("session");
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":3,"method":"session/prompt",
                    "params":{"sessionId":session_id,"prompt":[{"type":"text","text":"run"}]}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("prompt");

    let mut text = String::new();
    let mut stop = None;
    let mut created_terminal = false;
    while stop.is_none() {
        let message = read_json(&mut reader).await;
        let method = message["method"].as_str().unwrap_or_default();
        if method == "terminal/create" {
            created_terminal = true;
        }
        let result = match method {
            "terminal/create" => Some(json!({"terminalId": "term-1"})),
            "terminal/wait_for_exit" => Some(json!({"exitCode": 0, "signal": null})),
            "terminal/output" => Some(json!({
                "output": "hi\n", "truncated": false,
                "exitStatus": {"exitCode": 0, "signal": null}
            })),
            "terminal/release" => Some(json!({})),
            _ => None,
        };
        if let Some(result) = result {
            let request_id = message["id"].clone();
            client_write
                .write_all(
                    format!(
                        "{}\n",
                        json!({"jsonrpc":"2.0","id":request_id,"result":result})
                    )
                    .as_bytes(),
                )
                .await
                .expect("terminal reply");
        }
        if message["method"] == "session/update"
            && message["params"]["update"]["sessionUpdate"] == "agent_message_chunk"
        {
            text.push_str(
                message["params"]["update"]["content"]["text"]
                    .as_str()
                    .unwrap_or_default(),
            );
        }
        if message["id"] == 3 {
            stop = message["result"]["stopReason"]
                .as_str()
                .map(ToOwned::to_owned);
        }
    }
    assert!(created_terminal);
    assert!(text.contains("terminal done"), "{text}");
    assert_eq!(stop.as_deref(), Some("end_turn"));
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn loads_and_replays_a_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(SessionStore::new(dir.path().join("sessions")));
    let mut session = store
        .create("test", "test-model", &dir.path().to_string_lossy())
        .expect("create");
    let user_seq = session.next_seq();
    store
        .append(
            &mut session,
            dshr_session::SessionEvent::UserMessage {
                seq: user_seq,
                at: "2026-01-01T00:00:00Z".into(),
                turn: dshr_core::TurnId::new(),
                content: vec![dshr_llm::ContentBlock::text("old user")],
            },
        )
        .expect("user");
    let agent_seq = session.next_seq();
    store
        .append(
            &mut session,
            dshr_session::SessionEvent::AssistantMessage {
                seq: agent_seq,
                at: "2026-01-01T00:00:01Z".into(),
                turn: dshr_core::TurnId::new(),
                content: vec![dshr_llm::ContentBlock::text("old assistant")],
                usage: dshr_llm::Usage::default(),
                stop_reason: Some(dshr_llm::StopReason::EndTurn),
            },
        )
        .expect("assistant");
    let session_id = session.header.id.to_string();

    let provider = Arc::new(ScriptedProvider::new("test-model", vec![]));
    let server = AcpServer::new(
        provider,
        Arc::new(ToolRegistry::new()),
        Arc::clone(&store),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 2,
            max_tokens: 64,
            output_limit: 10_000,
        },
    );
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}})
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let initialized = read_json(&mut reader).await;
    assert_eq!(
        initialized["result"]["agentCapabilities"]["loadSession"],
        true
    );

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":2,"method":"session/load",
                    "params":{"sessionId":session_id,"cwd":dir.path()}
                })
            )
            .as_bytes(),
        )
        .await
        .expect("load");

    let mut text = String::new();
    let mut done = false;
    while !done {
        let message = read_json(&mut reader).await;
        if message["method"] == "session/update" {
            text.push_str(
                message["params"]["update"]["content"]["text"]
                    .as_str()
                    .unwrap_or_default(),
            );
        }
        if message["id"] == 2 {
            done = true;
        }
    }
    assert!(text.contains("old user"), "{text}");
    assert!(text.contains("old assistant"), "{text}");
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn resumes_and_closes_a_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(SessionStore::new(dir.path().join("sessions")));
    let session = store
        .create("test", "test-model", &dir.path().to_string_lossy())
        .expect("create");
    let session_id = session.header.id.to_string();
    let server = AcpServer::new(
        Arc::new(ScriptedProvider::new("test-model", vec![])),
        Arc::new(ToolRegistry::new()),
        Arc::clone(&store),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 2,
            max_tokens: 64,
            output_limit: 10_000,
        },
    );
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}})
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let initialized = read_json(&mut reader).await;
    assert!(
        initialized["result"]["agentCapabilities"]["sessionCapabilities"]["resume"].is_object()
    );

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/resume","params":{"sessionId":session_id}})
            )
            .as_bytes(),
        )
        .await
        .expect("resume");
    let resumed = read_json(&mut reader).await;
    assert!(resumed["result"].is_object());

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":3,"method":"session/close","params":{"sessionId":session_id}})
            )
            .as_bytes(),
        )
        .await
        .expect("close");
    let closed = read_json(&mut reader).await;
    assert!(closed["result"].is_object());
}

#[cfg(not(target_os = "windows"))]
#[tokio::test]
async fn accepts_image_prompts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(SessionStore::new(dir.path().join("sessions")));
    let provider = Arc::new(ScriptedProvider::new(
        "test-model",
        vec![vec![
            StreamChunk::Start {
                model: "test-model".into(),
            },
            StreamChunk::TextDelta {
                text: "image seen".into(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]],
    ));
    let server = AcpServer::new(
        provider,
        Arc::new(ToolRegistry::new()),
        Arc::clone(&store),
        AcpConfig {
            system_prompt: "test".into(),
            mode: PermissionMode::DangerFull,
            max_steps: 2,
            max_tokens: 64,
            output_limit: 10_000,
        },
    );
    let (server_io, client_io) = duplex(128 * 1024);
    let (server_read, server_write) = split(server_io);
    let (client_read, mut client_write) = split(client_io);
    tokio::spawn(async move { server.serve(server_read, server_write).await });
    let mut reader = BufReader::new(client_read);

    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}})
            )
            .as_bytes(),
        )
        .await
        .expect("initialize");
    let initialized = read_json(&mut reader).await;
    assert_eq!(
        initialized["result"]["agentCapabilities"]["promptCapabilities"]["image"],
        true
    );
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":dir.path()}})
            )
            .as_bytes(),
        )
        .await
        .expect("new");
    let created = read_json(&mut reader).await;
    let session_id = created["result"]["sessionId"].as_str().expect("session");
    client_write
        .write_all(
            format!(
                "{}\n",
                json!({
                    "jsonrpc":"2.0","id":3,"method":"session/prompt",
                    "params":{
                        "sessionId":session_id,
                        "prompt":[
                            {"type":"text","text":"what is this"},
                            {"type":"image","data":"aGk=","mimeType":"image/png"}
                        ]
                    }
                })
            )
            .as_bytes(),
        )
        .await
        .expect("prompt");
    let mut stop = None;
    while stop.is_none() {
        let message = read_json(&mut reader).await;
        if message["id"] == 3 {
            stop = message["result"]["stopReason"]
                .as_str()
                .map(ToOwned::to_owned);
        }
    }
    assert_eq!(stop.as_deref(), Some("end_turn"));
    let session = store.open(session_id).expect("session");
    assert!(session.events.iter().any(|event| matches!(
        event,
        dshr_session::SessionEvent::UserMessage { content, .. }
            if content.iter().any(|block| matches!(block, dshr_llm::ContentBlock::Image { .. }))
    )));
}
