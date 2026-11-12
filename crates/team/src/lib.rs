#![forbid(unsafe_code)]
//! Agent teams: a shared task board with ownership and dependencies.
//!
//! Tasks are defined as a DAG. Workers claim only ready tasks (all dependencies
//! done), so ownership is atomic and no two workers execute the same task.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dshr_agent::SubagentRunner;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamSpec {
    pub name: String,
    #[serde(default = "default_workers")]
    pub workers: usize,
    pub tasks: Vec<TeamTask>,
}

fn default_workers() -> usize {
    2
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamTask {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TeamError {
    #[error("read team: {0}")]
    Io(String),

    #[error("parse team: {0}")]
    Parse(String),

    #[error("invalid team: {0}")]
    Invalid(String),

    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Failed,
}

impl TaskStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
struct TaskState {
    status: TaskStatus,
    owner: Option<String>,
    output: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReport {
    pub id: String,
    pub title: String,
    pub status: String,
    pub owner: Option<String>,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TeamReport {
    pub name: String,
    pub tasks: Vec<TaskReport>,
    pub text: String,
}

impl TeamSpec {
    pub fn load(path: &Path) -> Result<Self, TeamError> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| TeamError::Io(format!("{}: {error}", path.display())))?;
        let spec: Self =
            serde_saphyr::from_str(&text).map_err(|error| TeamError::Parse(error.to_string()))?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), TeamError> {
        if self.name.trim().is_empty() {
            return Err(TeamError::Invalid("name is required".into()));
        }
        if self.tasks.is_empty() {
            return Err(TeamError::Invalid("team has no tasks".into()));
        }
        let mut ids = BTreeMap::new();
        for (index, task) in self.tasks.iter().enumerate() {
            if task.id.trim().is_empty() {
                return Err(TeamError::Invalid(format!("task {index} has an empty id")));
            }
            if ids.insert(task.id.clone(), index).is_some() {
                return Err(TeamError::Invalid(format!(
                    "duplicate task id '{}'",
                    task.id
                )));
            }
            if task.prompt.trim().is_empty() {
                return Err(TeamError::Invalid(format!(
                    "task '{}' has an empty prompt",
                    task.id
                )));
            }
        }
        for task in &self.tasks {
            for dependency in &task.depends_on {
                if !ids.contains_key(dependency) {
                    return Err(TeamError::Invalid(format!(
                        "task '{}' depends on unknown task '{dependency}'",
                        task.id
                    )));
                }
            }
        }
        self.order()?;
        Ok(())
    }

    pub fn order(&self) -> Result<Vec<usize>, TeamError> {
        let mut index_of = BTreeMap::new();
        for (index, task) in self.tasks.iter().enumerate() {
            index_of.insert(task.id.as_str(), index);
        }
        let mut indegree = vec![0_usize; self.tasks.len()];
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); self.tasks.len()];
        for (index, task) in self.tasks.iter().enumerate() {
            for dependency in &task.depends_on {
                let from = *index_of
                    .get(dependency.as_str())
                    .ok_or_else(|| TeamError::Invalid("unknown dependency".into()))?;
                edges[from].push(index);
                indegree[index] += 1;
            }
        }
        let mut ready: VecDeque<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, degree)| **degree == 0)
            .map(|(index, _)| index)
            .collect();
        let mut order = Vec::new();
        while let Some(index) = ready.pop_front() {
            order.push(index);
            for next in &edges[index] {
                indegree[*next] -= 1;
                if indegree[*next] == 0 {
                    ready.push_back(*next);
                }
            }
        }
        if order.len() != self.tasks.len() {
            return Err(TeamError::Invalid("dependency cycle detected".into()));
        }
        Ok(order)
    }
}

struct Board {
    spec: Arc<TeamSpec>,
    tasks: Mutex<BTreeMap<String, TaskState>>,
}

impl Board {
    fn new(spec: Arc<TeamSpec>) -> Self {
        let tasks = spec
            .tasks
            .iter()
            .map(|task| {
                (
                    task.id.clone(),
                    TaskState {
                        status: TaskStatus::Pending,
                        owner: None,
                        output: String::new(),
                        error: None,
                    },
                )
            })
            .collect();
        Self {
            spec,
            tasks: Mutex::new(tasks),
        }
    }

