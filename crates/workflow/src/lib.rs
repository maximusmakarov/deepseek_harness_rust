#![forbid(unsafe_code)]
//! A small DAG workflow engine.
//!
//! Workflows are YAML files with steps. Each step is delegated to a child agent
//! through [`dshr_agent::SubagentRunner`]; steps run in dependency order, and a
//! prompt may reference earlier outputs as `{{step_id.output}}`.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dshr_agent::SubagentRunner;
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowStep {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("read workflow: {0}")]
    Io(String),

    #[error("parse workflow: {0}")]
    Parse(String),

    #[error("invalid workflow: {0}")]
    Invalid(String),

    #[error("step '{0}' failed: {1}")]
    Step(String, String),

    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub id: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowReport {
    pub name: String,
    pub steps: Vec<StepResult>,
    pub text: String,
}

impl Workflow {
    pub fn load(path: &Path) -> Result<Self, WorkflowError> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| WorkflowError::Io(format!("{}: {error}", path.display())))?;
        let workflow: Self = serde_saphyr::from_str(&text)
            .map_err(|error| WorkflowError::Parse(error.to_string()))?;
        workflow.validate()?;
        Ok(workflow)
    }

    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.name.trim().is_empty() {
            return Err(WorkflowError::Invalid("name is required".into()));
        }
        if self.steps.is_empty() {
            return Err(WorkflowError::Invalid("workflow has no steps".into()));
        }
        let mut ids = BTreeMap::new();
        for (index, step) in self.steps.iter().enumerate() {
            if step.id.trim().is_empty() {
                return Err(WorkflowError::Invalid(format!(
                    "step {index} has an empty id"
                )));
            }
            if ids.insert(step.id.clone(), index).is_some() {
                return Err(WorkflowError::Invalid(format!(
                    "duplicate step id '{}'",
                    step.id
                )));
            }
            if step.prompt.trim().is_empty() {
                return Err(WorkflowError::Invalid(format!(
                    "step '{}' has an empty prompt",
                    step.id
                )));
            }
        }
        for step in &self.steps {
            for dependency in &step.depends_on {
                if !ids.contains_key(dependency) {
                    return Err(WorkflowError::Invalid(format!(
                        "step '{}' depends on unknown step '{dependency}'",
                        step.id
                    )));
                }
            }
        }
        self.order()?;
        Ok(())
    }

    /// Kahn topological order, preserving author order among ready steps.
    pub fn order(&self) -> Result<Vec<usize>, WorkflowError> {
        let mut index_of = BTreeMap::new();
        for (index, step) in self.steps.iter().enumerate() {
            index_of.insert(step.id.as_str(), index);
        }
        let mut indegree = vec![0_usize; self.steps.len()];
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); self.steps.len()];
        for (index, step) in self.steps.iter().enumerate() {
            for dependency in &step.depends_on {
                let from = *index_of
                    .get(dependency.as_str())
                    .ok_or_else(|| WorkflowError::Invalid("unknown dependency".into()))?;
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
        let mut order = Vec::with_capacity(self.steps.len());
        while let Some(index) = ready.pop_front() {
            order.push(index);
            for next in &edges[index] {
                indegree[*next] -= 1;
                if indegree[*next] == 0 {
                    ready.push_back(*next);
                }
            }
        }
        if order.len() != self.steps.len() {
            return Err(WorkflowError::Invalid("dependency cycle detected".into()));
        }
        Ok(order)
    }

    pub fn render(prompt: &str, outputs: &BTreeMap<String, String>, input: &str) -> String {
        let mut rendered = prompt.replace("{{input}}", input);
        for (id, output) in outputs {
            rendered = rendered.replace(&format!("{{{{{id}.output}}}}"), output);
        }
        rendered
    }
}

pub struct WorkflowRunner {
    runner: Arc<dyn SubagentRunner>,
}

impl WorkflowRunner {
    #[must_use]
    pub fn new(runner: Arc<dyn SubagentRunner>) -> Self {
        Self { runner }
    }

