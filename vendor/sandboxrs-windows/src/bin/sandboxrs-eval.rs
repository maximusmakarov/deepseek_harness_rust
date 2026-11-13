#![cfg_attr(
    not(windows),
    allow(dead_code, unused_assignments, unused_imports, unused_variables)
)]

use std::fmt;
#[cfg(windows)]
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::time::{Duration, Instant};

#[cfg(windows)]
use sandboxrs_windows::{
    BackendKind, BackendPreference, Sandbox, SandboxError, SandboxOutput, Stdio,
};
use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum EvalOutcome {
    Pass,
    Escape,
    Error,
    Unsupported,
}

impl fmt::Display for EvalOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => f.write_str("pass"),
            Self::Escape => f.write_str("escape"),
            Self::Error => f.write_str("error"),
            Self::Unsupported => f.write_str("unsupported"),
        }
    }
}

#[derive(Serialize, Clone)]
struct Evidence {
    id: String,
    name: String,
    section: String,
    backend: String,
    outcome: EvalOutcome,
    precondition_ok: bool,
    process_started: bool,
    operation_attempted: bool,
    exit_code: Option<i32>,
    postcondition_ok: bool,
    detail: String,
}

impl Evidence {
    fn pass(
        id: &str,
        name: &str,
        section: &str,
        backend: &str,
        exit_code: Option<i32>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            section: section.into(),
            backend: backend.into(),
            outcome: EvalOutcome::Pass,
            precondition_ok: true,
            process_started: true,
            operation_attempted: true,
            exit_code,
            postcondition_ok: true,
            detail: detail.into(),
        }
    }

    fn escape(
        id: &str,
        name: &str,
        section: &str,
        backend: &str,
        exit_code: Option<i32>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            section: section.into(),
            backend: backend.into(),
            outcome: EvalOutcome::Escape,
            precondition_ok: true,
            process_started: true,
            operation_attempted: true,
            exit_code,
            postcondition_ok: false,
            detail: detail.into(),
        }
    }

    fn error(
        id: &str,
        name: &str,
        section: &str,
        backend: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            section: section.into(),
            backend: backend.into(),
            outcome: EvalOutcome::Error,
            precondition_ok: false,
            process_started: false,
            operation_attempted: false,
            exit_code: None,
            postcondition_ok: false,
            detail: detail.into(),
        }
    }

    fn unsupported(
        id: &str,
        name: &str,
        section: &str,
        backend: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            section: section.into(),
            backend: backend.into(),
            outcome: EvalOutcome::Unsupported,
            precondition_ok: false,
            process_started: false,
            operation_attempted: false,
            exit_code: None,
            postcondition_ok: false,
            detail: detail.into(),
        }
    }
}

#[derive(Serialize, Clone, Default)]
struct Score {
    pass: usize,
    escape: usize,
    error: usize,
    unsupported: usize,
}

impl Score {
    fn add(&mut self, outcome: EvalOutcome) {
        match outcome {
            EvalOutcome::Pass => self.pass += 1,
            EvalOutcome::Escape => self.escape += 1,
            EvalOutcome::Error => self.error += 1,
            EvalOutcome::Unsupported => self.unsupported += 1,
        }
    }
}

#[derive(Serialize, Clone, Default)]
struct EvalReport {
    os: String,
    admin: bool,
    privilege: String,
    mode: String,
    backends: Vec<BackendInfo>,
    runs: Vec<BackendRun>,
    security_score: Score,
    compatibility_score: Score,
    gate_ok: bool,
}

#[derive(Serialize, Clone)]
struct BackendInfo {
    backend: String,
    export_present: bool,
    usable: bool,
    detail: String,
}

#[derive(Serialize, Clone)]
struct BackendRun {
    backend: String,
    selected: Option<String>,
    security: Vec<Evidence>,
    compatibility: Vec<Evidence>,
    security_score: Score,
    compatibility_score: Score,
}

