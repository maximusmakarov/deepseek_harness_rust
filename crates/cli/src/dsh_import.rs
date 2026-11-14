//! Import DSH v4 sessions (`session.v4.jsonl.zstd`).

use std::collections::HashMap;
use std::path::Path;

use dshr_core::{DshrError, Result, TurnId};
use dshr_llm::{ContentBlock, Usage};
use dshr_session::{Session, SessionEvent, TurnEndReason, now_rfc3339};
use dshr_session_jsonl::SessionStore;
use serde_json::{Value, json};

/// Imports one DSH session file or a session directory.
pub fn import(store: &SessionStore, path: &Path, provider: &str, model: &str) -> Result<String> {
    let file = if path.is_dir() {
        let candidate = path.join("session.v4.jsonl.zstd");
        if candidate.is_file() {
            candidate
        } else {
            return Err(DshrError::not_found(format!(
                "no session.v4.jsonl.zstd in {}",
                path.display()
            )));
        }
    } else {
        path.to_path_buf()
    };
    let bytes = std::fs::read(&file)?;
    let text = match zstd::decode_all(std::io::Cursor::new(&bytes)) {
        Ok(decoded) => String::from_utf8(decoded)
            .map_err(|_| DshrError::config("session is not valid UTF-8"))?,
        Err(_) => String::from_utf8(bytes.clone())
            .map_err(|_| DshrError::config("session is neither zstd nor UTF-8 JSONL"))?,
    };

    let mut cwd = ".".to_owned();
    let mut values = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value["type"] == "session"
            && let Some(path) = value["cwd"].as_str()
        {
            cwd = path.to_owned();
        }
        values.push(value);
    }
    let mut session = store.create(provider, model, &cwd)?;
    let mut turns: HashMap<u64, TurnId> = HashMap::new();
    let mut active_turn: Option<TurnId> = None;
    let mut tool_names: HashMap<String, String> = HashMap::new();
    let mut title = None;

    for value in &values {
        let kind = value["type"].as_str().unwrap_or_default();
        let data = &value["data"];
        match kind {
            "turn/start" => {
                let number = data["turn"].as_u64().unwrap_or(0);
                let turn = turns.entry(number).or_default().clone();
                let seq = session.next_seq();
                store.append(
                    &mut session,
                    SessionEvent::TurnStart {
                        seq,
                        at: now_rfc3339(),
                        turn: turn.clone(),
                    },
                )?;
                active_turn = Some(turn);
            }
            "user/message" => {
                let content = content_blocks(data["content"].as_array());
                if content.is_empty() {
                    continue;
                }
                let turn = ensure_turn(store, &mut session, &mut turns, &mut active_turn, data)?;
                let seq = session.next_seq();
                store.append(
                    &mut session,
                    SessionEvent::UserMessage {
                        seq,
                        at: now_rfc3339(),
                        turn,
                        content,
                    },
                )?;
            }
            "assistant/message" => {
                let content = content_blocks(data["message"]["content"].as_array());
                if content.is_empty() {
                    continue;
                }
                for block in &content {
                    if let ContentBlock::ToolUse { id, name, .. } = block {
                        tool_names.insert(id.clone(), name.clone());
                    }
                }
                let turn = ensure_turn(store, &mut session, &mut turns, &mut active_turn, data)?;
                let seq = session.next_seq();
                store.append(
                    &mut session,
                    SessionEvent::AssistantMessage {
                        seq,
                        at: now_rfc3339(),
                        turn,
                        content,
                        usage: Usage::default(),
                        stop_reason: None,
                    },
                )?;
            }
            "tool/result" => {
                let call_id = data["toolCallId"].as_str().unwrap_or_default().to_owned();
                if call_id.is_empty() {
                    continue;
                }
                let turn = ensure_turn(store, &mut session, &mut turns, &mut active_turn, data)?;
                let name = tool_names.remove(&call_id).unwrap_or_default();
                let content = data["message"]["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter_map(|block| block["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
                let seq = session.next_seq();
                store.append(
                    &mut session,
                    SessionEvent::ToolResult {
                        seq,
                        at: now_rfc3339(),
                        turn,
                        tool_call_id: call_id,
                        name,
                        content,
                        is_error: false,
                    },
                )?;
            }
            "turn/end" => {
                if let Some(turn) = active_turn.take() {
                    let seq = session.next_seq();
                    store.append(
                        &mut session,
                        SessionEvent::TurnEnd {
                            seq,
                            at: now_rfc3339(),
                            turn,
                            reason: TurnEndReason::Completed,
                        },
                    )?;
                }
            }
            "session/title" => {
                title = data["title"].as_str().map(ToOwned::to_owned);
            }
            _ => {}
        }
    }
    if let Some(title) = title
        && !title.trim().is_empty()
    {
        session = store.set_title(session.header.id.as_str(), &title)?;
    }
    Ok(session.header.id.to_string())
}

