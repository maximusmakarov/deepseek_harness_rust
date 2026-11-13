//! Windows: launch the target inside an AppContainer via sandboxrs-windows.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sandboxrs_windows::{Sandbox, Stdio};

pub fn execute(
    mode: &str,
    workspace: &Path,
    _network: &str,
    program: OsString,
    args: Vec<OsString>,
) -> Result<ExitCode, String> {
    let base = if mode == "read-only" {
        std::env::temp_dir()
    } else {
        workspace.to_path_buf()
    };
    let mut builder = Sandbox::builder(&base).read_write(std::env::temp_dir());
    if mode == "read-only" {
        builder = builder.read_only(workspace);
    }
    for root in read_only_roots() {
        builder = builder.read_only(root);
    }
    let sandbox = builder.build().map_err(|error| error.to_string())?;

    let mut child = sandbox
        .command(&program)
        .args(&args)
        .current_dir(workspace)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| error.to_string())?;
    let status = child.wait().map_err(|error| error.to_string())?;
    let code = status.code().unwrap_or(1);
    Ok(ExitCode::from((code & 0xff) as u8))
}

fn read_only_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for key in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ] {
        if let Some(value) = std::env::var_os(key) {
            roots.push(PathBuf::from(value));
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(&home).join(".cargo"));
        roots.push(PathBuf::from(&home).join(".rustup"));
    }
    roots
}
