use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};

pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Replace an exact string in a text file. Fails when the match is absent or ambiguous."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "old_string", "new_string"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "old_string": {"type": "string", "minLength": 1},
                "new_string": {"type": "string"},
                "replace_all": {"type": "boolean"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let old_string = args["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("old_string is required".into()))?;
        let new_string = args["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("new_string is required".into()))?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);
        let path = resolve_path(&ctx.cwd, path);

        let text = tokio::fs::read_to_string(&path)
            .await
            .map_err(|error| ToolError::Failed(format!("read {}: {error}", path.display())))?;
        let count = text.matches(old_string).count();
        if count == 0 {
            return Err(ToolError::Failed(format!(
                "old_string not found in {}",
                path.display()
            )));
        }
        if count > 1 && !replace_all {
            return Err(ToolError::Failed(format!(
                "old_string occurs {count} times; set replace_all=true or provide a unique match"
            )));
        }

        let updated = if replace_all {
            text.replace(old_string, new_string)
        } else {
            text.replacen(old_string, new_string, 1)
        };
        tokio::fs::write(&path, updated)
            .await
            .map_err(|error| ToolError::Failed(format!("write {}: {error}", path.display())))?;

        Ok(ToolOutcome {
            content: format!("replaced {count} occurrence(s) in {}", path.display()),
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
    async fn replaces_unique_match() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello world").expect("write");
        EditTool
            .execute(
                &ctx(dir.path()),
                json!({"path": "a.txt", "old_string": "world", "new_string": "there"}),
            )
            .await
            .expect("edit");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).expect("read"),
            "hello there"
        );
    }

    #[tokio::test]
    async fn rejects_ambiguous_match() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "x x").expect("write");
        let error = EditTool
            .execute(
                &ctx(dir.path()),
                json!({"path": "a.txt", "old_string": "x", "new_string": "y"}),
            )
            .await
            .expect_err("ambiguous");
        assert!(error.to_string().contains("2 times"));
    }

    #[tokio::test]
    async fn replace_all_works() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "x x").expect("write");
        EditTool
            .execute(
                &ctx(dir.path()),
                json!({"path": "a.txt", "old_string": "x", "new_string": "y", "replace_all": true}),
            )
            .await
            .expect("edit");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).expect("read"),
            "y y"
        );
    }
}
