//! Hidden helper that applies the platform sandbox, then runs the target.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

pub fn run() -> ExitCode {
    match run_inner() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("dshr sandbox: {error}");
            ExitCode::from(125)
        }
    }
}

fn run_inner() -> Result<ExitCode, String> {
    let mut args = std::env::args_os().skip(2);
    let mut mode = "workspace-write".to_owned();
    let mut workspace = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut network = "deny".to_owned();

    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--mode") => {
                mode = args
                    .next()
                    .and_then(|value| value.to_str().map(ToOwned::to_owned))
                    .ok_or("missing --mode value")?;
            }
            Some("--workspace") => {
                workspace = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("missing --workspace value")?;
            }
            Some("--network") => {
                network = args
                    .next()
                    .and_then(|value| value.to_str().map(ToOwned::to_owned))
                    .ok_or("missing --network value")?;
            }
            Some("--") => break,
            other => return Err(format!("unexpected argument: {other:?}")),
        }
    }

    let program = args.next().ok_or("missing command")?;
    let rest = args.collect::<Vec<_>>();
    platform_execute(&mode, &workspace, &network, program, rest)
}

#[cfg(target_os = "linux")]
fn platform_execute(
    mode: &str,
    workspace: &Path,
    network: &str,
    program: OsString,
    args: Vec<OsString>,
) -> Result<ExitCode, String> {
    linux::execute(mode, workspace, network, program, args)
}

#[cfg(target_os = "windows")]
fn platform_execute(
    mode: &str,
    workspace: &Path,
    network: &str,
    program: OsString,
    args: Vec<OsString>,
) -> Result<ExitCode, String> {
    windows::execute(mode, workspace, network, program, args)
}
