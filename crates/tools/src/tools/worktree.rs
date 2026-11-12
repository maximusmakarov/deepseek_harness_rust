use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome, resolve_path};

fn git(cwd: &Path, args: &[&str]) -> Result<String, ToolError> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|error| ToolError::Failed(format!("git: {error}")))?;
    if !output.status.success() {
        return Err(ToolError::Failed(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

pub struct WorktreeCreateTool;

#[async_trait]
impl Tool for WorktreeCreateTool {
    fn name(&self) -> &str {
        "worktree_create"
    }

    fn description(&self) -> &str {
        "Create a git worktree under .dshr/worktrees and return its path."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["name"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "minLength": 1},
                "branch": {"type": "string"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name is required".into()))?;
        if !valid_name(name) {
            return Err(ToolError::InvalidArgs(
                "name may contain only ASCII letters, digits, '-' and '_'".into(),
            ));
        }
        let branch = args["branch"]
            .as_str()
            .map_or_else(|| format!("dshr/{name}"), ToOwned::to_owned);
        let target = ctx.cwd.join(".dshr/worktrees").join(name);
        if target.exists() {
            return Err(ToolError::Failed(format!(
                "worktree target already exists: {}",
                target.display()
            )));
        }
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| ToolError::Failed(error.to_string()))?;
        }
        git(
            &ctx.cwd,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                target.to_str().unwrap_or_default(),
            ],
        )?;
        Ok(ToolOutcome {
            content: format!("created worktree {} on branch {branch}", target.display()),
            is_error: false,
            files: vec![target.display().to_string()],
        })
    }
}

pub struct WorktreeListTool;

#[async_trait]
impl Tool for WorktreeListTool {
    fn name(&self) -> &str {
        "worktree_list"
    }

    fn description(&self) -> &str {
        "List git worktrees for the workspace."
    }

    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    async fn execute(&self, ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        let output = git(&ctx.cwd, &["worktree", "list", "--porcelain"])?;
        Ok(ToolOutcome::text(if output.is_empty() {
            "(no worktrees)".to_owned()
        } else {
            output
        }))
    }
}

pub struct WorktreeRemoveTool;

#[async_trait]
impl Tool for WorktreeRemoveTool {
    fn name(&self) -> &str {
        "worktree_remove"
    }

    fn description(&self) -> &str {
        "Remove a git worktree created under .dshr/worktrees."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {"path": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let raw = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
        let path = resolve_path(&ctx.cwd, raw);
        let root = ctx.cwd.join(".dshr/worktrees");
        if !path.starts_with(&root) {
            return Err(ToolError::Denied(
                "only worktrees under .dshr/worktrees may be removed".into(),
            ));
        }
        git(
            &ctx.cwd,
            &["worktree", "remove", path.to_str().unwrap_or_default()],
        )?;
        Ok(ToolOutcome::text(format!(
            "removed worktree {}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;
    use std::path::PathBuf;

    fn repo(dir: &Path) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?}");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "test"]);
        std::fs::write(dir.join("a.txt"), "a").expect("write");
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "init"]);
    }

    #[tokio::test]
    async fn creates_lists_and_removes_worktree() {
        let dir = tempfile::tempdir().expect("tempdir");
        repo(dir.path());
        let ctx = ctx(dir.path());

        let created = WorktreeCreateTool
            .execute(&ctx, json!({"name": "feature"}))
            .await
            .expect("create");
        let path = PathBuf::from(&created.files[0]);
        assert!(path.exists(), "{}", path.display());

        let listed = WorktreeListTool
            .execute(&ctx, json!({}))
            .await
            .expect("list");
        assert!(listed.content.contains("feature"), "{}", listed.content);

        WorktreeRemoveTool
            .execute(&ctx, json!({"path": path.to_str().expect("utf8")}))
            .await
            .expect("remove");
        assert!(!path.exists());
    }
}