fn main() {
    if std::env::var_os("SANDBOXRS_PROBE_SELF").is_some() {
        println!("probe-ok");
        return;
    }
    let mut backend_arg = "all".to_string();
    let mut report_path = None;
    let mut require_standard_user = false;
    let mut allow_unsupported = false;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--backend" => {
                i += 1;
                backend_arg = args.get(i).cloned().unwrap_or_else(|| "all".into());
            }
            "--report" => {
                i += 1;
                report_path = args.get(i).cloned();
            }
            "--require-standard-user" => require_standard_user = true,
            "--allow-unsupported" => allow_unsupported = true,
            other => {
                eprintln!("unknown option: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    #[cfg(windows)]
    let report = run_windows_evals(&backend_arg, allow_unsupported);
    #[cfg(not(windows))]
    let report = EvalReport {
        os: std::env::consts::OS.to_string(),
        mode: backend_arg,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(path) = report_path {
        let path = PathBuf::from(&path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, &json);
    }
    println!("{json}");

    if require_standard_user && report.admin {
        eprintln!("eval ran elevated; standard-user requirement failed");
        std::process::exit(1);
    }
    if !report.gate_ok {
        eprintln!("security gate failed");
        std::process::exit(1);
    }
    if report.runs.is_empty() {
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn run_windows_evals(backend_arg: &str, allow_unsupported: bool) -> EvalReport {
    use std::fs;

    let attacker_source = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|dir| dir.join("sandboxrs-test-attacker.exe"))
        })
        .and_then(|path| path.to_str().map(str::to_owned))
        .expect("attacker helper must be built next to the eval binary");
    std::env::set_var("SANDBOXRS_ATTACKER", &attacker_source);

    let probe = Sandbox::probe();
    let backends: Vec<BackendInfo> = probe
        .entries
        .iter()
        .map(|entry| BackendInfo {
            backend: entry.backend.as_str().to_string(),
            export_present: entry.export_present,
            usable: entry.usable,
            detail: entry.detail.clone(),
        })
        .collect();

    let requested = match backend_arg {
        "auto" => vec![Sandbox::available_backends()
            .first()
            .copied()
            .unwrap_or(BackendKind::AppContainer)],
        "appcontainer" => vec![BackendKind::AppContainer],
        "windows-sandbox-api" | "modern" => vec![BackendKind::WindowsSandboxApi],
        _ => vec![BackendKind::WindowsSandboxApi, BackendKind::AppContainer],
    };

    let mut report = EvalReport {
        os: os_version(),
        admin: running_elevated(),
        privilege: if running_elevated() {
            "elevated"
        } else {
            "standard"
        }
        .into(),
        mode: backend_arg.into(),
        backends,
        ..Default::default()
    };

    let fixture = std::env::temp_dir().join(format!(
        "sandboxrs-eval-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let workspace = fixture.join("workspace");
    let readonly = fixture.join("readonly");
    let secret = fixture.join("secret");
    let outside = fixture.join("outside");
    for dir in [
        &workspace,
        &readonly,
        &secret,
        &outside,
        &workspace.join("nested"),
    ] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(workspace.join("allowed.txt"), b"allowed").unwrap();
    fs::write(readonly.join("readable.txt"), b"readable").unwrap();
    fs::write(secret.join("DO_NOT_READ.txt"), b"top-secret").unwrap();
    fs::write(outside.join("outside.txt"), b"outside").unwrap();
    let attacker_copy = workspace.join("attacker.exe");
    fs::copy(&attacker_source, &attacker_copy).expect("copy attacker into user-owned workspace");
    let attacker = attacker_copy
        .to_str()
        .expect("attacker workspace path must be UTF-8")
        .to_string();

    for backend in requested {
        let run = run_backend(backend, &workspace, &readonly, &secret, &outside, &attacker);
        report.runs.push(run);
    }

    for run in &report.runs {
        for evidence in &run.security {
            report.security_score.add(evidence.outcome);
        }
        for evidence in &run.compatibility {
            report.compatibility_score.add(evidence.outcome);
        }
    }

    let any_escape_or_error = report.runs.iter().any(|run| {
        run.security
            .iter()
            .any(|e| matches!(e.outcome, EvalOutcome::Escape | EvalOutcome::Error))
    });
    let required_executed = match backend_arg {
        "appcontainer" | "windows-sandbox-api" | "modern" | "auto" => {
            report.runs.iter().any(|run| {
                run.security
                    .iter()
                    .any(|e| e.outcome != EvalOutcome::Unsupported)
            })
        }
        _ => report.runs.iter().any(|run| {
            run.security
                .iter()
                .any(|e| e.outcome != EvalOutcome::Unsupported)
        }),
    };
    let required_ok = match backend_arg {
        "windows-sandbox-api" | "modern" => allow_unsupported || required_executed,
        _ => required_executed,
    };
    report.gate_ok = !any_escape_or_error && required_ok;
    let _ = fs_remove_all(&fixture);
    report
}

#[cfg(windows)]
fn run_backend(
    backend: BackendKind,
    workspace: &Path,
    readonly: &Path,
    secret: &Path,
    outside: &Path,
    attacker: &str,
) -> BackendRun {
    let backend_name = backend.as_str().to_string();
    let sandbox = match sandbox_for(backend, workspace)
        .read_only(readonly)
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(sandbox) => sandbox,
        Err(err) => {
            let unsupported = Evidence::unsupported(
                "backend.unavailable",
                "backend unavailable",
                "backend",
                &backend_name,
                err.to_string(),
            );
            return BackendRun {
                backend: backend_name,
                selected: None,
                security: vec![unsupported],
                compatibility: Vec::new(),
                security_score: Score {
                    unsupported: 1,
                    ..Default::default()
                },
                compatibility_score: Score::default(),
            };
        }
    };

    let mut security = Vec::new();
    let mut compatibility = Vec::new();

    filesystem_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        readonly,
        secret,
        outside,
        &backend_name,
    );
    descendant_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        secret,
        &backend_name,
    );
    reparse_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        outside,
        secret,
        &backend_name,
    );
    path_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        secret,
        &backend_name,
    );
    rename_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        outside,
        &backend_name,
    );
    hardlink_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        secret,
        &backend_name,
    );
    handle_suite(
        &mut security,
        &sandbox,
        attacker,
        workspace,
        secret,
        &backend_name,
    );
    job_breakaway_suite(&mut security, &sandbox, attacker, workspace, &backend_name);
    resource_suite(&mut security, backend, attacker, workspace, &backend_name);
    environment_suite(&mut security, &sandbox, attacker, workspace, &backend_name);
    cross_sandbox_isolation_suite(
        &mut security,
        backend,
        attacker,
        workspace,
        secret,
        &backend_name,
    );

    compatibility_suite(
        &mut compatibility,
        &sandbox,
        backend,
        attacker,
        workspace,
        &backend_name,
    );

    let mut security_score = Score::default();
    for evidence in &security {
        security_score.add(evidence.outcome);
    }
    let mut compatibility_score = Score::default();
    for evidence in &compatibility {
        compatibility_score.add(evidence.outcome);
    }

    BackendRun {
        backend: backend_name.clone(),
        selected: Some(sandbox.backend().as_str().to_string()),
        security,
        compatibility,
        security_score,
        compatibility_score,
    }
}

#[cfg(windows)]
fn sandbox_for(backend: BackendKind, workspace: &Path) -> sandboxrs_windows::SandboxBuilder {
    Sandbox::builder(workspace).preferred_backend(match backend {
        BackendKind::AppContainer => BackendPreference::AppContainer,
        BackendKind::WindowsSandboxApi => BackendPreference::WindowsSandboxApi,
    })
}

