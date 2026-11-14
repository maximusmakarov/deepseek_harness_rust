//! Live gate for ROADMAP 3.6 / G3: a 100k-token session must continue after
//! compaction without losing the last turns, and must survive a reopen.
//!
//! The gate is "live" in the sense that it runs the real journal on disk and
//! reopens it; the model reply is replayed, so the test stays offline.

use dshr_compaction::{CompactionConfig, estimate_tokens, maybe_compact};
use dshr_core::TurnId;
use dshr_llm::{ContentBlock, Message, StopReason, StreamChunk, Usage};
use dshr_llm_mock::{Cassette, ReplayProvider};
use dshr_session::{Session, SessionEvent, now_rfc3339};
use dshr_session_jsonl::SessionStore;
use tokio_util::sync::CancellationToken;

const TURNS: usize = 30;
const SUMMARY: &str = "SUMMARY: the session fixed the compaction gate.";

fn summary_provider() -> ReplayProvider {
    ReplayProvider::new(Cassette::new(
        "deepseek-flash",
        vec![
            StreamChunk::Start {
                model: "deepseek-flash".into(),
            },
            StreamChunk::TextDelta {
                text: SUMMARY.to_owned(),
            },
            StreamChunk::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ))
}

fn assistant_text(index: usize) -> String {
    format!(
        "turn {index} analysis: {}",
        "the harness keeps the journal authoritative. ".repeat(360)
    )
}

fn user_text(index: usize) -> String {
    format!("turn {index}: continue the migration {index}")
}

fn build_session(store: &SessionStore) -> Session {
    let mut session = store
        .create("deepseek-account", "deepseek-flash", "/workspace")
        .expect("create session");
    for index in 0..TURNS {
        let turn = TurnId::new();
        let seq = session.next_seq();
        store
            .append(
                &mut session,
                SessionEvent::TurnStart {
                    seq,
                    at: now_rfc3339(),
                    turn: turn.clone(),
                },
            )
            .expect("turn start");
        let seq = session.next_seq();
        store
            .append(
                &mut session,
                SessionEvent::UserMessage {
                    seq,
                    at: now_rfc3339(),
                    turn: turn.clone(),
                    content: vec![ContentBlock::text(user_text(index))],
                },
            )
            .expect("user message");
        let seq = session.next_seq();
        store
            .append(
                &mut session,
                SessionEvent::AssistantMessage {
                    seq,
                    at: now_rfc3339(),
                    turn: turn.clone(),
                    content: vec![ContentBlock::text(assistant_text(index))],
                    usage: Usage::default(),
                    stop_reason: Some(StopReason::EndTurn),
                },
            )
            .expect("assistant message");
        let seq = session.next_seq();
        store
            .append(
                &mut session,
                SessionEvent::TurnEnd {
                    seq,
                    at: now_rfc3339(),
                    turn,
                    reason: dshr_session::TurnEndReason::Completed,
                },
            )
            .expect("turn end");
    }
    session
}

#[tokio::test]
async fn hundred_k_session_continues_after_compaction() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SessionStore::new(dir.path());
    let mut session = build_session(&store);
    let id = session.header.id.as_str().to_owned();

    let before = estimate_tokens(&session.derive_messages());
    assert!(
        before > 100_000,
        "fixture must exceed the compaction threshold, got {before} tokens"
    );

    let config = CompactionConfig {
        threshold_tokens: 100_000,
        keep_last_turns: 2,
        max_summary_tokens: 128,
    };
    let provider = summary_provider();
    let did = maybe_compact(
        &provider,
        &mut session,
        &store,
        &config,
        CancellationToken::new(),
    )
    .await
    .expect("compact");
    assert!(did, "100k session must be compacted");

    let compaction = session
        .events
        .iter()
        .find_map(|event| match event {
            SessionEvent::Compaction {
                summary,
                replaced_until_seq,
                ..
            } => Some((summary.clone(), *replaced_until_seq)),
            _ => None,
        })
        .expect("compaction event");
    assert!(compaction.0.contains("SUMMARY"), "summary is stored");

    let messages = session.derive_messages();
    assert_eq!(messages.len(), 5, "summary + two kept turns");
    assert!(
        message_text(&messages[0]).contains("SUMMARY"),
        "first message is the summary"
    );
    // The last kept turns are preserved verbatim: the exact tail survives.
    assert_eq!(
        message_text(messages.last().expect("last")),
        assistant_text(TURNS - 1),
        "the newest assistant message must survive untouched"
    );
    assert_eq!(
        message_text(&messages[3]),
        user_text(TURNS - 1),
        "the newest user message must survive untouched"
    );
    let after = estimate_tokens(&messages);
    assert!(
        after < 20_000,
        "compacted context must drop well below the threshold, got {after}"
    );

    // A second run is a no-op: compaction is durable and idempotent.
    let did_again = maybe_compact(
        &provider,
        &mut session,
        &store,
        &config,
        CancellationToken::new(),
    )
    .await
    .expect("second compact");
    assert!(!did_again, "compaction must not repeat below threshold");

    // The live part: reopen the journal from disk and continue the session.
    let mut reopened = store.open(&id).expect("reopen");
    let resumed = reopened.derive_messages();
    assert_eq!(resumed.len(), 5, "reopen preserves the compacted shape");
    assert!(
        message_text(resumed.last().expect("last")).contains("turn 29"),
        "the tail still mentions the newest turn"
    );

    let turn = TurnId::new();
    let seq = reopened.next_seq();
    store
        .append(
            &mut reopened,
            SessionEvent::UserMessage {
                seq,
                at: now_rfc3339(),
                turn: turn.clone(),
                content: vec![ContentBlock::text("turn 30: keep going")],
            },
        )
        .expect("append after compaction");
    let continued = reopened.derive_messages();
    assert_eq!(continued.len(), 6, "the session continues after compaction");
    assert_eq!(
        message_text(continued.last().expect("last")),
        "turn 30: keep going"
    );
    assert!(
        estimate_tokens(&continued) < config.threshold_tokens,
        "the continued session stays under the threshold"
    );
}

fn message_text(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
