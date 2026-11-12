use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};

pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a UTF-8 text file. Returns numbered lines; rejects binary files."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "offset": {"type": "integer", "minimum": 1},
                "limit": {"type": "integer", "minimum": 1, "maximum": 5000}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let offset = args["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(2000).max(1) as usize;
        let path = resolve_path(&ctx.cwd, path);

        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| ToolError::Failed(format!("read {}: {error}", path.display())))?;
        if metadata.len() > 5 * 1024 * 1024 {
            return Err(ToolError::Failed("file is larger than 5 MiB".into()));
        }

        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|error| ToolError::Failed(format!("read {}: {error}", path.display())))?;
        if bytes.iter().take(8192).any(|byte| *byte == 0) {
            return Err(ToolError::Failed(
                "binary file is not readable as text".into(),
            ));
        }
        let text = String::from_utf8_lossy(&bytes);

        let selected: Vec<String> = text
            .lines()
            .enumerate()
            .skip(offset - 1)
            .take(limit)
            .map(|(index, line)| format!("{:>6}\t{line}", index + 1))
            .collect();

        if selected.is_empty() {
            return Ok(ToolOutcome::text(format!(
                "(no lines at offset {offset} in {})",
                path.display()
            )));
        }

        Ok(ToolOutcome::text(selected.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn reads_numbered_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "one\ntwo\nthree\n").expect("write");
        let outcome = ReadTool
            .execute(
                &ctx(dir.path()),
                json!({"path": "a.txt", "offset": 2, "limit": 1}),
            )
            .await
            .expect("read");
        assert!(outcome.content.contains("2\ttwo"), "{}", outcome.content);
        assert!(!outcome.content.contains("three"));
    }

    #[tokio::test]
    async fn rejects_binary() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("bin"), [0_u8, 1, 2]).expect("write");
        let error = ReadTool
            .execute(&ctx(dir.path()), json!({"path": "bin"}))
            .await
            .expect_err("binary");
        assert!(error.to_string().contains("binary"));
    }
}
