#![forbid(unsafe_code)]
//! Compaction: estimate context size and summarize older messages.
//!
//! The session log stays immutable; compaction is a durable event that
//! `derive_messages` applies as a fresh summary followed by recent messages.

use dshr_llm::{ContentBlock, LlmError, LlmProvider, Message, Request, Role, StreamChunk};
use dshr_session::{Session, SessionEvent, now_rfc3339};
use dshr_session_jsonl::SessionStore;
use futures_util::StreamExt;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub threshold_tokens: usize,
    pub keep_last_turns: usize,
    pub max_summary_tokens: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_tokens: 100_000,
            keep_last_turns: 2,
            max_summary_tokens: 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum CompactionError {
    #[error("llm: {0}")]
    Llm(String),

    #[error("session: {0}")]
    Session(String),

    #[error("summarizer returned an empty response")]
    EmptySummary,
}

impl From<LlmError> for CompactionError {
    fn from(error: LlmError) -> Self {
        Self::Llm(error.to_string())
    }
}

/// Rough token estimate: 4 characters per token plus per-block overhead.
#[must_use]
pub fn estimate_tokens(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(|message| {
            4 + message
                .content
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
                        text.chars().count() / 4 + 4
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        (name.len() + input.to_string().len()) / 4 + 8
                    }
                    ContentBlock::ToolResult { content, .. } => content.chars().count() / 4 + 8,
                    ContentBlock::Image { .. } | ContentBlock::Audio { .. } => 1000,
                })
                .sum::<usize>()
        })
        .sum()
}

/// Summarizes older messages when the estimate exceeds the threshold.
pub async fn maybe_compact(
    provider: &dyn LlmProvider,
    session: &mut Session,
    store: &SessionStore,
    config: &CompactionConfig,
    cancel: CancellationToken,
) -> Result<bool, CompactionError> {
    let turn_starts: Vec<u64> = session
        .events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::TurnStart { seq, .. } => Some(*seq),
            _ => None,
        })
        .collect();
    if turn_starts.len() <= config.keep_last_turns
        || estimate_tokens(&session.derive_messages()) <= config.threshold_tokens
    {
        return Ok(false);
    }

    let cutoff = turn_starts[turn_starts.len() - config.keep_last_turns] - 1;
    let prefix = session.derive_messages_until(cutoff);
    if prefix.is_empty() {
        return Ok(false);
    }
    let transcript = render_transcript(&prefix);
    let request = Request {
        model: provider.model().to_owned(),
        system: Some(
            "Summarize the coding session for future context. Preserve decisions, files, \
unresolved issues, and exact identifiers; omit pleasantries."
                .to_owned(),
        ),
        messages: vec![Message::user(transcript)],
        max_tokens: config.max_summary_tokens,
        ..Request::default()
    };

    let mut stream = provider.stream(request, cancel).await?;
    let mut summary = String::new();
    while let Some(item) = stream.next().await {
        match item? {
            StreamChunk::TextDelta { text } => summary.push_str(&text),
            StreamChunk::Done { .. } => break,
            _ => {}
        }
    }
    if summary.trim().is_empty() {
        return Err(CompactionError::EmptySummary);
    }

    let seq = session.next_seq();
    store
        .append(
            session,
            SessionEvent::Compaction {
                seq,
                at: now_rfc3339(),
                summary,
                replaced_until_seq: cutoff,
            },
        )
        .map_err(|error| CompactionError::Session(error.to_string()))?;
    Ok(true)
}

fn render_transcript(messages: &[Message]) -> String {
    let mut out = String::new();
    for message in messages {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };
        out.push_str(&format!("### {role}\n"));
        for block in &message.content {
            match block {
                ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
                    out.push_str(text);
                    out.push('\n');
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    out.push_str(&format!("[tool {name}: {input}]\n"));
                }
                ContentBlock::ToolResult { content, .. } => {
                    out.push_str(&format!("[tool result: {content}]\n"));
                }
                ContentBlock::Image { .. } => out.push_str("[image]\n"),
                ContentBlock::Audio { .. } => out.push_str("[audio]\n"),
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dshr_core::TurnId;
    use dshr_llm::Usage;
    use dshr_llm_mock::ScriptedProvider;
    use std::sync::Arc;

    fn session_with_history(store: &SessionStore, pairs: usize) -> Session {
        let mut session = store
            .create("scripted", "test-model", "/tmp")
            .expect("create");
        for index in 0..pairs {
            let turn = TurnId::new();
            let at = now_rfc3339();
            let events = [
                SessionEvent::TurnStart {
                    seq: session.next_seq(),
                    at: at.clone(),
                    turn: turn.clone(),
                },
                SessionEvent::UserMessage {
                    seq: session.next_seq() + 1,
                    at: at.clone(),
                    turn: turn.clone(),
                    content: vec![ContentBlock::text(format!(
                        "question {index} with enough text to count tokens"
                    ))],
                },
                SessionEvent::AssistantMessage {
                    seq: session.next_seq() + 2,
                    at: at.clone(),
                    turn: turn.clone(),
                    content: vec![ContentBlock::text(format!("answer {index}"))],
                    usage: Usage::default(),
                    stop_reason: None,
                },
                SessionEvent::TurnEnd {
                    seq: session.next_seq() + 3,
                    at,
                    turn,
                    reason: dshr_session::TurnEndReason::Completed,
                },
            ];
            for event in events {
                store.append(&mut session, event).expect("append");
            }
        }
        session
    }

    fn summary_provider() -> ScriptedProvider {
        ScriptedProvider::new(
            "test-model",
            vec![vec![
                StreamChunk::TextDelta {
                    text: "SUMMARY".into(),
                },
                StreamChunk::Done {
                    stop_reason: dshr_llm::StopReason::EndTurn,
                },
            ]],
        )
    }

    #[test]
    fn estimates_grow_with_content() {
        let short = vec![Message::user("hi")];
        let long = vec![Message::user("word ".repeat(1000))];
        assert!(estimate_tokens(&long) > estimate_tokens(&short));
    }

    #[tokio::test]
    async fn compacts_long_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = session_with_history(&store, 12);
        let provider = Arc::new(summary_provider());
        let config = CompactionConfig {
            threshold_tokens: 50,
            keep_last_turns: 2,
            max_summary_tokens: 64,
        };

        let did = maybe_compact(
            &*provider,
            &mut session,
            &store,
            &config,
            CancellationToken::new(),
        )
        .await
        .expect("compact");
        assert!(did);
        assert!(
            session
                .events
                .iter()
                .any(|event| matches!(event, SessionEvent::Compaction { .. }))
        );

        let messages = session.derive_messages();
        assert_eq!(messages.len(), 5);
        assert!(format!("{:?}", messages[0]).contains("SUMMARY"));
    }

    #[tokio::test]
    async fn skips_short_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SessionStore::new(dir.path());
        let mut session = session_with_history(&store, 1);
        let provider = Arc::new(summary_provider());
        let config = CompactionConfig::default();
        let did = maybe_compact(
            &*provider,
            &mut session,
            &store,
            &config,
            CancellationToken::new(),
        )
        .await
        .expect("compact");
        assert!(!did);
    }
}
