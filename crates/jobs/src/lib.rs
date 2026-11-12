#![forbid(unsafe_code)]
//! In-process background jobs.
//!
//! Each job owns a shell child process, a captured output buffer (tail-capped),
//! and a status. Jobs live for the lifetime of the harness process; durable
//! jobs are deferred.

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dshr_core::JobId;
use dshr_sandbox::Sandbox;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("spawn job: {0}")]
    Spawn(String),

    #[error("job '{0}' not found")]
    NotFound(String),

    #[error("job error: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Running,
    Exited(i32),
    Killed,
    Failed(String),
}

impl JobStatus {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Running => "running".to_owned(),
            Self::Exited(0) => "exited(0)".to_owned(),
            Self::Exited(code) => format!("exited({code})"),
            Self::Killed => "killed".to_owned(),
            Self::Failed(error) => format!("failed: {error}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: JobId,
    pub command: String,
    pub status: JobStatus,
    pub output_bytes: usize,
}

struct Job {
    id: JobId,
    command: String,
    status: Arc<Mutex<JobStatus>>,
    output: Arc<Mutex<String>>,
    child: Arc<Mutex<Child>>,
}

pub struct JobManager {
    jobs: Mutex<BTreeMap<String, Arc<Job>>>,
    sandbox: Option<Arc<Sandbox>>,
}

impl Default for JobManager {
    fn default() -> Self {
        Self {
            jobs: Mutex::new(BTreeMap::new()),
            sandbox: None,
        }
    }
}

impl JobManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_sandbox(sandbox: Option<Arc<Sandbox>>) -> Self {
        Self {
            jobs: Mutex::new(BTreeMap::new()),
            sandbox,
        }
    }

    pub async fn start(&self, command: &str, cwd: &std::path::Path) -> Result<JobId, JobError> {
        let (program, args) = if cfg!(windows) {
            ("cmd".to_owned(), vec!["/C".to_owned(), command.to_owned()])
        } else {
            ("sh".to_owned(), vec!["-c".to_owned(), command.to_owned()])
        };
        let (program, args) = if let Some(sandbox) = &self.sandbox {
            let confined = sandbox
                .confine(&program, &args)
                .map_err(|error| JobError::Failed(error.to_string()))?;
            (confined.program, confined.args)
        } else {
            (program, args)
        };

        let mut shell = Command::new(program);
        shell
            .args(args)
            .current_dir(cwd)
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = shell
            .spawn()
            .map_err(|error| JobError::Spawn(error.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| JobError::Spawn("missing stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| JobError::Spawn("missing stderr".into()))?;

        let id = JobId::new();
        let output = Arc::new(Mutex::new(String::new()));
        spawn_reader(stdout, Arc::clone(&output), "");
        spawn_reader(stderr, Arc::clone(&output), "[stderr] ");

        let status = Arc::new(Mutex::new(JobStatus::Running));
        let child = Arc::new(Mutex::new(child));
        {
            let child = Arc::clone(&child);
            let status = Arc::clone(&status);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let mut guard = child.lock().await;
                    match guard.try_wait() {
                        Ok(Some(exit)) => {
                            drop(guard);
                            *status.lock().await = JobStatus::Exited(exit.code().unwrap_or(-1));
                            break;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            drop(guard);
                            *status.lock().await = JobStatus::Failed(error.to_string());
                            break;
                        }
                    }
                }
            });
        }

        let job = Arc::new(Job {
            id: id.clone(),
            command: command.to_owned(),
            status,
            output,
            child,
        });
        self.jobs.lock().await.insert(id.to_string(), job);
        Ok(id)
    }

    pub async fn list(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.lock().await;
        let mut infos = Vec::new();
        for job in jobs.values() {
            infos.push(JobInfo {
                id: job.id.clone(),
                command: job.command.clone(),
                status: job.status.lock().await.clone(),
                output_bytes: job.output.lock().await.len(),
            });
        }
        infos
    }

    pub async fn output(&self, id: &str) -> Result<String, JobError> {
        let job = self.get(id).await?;
        Ok(job.output.lock().await.clone())
    }

    pub async fn status(&self, id: &str) -> Result<JobStatus, JobError> {
        let job = self.get(id).await?;
        Ok(job.status.lock().await.clone())
    }

    pub async fn kill(&self, id: &str) -> Result<(), JobError> {
        let job = self.get(id).await?;
        let mut child = job.child.lock().await;
        child
            .start_kill()
            .map_err(|error| JobError::Failed(error.to_string()))?;
        drop(child);
        *job.status.lock().await = JobStatus::Killed;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Arc<Job>, JobError> {
        self.jobs
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| JobError::NotFound(id.to_owned()))
    }
}

fn spawn_reader<R>(reader: R, output: Arc<Mutex<String>>, prefix: &'static str)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut buffer = output.lock().await;
            buffer.push_str(prefix);
            buffer.push_str(&line);
            buffer.push('\n');
            trim_tail(&mut buffer);
        }
    });
}

