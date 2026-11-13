use assert_cmd::Command;
use predicates::prelude::*;

fn dshr() -> Command {
    Command::cargo_bin("dshr").expect("dshr binary")
}

#[test]
fn version_is_reported() {
    dshr()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn doctor_json_works_without_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "doctor",
            "--json",
        ])
        .output()
        .expect("run doctor");

    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["config_found"], false);
    assert_eq!(report["sandbox"], "n/a (M5)");
    assert_eq!(report["provider"], "deepseek-account");
}

#[test]
fn config_show_applies_user_layer() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "model:\n  provider: openai-compatible\n  name: deepseek-v4-pro\n",
    )
    .expect("write config");

    let output = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "config",
            "show",
            "--json",
        ])
        .output()
        .expect("run config show");

    assert!(output.status.success());
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(config["model"]["provider"], "openai-compatible");
    assert_eq!(config["model"]["name"], "deepseek-v4-pro");
}

#[test]
fn schema_is_valid_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "config",
            "show",
            "--schema",
        ])
        .output()
        .expect("run schema");

    assert!(output.status.success());
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(schema["properties"]["model"].is_object(), "{schema}");
}

#[test]
fn missing_patch_fails_with_exit_code_2() {
    let dir = tempfile::tempdir().expect("tempdir");
    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "--patch",
            dir.path().join("absent.yaml").to_str().expect("utf8"),
            "config",
            "show",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("patch file does not exist"));
}

#[test]
fn invalid_config_fails_loudly() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("config.yaml"), "tools:\n  max_paralel: 4\n")
        .expect("write config");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "config",
            "show",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("max_paralel"));
}

#[test]
fn doctor_reports_invalid_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("config.yaml"), "version: 2\n").expect("write config");

    dshr()
        .args(["--home", dir.path().to_str().expect("utf8"), "doctor"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("status: error"))
        .stdout(predicate::str::contains("unsupported config version"));
}

#[test]
fn config_show_yaml_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "config",
            "show",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("version: 1"))
        .stdout(predicate::str::contains("provider: deepseek-account"));
}

fn write_cassette(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("cassette.json");
    std::fs::write(
        &path,
        r#"{"model":"deepseek-flash","chunks":[
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"pong"},
            {"type":"usage","usage":{"input_tokens":5,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},
            {"type":"done","stop_reason":"end_turn"}]}"#,
    )
    .expect("write cassette");
    path
}

#[test]
fn run_replays_cassette_and_persists_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "hi",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pong"));

    let output = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list sessions");
    let sessions: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(sessions.as_array().expect("array").len(), 1);
    assert_eq!(sessions[0]["turns"], 1);
    assert_eq!(sessions[0]["model"], "deepseek-flash");
}

#[test]
fn run_json_mode_returns_structured_result() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());

    let output = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--json",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "hi",
        ])
        .output()
        .expect("run");

    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(result["text"], "pong");
    assert_eq!(result["usage"]["input_tokens"], 5);
    assert!(
        result["session_id"]
            .as_str()
            .expect("id")
            .starts_with("ses_")
    );
}

#[test]
fn sessions_export_markdown_contains_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "hi",
        ])
        .assert()
        .success();

    let listed = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    let id = sessions[0]["id"].as_str().expect("id");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "export",
            id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## User"))
        .stdout(predicate::str::contains("pong"));
}

#[test]
fn run_records_a_replayable_cassette() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());
    let recorded = dir.path().join("recorded.json");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--record",
            recorded.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "hi",
        ])
        .assert()
        .success();

    let text = std::fs::read_to_string(&recorded).expect("recorded cassette");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(value["model"], "deepseek-flash");
    let calls = value["calls"].as_array().expect("calls");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].as_array().expect("chunks").len() >= 2);

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            recorded.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "again",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pong"));
}

