use std::sync::Mutex;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Deserialize)]
struct TodoItem {
    content: String,
    status: TodoStatus,
}

#[derive(Default)]
pub struct TodoTool {
    items: Mutex<Vec<TodoItem>>,
}

#[async_trait]
impl Tool for TodoTool {
    fn name(&self) -> &'static str {
        "todo"
    }

    fn description(&self) -> &'static str {
        "Replace the session todo list and render it back."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["items"],
            "additionalProperties": false,
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["content", "status"],
                        "additionalProperties": false,
                        "properties": {
                            "content": {"type": "string", "minLength": 1},
                            "status": {"type": "string", "enum": ["pending", "in_progress", "completed"]}
                        }
                    }
                }
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let items: Vec<TodoItem> = serde_json::from_value(args["items"].clone())
            .map_err(|error| ToolError::InvalidArgs(format!("invalid items: {error}")))?;

        let mut guard = self
            .items
            .lock()
            .map_err(|_| ToolError::Failed("todo lock poisoned".into()))?;
        *guard = items.clone();

        if items.is_empty() {
            return Ok(ToolOutcome::text("(todo list is empty)"));
        }

        let rendered = items
            .iter()
            .map(|item| {
                let marker = match item.status {
                    TodoStatus::Completed => "x",
                    TodoStatus::InProgress => "~",
                    TodoStatus::Pending => " ",
                };
                format!("[{marker}] {}", item.content)
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutcome::text(rendered))
    }
}

pub struct PresentTool;

#[async_trait]
impl Tool for PresentTool {
    fn name(&self) -> &'static str {
        "present"
    }

    fn description(&self) -> &'static str {
        "Mark files as deliverables for the user."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["paths"],
            "additionalProperties": false,
            "properties": {
                "paths": {
                    "type": "array",
                    "minItems": 1,
                    "items": {"type": "string", "minLength": 1}
                },
                "description": {"type": "string"}
            }
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let paths: Vec<String> = serde_json::from_value(args["paths"].clone())
            .map_err(|error| ToolError::InvalidArgs(format!("invalid paths: {error}")))?;
        let description = args["description"].as_str().unwrap_or("deliverables");
        Ok(ToolOutcome {
            content: format!("presented {} file(s): {}", paths.len(), paths.join(", ")),
            is_error: false,
            files: paths,
        })
        .map(|outcome: ToolOutcome| ToolOutcome {
            content: format!("{description}\n{}", outcome.content),
            ..outcome
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn renders_todo_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = TodoTool::default();
        let outcome = tool
            .execute(
                &ctx(dir.path()),
                json!({"items": [
                    {"content": "write tests", "status": "completed"},
                    {"content": "ship", "status": "in_progress"}
                ]}),
            )
            .await
            .expect("todo");
        assert!(outcome.content.contains("[x] write tests"));
        assert!(outcome.content.contains("[~] ship"));
    }

    #[tokio::test]
    async fn rejects_bad_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = TodoTool::default();
        let error = tool
            .execute(
                &ctx(dir.path()),
                json!({"items": [{"content": "x", "status": "done"}]}),
            )
            .await
            .expect_err("status");
        assert!(matches!(error, ToolError::InvalidArgs(_)));
    }
}
