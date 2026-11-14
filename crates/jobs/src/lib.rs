#![forbid(unsafe_code)]
//! Background jobs with durable status and output.
//!
//! Each job owns a shell child process, a captured output buffer (tail-capped),
//! and a status. With a state directory, metadata and logs survive a restart;
//! jobs that were running when the process died are marked interrupted.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    Running,
    Exited(i32),
    Killed,
    Interrupted,
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
            Self::Interrupted => "interrupted".to_owned(),
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
    child: Option<Arc<Mutex<Child>>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredJob {
    id: String,
    command: String,
    status: JobStatus,
}

pub struct JobManager {
    jobs: Mutex<BTreeMap<String, Arc<Job>>>,
    sandbox: Option<Arc<Sandbox>>,
    state_dir: Option<PathBuf>,
}

impl Default for JobManager {
    fn default() -> Self {
        Self {
            jobs: Mutex::new(BTreeMap::new()),
            sandbox: None,
            state_dir: None,
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
            state_dir: None,
        }
    }

    /// Opens a manager backed by `state_dir`, restoring finished jobs and
    /// marking jobs that were running during a crash as interrupted.
    #[must_use]
    pub fn with_state(sandbox: Option<Arc<Sandbox>>, state_dir: PathBuf) -> Self {
        let jobs = load_jobs(&state_dir);
        Self {
            jobs: Mutex::new(jobs),
            sandbox,
            state_dir: Some(state_dir),
        }
    }

    pub async fn start(&self, command: &str, cwd: &std::path::Path) -> Result<JobId, JobError> {
        let (program, args) = if cfg!(windows) {
            (
                std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_owned()),
                vec!["/C".to_owned(), command.to_owned()],
            )
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
        let log_path = self
            .state_dir
            .as_ref()
            .map(|dir| dir.join(format!("{id}.log")));
        let output = Arc::new(Mutex::new(String::new()));
        spawn_reader(stdout, Arc::clone(&output), "", log_path.clone());
        spawn_reader(stderr, Arc::clone(&output), "[stderr] ", log_path.clone());

        let status = Arc::new(Mutex::new(JobStatus::Running));
        let child = Arc::new(Mutex::new(child));
        {
            let child = Arc::clone(&child);
            let status = Arc::clone(&status);
            let output = Arc::clone(&output);
            let state_dir = self.state_dir.clone();
            let command = command.to_owned();
            let id_text = id.to_string();
            let log_path = log_path.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let mut guard = child.lock().await;
                    let next = match guard.try_wait() {
                        Ok(Some(exit)) => Some(JobStatus::Exited(exit.code().unwrap_or(-1))),
                        Ok(None) => None,
                        Err(error) => Some(JobStatus::Failed(error.to_string())),
                    };
                    drop(guard);
                    let Some(next) = next else { continue };
                    *status.lock().await = next.clone();
                    if let Some(path) = &log_path {
                        let _ = std::fs::write(path, &*output.lock().await);
                    }
                    if let Some(dir) = &state_dir {
                        persist_status(dir, &id_text, &command, &next);
                    }
                    break;
                }
            });
        }

        let job = Arc::new(Job {
            id: id.clone(),
            command: command.to_owned(),
            status,
            output,
            child: Some(child),
        });
        self.jobs.lock().await.insert(id.to_string(), job);
        if let Some(dir) = &self.state_dir {
            persist_status(dir, id.as_str(), command, &JobStatus::Running);
        }
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
        let Some(child) = &job.child else {
            return Err(JobError::Failed(format!("job '{id}' is not running")));
        };
        child
            .lock()
            .await
            .start_kill()
            .map_err(|error| JobError::Failed(error.to_string()))?;
        *job.status.lock().await = JobStatus::Killed;
        if let Some(dir) = &self.state_dir {
            persist_status(dir, &job.id.to_string(), &job.command, &JobStatus::Killed);
        }
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

