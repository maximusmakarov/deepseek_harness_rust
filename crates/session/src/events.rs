use dshr_core::{SessionId, TurnId};
use dshr_llm::{ContentBlock, StopReason, Usage};
use serde::{Deserialize, Serialize};

/// Current logical format of the event log.
pub const SESSION_FORMAT_VERSION: u32 = 1;

/// Header stored beside the JSONL file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionHeader {
    pub version: u32,
    pub id: SessionId,
    pub created_at: String,
    pub provider: String,
    pub model: String,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEndReason {
    Completed,
    Cancelled,
    Error,
    MaxSteps,
}

/// Append-only, model-visible facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionCreated {
        seq: u64,
        at: String,
        provider: String,
        model: String,
        cwd: String,
    },
    TurnStart {
        seq: u64,
        at: String,
        turn: TurnId,
    },
    UserMessage {
        seq: u64,
        at: String,
        turn: TurnId,
        content: Vec<ContentBlock>,
    },
    AssistantMessage {
        seq: u64,
        at: String,
        turn: TurnId,
        content: Vec<ContentBlock>,
        usage: Usage,
        #[serde(default)]
        stop_reason: Option<StopReason>,
        /// Wall-clock time of the provider call for this step (C5.4b). Optional
        /// so journals written before the field existed still replay.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
    },
    AssistantAttempt {
        seq: u64,
        at: String,
        turn: TurnId,
        error: String,
    },
    ToolResult {
        seq: u64,
        at: String,
        turn: TurnId,
        tool_call_id: String,
        name: String,
        content: String,
        is_error: bool,
    },
    Compaction {
        seq: u64,
        at: String,
        summary: String,
        replaced_until_seq: u64,
    },
    TurnEnd {
        seq: u64,
        at: String,
        turn: TurnId,
        reason: TurnEndReason,
    },
}

impl SessionEvent {
    #[must_use]
    pub fn seq(&self) -> u64 {
        match self {
            Self::SessionCreated { seq, .. }
            | Self::TurnStart { seq, .. }
            | Self::UserMessage { seq, .. }
            | Self::AssistantMessage { seq, .. }
            | Self::AssistantAttempt { seq, .. }
            | Self::ToolResult { seq, .. }
            | Self::Compaction { seq, .. }
            | Self::TurnEnd { seq, .. } => *seq,
        }
    }

    #[must_use]
    pub fn at(&self) -> &str {
        match self {
            Self::SessionCreated { at, .. }
            | Self::TurnStart { at, .. }
            | Self::UserMessage { at, .. }
            | Self::AssistantMessage { at, .. }
            | Self::AssistantAttempt { at, .. }
            | Self::ToolResult { at, .. }
            | Self::Compaction { at, .. }
            | Self::TurnEnd { at, .. } => at,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::SessionCreated { .. } => "session_created",
            Self::TurnStart { .. } => "turn_start",
            Self::UserMessage { .. } => "user_message",
            Self::AssistantMessage { .. } => "assistant_message",
            Self::AssistantAttempt { .. } => "assistant_attempt",
            Self::ToolResult { .. } => "tool_result",
            Self::Compaction { .. } => "compaction",
            Self::TurnEnd { .. } => "turn_end",
        }
    }
}

/// RFC 3339 UTC timestamp for event records.
#[must_use]
pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trips_with_type_tag() {
        let event = SessionEvent::UserMessage {
            seq: 2,
            at: "2026-11-12T00:00:00Z".into(),
            turn: TurnId::new(),
            content: vec![ContentBlock::text("hi")],
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains("\"type\":\"user_message\""));
        let back: SessionEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, back);
    }

    #[test]
    fn assistant_latency_is_optional_and_backward_compatible() {
        // A journal written before C5.4b has no `latency_ms` field.
        let legacy = r#"{"type":"assistant_message","seq":4,"at":"2026-11-16T00:00:00Z","turn":"turn_1","content":[],"usage":{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}"#;
        let event: SessionEvent = serde_json::from_str(legacy).expect("legacy event");
        let SessionEvent::AssistantMessage { latency_ms, .. } = &event else {
            panic!("expected assistant message");
        };
        assert_eq!(*latency_ms, None);

        let measured = SessionEvent::AssistantMessage {
            seq: 5,
            at: "2026-11-16T00:00:00Z".into(),
            turn: TurnId::new(),
            content: Vec::new(),
            usage: Usage::default(),
            stop_reason: Some(StopReason::EndTurn),
            latency_ms: Some(1234),
        };
        let json = serde_json::to_string(&measured).expect("serialize");
        assert!(json.contains("\"latency_ms\":1234"), "{json}");
        let back: SessionEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(measured, back);

        let unmeasured = SessionEvent::AssistantMessage {
            seq: 6,
            at: "2026-11-16T00:00:00Z".into(),
            turn: TurnId::new(),
            content: Vec::new(),
            usage: Usage::default(),
            stop_reason: None,
            latency_ms: None,
        };
        let json = serde_json::to_string(&unmeasured).expect("serialize");
        assert!(!json.contains("latency_ms"), "{json}");
    }

    #[test]
    fn timestamp_is_rfc3339() {
        let at = now_rfc3339();
        assert!(at.ends_with('Z'), "{at}");
        assert!(at.contains('T'), "{at}");
        assert!(at.len() >= 20, "{at}");
    }
}
