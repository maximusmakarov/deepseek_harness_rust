#![cfg(windows)]

use std::path::Path;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_dshr")
}

fn run(mode: &str, workspace: &Path, command: &str) -> std::process::Output {
    Command::new(binary())
        .args([
            "__sandbox-exec",
            "--mode",
            mode,
            "--workspace",
            &workspace.display().to_string(),
            "--",
            "cmd",
            "/C",
            command,
        ])
        .output()
        .expect("sandbox exec")
}

#[test]
#[ignore = "AppContainer opt-in; run on a real Windows host"]
fn appcontainer_allows_workspace_write_and_denies_outside() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let inside = workspace.join("inside.txt");

    let output = run(
        "workspace-write",
        &workspace,
        &format!("echo ok>\"{}\"", inside.display()),
    );
    assert!(
        output.status.success(),
        "inside write failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read_to_string(&inside).expect("inside").trim(), "ok");

    let outside = dir.path().join("outside.txt");
    let _ = run(
        "workspace-write",
        &workspace,
        &format!("echo no>\"{}\"", outside.display()),
    );
    assert!(!outside.exists(), "outside write was not denied");
}
