//! Deterministic eval suite (ROADMAP 2.12).
//!
//! Every task in `evals/tasks/*.json` describes a workspace, a replayed model
//! script (cassette `calls`) and success criteria. The suite drives the real
//! `dshr run` binary, so tools, approvals, the agent loop and the journal are
//! exercised end to end without network access.
//!
//! Report: `target/evals/report.json` (override with `DSHR_EVAL_REPORT`).

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use assert_cmd::Command;
use serde_json::{Value, json};

const MODEL: &str = "deepseek-flash";

struct Expect {
    stdout_contains: Vec<String>,
    files_contain: Vec<(String, String)>,
    files_absent: Vec<String>,
    steps_max: u64,
    tool_calls_max: u64,
}

struct Task {
    id: String,
    title: String,
    prompt: String,
    files: Vec<(String, String)>,
    calls: Vec<Value>,
    tokens_max: u64,
    expect: Expect,
}

struct Run {
    steps: u64,
    tool_calls: u64,
    input_tokens: u64,
    output_tokens: u64,
    elapsed_ms: u128,
    text: String,
}

struct Report {
    id: String,
    title: String,
    ok: bool,
    failures: Vec<String>,
    steps: u64,
    tool_calls: u64,
    tokens: u64,
    elapsed_ms: u128,
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn file_map(value: Option<&Value>) -> Vec<(String, String)> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(path, content)| {
                    content
                        .as_str()
                        .map(|content| (path.clone(), content.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn task_from_json(value: &Value) -> Task {
    let expect = &value["expect"];
    Task {
        id: value["id"].as_str().expect("task id").to_owned(),
        title: value["title"].as_str().unwrap_or_default().to_owned(),
        prompt: value["prompt"].as_str().expect("task prompt").to_owned(),
        files: file_map(value.get("files")),
        calls: value["calls"].as_array().expect("task calls").clone(),
        tokens_max: expect["tokens_max"].as_u64().unwrap_or(10_000),
        expect: Expect {
            stdout_contains: string_list(expect.get("stdout_contains")),
            files_contain: file_map(expect.get("files_contain")),
            files_absent: string_list(expect.get("files_absent")),
            steps_max: expect["steps_max"].as_u64().unwrap_or(4),
            tool_calls_max: expect["tool_calls_max"].as_u64().unwrap_or(4),
        },
    }
}

fn load_tasks() -> Vec<Task> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals/tasks");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path).expect("read task");
            let value: Value = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
            task_from_json(&value)
        })
        .collect()
}

/// Reject absolute paths and `..` components so a task cannot write outside
/// its private temp workspace.
fn confined(root: &Path, relative: &str) -> PathBuf {
    let candidate = Path::new(relative);
    assert!(
        candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "task path must stay inside the workspace: {relative}"
    );
    root.join(candidate)
}

fn run_task(task: &Task) -> Result<Run, String> {
    let home = tempfile::tempdir().map_err(|error| format!("home tempdir: {error}"))?;
    let project = tempfile::tempdir().map_err(|error| format!("project tempdir: {error}"))?;

    for (path, content) in &task.files {
        let target = confined(project.path(), path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("mkdir {path}: {error}"))?;
        }
        fs::write(&target, content).map_err(|error| format!("write {path}: {error}"))?;
    }

    let cassette_path = home.path().join("cassette.json");
    let cassette = json!({ "model": MODEL, "calls": task.calls });
    fs::write(
        &cassette_path,
        serde_json::to_vec(&cassette).map_err(|error| format!("cassette: {error}"))?,
    )
    .map_err(|error| format!("write cassette: {error}"))?;

    let started = Instant::now();
    let output = Command::cargo_bin("dshr")
        .map_err(|error| format!("binary: {error}"))?
        .current_dir(project.path())
        .args([
            "--home",
            home.path().to_str().expect("home utf8"),
            "run",
            "--danger",
            "--json",
            "--cassette",
            cassette_path.to_str().expect("cassette utf8"),
            "--model",
            MODEL,
            "--max-steps",
            &task.expect.steps_max.to_string(),
        ])
        .arg(&task.prompt)
        .output()
        .map_err(|error| format!("spawn run: {error}"))?;
    let elapsed_ms = started.elapsed().as_millis();

