use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};

/// Applies the `*** Begin Patch` format used by DSH/Codex.
pub struct ApplyPatchTool;

#[derive(Debug)]
enum PatchOp {
    Add {
        path: String,
        lines: Vec<String>,
    },
    Delete {
        path: String,
    },
    Update {
        path: String,
        move_to: Option<String>,
        hunks: Vec<Hunk>,
    },
}

#[derive(Debug, Default)]
struct Hunk {
    old: Vec<String>,
    new: Vec<String>,
}

impl Hunk {
    fn is_empty(&self) -> bool {
        self.old.is_empty() && self.new.is_empty()
    }
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &'static str {
        "apply_patch"
    }

    fn description(&self) -> &'static str {
        "Apply a multi-file patch in the `*** Begin Patch` format (Add/Update/Delete/Move File)."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["patch"],
            "additionalProperties": false,
            "properties": {
                "patch": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Patch text starting with *** Begin Patch and ending with *** End Patch"
                }
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let patch = args["patch"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("patch is required".into()))?;
        let operations = parse_patch(patch)?;
        if operations.is_empty() {
            return Err(ToolError::InvalidArgs(
                "patch contains no file operations".into(),
            ));
        }
        let mut files = Vec::new();
        let mut applied = 0usize;
        for operation in operations {
            match operation {
                PatchOp::Add { path, lines } => {
                    let target = safe_path(&ctx.cwd, &path)?;
                    if target.exists() {
                        return Err(ToolError::Failed(format!(
                            "add file {}: already exists",
                            target.display()
                        )));
                    }
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| {
                            ToolError::Failed(format!("create {}: {error}", parent.display()))
                        })?;
                    }
                    std::fs::write(&target, join_lines(&lines, true)).map_err(|error| {
                        ToolError::Failed(format!("write {}: {error}", target.display()))
                    })?;
                    files.push(target.display().to_string());
                    applied += 1;
                }
                PatchOp::Delete { path } => {
                    let target = safe_path(&ctx.cwd, &path)?;
                    std::fs::remove_file(&target).map_err(|error| {
                        ToolError::Failed(format!("delete {}: {error}", target.display()))
                    })?;
                    files.push(target.display().to_string());
                    applied += 1;
                }
                PatchOp::Update {
                    path,
                    move_to,
                    hunks,
                } => {
                    let target = safe_path(&ctx.cwd, &path)?;
                    let original = std::fs::read_to_string(&target).map_err(|error| {
                        ToolError::Failed(format!("read {}: {error}", target.display()))
                    })?;
                    let updated = apply_hunks(&original, &hunks, &target)?;
                    let destination = match move_to {
                        Some(destination) => {
                            let destination = safe_path(&ctx.cwd, &destination)?;
                            if let Some(parent) = destination.parent() {
                                std::fs::create_dir_all(parent).map_err(|error| {
                                    ToolError::Failed(format!(
                                        "create {}: {error}",
                                        parent.display()
                                    ))
                                })?;
                            }
                            std::fs::write(&destination, updated).map_err(|error| {
                                ToolError::Failed(format!(
                                    "write {}: {error}",
                                    destination.display()
                                ))
                            })?;
                            std::fs::remove_file(&target).map_err(|error| {
                                ToolError::Failed(format!("remove {}: {error}", target.display()))
                            })?;
                            destination
                        }
                        None => {
                            std::fs::write(&target, updated).map_err(|error| {
                                ToolError::Failed(format!("write {}: {error}", target.display()))
                            })?;
                            target
                        }
                    };
                    files.push(destination.display().to_string());
                    applied += 1;
                }
            }
        }
        Ok(ToolOutcome {
            content: format!("applied {applied} file operation(s): {}", files.join(", ")),
            is_error: false,
            files,
        })
    }
}