    /// Claims one ready task and returns it with dependency outputs.
    async fn claim_next(&self, owner: &str) -> Option<(TeamTask, Vec<(String, String)>)> {
        let mut tasks = self.tasks.lock().await;
        for task in &self.spec.tasks {
            let state = tasks.get(&task.id)?;
            if state.status != TaskStatus::Pending {
                continue;
            }
            let dependencies_ready = task.depends_on.iter().all(|dependency| {
                tasks
                    .get(dependency)
                    .is_some_and(|state| state.status == TaskStatus::Done)
            });
            if !dependencies_ready {
                continue;
            }
            let dependency_outputs = task
                .depends_on
                .iter()
                .filter_map(|dependency| {
                    tasks
                        .get(dependency)
                        .map(|state| (dependency.clone(), state.output.clone()))
                })
                .collect();
            let state = tasks.get_mut(&task.id)?;
            state.status = TaskStatus::InProgress;
            state.owner = Some(owner.to_owned());
            return Some((task.clone(), dependency_outputs));
        }
        None
    }

    async fn complete(&self, id: &str, output: String) {
        if let Some(state) = self.tasks.lock().await.get_mut(id) {
            state.status = TaskStatus::Done;
            state.output = output;
            state.error = None;
        }
    }

    async fn fail(&self, id: &str, error: String) {
        if let Some(state) = self.tasks.lock().await.get_mut(id) {
            state.status = TaskStatus::Failed;
            state.error = Some(error);
        }
    }

    async fn report(&self) -> Vec<TaskReport> {
        let tasks = self.tasks.lock().await;
        self.spec
            .tasks
            .iter()
            .filter_map(|task| {
                let state = tasks.get(&task.id)?;
                Some(TaskReport {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    status: state.status.label().to_owned(),
                    owner: state.owner.clone(),
                    output: state.output.clone(),
                    error: state.error.clone(),
                })
            })
            .collect()
    }
}

pub struct TeamRunner {
    runner: Arc<dyn SubagentRunner>,
}

impl TeamRunner {
    #[must_use]
    pub fn new(runner: Arc<dyn SubagentRunner>) -> Self {
        Self { runner }
    }

