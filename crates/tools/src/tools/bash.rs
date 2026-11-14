use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::registry::{Tool, ToolCtx, ToolError, ToolOutcome};
use dshr_sandbox::Sandbox;
use std::sync::Arc;

struct Shell {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[derive(Default)]
pub struct BashTool {
    sandbox: Option<Arc<Sandbox>>,
    shells: Arc<Mutex<HashMap<String, Shell>>>,
}

impl BashTool {
    #[must_use]
    pub fn new(sandbox: Option<Arc<Sandbox>>) -> Self {
        Self {
            sandbox,
            shells: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

const MARKER: &str = "__DSHR_DONE__";

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a shell command in a persistent workspace shell. Returns exit status, stdout, and stderr."
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
        if cfg!(windows) {
            return run_stateless(ctx, command, timeout, self.sandbox.as_ref()).await;
        }
        self.run_persistent(ctx, command, timeout).await
    }
}

impl BashTool {
    async fn run_persistent(
        &self,
        ctx: &ToolCtx,
        command: &str,
        timeout: Duration,
    ) -> Result<ToolOutcome, ToolError> {
        let mut shells = self.shells.lock().await;
        let key = ctx.session_id.as_str().to_owned();
        if !shells.contains_key(&key) {
            shells.insert(key.clone(), spawn_shell(ctx, self.sandbox.as_ref())?);
        }
        let shell = shells.get_mut(&key).expect("shell");
        let encoded = base64::engine::general_purpose::STANDARD.encode(command.as_bytes());
        let lines = format!("eval \"$(printf %s '{encoded}' | base64 -d)\"\n");
        if shell.stdin.write_all(lines.as_bytes()).await.is_err()
            || shell.stdin.flush().await.is_err()
        {
            if let Some(mut broken) = shells.remove(&key) {
                let _ = broken.child.start_kill();
            }
            return Err(ToolError::Failed("shell stdin closed".into()));
        }
        let read = read_until_marker(&mut shell.stdout, ctx);
        let result = tokio::select! {
            _ = ctx.cancel.cancelled() => None,
            result = tokio::time::timeout(timeout, read) => result.ok().and_then(Result::ok),
        };
        match result {
            Some((code, output)) => {
                let mut content = format!("exit: {code}");
                if !output.trim().is_empty() {
                    content.push_str("\nstdout:\n");
                    content.push_str(output.trim_end());
                }
                Ok(ToolOutcome {
                    content,
                    is_error: code != 0,
                    files: Vec::new(),
                })
            }
            None => {
                if let Some(mut shell) = shells.remove(&key) {
                    let _ = shell.child.start_kill();
                }
                if ctx.cancel.is_cancelled() {
                    Err(ToolError::Cancelled)
                } else {
                    Err(ToolError::Timeout)
                }
            }
        }
    }
}

fn spawn_shell(ctx: &ToolCtx, sandbox: Option<&Arc<Sandbox>>) -> Result<Shell, ToolError> {
    let script = "trap 'printf \"\\n__DSHR_DONE__%s\\n\" \"$?\"' EXIT\nexec 2>&1\nwhile IFS= read -r line; do eval \"$line\"; printf '\\n__DSHR_DONE__%s\\n' \"$?\"; done";
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    if let Some(sandbox) = sandbox {
        let confined = sandbox
            .confine("sh", &["-c".to_owned(), script.to_owned()])
            .map_err(|error| ToolError::Denied(error.to_string()))?;
        command = Command::new(confined.program);
        command.args(confined.args);
    }
    let mut child = command
        .current_dir(&ctx.cwd)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| ToolError::Failed(format!("spawn shell: {error}")))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ToolError::Failed("shell stdin missing".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ToolError::Failed("shell stdout missing".into()))?;
    Ok(Shell {
        child,
        stdin,
        stdout: BufReader::new(stdout),
    })
}

async fn read_until_marker(
    stdout: &mut BufReader<ChildStdout>,
    _ctx: &ToolCtx,
) -> Result<(i32, String), ToolError> {
    let mut output = String::new();
    loop {
        let mut line = String::new();
        let read = stdout
            .read_line(&mut line)
            .await
            .map_err(|error| ToolError::Failed(format!("read shell output: {error}")))?;
        if read == 0 {
            return Err(ToolError::Failed("shell exited".into()));
        }
        if let Some(position) = line.find(MARKER) {
            let code = line[position + MARKER.len()..].trim().parse().unwrap_or(-1);
            output.push_str(&line[..position]);
            return Ok((code, output));
        }
        output.push_str(&line);
        if output.len() > 1_000_000 {
            return Err(ToolError::Failed("shell output too large".into()));
        }
    }
}

async fn run_stateless(
    ctx: &ToolCtx,
    command: &str,
    timeout: Duration,
    sandbox: Option<&Arc<Sandbox>>,
) -> Result<ToolOutcome, ToolError> {
    let (program, args) = if cfg!(windows) {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_owned()),
            vec!["/C".to_owned(), command.to_owned()],
        )
    } else {
        ("sh".to_owned(), vec!["-c".to_owned(), command.to_owned()])
    };
    let (program, args) = if let Some(sandbox) = sandbox {
        let confined = sandbox
            .confine(&program, &args)
            .map_err(|error| ToolError::Denied(error.to_string()))?;
        (confined.program, confined.args)
    } else {
        (program, args)
    };
    let mut shell = Command::new(program);
    shell
        .args(args)
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::ctx;

    #[tokio::test]
    async fn persists_cwd_and_env_between_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = BashTool::default();
        let ctx = ctx(dir.path());
        tool.execute(
            &ctx,
            json!({"command": "cd /tmp && export DSHR_PERSIST=yes"}),
        )
        .await
        .expect("cd");
        let outcome = tool
            .execute(
                &ctx,
                json!({"command": "pwd; printf ' %s' \"$DSHR_PERSIST\""}),
            )
            .await
            .expect("pwd");
        assert!(outcome.content.contains("/tmp"), "{}", outcome.content);
        assert!(outcome.content.contains("yes"), "{}", outcome.content);
    }

    #[tokio::test]
    async fn keeps_sessions_isolated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tool = BashTool::default();
        let first = ctx(dir.path());
        let second = ctx(dir.path());
        tool.execute(&first, json!({"command": "cd /tmp"}))
            .await
            .expect("cd");
        let outcome = tool
            .execute(&second, json!({"command": "pwd"}))
            .await
            .expect("pwd");
        assert!(!outcome.content.contains("\n/tmp"), "{}", outcome.content);
    }

    #[tokio::test]
    async fn runs_command_and_captures_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = BashTool::default()
            .execute(&ctx(dir.path()), json!({"command": "printf hello"}))
            .await
            .expect("bash");
        assert!(outcome.content.contains("hello"), "{}", outcome.content);
        assert!(!outcome.is_error);
    }

    #[tokio::test]
    async fn reports_non_zero_exit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outcome = BashTool::default()
            .execute(&ctx(dir.path()), json!({"command": "exit 3"}))
            .await
            .expect("bash");
        assert!(outcome.is_error);
        assert!(outcome.content.contains("exit: 3"));
    }

    #[tokio::test]
    async fn honours_timeout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = BashTool::default()
            .execute(
                &ctx(dir.path()),
                json!({"command": "sleep 5", "timeout_secs": 1}),
            )
            .await
            .expect_err("timeout");
        assert!(matches!(error, ToolError::Timeout));
    }
}
