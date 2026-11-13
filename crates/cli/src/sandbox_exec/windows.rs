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
        .stdin(Stdio::Null)
        .stdout(Stdio::Piped)
        .stderr(Stdio::Piped)
        .spawn()
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    use std::io::Write;
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    let code = output.status.code().unwrap_or(1);
    Ok(ExitCode::from((code & 0xff) as u8))
}
