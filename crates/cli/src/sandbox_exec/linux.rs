//! Linux: apply Landlock, then exec the target command.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use landlock::{ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus};
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch, apply_filter};

pub fn execute(
    mode: &str,
    workspace: &Path,
    network: &str,
    program: OsString,
    args: Vec<OsString>,
) -> Result<ExitCode, String> {
    apply(mode, workspace, network)?;
    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(&program).args(&args).exec();
    Err(format!("exec {}: {error}", program.to_string_lossy()))
}

fn apply(mode: &str, workspace: &Path, network: &str) -> Result<(), String> {
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
    if network == "deny" {
        deny_network()?;
    }
    Ok(())
}

fn deny_network() -> Result<(), String> {
    let syscalls = [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_connect,
        libc::SYS_bind,
        libc::SYS_listen,
        libc::SYS_accept,
        libc::SYS_accept4,
        libc::SYS_sendto,
        libc::SYS_recvfrom,
        libc::SYS_sendmsg,
        libc::SYS_recvmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvmmsg,
        libc::SYS_getsockname,
        libc::SYS_getpeername,
        libc::SYS_setsockopt,
        libc::SYS_getsockopt,
        libc::SYS_shutdown,
    ];
    let arch = TargetArch::try_from(std::env::consts::ARCH)
        .map_err(|error| format!("seccomp arch: {error}"))?;
    let filter: BpfProgram = SeccompFilter::new(
        syscalls
            .into_iter()
            .map(|number| (number, Vec::new()))
            .collect(),
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|error| format!("seccomp filter: {error}"))?
    .try_into()
    .map_err(|error| format!("seccomp compile: {error}"))?;
    apply_filter(&filter).map_err(|error| format!("seccomp apply: {error}"))
}