#[cfg(windows)]
fn filesystem_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    readonly: &Path,
    secret: &Path,
    outside: &Path,
    backend: &str,
) {
    let ws = workspace.join("allowed.txt");
    let ws_write = workspace.join("workspace-write.txt");
    let ws_delete = workspace.join("workspace-delete.txt");
    let ro_read = readonly.join("readable.txt");
    let ro_write = readonly.join("readable-new.txt");
    let ro_delete = readonly.join("readable.txt");
    let secret_read = secret.join("DO_NOT_READ.txt");
    let secret_write = secret.join("pwned.txt");
    let secret_delete = secret.join("DO_NOT_READ.txt");
    let secret_rename = secret.join("renamed.txt");
    let outside_read = outside.join("outside.txt");
    let outside_write = outside.join("pwned.txt");
    let outside_delete = outside.join("outside.txt");
    let outside_rename = outside.join("renamed.txt");

    allowed_case(
        security,
        "fs.workspace.read",
        "read workspace",
        sandbox,
        attacker,
        &["read", p(&ws)],
        workspace,
        backend,
        || true,
    );
    allowed_case(
        security,
        "fs.workspace.write",
        "write workspace",
        sandbox,
        attacker,
        &["write", p(&ws_write)],
        workspace,
        backend,
        || ws_write.exists(),
    );
    let delete_target = ws_delete.clone();
    fs::write(&ws_delete, b"x").unwrap();
    allowed_case(
        security,
        "fs.workspace.delete",
        "delete workspace",
        sandbox,
        attacker,
        &["delete", p(&ws_delete)],
        workspace,
        backend,
        || !delete_target.exists(),
    );
    allowed_case(
        security,
        "fs.readonly.read",
        "read readonly",
        sandbox,
        attacker,
        &["read", p(&ro_read)],
        workspace,
        backend,
        || true,
    );

    forbidden_case(
        security,
        "fs.readonly.write",
        "write readonly",
        sandbox,
        attacker,
        &["write", p(&ro_write)],
        workspace,
        backend,
        || !ro_write.exists(),
    );
    forbidden_case(
        security,
        "fs.readonly.delete",
        "delete readonly",
        sandbox,
        attacker,
        &["delete", p(&ro_delete)],
        workspace,
        backend,
        || ro_delete.exists(),
    );
    forbidden_case(
        security,
        "fs.secret.read",
        "read secret",
        sandbox,
        attacker,
        &["read", p(&secret_read)],
        workspace,
        backend,
        || true,
    );
    forbidden_case(
        security,
        "fs.secret.write",
        "write secret",
        sandbox,
        attacker,
        &["write", p(&secret_write)],
        workspace,
        backend,
        || !secret_write.exists(),
    );
    forbidden_case(
        security,
        "fs.secret.delete",
        "delete secret",
        sandbox,
        attacker,
        &["delete", p(&secret_delete)],
        workspace,
        backend,
        || secret_delete.exists(),
    );
    forbidden_case(
        security,
        "fs.secret.rename",
        "rename secret",
        sandbox,
        attacker,
        &["move", p(&secret_read), p(&secret_rename)],
        workspace,
        backend,
        || secret_read.exists() && !secret_rename.exists(),
    );
    forbidden_case(
        security,
        "fs.outside.read",
        "read outside",
        sandbox,
        attacker,
        &["read", p(&outside_read)],
        workspace,
        backend,
        || true,
    );
    forbidden_case(
        security,
        "fs.outside.write",
        "write outside",
        sandbox,
        attacker,
        &["write", p(&outside_write)],
        workspace,
        backend,
        || !outside_write.exists(),
    );
    forbidden_case(
        security,
        "fs.outside.delete",
        "delete outside",
        sandbox,
        attacker,
        &["delete", p(&outside_delete)],
        workspace,
        backend,
        || outside_delete.exists(),
    );
    forbidden_case(
        security,
        "fs.outside.rename",
        "rename outside",
        sandbox,
        attacker,
        &["move", p(&outside_read), p(&outside_rename)],
        workspace,
        backend,
        || outside_read.exists() && !outside_rename.exists(),
    );
}

#[cfg(windows)]
fn descendant_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    secret: &Path,
    backend: &str,
) {
    let ws_child = workspace.join("child-workspace.txt");
    let ws_grand = workspace.join("grandchild-workspace.txt");
    let ws_great = workspace.join("great-grandchild-workspace.txt");
    let secret_child = secret.join("child-pwned.txt");
    let secret_grand = secret.join("grandchild-pwned.txt");
    let secret_great = secret.join("great-grandchild-pwned.txt");

    allowed_case(
        security,
        "desc.child.write.workspace",
        "child write workspace",
        sandbox,
        attacker,
        &["spawn-write", p(&ws_child)],
        workspace,
        backend,
        || ws_child.exists(),
    );
    allowed_case(
        security,
        "desc.grandchild.write.workspace",
        "grandchild write workspace",
        sandbox,
        attacker,
        &["grandchild-write", p(&ws_grand)],
        workspace,
        backend,
        || ws_grand.exists(),
    );
    allowed_case(
        security,
        "desc.great-grandchild.write.workspace",
        "great-grandchild write workspace",
        sandbox,
        attacker,
        &["great-grandchild-write", p(&ws_great)],
        workspace,
        backend,
        || ws_great.exists(),
    );
    forbidden_case(
        security,
        "desc.child.write.secret",
        "child write secret",
        sandbox,
        attacker,
        &["spawn-write", p(&secret_child)],
        workspace,
        backend,
        || !secret_child.exists(),
    );
    forbidden_case(
        security,
        "desc.grandchild.write.secret",
        "grandchild write secret",
        sandbox,
        attacker,
        &["grandchild-write", p(&secret_grand)],
        workspace,
        backend,
        || !secret_grand.exists(),
    );
    forbidden_case(
        security,
        "desc.great-grandchild.write.secret",
        "great-grandchild write secret",
        sandbox,
        attacker,
        &["great-grandchild-write", p(&secret_great)],
        workspace,
        backend,
        || !secret_great.exists(),
    );
}

#[cfg(windows)]
fn reparse_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    outside: &Path,
    secret: &Path,
    backend: &str,
) {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    let dir_link = workspace.join("dir-symlink");
    let file_link = workspace.join("file-symlink.txt");
    let junction = workspace.join("junction");
    let nested_junction = workspace.join("nested").join("escape");

    let _ = fs_remove_all(&dir_link);
    let _ = fs::remove_file(&file_link);
    let _ = fs_remove_all(&junction);
    let _ = fs_remove_all(&nested_junction);

    let dir_created = symlink_dir(outside, &dir_link).is_ok() && dir_link.exists();
    let file_created =
        symlink_file(secret.join("DO_NOT_READ.txt"), &file_link).is_ok() && file_link.exists();
    let junction_created = create_junction(&junction, outside);
    let nested_created = create_junction(&nested_junction, secret);

    reparse_case(
        security,
        "reparse.dir-symlink.write",
        "directory symlink write",
        sandbox,
        attacker,
        workspace,
        &dir_link,
        backend,
        dir_created,
        false,
        false,
    );
    reparse_case(
        security,
        "reparse.file-symlink.read",
        "file symlink read",
        sandbox,
        attacker,
        workspace,
        &file_link,
        backend,
        file_created,
        false,
        true,
    );
    reparse_case(
        security,
        "reparse.junction.write",
        "NTFS junction write",
        sandbox,
        attacker,
        workspace,
        &junction,
        backend,
        junction_created,
        true,
        false,
    );
    reparse_case(
        security,
        "reparse.nested-junction.write",
        "nested junction write",
        sandbox,
        attacker,
        workspace,
        &nested_junction,
        backend,
        nested_created,
        true,
        false,
    );
}

