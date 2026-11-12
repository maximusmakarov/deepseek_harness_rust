#![forbid(unsafe_code)]
//! Durable JSONL session store.
//!
//! Layout:
//!
//! ```text
//! <root>/<session-id>/
//!   header.json
//!   session.v1.jsonl
//! ```
//!
//! Appends are line-oriented and fsynced. On open, a torn trailing line is
//! truncated back to the last complete newline, so a `kill -9` during a write
//! cannot corrupt the log.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use dshr_core::{DshrError, Result};
use dshr_llm::ContentBlock;
use dshr_session::{
    SESSION_FORMAT_VERSION, Session, SessionEvent, SessionHeader, SessionSummary, now_rfc3339,
};

const HEADER_FILE: &str = "header.json";
const EVENTS_FILE: &str = "session.v1.jsonl";

#[derive(Debug, Clone)]
pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create(&self, provider: &str, model: &str, cwd: &str) -> Result<Session> {
        fs::create_dir_all(&self.root)?;
        let id = dshr_core::SessionId::new();
        let dir = self.root.join(id.as_str());
        fs::create_dir_all(&dir)?;

        let header = SessionHeader {
            version: SESSION_FORMAT_VERSION,
            id,
            created_at: now_rfc3339(),
            provider: provider.to_owned(),
            model: model.to_owned(),
            cwd: cwd.to_owned(),
        };
        write_header(&dir, &header)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(EVENTS_FILE))?;

        let mut session = Session::empty(header, dir);
        self.append(
            &mut session,
            SessionEvent::SessionCreated {
                seq: 1,
                at: now_rfc3339(),
                provider: provider.to_owned(),
                model: model.to_owned(),
                cwd: cwd.to_owned(),
            },
        )?;
        Ok(session)
    }

    pub fn open(&self, id: &str) -> Result<Session> {
        let dir = self.root.join(id);
        if !dir.is_dir() {
            return Err(DshrError::not_found(format!("session '{id}' not found")));
        }
        let header = read_header(&dir)?;
        if header.version != SESSION_FORMAT_VERSION {
            return Err(DshrError::config(format!(
                "unsupported session version {} (expected {SESSION_FORMAT_VERSION})",
                header.version
            )));
        }
        if header.id.as_str() != id {
            return Err(DshrError::config(format!(
                "session id mismatch: header '{}' vs directory '{id}'",
                header.id
            )));
        }
        let events = read_events(&dir.join(EVENTS_FILE))?;
        Ok(Session {
            header,
            events,
            dir,
        })
    }

    pub fn append(&self, session: &mut Session, event: SessionEvent) -> Result<()> {
        let expected = session.next_seq();
        if event.seq() != expected {
            return Err(DshrError::invalid_input(format!(
                "event seq {} does not follow {}",
                event.seq(),
                expected - 1
            )));
        }

        let mut line = serde_json::to_vec(&event)
            .map_err(|err| DshrError::config(format!("serialize event: {err}")))?;
        line.push(b'\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(session.dir.join(EVENTS_FILE))?;
        file.write_all(&line)?;
        file.sync_data()?;

        session.events.push(event);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<SessionSummary>> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut summaries = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let dir = entry.path();
            if !dir.join(HEADER_FILE).is_file() {
                continue;
            }
            let Ok(header) = read_header(&dir) else {
                continue;
            };
            let events = read_events(&dir.join(EVENTS_FILE)).unwrap_or_default();
            let session = Session {
                header,
                events,
                dir,
            };
            summaries.push(session.summary());
        }
        summaries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(summaries)
    }

    pub fn export_json(&self, session: &Session) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Export<'a> {
            header: &'a SessionHeader,
            events: &'a [SessionEvent],
        }
        serde_json::to_string_pretty(&Export {
            header: &session.header,
            events: &session.events,
        })
        .map_err(|err| DshrError::config(format!("serialize session: {err}")))
    }

    pub fn export_markdown(&self, session: &Session) -> Result<String> {
        let mut out = String::new();
        out.push_str(&format!(
            "# Session {}\n\n- created: {}\n- provider: {}\n- model: {}\n- cwd: {}\n\n",
            session.header.id,
            session.header.created_at,
            session.header.provider,
            session.header.model,
            session.header.cwd
        ));
        for event in &session.events {
            match event {
                SessionEvent::UserMessage { content, .. } => {
                    out.push_str("## User\n\n");
                    out.push_str(&render_blocks(content));
                    out.push_str("\n\n");
                }
                SessionEvent::AssistantMessage { content, .. } => {
                    out.push_str("## Assistant\n\n");
                    out.push_str(&render_blocks(content));
                    out.push_str("\n\n");
                }
                _ => {}
            }
        }
        Ok(out)
    }
}

