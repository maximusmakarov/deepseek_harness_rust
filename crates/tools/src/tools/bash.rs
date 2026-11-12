use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a shell command in the workspace. Returns exit status, stdout, and stderr."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "additionalProperties": false,
            "properties": {
                "command": {"type": "string", "minLength": 1},
                "timeout_secs": {"type": "integer", "minimum": 1, "maximum": 600}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command is required".into()))?;
        let timeout =
            Duration::from_secs(args["timeout_secs"].as_u64().unwrap_or(60).clamp(1, 600));

        let mut shell = if cfg!(windows) {
            let mut shell = Command::new("cmd");
            shell.arg("/C");
            shell
        } else {
            let mut shell = Command::new("sh");
            shell.arg("-c");
            shell
        };
        shell
            .arg(command)
            .current_dir(&ctx.cwd)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = shell
            .spawn()
            .map_err(|error| ToolError::Failed(format!("spawn shell: {error}")))?;

        let output = tokio::select! {
            _ = ctx.cancel.cancelled() => return Err(ToolError::Cancelled),
            result = tokio::time::timeout(timeout, child.wait_with_output()) => {
                match result {
                    Ok(Ok(output)) => output,
                    Ok(Err(error)) => return Err(ToolError::Failed(format!("wait for command: {error}"))),
                    Err(_) => return Err(ToolError::Timeout),
                }
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);

        let mut content = format!("exit: {code}");
        if !stdout.trim().is_empty() {
            content.push_str("\nstdout:\n");
            content.push_str(stdout.trim_end());
        }
        if !stderr.trim().is_empty() {
            content.push_str("\nstderr:\n");
            content.push_str(stderr.trim_end());
        }

        Ok(ToolOutcome {
            content,
            is_error: !output.status.success(),
            files: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn runs_command_and_captures_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = BashTool
            .execute(&ctx(dir.path()), json!({"command": "printf hello"}))
            .await
            .expect("bash");
        assert!(outcome.content.contains("hello"), "{}", outcome.content);
        assert!(!outcome.is_error);
    }

    #[tokio::test]
    async fn reports_non_zero_exit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = BashTool
            .execute(&ctx(dir.path()), json!({"command": "exit 3"}))
            .await
            .expect("bash");
        assert!(outcome.is_error);
        assert!(outcome.content.contains("exit: 3"));
    }

    #[tokio::test]
    async fn honours_timeout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = BashTool
            .execute(
                &ctx(dir.path()),
                json!({"command": "sleep 5", "timeout_secs": 1}),
            )
            .await
            .expect_err("timeout");
        assert!(matches!(error, ToolError::Timeout));
    }
}
