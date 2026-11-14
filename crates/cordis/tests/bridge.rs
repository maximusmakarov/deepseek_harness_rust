//! Runs the JS bridge harness (tests/bridge_harness.mjs) under node.
//!
//! The harness stubs the iframe DOM and asserts the protocol and lifecycle
//! contract: reference-shaped slot registration, `inject` groups, `effect`
//! disposal, theme subscription, activate/deactivate and `ctx.action`.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn bridge_protocol_and_lifecycle() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is not installed; skipping the bridge harness");
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .arg(root.join("tests/bridge_harness.mjs"))
        .arg(root.join("src/bridge.js"))
        .output()
        .expect("run node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "bridge harness failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("all checks passed"), "{stdout}");
}