fn write_header(dir: &Path, header: &SessionHeader) -> Result<()> {
    let path = dir.join(HEADER_FILE);
    let temporary = dir.join("header.json.tmp");
    let text = serde_json::to_string_pretty(header)
        .map_err(|err| DshrError::config(format!("serialize header: {err}")))?;
    fs::write(&temporary, text)?;
    fs::rename(&temporary, &path)?;
    Ok(())
}

fn read_header(dir: &Path) -> Result<SessionHeader> {
    let path = dir.join(HEADER_FILE);
    let text = fs::read_to_string(&path)?;
    serde_json::from_str(&text)
        .map_err(|err| DshrError::config(format!("invalid header {}: {err}", path.display())))
}

fn read_events(path: &Path) -> Result<Vec<SessionEvent>> {
    let bytes = fs::read(path)?;
    let valid_len = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |position| position + 1);

    if valid_len < bytes.len() {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(valid_len as u64)?;
        file.sync_data()?;
    }

    let text = String::from_utf8_lossy(&bytes[..valid_len]);
    let mut events = Vec::new();
    let mut expected = 1;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: SessionEvent = serde_json::from_str(line).map_err(|err| {
            DshrError::config(format!(
                "invalid event at {}:{}: {err}",
                path.display(),
                index + 1
            ))
        })?;
        if event.seq() != expected {
            return Err(DshrError::config(format!(
                "event sequence gap at {}:{}: expected {expected}, got {}",
                path.display(),
                index + 1,
                event.seq()
            )));
        }
        expected += 1;
        events.push(event);
    }
    Ok(events)
}

fn render_blocks(blocks: &[ContentBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            ContentBlock::Text { text } => out.push_str(text),
            ContentBlock::Thinking { text } => {
                out.push_str("<details><summary>thinking</summary>\n\n");
                out.push_str(text);
                out.push_str("\n\n</details>\n");
            }
            ContentBlock::ToolUse { name, input, .. } => {
                out.push_str(&format!("`{name}` → `{input}`\n"));
            }
            ContentBlock::ToolResult { content, .. } => {
                out.push_str(&format!("result: {content}\n"));
            }
            ContentBlock::Image { .. } => out.push_str("[image]\n"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_core::TurnId;
    use dshr_llm::{Message, Role, StopReason, Usage};

    fn create(store: &SessionStore) -> Session {
        store
            .create("deepseek-account", "deepseek-flash", "/tmp/project")
            .expect("create")
    }

    fn round_trip(store: &SessionStore, session: &mut Session) {
        let turn = TurnId::new();
        let at = now_rfc3339();
        let events = vec![
            SessionEvent::TurnStart {
                seq: session.next_seq(),
                at: at.clone(),
                turn: turn.clone(),
            },
            SessionEvent::UserMessage {
                seq: session.next_seq() + 1,
                at: at.clone(),
                turn: turn.clone(),
                content: vec![ContentBlock::text("hi")],
            },
            SessionEvent::AssistantMessage {
                seq: session.next_seq() + 2,
                at: at.clone(),
                turn: turn.clone(),
                content: vec![ContentBlock::text("hello")],
                usage: Usage {
                    input_tokens: 3,
                    output_tokens: 1,
                    ..Usage::default()
                },
                stop_reason: Some(StopReason::EndTurn),
            },
            SessionEvent::TurnEnd {
                seq: session.next_seq() + 3,
                at,
                turn,
                reason: dshr_session::TurnEndReason::Completed,
            },
        ];
        for event in events {
            store.append(session, event).expect("append");
        }
    }

    #[test]
    fn create_append_and_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);

        let reopened = store.open(session.header.id.as_str()).expect("open");
        assert_eq!(reopened.events.len(), session.events.len());
        let messages = reopened.derive_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(
            messages[1],
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::text("hello")],
            }
        );
        assert_eq!(reopened.summary().turns, 1);
    }

    #[test]
    fn repairs_torn_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);

        let path = session.dir.join(EVENTS_FILE);
        let mut file = OpenOptions::new().append(true).open(&path).expect("open");
        file.write_all(b"{\"type\":\"turn_start\"")
            .expect("write torn");

        let reopened = store.open(session.header.id.as_str()).expect("open");
        assert_eq!(reopened.events.len(), session.events.len());
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.ends_with('\n'));
        assert_eq!(text.lines().count(), 5);
    }

    #[test]
    fn lists_sessions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let session = create(&store);
        let summaries = store.list().expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, session.header.id);
    }

    #[test]
    fn exports_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);
        let markdown = store.export_markdown(&session).expect("export");
        assert!(markdown.contains("## User"));
        assert!(markdown.contains("hi"));
        assert!(markdown.contains("## Assistant"));
    }

    #[test]
    fn rejects_sequence_gaps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        let event = SessionEvent::TurnStart {
            seq: 5,
            at: now_rfc3339(),
            turn: TurnId::new(),
        };
        let error = store.append(&mut session, event).expect_err("gap");
        assert_eq!(error.code(), "invalid_input");
    }
}
