//! Hidden Linux helper: apply Landlock, then exec the target command.
//!
//! Invoked by `dshr` itself through `dshr-sandbox`'s Linux backend. Keeping the
//! confinement in a separate process avoids `unsafe` process hooks and makes
//! the wrapper testable as a normal executable.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use landlock::{ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus};

pub fn run() -> ExitCode {
    match run_inner() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dshr sandbox: {error}");
            ExitCode::from(125)
        }
    }
}

fn run_inner() -> Result<(), String> {
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
    apply(&mode, &workspace, &network)?;

    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(&program).args(&rest).exec();
    Err(format!("exec {}: {error}", program.to_string_lossy()))
}

fn apply(mode: &str, workspace: &Path, _network: &str) -> Result<(), String> {
    if mode == "danger-full-access" {
        return Ok(());
    }
    let abi = ABI::V1;
    let all = AccessFs::from_all(abi);
    let read = AccessFs::from_read(abi);

    let mut ruleset = Ruleset::default()
        .handle_access(all)
        .map_err(|error| format!("handle_access: {error}"))?
        .create()
        .map_err(|error| format!("create ruleset: {error}"))?;

    ruleset = ruleset
        .add_rules(landlock::path_beneath_rules([Path::new("/")], read))
        .map_err(|error| format!("allow read: {error}"))?;

    match mode {
        "read-only" => {
            ruleset = ruleset
                .add_rules(landlock::path_beneath_rules(
                    [Path::new("/dev/null")],
                    AccessFs::WriteFile,
                ))
                .map_err(|error| format!("allow /dev/null: {error}"))?;
        }
        "workspace-write" => {
            let mut paths = vec![workspace.to_path_buf(), std::env::temp_dir()];
            let dev_null = PathBuf::from("/dev/null");
            if dev_null.exists() {
                paths.push(dev_null);
            }
            ruleset = ruleset
                .add_rules(landlock::path_beneath_rules(paths, all))
                .map_err(|error| format!("allow workspace: {error}"))?;
        }
        other => return Err(format!("unknown mode '{other}'")),
    }

    let status = ruleset
        .restrict_self()
        .map_err(|error| format!("restrict_self: {error}"))?;
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err("Landlock is not enforced by this kernel".into());
    }
    Ok(())
}
