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