fn parse_patch(text: &str) -> Result<Vec<PatchOp>, ToolError> {
    let mut lines: Vec<&str> = text.lines().collect();
    // Trim leading blank lines and the Begin marker.
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.first().map(|line| line.trim()) != Some("*** Begin Patch") {
        return Err(ToolError::InvalidArgs(
            "patch must start with *** Begin Patch".into(),
        ));
    }
    lines.remove(0);
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.last().map(|line| line.trim()) != Some("*** End Patch") {
        return Err(ToolError::InvalidArgs(
            "patch must end with *** End Patch".into(),
        ));
    }
    lines.pop();

    // Group lines into file sections by their header.
    let mut sections: Vec<(String, String, Vec<&str>)> = Vec::new();
    for line in lines {
        if let Some(path) = line.strip_prefix("*** Add File: ") {
            sections.push(("add".to_owned(), path.trim().to_owned(), Vec::new()));
        } else if let Some(path) = line.strip_prefix("*** Delete File: ") {
            sections.push(("delete".to_owned(), path.trim().to_owned(), Vec::new()));
        } else if let Some(path) = line.strip_prefix("*** Update File: ") {
            sections.push(("update".to_owned(), path.trim().to_owned(), Vec::new()));
        } else if line.trim() == "*** End Patch" {
            continue;
        } else if line.starts_with("*** Move to: ") {
            if let Some((kind, _, body)) = sections.last_mut()
                && kind == "update"
            {
                body.push(line);
            } else {
                return Err(ToolError::InvalidArgs(
                    "*** Move to: outside an update section".into(),
                ));
            }
        } else if line.starts_with("*** ") {
            return Err(ToolError::InvalidArgs(format!(
                "unexpected patch marker: {}",
                line.trim()
            )));
        } else if let Some((_, _, body)) = sections.last_mut() {
            body.push(line);
        } else if !line.trim().is_empty() {
            return Err(ToolError::InvalidArgs(format!(
                "patch line outside a file section: {line}"
            )));
        }
    }

    let mut operations = Vec::new();
    for (kind, path, body) in sections {
        match kind.as_str() {
            "add" => {
                let mut content = Vec::new();
                for line in body {
                    let Some(value) = line.strip_prefix('+') else {
                        return Err(ToolError::InvalidArgs(format!(
                            "add file {path}: every line must start with '+'"
                        )));
                    };
                    content.push(value.to_owned());
                }
                operations.push(PatchOp::Add {
                    path,
                    lines: content,
                });
            }
            "delete" => operations.push(PatchOp::Delete { path }),
            "update" => {
                let mut move_to = None;
                let mut hunks: Vec<Hunk> = Vec::new();
                let mut current = Hunk::default();
                for line in body {
                    if let Some(destination) = line.strip_prefix("*** Move to: ") {
                        move_to = Some(destination.trim().to_owned());
                        continue;
                    }
                    if line.starts_with("@@") {
                        if !current.is_empty() {
                            hunks.push(std::mem::take(&mut current));
                        }
                        continue;
                    }
                    if line == "\\ No newline at end of file" {
                        continue;
                    }
                    if let Some(value) = line.strip_prefix(' ') {
                        current.old.push(value.to_owned());
                        current.new.push(value.to_owned());
                    } else if let Some(value) = line.strip_prefix('-') {
                        current.old.push(value.to_owned());
                    } else if let Some(value) = line.strip_prefix('+') {
                        current.new.push(value.to_owned());
                    } else if line.is_empty() {
                        current.old.push(String::new());
                        current.new.push(String::new());
                    } else {
                        return Err(ToolError::InvalidArgs(format!(
                            "invalid patch line in {path}: {line}"
                        )));
                    }
                }
                if !current.is_empty() {
                    hunks.push(current);
                }
                operations.push(PatchOp::Update {
                    path,
                    move_to,
                    hunks,
                });
            }
            _ => unreachable!("section kind"),
        }
    }
    Ok(operations)
}

fn safe_path(cwd: &Path, value: &str) -> Result<PathBuf, ToolError> {
    if value.trim().is_empty() {
        return Err(ToolError::InvalidArgs("patch path is empty".into()));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ToolError::InvalidArgs(format!(
            "patch path must stay inside the workspace: {value}"
        )));
    }
    Ok(cwd.join(path))
}