#[cfg(windows)]
fn reparse_case(
    security: &mut Vec<Evidence>,
    id: &str,
    name: &str,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    link: &Path,
    backend: &str,
    created: bool,
    required: bool,
    read_mode: bool,
) {
    if !created {
        if required {
            security.push(Evidence::error(
                id,
                name,
                "reparse",
                backend,
                "link fixture could not be created",
            ));
        } else {
            security.push(Evidence::unsupported(
                id,
                name,
                "reparse",
                backend,
                "link creation requires privilege not available in this context",
            ));
        }
        return;
    }
    if read_mode {
        if native_succeeds(Command::new(attacker).arg("read").arg(p(link))) {
            let output = sandbox_output(sandbox, attacker, &["read", p(link)], workspace);
            match output {
                Ok(output) if !output.status.success() => {
                    security.push(Evidence::pass(
                        id,
                        name,
                        "reparse",
                        backend,
                        output.status.code(),
                        "sandbox blocked read through file symlink",
                    ));
                }
                Ok(output) => {
                    security.push(Evidence::escape(
                        id,
                        name,
                        "reparse",
                        backend,
                        output.status.code(),
                        "sandbox read through file symlink",
                    ));
                }
                Err(err) => {
                    security.push(Evidence::error(
                        id,
                        name,
                        "reparse",
                        backend,
                        format!("sandbox process did not start: {err}"),
                    ));
                }
            }
        } else {
            security.push(Evidence::error(
                id,
                name,
                "reparse",
                backend,
                "native read through file symlink failed; fixture is not a real escape vector",
            ));
        }
        return;
    }
    let target = link.join("pwned.txt");
    if native_succeeds(Command::new(attacker).arg("write").arg(p(&target))) {
        let _ = fs::remove_file(&target);
        let output = sandbox_output(sandbox, attacker, &["write", p(&target)], workspace);
        match output {
            Ok(output) if !output.status.success() && !target.exists() => {
                security.push(Evidence::pass(
                    id,
                    name,
                    "reparse",
                    backend,
                    output.status.code(),
                    format!(
                        "OS blocked write through reparse point, stderr={:?}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            Ok(output) if output.status.success() => {
                security.push(Evidence::escape(
                    id,
                    name,
                    "reparse",
                    backend,
                    output.status.code(),
                    "write through reparse point succeeded",
                ));
            }
            Ok(output) => {
                security.push(Evidence::error(
                    id,
                    name,
                    "reparse",
                    backend,
                    format!(
                        "write denied but postcondition invalid: {} stderr={:?}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            Err(err) => {
                security.push(Evidence::error(
                    id,
                    name,
                    "reparse",
                    backend,
                    format!("sandbox process did not start: {err}"),
                ));
            }
        }
    } else {
        security.push(Evidence::error(
            id,
            name,
            "reparse",
            backend,
            "native write through reparse point failed; fixture is not a real escape vector",
        ));
    }
}

#[cfg(windows)]
fn path_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    secret: &Path,
    backend: &str,
) {
    let target = secret.join("path-pwned.txt");
    let attacks: Vec<(&str, PathBuf)> = vec![
        (
            "dotdot relative",
            PathBuf::from(r"..\secret\path-pwned.txt"),
        ),
        (
            "workspace dotdot",
            workspace.join(r"..\secret\path-pwned.txt"),
        ),
        ("absolute path", target.clone()),
        ("case variation", secret.join("PATH-PWNED.TXT")),
        (
            "extended path",
            PathBuf::from(format!(r"\\?\{}", target.display())),
        ),
        (
            "relative path",
            PathBuf::from(r".\..\secret\path-pwned.txt"),
        ),
    ];
    for (name, path) in attacks {
        forbidden_case(
            security,
            &format!("path.{name}"),
            name,
            sandbox,
            attacker,
            &["write", p(&path)],
            workspace,
            backend,
            || !target.exists(),
        );
    }
}

#[cfg(windows)]
fn rename_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    outside: &Path,
    backend: &str,
) {
    let inside = workspace.join("move-target.txt");
    let outside_target = outside.join("move-pwned.txt");
    let _ = fs::remove_file(&inside);
    fs::write(&inside, b"x").unwrap();
    forbidden_case(
        security,
        "rename.inside-to-outside",
        "move inside to outside",
        sandbox,
        attacker,
        &["move", p(&inside), p(&outside_target)],
        workspace,
        backend,
        || inside.exists() && !outside_target.exists(),
    );
}

#[cfg(windows)]
fn hardlink_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    secret: &Path,
    backend: &str,
) {
    let outside_secret = secret.join("DO_NOT_READ.txt");
    let link = workspace.join("hardlink-leak.txt");
    let _ = fs::remove_file(&link);
    forbidden_case(
        security,
        "hardlink.outside-to-workspace",
        "hard link outside secret",
        sandbox,
        attacker,
        &["link", p(&outside_secret), p(&link)],
        workspace,
        backend,
        || !link.exists(),
    );
}

#[cfg(windows)]
fn handle_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    secret: &Path,
    backend: &str,
) {
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT};

    let file = match std::fs::OpenOptions::new()
        .read(true)
        .open(secret.join("DO_NOT_READ.txt"))
    {
        Ok(file) => file,
        Err(err) => {
            security.push(Evidence::error(
                "handles.open",
                "open secret file",
                "handles",
                backend,
                err.to_string(),
            ));
            return;
        }
    };
    let raw = file.as_raw_handle();
    // SAFETY: `raw` is a live handle owned by `file`; the flag only changes
    // inheritance.
    let ok = unsafe { SetHandleInformation(raw, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) };
    if ok == 0 {
        security.push(Evidence::error(
            "handles.inheritable",
            "mark handle inheritable",
            "handles",
            backend,
            "SetHandleInformation failed",
        ));
        return;
    }
    let value = format!("{:x}", raw as usize);
    if native_succeeds(Command::new(attacker).arg("read-handle").arg(&value)) {
        let output = sandbox_output(
            sandbox,
            attacker,
            &["read-handle", value.as_str()],
            workspace,
        );
        match output {
            Ok(output) if !output.status.success() => {
                security.push(Evidence::pass(
                    "handles.secret",
                    "inherited secret handle blocked",
                    "handles",
                    backend,
                    output.status.code(),
                    "sandbox did not inherit parent handle",
                ));
            }
            Ok(output) => {
                security.push(Evidence::escape(
                    "handles.secret",
                    "inherited secret handle blocked",
                    "handles",
                    backend,
                    output.status.code(),
                    "sandbox read through inherited handle",
                ));
            }
            Err(err) => {
                security.push(Evidence::error(
                    "handles.secret",
                    "inherited secret handle blocked",
                    "handles",
                    backend,
                    format!("process did not start: {err}"),
                ));
            }
        }
    } else {
        security.push(Evidence::error(
            "handles.secret",
            "inherited secret handle blocked",
            "handles",
            backend,
            "native child could not read the inheritable handle; control failed",
        ));
    }
}

#[cfg(windows)]
fn job_breakaway_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    backend: &str,
) {
    for (mode, id, name) in [
        (
            "detached",
            "job.detached",
            "detached child cannot outlive sandbox",
        ),
        (
            "breakaway",
            "job.breakaway",
            "breakaway child cannot outlive sandbox",
        ),
        (
            "suspended",
            "job.suspended",
            "suspended-resume child cannot outlive sandbox",
        ),
    ] {
        let pidfile = workspace.join(format!("breakaway-{mode}.pid"));
        let _ = fs::remove_file(&pidfile);

        // Native control: prove the same breakaway spawn is possible outside
        // the sandbox before the sandboxed run can take credit for blocking it.
        let native_status = Command::new(attacker)
            .args(["spawn-breakaway", mode, p(&pidfile)])
            .current_dir(workspace)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let native_pid: u32 = fs::read_to_string(&pidfile)
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        let native_ok = native_status
            .as_ref()
            .map(|status| status.success())
            .unwrap_or(false);
        if !native_ok && native_pid == 0 {
            // The host itself is inside a Job Object that denies
            // CREATE_BREAKAWAY_FROM_JOB/DETACHED_PROCESS, so this mode cannot
            // be exercised on this environment. That is not a sandbox pass.
            security.push(Evidence::unsupported(
                id,
                name,
                "job",
                backend,
                format!(
                    "native breakaway is blocked by the host Job Object \
                     (status={native_status:?}); not creditable to the sandbox"
                ),
            ));
            continue;
        }
        if !native_ok || native_pid == 0 || !process_exists(native_pid) {
            if native_pid != 0 {
                let _ = kill_process(native_pid);
            }
            security.push(Evidence::error(
                id,
                name,
                "job",
                backend,
                format!(
                    "native breakaway control failed (pid={native_pid}, status={native_status:?}); \
                     sandbox cannot be credited for this boundary"
                ),
            ));
            continue;
        }
        let _ = kill_process(native_pid);
        let _ = fs::remove_file(&pidfile);

        let mut parent = match sandbox
            .command(attacker)
            .arg("spawn-breakaway")
            .arg(mode)
            .arg(p(&pidfile))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(parent) => parent,
            Err(err) => {
                security.push(Evidence::error(
                    id,
                    name,
                    "job",
                    backend,
                    format!("process did not start: {err}"),
                ));
                continue;
            }
        };
        let status = parent.wait().ok();
        let child_pid: u32 = fs::read_to_string(&pidfile)
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        if child_pid == 0 {
            if status.as_ref().map(|s| s.success()).unwrap_or(false) {
                security.push(Evidence::error(
                    id,
                    name,
                    "job",
                    backend,
                    "child PID was not produced; test never exercised breakaway",
                ));
            } else {
                security.push(Evidence::pass(
                    id,
                    name,
                    "job",
                    backend,
                    status.and_then(|s| s.code()),
                    "breakaway creation was denied",
                ));
            }
            continue;
        }
        if !process_exists(child_pid) {
            security.push(Evidence::error(
                id,
                name,
                "job",
                backend,
                format!("child {child_pid} was not alive before kill"),
            ));
            continue;
        }
        drop(parent);
        std::thread::sleep(Duration::from_millis(300));
        if process_exists(child_pid) {
            security.push(Evidence::escape(
                id,
                name,
                "job",
                backend,
                None,
                format!("child {child_pid} outlived sandbox teardown"),
            ));
        } else {
            security.push(Evidence::pass(
                id,
                name,
                "job",
                backend,
                Some(0),
                format!("child {child_pid} died with sandbox"),
            ));
        }
    }
}