#[test]
fn run_resumes_existing_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());
    let home = dir.path().to_str().expect("utf8");

    dshr()
        .args([
            "--home",
            home,
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "first",
        ])
        .assert()
        .success();

    let listed = dshr()
        .args(["--home", home, "sessions", "list", "--json"])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    let id = sessions[0]["id"].as_str().expect("id");

    dshr()
        .args([
            "--home",
            home,
            "run",
            "--session",
            id,
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "second",
        ])
        .assert()
        .success();

    let shown = dshr()
        .args(["--home", home, "sessions", "show", id])
        .output()
        .expect("show");
    let text = String::from_utf8_lossy(&shown.stdout);
    assert_eq!(text.matches("## User").count(), 2, "{text}");
    assert_eq!(text.matches("## Assistant").count(), 2, "{text}");
}

#[test]
fn run_executes_tool_loop_from_cassette() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = dir.path().join("tools.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"read"},
            {"type":"tool_input_delta","index":0,"json":"{\"path\":\"Cargo.toml\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("write cassette");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "read",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let listed = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions[0]["turns"], 1);
}

#[test]
fn sessions_support_title_search_fork_and_rewind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = write_cassette(dir.path());
    let home = dir.path().to_str().expect("utf8");

    for prompt in ["first", "second"] {
        dshr()
            .args([
                "--home",
                home,
                "run",
                "--cassette",
                cassette.to_str().expect("utf8"),
                "--model",
                "deepseek-flash",
                prompt,
            ])
            .assert()
            .success();
    }

    let listed = dshr()
        .args(["--home", home, "sessions", "list", "--json"])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    let id = sessions[0]["id"].as_str().expect("id");

    dshr()
        .args(["--home", home, "sessions", "title", id, "demo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("demo"));

    let searched = dshr()
        .args(["--home", home, "sessions", "search", "pong", "--json"])
        .output()
        .expect("search");
    let hits: serde_json::Value = serde_json::from_slice(&searched.stdout).expect("json");
    assert!(hits.as_array().expect("hits").len() >= 2);

    let forked = dshr()
        .args(["--home", home, "sessions", "fork", id, "--at", "5"])
        .output()
        .expect("fork");
    let fork_id = String::from_utf8_lossy(&forked.stdout).trim().to_owned();
    assert!(fork_id.starts_with("ses_"));

    dshr()
        .args(["--home", home, "sessions", "rewind", id, "--turn", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 turn(s)"));

    let shown = dshr()
        .args(["--home", home, "sessions", "show", id])
        .output()
        .expect("show");
    let text = String::from_utf8_lossy(&shown.stdout);
    assert_eq!(text.matches("## User").count(), 1, "{text}");
}

#[test]
fn prompt_includes_agents_md_and_skills() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&home).expect("home");
    std::fs::create_dir_all(&project).expect("project");
    std::fs::write(project.join("AGENTS.md"), "PROJECT RULES").expect("agents");
    let skill = home.join("skills/rust");
    std::fs::create_dir_all(&skill).expect("skill dir");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: rust\ndescription: Rust guidelines\n---\nUse cargo fmt.\n",
    )
    .expect("skill");

    let output = dshr()
        .current_dir(&project)
        .args(["--home", home.to_str().expect("utf8"), "prompt"])
        .output()
        .expect("prompt");
    assert!(output.status.success());
    let prompt = String::from_utf8_lossy(&output.stdout);
    assert!(prompt.contains("PROJECT RULES"), "{prompt}");
    assert!(prompt.contains("rust: Rust guidelines"), "{prompt}");
    assert!(prompt.contains("- read"), "{prompt}");
    assert!(prompt.contains("- skill"), "{prompt}");
}

#[test]
fn mcp_failure_does_not_block_prompt() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "mcp:\n  servers:\n    - name: broken\n      command: dshr-no-such-command\n",
    )
    .expect("config");

    dshr()
        .args(["--home", dir.path().to_str().expect("utf8"), "prompt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Tools"));
}

