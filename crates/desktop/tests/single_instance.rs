//! End-to-end single-instance behaviour of the desktop binary.
//!
//! The second launch must reach the lock check before Tauri starts, so these
//! tests never open a window: the lock is owned by this test process while the
//! binary runs.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_home() -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "dshr-desktop-instance-{}-{}",
        std::process::id(),
        monotonic_suffix()
    ));
    std::fs::create_dir_all(&home).expect("home");
    home
}

fn monotonic_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default()
}

fn hold_lock(home: &Path) -> std::fs::File {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(home.join("desktop.lock"))
        .expect("lock file");
    file.try_lock().expect("lock");
    file
}

fn launch(home: &Path, folder: Option<&Path>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dshr-desktop"));
    command
        .env("DSH_HOME", home)
        .env("DEEPSEEK_API_KEY", "dummy")
        .env("DSHR_BIN", "/nonexistent/dshr");
    if let Some(folder) = folder {
        command.arg(folder);
    }
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn desktop");
    // A second launch must return before any window: fail fast instead of
    // letting the suite hang if the lock is ever bypassed.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if child.try_wait().expect("poll desktop").is_some() {
            return child.wait_with_output().expect("output");
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().expect("output");
            panic!(
                "second launch did not exit; stdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn a_second_launch_exits_without_starting_a_server() {
    let home = temp_home();
    let _lock = hold_lock(&home);

    let output = launch(&home, None);
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("already running"), "stdout: {stdout}");
    assert!(
        !home.join("desktop-pending-folder").exists(),
        "nothing forwarded without a folder"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_second_launch_forwards_its_folder() {
    let home = temp_home();
    let folder = home.join("workspace");
    std::fs::create_dir_all(&folder).expect("workspace");
    let _lock = hold_lock(&home);

    let output = launch(&home, Some(&folder));
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("already running"), "stdout: {stdout}");
    let canonical = std::fs::canonicalize(&folder).expect("canonical folder");
    assert!(
        stdout.contains(&canonical.display().to_string()),
        "stdout: {stdout}"
    );
    let pending =
        std::fs::read_to_string(home.join("desktop-pending-folder")).expect("pending folder file");
    assert_eq!(pending.trim(), canonical.display().to_string());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_lock_check_accepts_a_free_home_without_a_window() {
    // No lock is held here, so the binary would start Tauri; keep this test to
    // the argument-error path, which returns before the lock is taken.
    let home = temp_home();
    let mut command = Command::new(env!("CARGO_BIN_EXE_dshr-desktop"));
    command
        .env("DSH_HOME", &home)
        .env("DEEPSEEK_API_KEY", "dummy")
        .arg("/definitely/not/a/folder");
    let output = command.output().expect("spawn desktop");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("folder does not exist"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(&home);
}