    pub async fn run(
        &self,
        spec: TeamSpec,
        input: &str,
        cancel: CancellationToken,
    ) -> Result<TeamReport, TeamError> {
        spec.validate()?;
        let spec = Arc::new(spec);
        let board = Arc::new(Board::new(Arc::clone(&spec)));
        let workers = spec.workers.clamp(1, 8);
        let mut handles = Vec::new();

        for index in 0..workers {
            let board = Arc::clone(&board);
            let runner = Arc::clone(&self.runner);
            let cancel = cancel.clone();
            let input = input.to_owned();
            handles.push(tokio::spawn(async move {
                let owner = format!("worker-{}", index + 1);
                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let Some((task, dependencies)) = board.claim_next(&owner).await else {
                        break;
                    };
                    let mut prompt = format!(
                        "You are {owner} on team '{}'.\nTask {}: {}\n\n{}",
                        board.spec.name, task.id, task.title, task.prompt
                    );
                    if !input.is_empty() {
                        prompt.push_str(&format!("\n\nTeam input: {input}"));
                    }
                    for (dependency, output) in dependencies {
                        prompt.push_str(&format!("\n\n[{dependency}]\n{output}"));
                    }
                    match runner.run(prompt, cancel.clone()).await {
                        Ok(output) => board.complete(&task.id, output).await,
                        Err(error) => board.fail(&task.id, error).await,
                    }
                }
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }
        if cancel.is_cancelled() {
            return Err(TeamError::Cancelled);
        }

        let tasks = board.report().await;
        let text = tasks
            .iter()
            .map(|task| {
                format!(
                    "## {} [{}] {}\n{}",
                    task.id,
                    task.status,
                    task.title,
                    task.error.as_deref().unwrap_or(&task.output)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(TeamReport {
            name: spec.name.clone(),
            tasks,
            text,
        })
    }
}

pub struct TeamTool {
    runner: Arc<dyn SubagentRunner>,
    dirs: Vec<PathBuf>,
}

impl TeamTool {
    #[must_use]
    pub fn new(runner: Arc<dyn SubagentRunner>, dirs: Vec<PathBuf>) -> Self {
        Self { runner, dirs }
    }

    fn load(&self, name: &str) -> Result<TeamSpec, TeamError> {
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(TeamError::Invalid("invalid team name".into()));
        }
        for dir in &self.dirs {
            for extension in ["yaml", "yml"] {
                let path = dir.join(format!("{name}.{extension}"));
                if path.is_file() {
                    return TeamSpec::load(&path);
                }
            }
        }
        Err(TeamError::Io(format!("team '{name}' not found")))
    }
}

#[async_trait]
impl Tool for TeamTool {
    fn name(&self) -> &str {
        "team"
    }

    fn description(&self) -> &str {
        "Run a named agent team from ~/.dshr/teams or .dshr/teams using the shared task board."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["name"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "minLength": 1},
                "input": {"type": "string"}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name is required".into()))?;
        let input = args["input"].as_str().unwrap_or("");
        let spec = self
            .load(name)
            .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        let report = TeamRunner::new(Arc::clone(&self.runner))
            .run(spec, input, ctx.cancel.clone())
            .await
            .map_err(|error| match error {
                TeamError::Cancelled => ToolError::Cancelled,
                other => ToolError::Failed(other.to_string()),
            })?;
        Ok(ToolOutcome::text(report.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRunner;

    #[async_trait]
    impl SubagentRunner for MockRunner {
        async fn run(&self, prompt: String, _cancel: CancellationToken) -> Result<String, String> {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok(format!("done:{}", prompt.lines().next().unwrap_or("")))
        }
    }

    fn spec(workers: usize, tasks: Vec<TeamTask>) -> TeamSpec {
        TeamSpec {
            name: "demo".into(),
            workers,
            tasks,
        }
    }

    fn task(id: &str, depends_on: &[&str]) -> TeamTask {
        TeamTask {
            id: id.into(),
            title: id.into(),
            prompt: format!("do {id}"),
            depends_on: depends_on.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    #[tokio::test]
    async fn runs_dependent_tasks_in_order() {
        let spec = spec(1, vec![task("a", &[]), task("b", &["a"])]);
        let report = TeamRunner::new(Arc::new(MockRunner))
            .run(spec, "input", CancellationToken::new())
            .await
            .expect("run");
        assert_eq!(report.tasks.len(), 2);
        assert!(report.tasks.iter().all(|task| task.status == "done"));
        assert_eq!(report.tasks[1].owner.as_deref(), Some("worker-1"));
    }

    #[tokio::test]
    async fn workers_do_not_claim_the_same_task() {
        let spec = spec(4, vec![task("a", &[]), task("b", &[]), task("c", &[])]);
        let report = TeamRunner::new(Arc::new(MockRunner))
            .run(spec, "", CancellationToken::new())
            .await
            .expect("run");
        let owners: Vec<_> = report
            .tasks
            .iter()
            .filter_map(|task| task.owner.clone())
            .collect();
        assert_eq!(owners.len(), 3);
        let unique: std::collections::BTreeSet<_> = owners.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn rejects_cycles() {
        let spec = spec(1, vec![task("a", &["b"]), task("b", &["a"])]);
        assert!(spec.validate().is_err());
    }

    #[tokio::test]
    async fn tool_loads_team_from_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("demo.yaml"),
            "name: demo\nworkers: 1\ntasks:\n  - id: a\n    title: A\n    prompt: do a\n",
        )
        .expect("write");
        let tool = TeamTool::new(Arc::new(MockRunner), vec![dir.path().to_path_buf()]);
        let ctx = dshr_tools::ToolCtx {
            cwd: dir.path().to_path_buf(),
            session_id: dshr_core::SessionId::new(),
            output_limit: 10_000,
            spill_dir: dir.path().join("spills"),
            cancel: CancellationToken::new(),
        };
        let outcome = tool
            .execute(&ctx, json!({"name": "demo"}))
            .await
            .expect("team");
        assert!(outcome.content.contains("[done]"), "{}", outcome.content);
    }
}