#[cfg(windows)]
fn resource_suite(
    security: &mut Vec<Evidence>,
    backend_kind: BackendKind,
    attacker: &str,
    workspace: &Path,
    backend: &str,
) {
    // Process-count boundary: 8 under limit succeeds; 1000 hits the limit.
    let process_ok = sandbox_for(backend_kind, workspace)
        .max_processes(16)
        .timeout(Duration::from_secs(30))
        .build();
    match process_ok {
        Ok(sandbox) => {
            let output8 = sandbox_output(&sandbox, attacker, &["spawn-many", "8"], workspace);
            match output8 {
                Ok(output) if output.status.success() => {
                    let output1000 =
                        sandbox_output(&sandbox, attacker, &["spawn-many", "1000"], workspace);
                    match output1000 {
                        Ok(output) if !output.status.success() => {
                            security.push(Evidence::pass(
                                "resource.process-boundary",
                                "process boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                "8 succeeded, 1000 contained",
                            ));
                        }
                        Ok(output) => {
                            security.push(Evidence::escape(
                                "resource.process-boundary",
                                "process boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                "1000-process bomb succeeded",
                            ));
                        }
                        Err(err) => {
                            security.push(Evidence::error(
                                "resource.process-boundary",
                                "process boundary",
                                "resources",
                                backend,
                                format!("bomb process did not start: {err}"),
                            ));
                        }
                    }
                }
                Ok(output) => {
                    security.push(Evidence::error(
                        "resource.process-boundary",
                        "process boundary",
                        "resources",
                        backend,
                        format!("8 processes did not succeed: {}", output.status),
                    ));
                }
                Err(err) => {
                    security.push(Evidence::error(
                        "resource.process-boundary",
                        "process boundary",
                        "resources",
                        backend,
                        format!("process did not start: {err}"),
                    ));
                }
            }
        }
        Err(err) => {
            security.push(Evidence::error(
                "resource.process-boundary",
                "process boundary",
                "resources",
                backend,
                format!("sandbox did not build: {err}"),
            ));
        }
    }

    // Memory boundary: 64 MB succeeds; 512 MB terminates.
    let memory_ok = sandbox_for(backend_kind, workspace)
        .max_memory(256 * 1024 * 1024)
        .timeout(Duration::from_secs(30))
        .build();
    match memory_ok {
        Ok(sandbox) => {
            let output64 =
                sandbox_output(&sandbox, attacker, &["allocate-memory", "64"], workspace);
            match output64 {
                Ok(output) if output.status.success() => {
                    let output512 =
                        sandbox_output(&sandbox, attacker, &["allocate-memory", "512"], workspace);
                    match output512 {
                        Ok(output) if !output.status.success() => {
                            security.push(Evidence::pass(
                                "resource.memory-boundary",
                                "memory boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                "64 MB succeeded, 512 MB terminated",
                            ));
                        }
                        Ok(output) => {
                            security.push(Evidence::escape(
                                "resource.memory-boundary",
                                "memory boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                "512 MB allocation succeeded",
                            ));
                        }
                        Err(err) => {
                            security.push(Evidence::error(
                                "resource.memory-boundary",
                                "memory boundary",
                                "resources",
                                backend,
                                format!("allocation process did not start: {err}"),
                            ));
                        }
                    }
                }
                Ok(output) => {
                    security.push(Evidence::error(
                        "resource.memory-boundary",
                        "memory boundary",
                        "resources",
                        backend,
                        format!("64 MB allocation did not succeed: {}", output.status),
                    ));
                }
                Err(err) => {
                    security.push(Evidence::error(
                        "resource.memory-boundary",
                        "memory boundary",
                        "resources",
                        backend,
                        format!("process did not start: {err}"),
                    ));
                }
            }
        }
        Err(err) => {
            security.push(Evidence::error(
                "resource.memory-boundary",
                "memory boundary",
                "resources",
                backend,
                format!("sandbox did not build: {err}"),
            ));
        }
    }

    // Timeout boundary: 1s under 5s succeeds; 60s under 2s times out.
    let timeout_ok = sandbox_for(backend_kind, workspace)
        .timeout(Duration::from_secs(5))
        .build();
    match timeout_ok {
        Ok(sandbox) => {
            let short = sandbox_output(&sandbox, attacker, &["sleep", "1"], workspace);
            match short {
                Ok(output) if output.status.success() => {
                    let timeout_sandbox = sandbox_for(backend_kind, workspace)
                        .timeout(Duration::from_secs(2))
                        .build()
                        .expect("timeout sandbox");
                    let start = Instant::now();
                    let long =
                        sandbox_output(&timeout_sandbox, attacker, &["sleep", "60"], workspace);
                    match long {
                        Ok(output)
                            if !output.status.success()
                                && start.elapsed() < Duration::from_secs(15) =>
                        {
                            security.push(Evidence::pass(
                                "resource.timeout-boundary",
                                "timeout boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                format!("short succeeded; long timed out in {:?}", start.elapsed()),
                            ));
                        }
                        Ok(output) => {
                            security.push(Evidence::escape(
                                "resource.timeout-boundary",
                                "timeout boundary",
                                "resources",
                                backend,
                                output.status.code(),
                                "long sleep was not terminated",
                            ));
                        }
                        Err(err) => {
                            security.push(Evidence::error(
                                "resource.timeout-boundary",
                                "timeout boundary",
                                "resources",
                                backend,
                                format!("long process did not start: {err}"),
                            ));
                        }
                    }
                }
                Ok(output) => {
                    security.push(Evidence::error(
                        "resource.timeout-boundary",
                        "timeout boundary",
                        "resources",
                        backend,
                        format!("1s sleep did not succeed: {}", output.status),
                    ));
                }
                Err(err) => {
                    security.push(Evidence::error(
                        "resource.timeout-boundary",
                        "timeout boundary",
                        "resources",
                        backend,
                        format!("process did not start: {err}"),
                    ));
                }
            }
        }
        Err(err) => {
            security.push(Evidence::error(
                "resource.timeout-boundary",
                "timeout boundary",
                "resources",
                backend,
                format!("sandbox did not build: {err}"),
            ));
        }
    }
}