/// Imports every `session.v4.jsonl.zstd` under `root`.
pub fn import_tree(
    store: &SessionStore,
    root: &Path,
    provider: &str,
    model: &str,
) -> Result<usize> {
    let mut imported = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str())
                == Some("session.v4.jsonl.zstd")
            {
                if import(store, &path, provider, model).is_ok() {
                    imported += 1;
                }
            }
        }
    }
    Ok(imported)
}

fn ensure_turn(
    store: &SessionStore,
    session: &mut Session,
    turns: &mut HashMap<u64, TurnId>,
    active: &mut Option<TurnId>,
    data: &Value,
) -> Result<TurnId> {
    let number = data["turn"].as_u64().unwrap_or(0);
    if let Some(turn) = turns.get(&number) {
        return Ok(turn.clone());
    }
    let turn = TurnId::new();
    turns.insert(number, turn.clone());
    let seq = session.next_seq();
    store.append(
        session,
        SessionEvent::TurnStart {
            seq,
            at: now_rfc3339(),
            turn: turn.clone(),
        },
    )?;
    *active = Some(turn.clone());
    Ok(turn)
}

fn content_blocks(blocks: Option<&Vec<Value>>) -> Vec<ContentBlock> {
    let mut out = Vec::new();
    for block in blocks.into_iter().flatten() {
        match block["type"].as_str().unwrap_or_default() {
            "text" => {
                if let Some(text) = block["text"].as_str()
                    && !text.is_empty()
                {
                    out.push(ContentBlock::Text {
                        text: text.to_owned(),
                    });
                }
            }
            "reasoning" => {
                if let Some(text) = block["text"].as_str()
                    && !text.is_empty()
                {
                    out.push(ContentBlock::Thinking {
                        text: text.to_owned(),
                    });
                }
            }
            "tool-call" => {
                let id = block["id"].as_str().unwrap_or_default().to_owned();
                let name = block["name"].as_str().unwrap_or_default().to_owned();
                if id.is_empty() || name.is_empty() {
                    continue;
                }
                let input = block["arguments"]
                    .as_str()
                    .and_then(|arguments| serde_json::from_str(arguments).ok())
                    .or_else(|| block.get("arguments").cloned())
                    .unwrap_or_else(|| json!({}));
                out.push(ContentBlock::ToolUse { id, name, input });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_session_jsonl::SessionStore;

    #[test]
    fn imports_all_sessions_under_a_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().join("sessions"));
        let events = vec![
            json!({"type":"session","cwd":"/tmp/a"}),
            json!({"type":"turn/start","data":{"turn":1}}),
            json!({"type":"user/message","data":{"turn":1,"content":[{"type":"text","text":"hi"}]}}),
        ];
        let text = events
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let compressed = zstd::encode_all(text.as_bytes(), 3).expect("zstd");
        for name in ["one", "two"] {
            let session_dir = dir.path().join("root").join(name);
            std::fs::create_dir_all(&session_dir).expect("dir");
            std::fs::write(session_dir.join("session.v4.jsonl.zstd"), &compressed).expect("write");
        }
        let count = import_tree(&store, &dir.path().join("root"), "dsh-import", "dsh-import")
            .expect("import");
        assert_eq!(count, 2);
        assert_eq!(store.list().expect("list").len(), 2);
    }

    #[test]
    fn imports_a_v4_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path().join("sessions"));
        let events = [
            json!({"type":"session","id":"session-x","cwd":"/tmp/work","createdAt":1}),
            json!({"type":"session/title","data":{"title":"Imported"}}),
            json!({"type":"turn/start","data":{"turn":1}}),
            json!({"type":"user/message","data":{"turn":1,"content":[{"type":"text","text":"hello"}]}}),
            json!({"type":"assistant/message","data":{"turn":1,"message":{"content":[
                {"type":"text","text":"hi"},
                {"type":"tool-call","id":"c1","name":"read","arguments":"{\"path\":\"a.txt\"}"}
            ]}}}),
            json!({"type":"tool/result","data":{"turn":1,"toolCallId":"c1","message":{"content":[{"type":"text","text":"file"}]}}}),
            json!({"type":"turn/end","data":{"turn":1}}),
        ];
        let text = events
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let compressed = zstd::encode_all(text.as_bytes(), 3).expect("zstd");
        let path = dir.path().join("session.v4.jsonl.zstd");
        std::fs::write(&path, compressed).expect("write");
        let id = import(&store, &path, "dsh-import", "dsh-import").expect("import");
        let session = store.open(&id).expect("open");
        assert_eq!(session.header.title.as_deref(), Some("Imported"));
        assert_eq!(session.header.cwd, "/tmp/work");
        assert!(session.events.iter().any(|event| matches!(
            event,
            SessionEvent::ToolResult { content, name, .. } if content == "file" && name == "read"
        )));
        let messages = session.derive_messages();
        assert!(messages.iter().any(|message| {
            message
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::Text { text } if text == "hello"))
        }));
    }
}
