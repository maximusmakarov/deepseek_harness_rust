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
