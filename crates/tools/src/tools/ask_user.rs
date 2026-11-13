use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};

/// Questions are answered by the active interface through `AgentObserver`.
/// Direct execution means no interface attached, so the tool fails closed.
pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &'static str {
        "ask_user"
    }

    fn description(&self) -> &'static str {
        "Ask the user a question and wait for the answer. Use for choices that cannot be inferred."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["question"],
            "additionalProperties": false,
            "properties": {
                "question": {"type": "string", "minLength": 1},
                "options": {
                    "type": "array",
                    "items": {"type": "string", "minLength": 1},
                    "maxItems": 9
                }
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        Err(ToolError::Failed(
            "ask_user is not available in this interface".into(),
        ))
    }
}