    pub async fn run(
        &self,
        workflow: &Workflow,
        input: &str,
        cancel: CancellationToken,
    ) -> Result<WorkflowReport, WorkflowError> {
        workflow.validate()?;
        let order = workflow.order()?;
        let mut outputs = BTreeMap::new();
        let mut results = Vec::new();

        for index in order {
            if cancel.is_cancelled() {
                return Err(WorkflowError::Cancelled);
            }
            let step = &workflow.steps[index];
            let prompt = Workflow::render(&step.prompt, &outputs, input);
            let output = self
                .runner
                .run(prompt, cancel.clone())
                .await
                .map_err(|error| WorkflowError::Step(step.id.clone(), error))?;
            outputs.insert(step.id.clone(), output.clone());
            results.push(StepResult {
                id: step.id.clone(),
                output,
            });
        }

        let text = results
            .iter()
            .map(|step| format!("## {}\n{}", step.id, step.output))
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(WorkflowReport {
            name: workflow.name.clone(),
            steps: results,
            text,
        })
    }
}

/// Tool that loads a workflow from disk and runs it.
pub struct WorkflowTool {
    runner: Arc<dyn SubagentRunner>,
    dirs: Vec<PathBuf>,
}

impl WorkflowTool {
    #[must_use]
    pub fn new(runner: Arc<dyn SubagentRunner>, dirs: Vec<PathBuf>) -> Self {
        Self { runner, dirs }
    }

    fn load(&self, name: &str) -> Result<Workflow, WorkflowError> {
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(WorkflowError::Invalid("invalid workflow name".into()));
        }
        for dir in &self.dirs {
            for extension in ["yaml", "yml"] {
                let path = dir.join(format!("{name}.{extension}"));
                if path.is_file() {
                    return Workflow::load(&path);
                }
            }
        }
        Err(WorkflowError::Io(format!(
            "workflow '{name}' not found in {}",
            self.dirs
                .iter()
                .map(|dir| dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

#[async_trait]
impl Tool for WorkflowTool {
    fn name(&self) -> &str {
        "workflow"
    }

    fn description(&self) -> &str {
        "Run a named multi-step workflow from ~/.dshr/workflows or .dshr/workflows."
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
        let workflow = self
            .load(name)
            .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        let report = WorkflowRunner::new(Arc::clone(&self.runner))
            .run(&workflow, input, ctx.cancel.clone())
            .await
            .map_err(|error| match error {
                WorkflowError::Cancelled => ToolError::Cancelled,
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
            Ok(format!("<{prompt}>"))
        }
    }

    fn workflow() -> Workflow {
        Workflow {
            name: "demo".into(),
            description: String::new(),
            steps: vec![
                WorkflowStep {
                    id: "a".into(),
                    prompt: "first {{input}}".into(),
                    depends_on: Vec::new(),
                },
                WorkflowStep {
                    id: "b".into(),
                    prompt: "second {{a.output}}".into(),
                    depends_on: vec!["a".into()],
                },
            ],
        }
    }

    #[test]
    fn validates_and_orders_steps() {
        let workflow = workflow();
        workflow.validate().expect("valid");
        assert_eq!(workflow.order().expect("order"), vec![0, 1]);
    }

    #[test]
    fn rejects_cycles() {
        let workflow = Workflow {
            name: "cycle".into(),
            description: String::new(),
            steps: vec![
                WorkflowStep {
                    id: "a".into(),
                    prompt: "a".into(),
                    depends_on: vec!["b".into()],
                },
                WorkflowStep {
                    id: "b".into(),
                    prompt: "b".into(),
                    depends_on: vec!["a".into()],
                },
            ],
        };
        assert!(workflow.validate().is_err());
    }

    #[test]
    fn renders_placeholders() {
        let mut outputs = BTreeMap::new();
        outputs.insert("a".to_owned(), "OUT".to_owned());
        assert_eq!(
            Workflow::render("x {{input}} {{a.output}}", &outputs, "IN"),
            "x IN OUT"
        );
    }

    #[tokio::test]
    async fn runs_steps_in_order() {
        let runner = WorkflowRunner::new(Arc::new(MockRunner));
        let report = runner
            .run(&workflow(), "input", CancellationToken::new())
            .await
            .expect("run");
        assert_eq!(report.steps[0].output, "<first input>");
        assert_eq!(report.steps[1].output, "<second <first input>>");
    }

    #[tokio::test]
    async fn tool_loads_workflow_from_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("demo.yaml"),
            "name: demo\nsteps:\n  - id: a\n    prompt: hi\n",
        )
        .expect("write");
        let tool = WorkflowTool::new(Arc::new(MockRunner), vec![dir.path().to_path_buf()]);
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
            .expect("workflow");
        assert!(outcome.content.contains("<hi>"));
    }
}