fn spawn_reader<R>(
    reader: R,
    output: Arc<Mutex<String>>,
    prefix: &'static str,
    log_path: Option<PathBuf>,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        if let Some(path) = &log_path
            && let Some(parent) = path.parent()
        {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut log = log_path.as_ref().and_then(|path| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
        });
        let mut lines = BufReader::new(reader).lines();
        let mut written = 0usize;
        while let Ok(Some(line)) = lines.next_line().await {
            let mut buffer = output.lock().await;
            buffer.push_str(prefix);
            buffer.push_str(&line);
            buffer.push('\n');
            trim_tail(&mut buffer);
            if let Some(file) = log.as_mut() {
                let _ = writeln!(file, "{prefix}{line}");
                written += 1;
                if written.is_multiple_of(1024)
                    && let Some(path) = &log_path
                    && std::fs::metadata(path)
                        .is_ok_and(|meta| meta.len() > 4 * MAX_OUTPUT_BYTES as u64)
                {
                    let _ = std::fs::write(path, &*buffer);
                }
            }
        }
    });
}

fn persist_status(dir: &Path, id: &str, command: &str, status: &JobStatus) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(text) = serde_json::to_string(&StoredJob {
        id: id.to_owned(),
        command: command.to_owned(),
        status: status.clone(),
    }) else {
        return;
    };
    let path = dir.join(format!("{id}.json"));
    let temporary = dir.join(format!("{id}.json.tmp"));
    if std::fs::write(&temporary, text).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

fn load_jobs(dir: &Path) -> BTreeMap<String, Arc<Job>> {
    let mut jobs = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return jobs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(stored) = serde_json::from_str::<StoredJob>(&text) else {
            continue;
        };
        let Ok(id) = stored.id.parse::<JobId>() else {
            continue;
        };
        let mut status = stored.status;
        if matches!(status, JobStatus::Running) {
            status = JobStatus::Interrupted;
            persist_status(dir, &stored.id, &stored.command, &status);
        }
        let mut output =
            std::fs::read_to_string(dir.join(format!("{}.log", stored.id))).unwrap_or_default();
        trim_tail(&mut output);
        jobs.insert(
            stored.id.clone(),
            Arc::new(Job {
                id,
                command: stored.command,
                status: Arc::new(Mutex::new(status)),
                output: Arc::new(Mutex::new(output)),
                child: None,
            }),
        );
    }
    jobs
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
            prompt: None,
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
    async fn restores_finished_job_and_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = dir.path().join("jobs");
        let manager = JobManager::with_state(None, state.clone());
        let id = manager
            .start("printf 'durable'", dir.path())
            .await
            .expect("start");
        tokio::time::sleep(Duration::from_millis(800)).await;
        assert_eq!(
            manager.status(id.as_str()).await.expect("status"),
            JobStatus::Exited(0)
        );
        drop(manager);

        let restored = JobManager::with_state(None, state);
        assert_eq!(
            restored.status(id.as_str()).await.expect("status"),
            JobStatus::Exited(0)
        );
        let output = restored.output(id.as_str()).await.expect("output");
        assert!(output.contains("durable"), "{output}");
    }

    #[tokio::test]
    async fn marks_running_job_interrupted_on_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = dir.path().join("jobs");
        std::fs::create_dir_all(&state).expect("mkdir");
        let id = JobId::new();
        std::fs::write(
            state.join(format!("{id}.json")),
            serde_json::to_string(&StoredJob {
                id: id.to_string(),
                command: "sleep 10".into(),
                status: JobStatus::Running,
            })
            .expect("json"),
        )
        .expect("metadata");
        std::fs::write(state.join(format!("{id}.log")), "partial output\n").expect("log");

        let manager = JobManager::with_state(None, state.clone());
        assert_eq!(
            manager.status(id.as_str()).await.expect("status"),
            JobStatus::Interrupted
        );
        let output = manager.output(id.as_str()).await.expect("output");
        assert!(output.contains("partial output"), "{output}");
        let stored = std::fs::read_to_string(state.join(format!("{id}.json"))).expect("metadata");
        assert!(stored.contains("Interrupted"), "{stored}");
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