fn apply_hunks(original: &str, hunks: &[Hunk], path: &Path) -> Result<String, ToolError> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let trailing = original.ends_with(newline);
    let mut lines: Vec<String> = if original.is_empty() {
        Vec::new()
    } else {
        original
            .split(newline)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };
    if trailing {
        lines.pop();
    }
    let mut cursor = 0usize;
    for (index, hunk) in hunks.iter().enumerate() {
        if hunk.old.is_empty() {
            let at = cursor.min(lines.len());
            lines.splice(at..at, hunk.new.iter().cloned());
            cursor = at + hunk.new.len();
            continue;
        }
        let end = lines.len().saturating_sub(hunk.old.len());
        let found =
            (cursor..=end).find(|&start| lines[start..start + hunk.old.len()] == hunk.old[..]);
        let Some(start) = found else {
            return Err(ToolError::Failed(format!(
                "update {}: hunk {} context not found",
                path.display(),
                index + 1
            )));
        };
        let removed = hunk.old.len();
        lines.splice(start..start + removed, hunk.new.iter().cloned());
        cursor = start + hunk.new.len();
    }
    let mut output = lines.join(newline);
    if trailing || !output.is_empty() {
        output.push_str(newline);
    }
    Ok(output)
}

fn join_lines(lines: &[String], trailing: bool) -> String {
    let mut output = lines.join("\n");
    if trailing {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn adds_updates_and_deletes_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("keep.txt"), "one\ntwo\nthree\n").expect("write");
        let patch = "*** Begin Patch\n*** Add File: nested/new.txt\n+hello\n+world\n*** Update File: keep.txt\n@@\n one\n-two\n+TWO\n three\n*** Delete File: keep.txt\n*** End Patch";
        // Delete and update the same file in one patch is odd; split instead.
        let patch = patch.replace("*** Delete File: keep.txt\n", "");
        ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": patch}))
            .await
            .expect("apply");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("nested/new.txt")).expect("new"),
            "hello\nworld\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("keep.txt")).expect("keep"),
            "one\nTWO\nthree\n"
        );

        let delete = "*** Begin Patch\n*** Delete File: keep.txt\n*** End Patch";
        ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": delete}))
            .await
            .expect("delete");
        assert!(!dir.path().join("keep.txt").exists());
    }

    #[tokio::test]
    async fn moves_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("old.txt"), "content\n").expect("write");
        let patch = "*** Begin Patch\n*** Update File: old.txt\n*** Move to: dir/new.txt\n@@\n-content\n+changed\n*** End Patch";
        ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": patch}))
            .await
            .expect("move");
        assert!(!dir.path().join("old.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("dir/new.txt")).expect("new"),
            "changed\n"
        );
    }

    #[tokio::test]
    async fn preserves_crlf_line_endings() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("win.txt"), "a\r\nb\r\n").expect("write");
        let patch = "*** Begin Patch\n*** Update File: win.txt\n@@\n a\n-b\n+c\n*** End Patch";
        ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": patch}))
            .await
            .expect("apply");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("win.txt")).expect("read"),
            "a\r\nc\r\n"
        );
    }

    #[tokio::test]
    async fn rejects_missing_context_and_escaping_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write");
        let missing = "*** Begin Patch\n*** Update File: a.txt\n@@\n-nope\n+x\n*** End Patch";
        let error = ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": missing}))
            .await
            .expect_err("missing context");
        assert!(error.to_string().contains("context not found"), "{error}");

        let escape = "*** Begin Patch\n*** Add File: ../evil.txt\n+x\n*** End Patch";
        let error = ApplyPatchTool
            .execute(&ctx(dir.path()), json!({"patch": escape}))
            .await
            .expect_err("escape");
        assert!(
            error.to_string().contains("inside the workspace"),
            "{error}"
        );
    }
}