#[test]
fn run_delegates_to_subagent_from_cassette() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = dir.path().join("subagent.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"subagent"},
            {"type":"tool_input_delta","index":0,"json":"{\"prompt\":\"child task\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"child result"},
            {"type":"done","stop_reason":"end_turn"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("cassette");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--danger",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "delegate",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let listed = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions.as_array().expect("sessions").len(), 2);

    let searched = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "search",
            "child result",
            "--json",
        ])
        .output()
        .expect("search");
    let hits: serde_json::Value = serde_json::from_slice(&searched.stdout).expect("json");
    assert!(!hits.as_array().expect("hits").is_empty());
}

#[test]
fn plugin_manager_adds_lists_and_removes_mcp_servers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_str().expect("utf8");

    dshr()
        .args([
            "--home",
            home,
            "plugin",
            "add",
            "fs",
            "--command",
            "npx",
            "--arg",
            "-y",
            "--arg",
            "@modelcontextprotocol/server-filesystem",
            "--arg",
            ".",
        ])
        .assert()
        .success();

    let listed = dshr()
        .args(["--home", home, "plugin", "list", "--json"])
        .output()
        .expect("list");
    let servers: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(servers[0]["name"], "fs");
    assert_eq!(servers[0]["command"], "npx");

    dshr()
        .args(["--home", home, "plugin", "remove", "fs"])
        .assert()
        .success();

    dshr()
        .args(["--home", home, "plugin", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(no plugins)"));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn sandbox_blocks_writes_outside_workspace() {
    let home = tempfile::tempdir().expect("home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let outside = format!("dshr-outside-{}.txt", uuid::Uuid::new_v4().simple());
    let command = if cfg!(windows) {
        format!("cd /d \"%USERPROFILE%\" && echo no > {outside}")
    } else {
        format!("cd \"$HOME\" && echo no > {outside}")
    };
    let cassette = home.path().join("sandbox.json");
    let cassette_json = serde_json::json!({
        "model": "deepseek-flash",
        "calls": [
            [
                {"type": "start", "model": "deepseek-flash"},
                {"type": "tool_use_start", "index": 0, "id": "call_1", "name": "bash"},
                {"type": "tool_input_delta", "index": 0, "json": serde_json::json!({"command": command}).to_string()},
                {"type": "done", "stop_reason": "tool_use"}
            ],
            [
                {"type": "start", "model": "deepseek-flash"},
                {"type": "text_delta", "text": "done"},
                {"type": "done", "stop_reason": "end_turn"}
            ]
        ]
    });
    std::fs::write(&cassette, serde_json::to_vec(&cassette_json).expect("json")).expect("cassette");

    dshr()
        .current_dir(&project)
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "run",
            "--yes",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "try escape",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let user_home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(std::path::PathBuf::from)
        .expect("user home");
    assert!(!user_home.join(&outside).exists());
}

#[test]
fn run_executes_workflow_from_cassette() {
    let home = tempfile::tempdir().expect("home");
    let project = tempfile::tempdir().expect("project");
    let workflow_dir = project.path().join(".dshr/workflows");
    std::fs::create_dir_all(&workflow_dir).expect("workflow dir");
    std::fs::write(
        workflow_dir.join("demo.yaml"),
        "name: demo\nsteps:\n  - id: a\n    prompt: do the work\n",
    )
    .expect("workflow");

    let cassette = home.path().join("workflow.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"workflow"},
            {"type":"tool_input_delta","index":0,"json":"{\"name\":\"demo\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"child result"},
            {"type":"done","stop_reason":"end_turn"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("cassette");

    dshr()
        .current_dir(&project)
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "run",
            "--danger",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "use workflow demo",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let listed = dshr()
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions.as_array().expect("sessions").len(), 2);
}

#[test]
fn run_executes_agent_team_from_cassette() {
    let home = tempfile::tempdir().expect("home");
    let project = tempfile::tempdir().expect("project");
    let team_dir = project.path().join(".dshr/teams");
    std::fs::create_dir_all(&team_dir).expect("team dir");
    std::fs::write(
        team_dir.join("demo.yaml"),
        "name: demo\nworkers: 1\ntasks:\n  - id: a\n    title: A\n    prompt: do a\n",
    )
    .expect("team");

    let cassette = home.path().join("team.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"team"},
            {"type":"tool_input_delta","index":0,"json":"{\"name\":\"demo\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"task a result"},
            {"type":"done","stop_reason":"end_turn"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("cassette");

    dshr()
        .current_dir(&project)
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "run",
            "--danger",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "use team demo",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let listed = dshr()
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions.as_array().expect("sessions").len(), 2);
}

#[test]
fn schedule_add_list_disable_and_remove() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_str().expect("utf8");

    dshr()
        .args([
            "--home",
            home,
            "schedule",
            "add",
            "--name",
            "sch_demo",
            "--cron",
            "0 */5 * * * *",
            "--prompt",
            "hello",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("added sch_demo"));

    dshr()
        .args(["--home", home, "schedule", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sch_demo").and(predicate::str::contains("enabled")));

    dshr()
        .args(["--home", home, "schedule", "disable", "sch_demo"])
        .assert()
        .success();

    dshr()
        .args(["--home", home, "schedule", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("disabled"));

    dshr()
        .args(["--home", home, "schedule", "remove", "sch_demo"])
        .assert()
        .success();

    dshr()
        .args(["--home", home, "schedule", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(no schedules)"));
}

#[test]
fn schedule_run_due_executes_and_advances() {
    let home = tempfile::tempdir().expect("home");
    std::fs::write(home.path().join("schedules.yaml"), "[]").expect("schedules");

    let cassette = home.path().join("schedule.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[[
          {"type":"start","model":"deepseek-flash"},
          {"type":"text_delta","text":"scheduled ok"},
          {"type":"done","stop_reason":"end_turn"}
        ]]}"#,
    )
    .expect("cassette");

    std::fs::write(home.path().join("schedules.yaml"), "[]").expect("schedules");
    // Insert a due entry after the store has a valid file.
    std::fs::write(
        home.path().join("schedules.yaml"),
        r#"- id: sch_test
  cron: "0 */5 * * * *"
  prompt: hello
  enabled: true
  next_run: "2020-01-01T00:00:00Z"
"#,
    )
    .expect("due schedule");

    dshr()
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "schedule",
            "run-due",
            "--now",
            "2030-01-01T00:00:00Z",
            "--cassette",
            cassette.to_str().expect("utf8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ran sch_test: scheduled ok"));

    let stored = std::fs::read_to_string(home.path().join("schedules.yaml")).expect("read");
    assert!(stored.contains("2030-01-01T00:05:00Z"), "{stored}");

    let listed = dshr()
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions.as_array().expect("sessions").len(), 1);
    assert_eq!(sessions[0]["title"], "schedule: sch_test");
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn sandbox_allows_workspace_write() {
    let home = tempfile::tempdir().expect("home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let cassette = home.path().join("sandbox-inside.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"bash"},
            {"type":"tool_input_delta","index":0,"json":"{\"command\":\"echo ok > inside-sandbox.txt\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("cassette");

    dshr()
        .current_dir(&project)
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "run",
            "--yes",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "write inside",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    assert!(project.join("inside-sandbox.txt").exists());
}

#[test]
fn prompt_respects_tool_allowlist() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        "tools:\n  allowed: [read]\n",
    )
    .expect("config");

    dshr()
        .args(["--home", dir.path().to_str().expect("utf8"), "prompt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("- read"))
        .stdout(predicate::str::contains("- bash").not());
}

#[test]
fn run_sets_title_from_first_prompt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cassette = dir.path().join("title.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[[
          {"type":"start","model":"deepseek-flash"},
          {"type":"text_delta","text":"ok"},
          {"type":"done","stop_reason":"end_turn"}
        ]]}"#,
    )
    .expect("cassette");

    dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "title me",
        ])
        .assert()
        .success();

    let listed = dshr()
        .args([
            "--home",
            dir.path().to_str().expect("utf8"),
            "sessions",
            "list",
            "--json",
        ])
        .output()
        .expect("list");
    let sessions: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(sessions[0]["title"], "title me");
}

#[cfg(target_os = "linux")]
#[test]
fn sandbox_blocks_network_syscalls() {
    let home = tempfile::tempdir().expect("home");
    let project = home.path().join("project");
    std::fs::create_dir_all(&project).expect("project");
    let cassette = home.path().join("network.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"tool_use_start","index":0,"id":"call_1","name":"bash"},
            {"type":"tool_input_delta","index":0,"json":"{\"command\":\"python3 -c 'import socket; socket.socket()'\"}"},
            {"type":"done","stop_reason":"tool_use"}
          ],
          [
            {"type":"start","model":"deepseek-flash"},
            {"type":"text_delta","text":"done"},
            {"type":"done","stop_reason":"end_turn"}
          ]
        ]}"#,
    )
    .expect("cassette");

    dshr()
        .current_dir(&project)
        .args([
            "--home",
            home.path().to_str().expect("utf8"),
            "run",
            "--yes",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "network check",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    assert!(contains_text(
        &home.path().join("sessions"),
        "Operation not permitted"
    ));
}

#[cfg(target_os = "linux")]
fn contains_text(dir: &std::path::Path, needle: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if contains_text(&path, needle) {
                return true;
            }
        } else if std::fs::read_to_string(&path).is_ok_and(|text| text.contains(needle)) {
            return true;
        }
    }
    false
}

