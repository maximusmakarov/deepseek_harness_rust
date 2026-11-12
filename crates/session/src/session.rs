use std::path::PathBuf;

use dshr_core::{SessionId, TurnId};
use dshr_llm::{Message, Role};
use serde::{Deserialize, Serialize};

use crate::events::{SESSION_FORMAT_VERSION, SessionEvent, SessionHeader, TurnEndReason};

#[derive(Debug, Clone)]
pub struct Session {
    pub header: SessionHeader,
    pub events: Vec<SessionEvent>,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    pub created_at: String,
    pub provider: String,
    pub model: String,
    pub turns: usize,
    pub events: usize,
}

impl Session {
    #[must_use]
    pub fn empty(header: SessionHeader, dir: PathBuf) -> Self {
        Self {
            header,
            events: Vec::new(),
            dir,
        }
    }

    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.events.last().map_or(1, |event| event.seq() + 1)
    }

    #[must_use]
    pub fn last_turn(&self) -> Option<TurnId> {
        self.events.iter().rev().find_map(|event| match event {
            SessionEvent::TurnStart { turn, .. } => Some(turn.clone()),
            _ => None,
        })
    }

    #[must_use]
    pub fn turn_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, SessionEvent::TurnStart { .. }))
            .count()
    }

    /// Projects the durable log into the exact history the model receives.
    #[must_use]
    pub fn derive_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in &self.events {
            match event {
                SessionEvent::UserMessage { content, .. } if !content.is_empty() => {
                    messages.push(Message {
                        role: Role::User,
                        content: content.clone(),
                    });
                }
                SessionEvent::AssistantMessage { content, .. } if !content.is_empty() => {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: content.clone(),
                    });
                }
                SessionEvent::ToolResult {
                    tool_call_id,
                    content,
                    is_error,
                    ..
                } => {
                    let block = dshr_llm::ContentBlock::ToolResult {
                        tool_use_id: tool_call_id.clone(),
                        content: content.clone(),
                        is_error: *is_error,
                    };
                    match messages.last_mut() {
                        Some(Message {
                            role: Role::User,
                            content: blocks,
                        }) if blocks.iter().any(|block| {
                            matches!(block, dshr_llm::ContentBlock::ToolResult { .. })
                        }) =>
                        {
                            blocks.push(block);
                        }
                        _ => messages.push(Message {
                            role: Role::User,
                            content: vec![block],
                        }),
                    }
                }
                _ => {}
            }
        }
        messages
    }

    #[must_use]
    pub fn summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.header.id.clone(),
            created_at: self.header.created_at.clone(),
            provider: self.header.provider.clone(),
            model: self.header.model.clone(),
            turns: self.turn_count(),
            events: self.events.len(),
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.events
            .last()
            .is_some_and(|event| matches!(event, SessionEvent::TurnEnd { .. }))
    }

    #[must_use]
    pub fn format_version(&self) -> u32 {
        SESSION_FORMAT_VERSION
    }

    #[must_use]
    pub fn ended_reason(&self) -> Option<TurnEndReason> {
        self.events.iter().rev().find_map(|event| match event {
            SessionEvent::TurnEnd { reason, .. } => Some(*reason),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::now_rfc3339;
    use dshr_llm::ContentBlock;

    fn session() -> Session {
        let header = SessionHeader {
            version: SESSION_FORMAT_VERSION,
            id: SessionId::new(),
            created_at: now_rfc3339(),
            provider: "deepseek-account".into(),
            model: "deepseek-flash".into(),
            cwd: "/tmp".into(),
        };
        Session::empty(header, PathBuf::from("/tmp/session"))
    }

    #[test]
    fn derives_only_model_visible_messages() {
        let mut session = session();
        let turn = TurnId::new();
        session.events.push(SessionEvent::TurnStart {
            seq: 1,
            at: now_rfc3339(),
            turn: turn.clone(),
        });
        session.events.push(SessionEvent::UserMessage {
            seq: 2,
            at: now_rfc3339(),
            turn: turn.clone(),
            content: vec![ContentBlock::text("hi")],
        });
        session.events.push(SessionEvent::AssistantAttempt {
            seq: 3,
            at: now_rfc3339(),
            turn: turn.clone(),
            error: "retry".into(),
        });
        session.events.push(SessionEvent::AssistantMessage {
            seq: 4,
            at: now_rfc3339(),
            turn,
            content: vec![ContentBlock::text("hello")],
            usage: dshr_llm::Usage::default(),
            stop_reason: None,
        });

        let messages = session.derive_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn groups_tool_results_into_one_user_message() {
        use dshr_llm::ContentBlock;
        let mut session = session();
        let turn = TurnId::new();
        session.events.push(SessionEvent::AssistantMessage {
            seq: 1,
            at: now_rfc3339(),
            turn: turn.clone(),
            content: vec![ContentBlock::ToolUse {
                id: "call_1".into(),
                name: "read".into(),
                input: serde_json::json!({"path": "a"}),
            }],
            usage: dshr_llm::Usage::default(),
            stop_reason: None,
        });
        for (seq, id) in [(2, "call_1"), (3, "call_2")] {
            session.events.push(SessionEvent::ToolResult {
                seq,
                at: now_rfc3339(),
                turn: turn.clone(),
                tool_call_id: id.into(),
                name: "read".into(),
                content: "ok".into(),
                is_error: false,
            });
        }

        let messages = session.derive_messages();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(messages[1].role, Role::User);
        assert_eq!(messages[1].content.len(), 2);
        assert!(
            messages[1]
                .content
                .iter()
                .all(|block| matches!(block, ContentBlock::ToolResult { .. }))
        );
    }

    #[test]
    fn next_seq_follows_the_log() {
        let mut session = session();
        assert_eq!(session.next_seq(), 1);
        session.events.push(SessionEvent::TurnStart {
            seq: 7,
            at: now_rfc3339(),
            turn: TurnId::new(),
        });
        assert_eq!(session.next_seq(), 8);
    }
}