fn trim_tail(buffer: &mut String) {
    if buffer.len() <= MAX_OUTPUT_BYTES {
        return;
    }
    let mut cut = buffer.len() - MAX_OUTPUT_BYTES;
    while !buffer.is_char_boundary(cut) {
        cut += 1;
    }
    buffer.drain(..cut);
}

pub fn register_tools(registry: &mut dshr_tools::ToolRegistry, manager: Arc<JobManager>) {
    registry.register_arc(Arc::new(JobStartTool {
        manager: Arc::clone(&manager),
    }));
    registry.register_arc(Arc::new(JobListTool {
        manager: Arc::clone(&manager),
    }));
    registry.register_arc(Arc::new(JobOutputTool {
        manager: Arc::clone(&manager),
    }));
    registry.register_arc(Arc::new(JobKillTool { manager }));
}

pub struct JobStartTool {
    manager: Arc<JobManager>,
}

#[async_trait]
impl Tool for JobStartTool {
    fn name(&self) -> &str {
        "job_start"
    }

    fn description(&self) -> &str {
        "Start a background shell command and return its job id."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "additionalProperties": false,
            "properties": {"command": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("command is required".into()))?;
        let id = self
            .manager
            .start(command, &ctx.cwd)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("started job {id}")))
    }
}

pub struct JobListTool {
    manager: Arc<JobManager>,
}

#[async_trait]
impl Tool for JobListTool {
    fn name(&self) -> &str {
        "job_list"
    }

    fn description(&self) -> &str {
        "List background jobs with their status."
    }

    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": {}})
    }

    async fn execute(&self, _ctx: &ToolCtx, _args: Value) -> Result<ToolOutcome, ToolError> {
        let jobs = self.manager.list().await;
        if jobs.is_empty() {
            return Ok(ToolOutcome::text("(no jobs)"));
        }
        let text = jobs
            .iter()
            .map(|job| {
                format!(
                    "{} {} {} ({} bytes)",
                    job.id,
                    job.status.label(),
                    job.command,
                    job.output_bytes
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolOutcome::text(text))
    }
}

pub struct JobOutputTool {
    manager: Arc<JobManager>,
}

#[async_trait]
impl Tool for JobOutputTool {
    fn name(&self) -> &str {
        "job_output"
    }

    fn description(&self) -> &str {
        "Read captured output of a background job."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["id"],
            "additionalProperties": false,
            "properties": {"id": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let id = args["id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("id is required".into()))?;
        let output = self
            .manager
            .output(id)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        let status = self
            .manager
            .status(id)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!(
            "status: {}\n{}",
            status.label(),
            if output.is_empty() {
                "(no output)"
            } else {
                &output
            }
        )))
    }
}

pub struct JobKillTool {
    manager: Arc<JobManager>,
}

#[async_trait]
impl Tool for JobKillTool {
    fn name(&self) -> &str {
        "job_kill"
    }

    fn description(&self) -> &str {
        "Terminate a running background job."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["id"],
            "additionalProperties": false,
            "properties": {"id": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, _ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let id = args["id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("id is required".into()))?;
        self.manager
            .kill(id)
            .await
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("killed job {id}")))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use dshr_core::SessionId;
    use tokio_util::sync::CancellationToken;

    fn ctx(dir: &std::path::Path) -> ToolCtx {
        ToolCtx {
            cwd: dir.to_path_buf(),
            session_id: SessionId::new(),
            output_limit: 100_000,
            spill_dir: dir.join("spills"),
            cancel: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn starts_and_reads_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = Arc::new(JobManager::new());
        let id = manager
            .start("printf 'hello'", dir.path())
            .await
            .expect("start");
        tokio::time::sleep(Duration::from_millis(600)).await;
        let output = manager.output(id.as_str()).await.expect("output");
        assert!(output.contains("hello"), "{output}");
        assert_eq!(
            manager.status(id.as_str()).await.expect("status"),
            JobStatus::Exited(0)
        );
    }

    #[tokio::test]
    async fn kills_running_job() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = Arc::new(JobManager::new());
        let id = manager.start("sleep 10", dir.path()).await.expect("start");
        manager.kill(id.as_str()).await.expect("kill");
        assert_eq!(
            manager.status(id.as_str()).await.expect("status"),
            JobStatus::Killed
        );
    }

    #[tokio::test]
    async fn tools_register_and_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = Arc::new(JobManager::new());
        let mut registry = dshr_tools::ToolRegistry::new();
        register_tools(&mut registry, Arc::clone(&manager));
        let outcome = registry
            .execute(
                "job_start",
                json!({"command": "printf 'x'"}),
                &ctx(dir.path()),
            )
            .await
            .expect("start");
        assert!(outcome.content.starts_with("started job"));
        let jobs = registry
            .execute("job_list", json!({}), &ctx(dir.path()))
            .await
            .expect("list");
        assert!(jobs.content.contains("job_"), "{}", jobs.content);
    }
}
