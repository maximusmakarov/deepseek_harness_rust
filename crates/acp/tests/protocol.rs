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
