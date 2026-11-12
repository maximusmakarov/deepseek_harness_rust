use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};

pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Create or overwrite a UTF-8 text file, creating parent directories."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "content"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "content": {"type": "string"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("content is required".into()))?;
        let path = resolve_path(&ctx.cwd, path);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| ToolError::Failed(format!("create directory: {error}")))?;
        }
        tokio::fs::write(&path, content)
            .await
            .map_err(|error| ToolError::Failed(format!("write {}: {error}", path.display())))?;

        Ok(ToolOutcome {
            content: format!("wrote {} bytes to {}", content.len(), path.display()),
            is_error: false,
            files: vec![path.display().to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn writes_and_creates_parents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = WriteTool
            .execute(
                &ctx(dir.path()),
                json!({"path": "nested/file.txt", "content": "hello"}),
            )
            .await
            .expect("write");
        assert!(
            std::path::Path::new(&outcome.files[0]).ends_with("nested/file.txt"),
            "{:?}",
            outcome.files
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("nested/file.txt")).expect("read"),
            "hello"
        );
    }
}
