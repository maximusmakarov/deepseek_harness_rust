use std::collections::BTreeMap;

use serde_json::Value;

use crate::{ContentBlock, LlmError, StopReason, StreamChunk, Usage};

#[derive(Debug, Clone, PartialEq)]
pub struct AssistantOutput {
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
    pub stop_reason: Option<StopReason>,
}

#[derive(Debug, Default)]
struct ToolAccum {
    id: String,
    name: String,
    json: String,
}

/// Assembles a normalized chunk stream into one assistant message.
#[derive(Debug, Default)]
pub struct BlockAssembler {
    thinking: String,
    text: String,
    tools: BTreeMap<u32, ToolAccum>,
    usage: Usage,
    stop_reason: Option<StopReason>,
}

impl BlockAssembler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: StreamChunk) {
        match chunk {
            StreamChunk::Start { .. } => {}
            StreamChunk::TextDelta { text } => self.text.push_str(&text),
            StreamChunk::ThinkingDelta { text } => self.thinking.push_str(&text),
            StreamChunk::ToolUseStart { index, id, name } => {
                self.tools.insert(
                    index,
                    ToolAccum {
                        id,
                        name,
                        json: String::new(),
                    },
                );
            }
            StreamChunk::ToolInputDelta { index, json } => {
                if let Some(tool) = self.tools.get_mut(&index) {
                    tool.json.push_str(&json);
                }
            }
            StreamChunk::Usage { usage } => self.usage.merge(usage),
            StreamChunk::Done { stop_reason } => self.stop_reason = Some(stop_reason),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn finish(self) -> Result<AssistantOutput, LlmError> {
        let mut content = Vec::new();

        if !self.thinking.is_empty() {
            content.push(ContentBlock::Thinking {
                text: self.thinking,
            });
        }
        if !self.text.is_empty() {
            content.push(ContentBlock::Text { text: self.text });
        }
        for tool in self.tools.into_values() {
            let input = if tool.json.trim().is_empty() {
                Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_str(&tool.json).map_err(|err| {
                    LlmError::Protocol(format!("invalid tool input for '{}': {err}", tool.name))
                })?
            };
            content.push(ContentBlock::ToolUse {
                id: tool.id,
                name: tool.name,
                input,
            });
        }

        Ok(AssistantOutput {
            content,
            usage: self.usage,
            stop_reason: self.stop_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_text_and_usage() {
        let mut assembler = BlockAssembler::new();
        assembler.push(StreamChunk::Start {
            model: "deepseek-flash".into(),
        });
        assembler.push(StreamChunk::TextDelta {
            text: "pong".into(),
        });
        assembler.push(StreamChunk::Usage {
            usage: Usage {
                input_tokens: 3,
                output_tokens: 1,
                ..Usage::default()
            },
        });
        assembler.push(StreamChunk::Done {
            stop_reason: StopReason::EndTurn,
        });

        let output = assembler.finish().expect("assemble");
        assert_eq!(output.content, vec![ContentBlock::text("pong")]);
        assert_eq!(output.usage.input_tokens, 3);
        assert_eq!(output.stop_reason, Some(StopReason::EndTurn));
    }

    #[test]
    fn keeps_thinking_before_text() {
        let mut assembler = BlockAssembler::new();
        assembler.push(StreamChunk::ThinkingDelta {
            text: "plan".into(),
        });
        assembler.push(StreamChunk::TextDelta {
            text: "answer".into(),
        });

        let output = assembler.finish().expect("assemble");
        assert_eq!(
            output.content,
            vec![
                ContentBlock::Thinking {
                    text: "plan".into()
                },
                ContentBlock::text("answer")
            ]
        );
    }

    #[test]
    fn assembles_tool_input_fragments() {
        let mut assembler = BlockAssembler::new();
        assembler.push(StreamChunk::ToolUseStart {
            index: 0,
            id: "call_1".into(),
            name: "read".into(),
        });
        assembler.push(StreamChunk::ToolInputDelta {
            index: 0,
            json: "{\"path\":".into(),
        });
        assembler.push(StreamChunk::ToolInputDelta {
            index: 0,
            json: "\"Cargo.toml\"}".into(),
        });

        let output = assembler.finish().expect("assemble");
        assert_eq!(
            output.content,
            vec![ContentBlock::ToolUse {
                id: "call_1".into(),
                name: "read".into(),
                input: serde_json::json!({"path": "Cargo.toml"}),
            }]
        );
    }

    #[test]
    fn rejects_broken_tool_json() {
        let mut assembler = BlockAssembler::new();
        assembler.push(StreamChunk::ToolUseStart {
            index: 0,
            id: "call_1".into(),
            name: "read".into(),
        });
        assembler.push(StreamChunk::ToolInputDelta {
            index: 0,
            json: "{not json".into(),
        });

        let error = assembler.finish().expect_err("broken json");
        assert!(matches!(error, LlmError::Protocol(_)));
    }
}