    if !output.status.success() {
        return Err(format!(
            "run failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let result: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| format!("parse json: {error}"))?;
    let usage = &result["usage"];
    let run = Run {
        steps: result["steps"].as_u64().unwrap_or_default(),
        tool_calls: result["tool_calls"].as_u64().unwrap_or_default(),
        input_tokens: usage["input_tokens"].as_u64().unwrap_or_default(),
        output_tokens: usage["output_tokens"].as_u64().unwrap_or_default(),
        elapsed_ms,
        text: result["text"].as_str().unwrap_or_default().to_owned(),
    };

    let mut failures = Vec::new();
    for needle in &task.expect.stdout_contains {
        if !run.text.contains(needle.as_str()) {
            failures.push(format!("stdout missing {needle:?} in {:?}", run.text));
        }
    }
    for (path, needle) in &task.expect.files_contain {
        let target = confined(project.path(), path);
        match fs::read_to_string(&target) {
            Ok(content) if content.contains(needle.as_str()) => {}
            Ok(content) => failures.push(format!("{path} missing {needle:?} in {content:?}")),
            Err(error) => failures.push(format!("{path} unreadable: {error}")),
        }
    }
    for path in &task.expect.files_absent {
        if confined(project.path(), path).exists() {
            failures.push(format!("{path} should not exist"));
        }
    }
    if run.steps > task.expect.steps_max {
        failures.push(format!("steps {} > {}", run.steps, task.expect.steps_max));
    }
    if run.tool_calls > task.expect.tool_calls_max {
        failures.push(format!(
            "tool calls {} > {}",
            run.tool_calls, task.expect.tool_calls_max
        ));
    }
    let tokens = run.input_tokens + run.output_tokens;
    if tokens > task.tokens_max {
        failures.push(format!("tokens {tokens} > {}", task.tokens_max));
    }

    if failures.is_empty() {
        Ok(run)
    } else {
        Err(failures.join("; "))
    }
}

fn report_path() -> PathBuf {
    std::env::var_os("DSHR_EVAL_REPORT").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evals")
                .join("report.json")
        },
        PathBuf::from,
    )
}

#[test]
fn eval_suite_meets_success_and_cost_targets() {
    let tasks = load_tasks();
    assert_eq!(tasks.len(), 10, "the eval set keeps ten focused tasks");

    let mut reports = Vec::new();
    let mut failed = Vec::new();

    for task in &tasks {
        match run_task(task) {
            Ok(run) => {
                let tokens = run.input_tokens + run.output_tokens;
                println!(
                    "PASS {:<18} steps={} tools={} tokens={} {}ms  {}",
                    task.id, run.steps, run.tool_calls, tokens, run.elapsed_ms, task.title
                );
                reports.push(Report {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    ok: true,
                    failures: Vec::new(),
                    steps: run.steps,
                    tool_calls: run.tool_calls,
                    tokens,
                    elapsed_ms: run.elapsed_ms,
                });
            }
            Err(failure) => {
                println!("FAIL {:<18} {failure}", task.id);
                failed.push(format!("{}: {failure}", task.id));
                reports.push(Report {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    ok: false,
                    failures: vec![failure],
                    steps: 0,
                    tool_calls: 0,
                    tokens: 0,
                    elapsed_ms: 0,
                });
            }
        }
    }

    let passed = reports.iter().filter(|report| report.ok).count();
    let total_tokens: u64 = reports.iter().map(|report| report.tokens).sum();
    let total_ms: u128 = reports.iter().map(|report| report.elapsed_ms).sum();
    let payload = json!({
        "suite": "dshr-evals",
        "tasks": reports.iter().map(|report| json!({
            "id": report.id,
            "title": report.title,
            "ok": report.ok,
            "failures": report.failures,
            "steps": report.steps,
            "tool_calls": report.tool_calls,
            "tokens": report.tokens,
            "elapsed_ms": report.elapsed_ms,
        })).collect::<Vec<_>>(),
        "summary": {
            "passed": passed,
            "total": reports.len(),
            "success_rate": passed as f64 / reports.len() as f64,
            "tokens": total_tokens,
            "elapsed_ms": total_ms,
        },
    });
    let path = report_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("serialize report"),
    )
    .expect("write eval report");
    println!(
        "evals: {passed}/{} passed, {total_tokens} tokens, {total_ms}ms, report={}",
        reports.len(),
        path.display()
    );

    assert!(
        failed.is_empty(),
        "eval suite regressions: {}",
        failed.join(" | ")
    );
    assert_eq!(passed, reports.len(), "success rate must stay at 100%");
}
