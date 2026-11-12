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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEndReason {
    Completed,
    Cancelled,
    Error,
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
    },
    AssistantAttempt {
        seq: u64,
        at: String,
        turn: TurnId,
        error: String,
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
    fn timestamp_is_rfc3339() {
        let at = now_rfc3339();
        assert!(at.ends_with('Z'), "{at}");
        assert!(at.contains('T'), "{at}");
        assert!(at.len() >= 20, "{at}");
    }
}
