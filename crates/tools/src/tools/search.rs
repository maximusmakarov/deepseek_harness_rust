use async_trait::async_trait;
use globset::{Glob, GlobMatcher};
use ignore::WalkBuilder;
use regex::Regex;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files by glob pattern, respecting .gitignore."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern"],
            "additionalProperties": false,
            "properties": {
                "pattern": {"type": "string", "minLength": 1},
                "path": {"type": "string"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("pattern is required".into()))?;
        let root = args["path"]
            .as_str()
            .map_or_else(|| ctx.cwd.clone(), |path| resolve_path(&ctx.cwd, path));
        let matcher = Glob::new(pattern)
            .map_err(|error| ToolError::InvalidArgs(format!("invalid glob: {error}")))?
            .compile_matcher();

        let mut matches = Vec::new();
        for entry in WalkBuilder::new(&root).hidden(false).build() {
            let entry = entry
                .map_err(|error| ToolError::Failed(format!("walk {}: {error}", root.display())))?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(&root).unwrap_or(path);
            if matcher.is_match(relative) {
                matches.push(relative.display().to_string());
                if matches.len() >= 500 {
                    break;
                }
            }
        }
        matches.sort();

        if matches.is_empty() {
            Ok(ToolOutcome::text("(no matches)".to_owned()))
        } else {
            Ok(ToolOutcome::text(matches.join("\n")))
        }
    }
}

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents with a regular expression."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["pattern"],
            "additionalProperties": false,
            "properties": {
                "pattern": {"type": "string", "minLength": 1},
                "path": {"type": "string"},
                "glob": {"type": "string"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("pattern is required".into()))?;
        let root = args["path"]
            .as_str()
            .map_or_else(|| ctx.cwd.clone(), |path| resolve_path(&ctx.cwd, path));
        let regex = Regex::new(pattern)
            .map_err(|error| ToolError::InvalidArgs(format!("invalid regex: {error}")))?;
        let glob: Option<GlobMatcher> = args["glob"]
            .as_str()
            .map(|pattern| {
                Glob::new(pattern)
                    .map(|glob| glob.compile_matcher())
                    .map_err(|error| ToolError::InvalidArgs(format!("invalid glob: {error}")))
            })
            .transpose()?;

        let mut matches = Vec::new();
        for entry in WalkBuilder::new(&root).hidden(false).build() {
            let entry = entry
                .map_err(|error| ToolError::Failed(format!("walk {}: {error}", root.display())))?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(&root).unwrap_or(path);
            if let Some(glob) = &glob
                && !glob.is_match(relative)
            {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.len() > 1024 * 1024 {
                continue;
            }
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if bytes.iter().take(8192).any(|byte| *byte == 0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            for (index, line) in text.lines().enumerate() {
                if regex.is_match(line) {
                    matches.push(format!(
                        "{}:{}:{}",
                        relative.display(),
                        index + 1,
                        line.trim_end()
                    ));
                    if matches.len() >= 200 {
                        break;
                    }
                }
            }
            if matches.len() >= 200 {
                break;
            }
        }

        if matches.is_empty() {
            Ok(ToolOutcome::text("(no matches)".to_owned()))
        } else {
            Ok(ToolOutcome::text(matches.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    fn tree(dir: &std::path::Path) {
        std::fs::create_dir_all(dir.join("src")).expect("mkdir");
        std::fs::write(dir.join("README.md"), "hello world").expect("write");
        std::fs::write(dir.join("src/lib.rs"), "pub fn hello() {}\n").expect("write");
    }

    #[tokio::test]
    async fn glob_finds_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        tree(dir.path());
        let outcome = GlobTool
            .execute(&ctx(dir.path()), json!({"pattern": "**/*.rs"}))
            .await
            .expect("glob");
        assert!(
            outcome.content.contains("src/lib.rs"),
            "{}",
            outcome.content
        );
        assert!(!outcome.content.contains("README"));
    }

    #[tokio::test]
    async fn grep_finds_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        tree(dir.path());
        let outcome = GrepTool
            .execute(
                &ctx(dir.path()),
                json!({"pattern": "hello", "glob": "**/*.rs"}),
            )
            .await
            .expect("grep");
        assert!(
            outcome.content.contains("src/lib.rs:1"),
            "{}",
            outcome.content
        );
    }

    #[tokio::test]
    async fn rejects_bad_regex() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = GrepTool
            .execute(&ctx(dir.path()), json!({"pattern": "("}))
            .await
            .expect_err("regex");
        assert!(error.to_string().contains("invalid regex"));
    }
}