#[test]
fn plugin_manager_supports_http_servers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_str().expect("utf8");

    dshr()
        .args([
            "--home",
            home,
            "plugin",
            "add",
            "remote",
            "--url",
            "https://mcp.example.com/mcp",
        ])
        .assert()
        .success();

    let listed = dshr()
        .args(["--home", home, "plugin", "list", "--json"])
        .output()
        .expect("list");
    let servers: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("json");
    assert_eq!(servers[0]["url"], "https://mcp.example.com/mcp");
}

#[test]
fn sessions_reindex_and_fts_search() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_str().expect("utf8");
    let cassette = dir.path().join("fts.json");
    std::fs::write(
        &cassette,
        r#"{"model":"deepseek-flash","calls":[[
          {"type":"start","model":"deepseek-flash"},
          {"type":"text_delta","text":"fts unique needle"},
          {"type":"done","stop_reason":"end_turn"}
        ]]}"#,
    )
    .expect("cassette");

    dshr()
        .args([
            "--home",
            home,
            "run",
            "--cassette",
            cassette.to_str().expect("utf8"),
            "--model",
            "deepseek-flash",
            "hello",
        ])
        .assert()
        .success();

    dshr()
        .args(["--home", home, "sessions", "reindex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("indexed"));

    dshr()
        .args([
            "--home",
            home,
            "sessions",
            "search",
            "unique needle",
            "--fts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("unique needle"));
}

#[cfg(target_os = "windows")]
#[test]
fn windows_sandbox_helper_runs_cmd() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dshr()
        .args([
            "__sandbox-exec",
            "--mode",
            "workspace-write",
            "--workspace",
            dir.path().to_str().expect("utf8"),
            "--",
            "cmd",
            "/C",
            "echo ok > inside-helper.txt",
        ])
        .output()
        .expect("run helper");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "helper failed: {}\nstdout: {stdout}\nstderr: {stderr}",
        output.status
    );
    assert!(
        dir.path().join("inside-helper.txt").exists(),
        "file missing; stdout: {stdout}; stderr: {stderr}"
    );
}