#[cfg(windows)]
fn environment_suite(
    security: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    attacker: &str,
    workspace: &Path,
    backend: &str,
) {
    std::env::set_var("SANDBOXRS_TEST_SECRET", "super-secret-value");
    let mut command = sandbox.command(attacker);
    command
        .env_clear()
        .args(["env", "SANDBOXRS_TEST_SECRET"])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if stdout.contains("super-secret-value") {
                security.push(Evidence::escape(
                    "env.clear-secret",
                    "env_clear removes secret",
                    "environment",
                    backend,
                    output.status.code(),
                    "secret leaked through env_clear",
                ));
            } else {
                let cwd = std::env::current_dir()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|_| "unknown".into());
                security.push(Evidence::pass(
                    "env.clear-secret",
                    "env_clear removes secret",
                    "environment",
                    backend,
                    output.status.code(),
                    format!("secret absent; cwd={cwd}; stderr={stderr:?}"),
                ));
            }
        }
        Err(err) => {
            security.push(Evidence::error(
                "env.clear-secret",
                "env_clear removes secret",
                "environment",
                backend,
                format!("process did not start: {err}"),
            ));
        }
    }
}

#[cfg(windows)]
fn cross_sandbox_isolation_suite(
    security: &mut Vec<Evidence>,
    backend_kind: BackendKind,
    attacker: &str,
    workspace_a: &Path,
    secret: &Path,
    backend: &str,
) {
    let workspace_b = std::env::temp_dir().join(format!(
        "sandboxrs-eval-b-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs_remove_all(&workspace_b);
    fs::create_dir_all(&workspace_b).unwrap();
    let sandbox_a = sandbox_for(backend_kind, workspace_a)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let sandbox_b = sandbox_for(backend_kind, &workspace_b)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let a_write = workspace_a.join("concurrent-a.txt");
    let b_write = workspace_b.join("concurrent-b.txt");
    allowed_case(
        security,
        "concurrent.a-write-a",
        "sandbox A writes A",
        &sandbox_a,
        attacker,
        &["write", p(&a_write)],
        workspace_a,
        backend,
        || a_write.exists(),
    );
    forbidden_case(
        security,
        "concurrent.a-write-b",
        "sandbox A writes B",
        &sandbox_a,
        attacker,
        &["write", p(&b_write)],
        workspace_a,
        backend,
        || !b_write.exists(),
    );
    allowed_case(
        security,
        "concurrent.b-write-b",
        "sandbox B writes B",
        &sandbox_b,
        attacker,
        &["write", p(&b_write)],
        &workspace_b,
        backend,
        || b_write.exists(),
    );
    forbidden_case(
        security,
        "concurrent.b-write-a",
        "sandbox B writes A",
        &sandbox_b,
        attacker,
        &["write", p(&a_write)],
        &workspace_b,
        backend,
        || !a_write.exists(),
    );
    let _ = secret;
    let _ = fs_remove_all(&workspace_b);
}

#[cfg(windows)]
fn compatibility_suite(
    compatibility: &mut Vec<Evidence>,
    sandbox: &Sandbox,
    backend_kind: BackendKind,
    attacker: &str,
    workspace: &Path,
    backend: &str,
) {
    for (name, program, args) in [
        ("cmd", "cmd", vec!["/c", "echo", "hello"]),
        ("git", "git", vec!["--version"]),
        ("node", "node", vec!["--version"]),
        ("python", "python", vec!["--version"]),
        ("cargo", "cargo", vec!["--version"]),
    ] {
        if !native_succeeds(Command::new(program).arg(args[0])) {
            continue;
        }
        let output = sandbox_output(sandbox, program, &args, workspace);
        match output {
            Ok(output) if output.status.success() => {
                compatibility.push(Evidence::pass(
                    &format!("compat.{name}"),
                    name,
                    "compatibility",
                    backend,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                ));
            }
            Ok(output) => {
                compatibility.push(Evidence::error(
                    &format!("compat.{name}"),
                    name,
                    "compatibility",
                    backend,
                    format!(
                        "{} status={} stderr={:?}",
                        output.status,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            Err(err) => {
                compatibility.push(Evidence::error(
                    &format!("compat.{name}"),
                    name,
                    "compatibility",
                    backend,
                    err.to_string(),
                ));
            }
        }
    }

    // Real cargo workload: build and run a tiny fixture with a hostile build.rs.
    let fixture = workspace.join("malicious-rust");
    let _ = fs_remove_all(&fixture);
    fs::create_dir_all(fixture.join("src")).unwrap();
    fs::write(
        fixture.join("Cargo.toml"),
        "[package]\nname = \"malicious-rust\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        fixture.join("src/main.rs"),
        "fn main() { println!(\"hostile-ok\"); }\n",
    )
    .unwrap();
    let hostile_root = workspace.parent().unwrap_or(workspace);
    let secret_path = hostile_root.join("sandboxrs-hostile-secret.txt");
    let outside_target = hostile_root.join("sandboxrs-hostile-outside.txt");
    let child_target = hostile_root.join("sandboxrs-hostile-child.txt");
    let leak_path = fixture.join("LEAKED_SECRET.txt");
    fs::write(&secret_path, b"top-secret").unwrap();
    fs::write(
        fixture.join("build.rs"),
        format!(
            r#"fn main() {{
    let secret = std::path::Path::new(r"{secret}");
    if let Ok(contents) = std::fs::read(secret) {{
        std::fs::write("LEAKED_SECRET.txt", contents).unwrap();
    }}
    let _ = std::fs::write(r"{outside}", b"pwned");
    let _ = std::process::Command::new(r"{attacker}")
        .args(["write", r"{child_target}"])
        .status();
}}
"#,
            secret = secret_path.display(),
            outside = outside_target.display(),
            attacker = attacker,
            child_target = child_target.display(),
        ),
    )
    .unwrap();
    let cargo_sandbox = match sandbox_for(backend_kind, &fixture)
        .read_only(
            std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:\\Users\\runneradmin\\.cargo")),
        )
        .read_only(
            std::env::var_os("RUSTUP_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:\\Users\\runneradmin\\.rustup")),
        )
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(sandbox) => sandbox,
        Err(err) => {
            compatibility.push(Evidence::error(
                "compat.cargo-build",
                "cargo build malicious fixture",
                "compatibility",
                backend,
                format!("sandbox did not build: {err}"),
            ));
            let _ = attacker;
            return;
        }
    };
    let output = sandbox_output(&cargo_sandbox, "cargo", &["build"], &fixture);
    match output {
        Ok(output)
            if output.status.success()
                && !leak_path.exists()
                && !outside_target.exists()
                && !child_target.exists() =>
        {
            compatibility.push(Evidence::pass(
                "compat.cargo-build",
                "cargo build malicious fixture",
                "compatibility",
                backend,
                output.status.code(),
                "build succeeded; secret read, outside write, and child escape all stayed blocked",
            ));
        }
        Ok(output) if output.status.success() => {
            compatibility.push(Evidence::escape(
                "compat.cargo-build",
                "cargo build malicious fixture",
                "compatibility",
                backend,
                output.status.code(),
                format!(
                    "build succeeded but hostile build.rs leaked: leak={} outside={} child={}",
                    leak_path.exists(),
                    outside_target.exists(),
                    child_target.exists()
                ),
            ));
        }
        Ok(output) => {
            compatibility.push(Evidence::error(
                "compat.cargo-build",
                "cargo build malicious fixture",
                "compatibility",
                backend,
                format!(
                    "status={} stderr={:?}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        Err(err) => {
            compatibility.push(Evidence::error(
                "compat.cargo-build",
                "cargo build malicious fixture",
                "compatibility",
                backend,
                format!("process did not start: {err}"),
            ));
        }
    }
    let _ = fs::remove_file(&leak_path);
    let _ = fs::remove_file(&outside_target);
    let _ = fs::remove_file(&child_target);
    let _ = fs::remove_file(&secret_path);
}

#[cfg(windows)]
fn allowed_case(
    security: &mut Vec<Evidence>,
    id: &str,
    name: &str,
    sandbox: &Sandbox,
    attacker: &str,
    args: &[&str],
    cwd: &Path,
    backend: &str,
    postcondition: impl FnOnce() -> bool,
) {
    let output = sandbox_output(sandbox, attacker, args, cwd);
    match output {
        Ok(output) if output.status.success() && postcondition() => {
            security.push(Evidence::pass(
                id,
                name,
                "filesystem",
                backend,
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ));
        }
        Ok(output) => {
            security.push(Evidence::error(
                id,
                name,
                "filesystem",
                backend,
                format!(
                    "allowed operation failed or postcondition invalid: {} stderr={:?}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        Err(err) => {
            security.push(Evidence::error(
                id,
                name,
                "filesystem",
                backend,
                format!("process did not start: {err}"),
            ));
        }
    }
}

#[cfg(windows)]
fn forbidden_case(
    security: &mut Vec<Evidence>,
    id: &str,
    name: &str,
    sandbox: &Sandbox,
    attacker: &str,
    args: &[&str],
    cwd: &Path,
    backend: &str,
    postcondition: impl FnOnce() -> bool,
) {
    if !native_succeeds(Command::new(attacker).args(args).current_dir(cwd)) {
        security.push(Evidence::error(
            id,
            name,
            "security",
            backend,
            "native control failed; attack is not possible outside the sandbox",
        ));
        return;
    }
    restore_fixture_state(args, cwd);
    let output = sandbox_output(sandbox, attacker, args, cwd);
    match output {
        Ok(output) if output.status.success() => {
            security.push(Evidence::escape(
                id,
                name,
                "security",
                backend,
                output.status.code(),
                "forbidden operation succeeded inside sandbox",
            ));
        }
        Ok(output) if !output.status.success() && postcondition() => {
            security.push(Evidence::pass(
                id,
                name,
                "security",
                backend,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        Ok(output) => {
            security.push(Evidence::error(
                id,
                name,
                "security",
                backend,
                format!(
                    "blocked but postcondition invalid: {} stderr={:?}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        Err(err) => {
            security.push(Evidence::error(
                id,
                name,
                "security",
                backend,
                format!("process did not start; attack was not actually attempted: {err}"),
            ));
        }
    }
}

#[cfg(windows)]
fn restore_fixture_state(args: &[&str], cwd: &Path) {
    // The native control intentionally mutates the same path family. Restore
    // the fixture to its pre-attack state so the sandbox postcondition checks
    // the sandbox result, not leftover control side effects.
    let op = args.first().copied().unwrap_or("");
    let resolved = |index: usize| -> Option<PathBuf> {
        let raw = args.get(index)?;
        let path = Path::new(raw);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            Some(cwd.join(path))
        }
    };
    match op {
        "write" | "spawn-write" | "grandchild-write" | "great-grandchild-write" => {
            if let Some(path) = resolved(1) {
                let _ = fs::remove_file(path);
            }
        }
        "link" => {
            if let Some(path) = resolved(2) {
                let _ = fs::remove_file(path);
            }
        }
        "delete" | "spawn-delete" | "grandchild-delete" | "great-grandchild-delete" => {
            if let Some(path) = resolved(1) {
                if !path.exists() {
                    let _ = fs::write(path, b"x");
                }
            }
        }
        "move" | "spawn-move" | "grandchild-move" => {
            if let Some(source) = resolved(1) {
                if !source.exists() {
                    let _ = fs::write(source, b"x");
                }
            }
            if let Some(destination) = resolved(2) {
                let _ = fs::remove_file(destination);
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
fn sandbox_output(
    sandbox: &Sandbox,
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<SandboxOutput, SandboxError> {
    let mut command = sandbox.command(program);
    command.args(args);
    command.current_dir(cwd);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command.output()
}

#[cfg(windows)]
fn native_succeeds(command: &mut Command) -> bool {
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: OpenProcess with a pid is a normal Win32 query.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut code = 0u32;
    // SAFETY: The handle is valid and `code` is a valid out-parameter.
    let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
    let active = ok != 0 && code == STILL_ACTIVE as u32;
    // SAFETY: The handle was opened above and is no longer needed.
    unsafe {
        CloseHandle(handle);
    }
    active
}

#[cfg(windows)]
fn kill_process(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    };
    // SAFETY: pid belongs to the control child and the requested access rights
    // are standard process termination rights.
    let handle = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return false;
    }
    // SAFETY: The handle is valid and owned by this function.
    let ok = unsafe { TerminateProcess(handle, 1) };
    if ok != 0 {
        unsafe {
            let _ = WaitForSingleObject(handle, 5000);
        }
    }
    // SAFETY: The handle is closed exactly once after use.
    unsafe {
        let _ = CloseHandle(handle);
    }
    ok != 0
}

#[cfg(windows)]
fn create_junction(link: &Path, target: &Path) -> bool {
    let output = Command::new("cmd")
        .args(["/c", "mklink", "/J", p(link), p(target)])
        .output()
        .ok();
    output.map(|o| o.status.success()).unwrap_or(false) && link.exists()
}

#[cfg(windows)]
fn fs_remove_all(path: &Path) -> std::io::Result<()> {
    std::fs::remove_dir_all(path)
}

#[cfg(windows)]
fn p(path: &Path) -> &str {
    path.to_str()
        .expect("path must be valid UTF-8 for eval args")
}

#[cfg(windows)]
fn running_elevated() -> bool {
    Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn os_version() -> String {
    Command::new("cmd")
        .args(["/c", "ver"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| std::env::consts::OS.to_string())
}
