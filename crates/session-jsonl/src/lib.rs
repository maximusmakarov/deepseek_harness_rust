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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SearchHit {
    pub session_id: String,
    pub seq: u64,
    pub kind: String,
    pub snippet: String,
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
            title: None,
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

    pub fn set_title(&self, id: &str, title: &str) -> Result<Session> {
        let mut session = self.open(id)?;
        session.header.title = Some(title.to_owned());
        write_header(&session.dir, &session.header)?;
        Ok(session)
    }

    /// Creates a new session from the events of `id` up to `at_seq` (inclusive).
    pub fn fork(&self, id: &str, at_seq: Option<u64>) -> Result<Session> {
        let source = self.open(id)?;
        let cutoff = at_seq.unwrap_or(u64::MAX);
        let selected: Vec<SessionEvent> = source
            .events
            .iter()
            .filter(|event| event.seq() <= cutoff)
            .cloned()
            .collect();
        if selected.is_empty() {
            return Err(DshrError::invalid_input("nothing to fork at this point"));
        }

        let new_id = dshr_core::SessionId::new();
        let dir = self.root.join(new_id.as_str());
        fs::create_dir_all(&dir)?;

        let mut events = Vec::with_capacity(selected.len());
        for (index, event) in selected.into_iter().enumerate() {
            let mut value = serde_json::to_value(&event)
                .map_err(|error| DshrError::config(format!("serialize event: {error}")))?;
            value["seq"] = serde_json::json!(index as u64 + 1);
            events.push(
                serde_json::from_value(value)
                    .map_err(|error| DshrError::config(format!("rebuild event: {error}")))?,
            );
        }

        let header = SessionHeader {
            version: SESSION_FORMAT_VERSION,
            id: new_id,
            created_at: now_rfc3339(),
            provider: source.header.provider.clone(),
            model: source.header.model.clone(),
            cwd: source.header.cwd.clone(),
            title: source.header.title.clone(),
        };
        write_header(&dir, &header)?;
        write_events(&dir.join(EVENTS_FILE), &events)?;

        Ok(Session {
            header,
            events,
            dir,
        })
    }

    /// Keeps only the first `keep_turns` turns; the previous log is backed up.
    pub fn rewind(&self, id: &str, keep_turns: usize) -> Result<Session> {
        let session = self.open(id)?;
        let boundary = session
            .events
            .iter()
            .filter_map(|event| match event {
                SessionEvent::TurnStart { seq, .. } => Some(*seq),
                _ => None,
            })
            .nth(keep_turns);
        let kept: Vec<SessionEvent> = match boundary {
            Some(seq) => session
                .events
                .iter()
                .filter(|event| event.seq() < seq)
                .cloned()
                .collect(),
            None => session.events.clone(),
        };

        let path = session.dir.join(EVENTS_FILE);
        if path.exists() {
            fs::copy(&path, path.with_extension("jsonl.bak"))?;
        }
        write_events(&path, &kept)?;
        self.open(id)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        let needle = query.to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let mut hits = Vec::new();
        for summary in self.list()? {
            let Ok(session) = self.open(summary.id.as_str()) else {
                continue;
            };
            for event in &session.events {
                let text = event_text(event);
                if text.is_empty() {
                    continue;
                }
                let lower = text.to_lowercase();
                if let Some(position) = lower.find(&needle) {
                    let chars: Vec<char> = text.chars().collect();
                    let position_chars = text[..position].chars().count();
                    let start = position_chars.saturating_sub(40);
                    let end = (position_chars + needle.chars().count() + 40).min(chars.len());
                    hits.push(SearchHit {
                        session_id: session.header.id.to_string(),
                        seq: event.seq(),
                        kind: event.kind().to_owned(),
                        snippet: chars[start..end]
                            .iter()
                            .collect::<String>()
                            .replace('\n', " "),
                    });
                    if hits.len() >= limit {
                        return Ok(hits);
                    }
                }
            }
        }
        Ok(hits)
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

fn write_events(path: &Path, events: &[SessionEvent]) -> Result<()> {
    let temporary = path.with_extension("jsonl.tmp");
    let mut text = String::new();
    for event in events {
        let line = serde_json::to_string(event)
            .map_err(|error| DshrError::config(format!("serialize event: {error}")))?;
        text.push_str(&line);
        text.push('\n');
    }
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn event_text(event: &SessionEvent) -> String {
    match event {
        SessionEvent::UserMessage { content, .. }
        | SessionEvent::AssistantMessage { content, .. } => content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.as_str()),
                ContentBlock::Thinking { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        SessionEvent::ToolResult { content, .. } => content.clone(),
        SessionEvent::Compaction { summary, .. } => summary.clone(),
        _ => String::new(),
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
            ContentBlock::Audio { .. } => out.push_str("[audio]\n"),
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
    fn forks_at_a_sequence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);

        let fork = store
            .fork(session.header.id.as_str(), Some(5))
            .expect("fork");
        assert_ne!(fork.header.id, session.header.id);
        assert_eq!(fork.events.len(), 5);
        assert!(fork.events.iter().map(SessionEvent::seq).eq(1..=5));
        assert_eq!(fork.derive_messages().len(), 2);
    }

    #[test]
    fn rewinds_to_keep_turns() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);
        round_trip(&store, &mut session);

        let rewound = store.rewind(session.header.id.as_str(), 1).expect("rewind");
        assert_eq!(rewound.summary().turns, 1);
        assert_eq!(rewound.derive_messages().len(), 2);
        assert!(
            session
                .dir
                .join(EVENTS_FILE)
                .with_extension("jsonl.bak")
                .exists()
        );
    }

    #[test]
    fn searches_text_and_sets_title() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = create(&store);
        round_trip(&store, &mut session);

        let hits = store.search("hello", 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("hello"));

        let titled = store
            .set_title(session.header.id.as_str(), "demo")
            .expect("title");
        assert_eq!(titled.header.title.as_deref(), Some("demo"));
        let reopened = store.open(session.header.id.as_str()).expect("open");
        assert_eq!(reopened.header.title.as_deref(), Some("demo"));
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
