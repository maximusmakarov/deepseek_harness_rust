//! Windows: launch the target inside an AppContainer via sandboxrs-windows.

use std::ffi::OsString;
use std::path::Path;
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
        let base = std::env::temp_dir().join(format!("dshr-readonly-{}", std::process::id()));
        std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
        base
    } else {
        workspace.to_path_buf()
    };
    let mut builder = Sandbox::builder(&base);
    if mode == "read-only" {
        builder = builder.read_only(workspace);
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
