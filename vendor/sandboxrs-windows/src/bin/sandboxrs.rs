use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use sandboxrs_windows::{BackendPreference, Sandbox};
use serde::Serialize;

#[derive(Debug)]
enum Command {
    Doctor {
        json: bool,
    },
    Exec {
        json: bool,
        workspace: PathBuf,
        read_only: Vec<PathBuf>,
        read_write: Vec<PathBuf>,
        timeout_ms: Option<u64>,
        max_memory_bytes: Option<u64>,
        max_processes: Option<u32>,
        argv: Vec<String>,
    },
}

struct ExecOptions {
    json: bool,
    workspace: PathBuf,
    read_only: Vec<PathBuf>,
    read_write: Vec<PathBuf>,
    timeout_ms: Option<u64>,
    max_memory_bytes: Option<u64>,
    max_processes: Option<u32>,
    argv: Vec<String>,
}

fn main() -> ExitCode {
    if std::env::var_os("SANDBOXRS_PROBE_SELF").is_some() {
        println!("probe-ok");
        return ExitCode::SUCCESS;
    }
    match parse_args(std::env::args().skip(1)) {
        Ok(Command::Doctor { json }) => run_doctor(json),
        Ok(Command::Exec {
            json,
            workspace,
            read_only,
            read_write,
            timeout_ms,
            max_memory_bytes,
            max_processes,
            argv,
        }) => run_exec(ExecOptions {
            json,
            workspace,
            read_only,
            read_write,
            timeout_ms,
            max_memory_bytes,
            max_processes,
            argv,
        }),
        Err(message) => {
            eprintln!("sandboxrs: {message}");
            eprintln!("usage:");
            eprintln!("  sandboxrs doctor [--json]");
            eprintln!("  sandboxrs exec [--json] --workspace PATH [--ro PATH]... [--rw PATH]...");
            eprintln!(
                "    [--timeout-ms N] [--max-memory-bytes N] [--max-processes N] -- CMD ARGS..."
            );
            ExitCode::from(2)
        }
    }
}

fn run_doctor(json: bool) -> ExitCode {
    let probe = Sandbox::probe();
    if json {
        let value: Vec<_> = probe
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "backend": entry.backend.as_str(),
                    "export_present": entry.export_present,
                    "usable": entry.usable,
                    "detail": entry.detail,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "backends": value,
            }))
            .unwrap()
        );
    } else {
        println!("sandboxrs-windows {}", env!("CARGO_PKG_VERSION"));
        for entry in &probe.entries {
            println!("{}", entry.backend.label());
            println!("  export      {}", yes_no(entry.export_present));
            println!(
                "  probe       {}",
                if entry.usable {
                    "success"
                } else {
                    "unavailable"
                }
            );
            println!("  detail      {}", entry.detail);
        }
        println!("Admin required: no");
    }
    if probe.entries.iter().any(|entry| entry.usable) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn run_exec(options: ExecOptions) -> ExitCode {
    let ExecOptions {
        json,
        workspace,
        read_only,
        read_write,
        timeout_ms,
        max_memory_bytes,
        max_processes,
        argv,
    } = options;
    if argv.is_empty() {
        eprintln!("sandboxrs: exec requires a command after '--'");
        return ExitCode::from(2);
    }

    let mut builder = Sandbox::builder(workspace).preferred_backend(BackendPreference::Auto);
    for path in read_only {
        builder = builder.read_only(path);
    }
    for path in read_write {
        builder = builder.read_write(path);
    }
    if let Some(ms) = timeout_ms {
        builder = builder.timeout(Duration::from_millis(ms));
    }
    if let Some(bytes) = max_memory_bytes {
        builder = builder.max_memory(bytes);
    }
    if let Some(count) = max_processes {
        builder = builder.max_processes(count);
    }

    match builder.build().and_then(|sandbox| {
        let mut command = sandbox.command(&argv[0]);
        command.args(argv[1..].iter().cloned());
        command
            .stdin(sandboxrs_windows::Stdio::inherit())
            .stdout(sandboxrs_windows::Stdio::piped())
            .stderr(sandboxrs_windows::Stdio::piped());
        command.output()
    }) {
        Ok(output) => {
            if json {
                #[derive(Serialize)]
                struct MachineOutput {
                    backend: String,
                    exit_code: Option<i32>,
                    duration_ms: u128,
                    stdout: String,
                    stderr: String,
                }
                let result = MachineOutput {
                    backend: output.backend.as_str().to_string(),
                    exit_code: output.status.code(),
                    duration_ms: output.duration.as_millis(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                };
                println!(
                    "{}",
                    serde_json::to_string(&result).expect("serialize output")
                );
            } else {
                use std::io::Write;
                let mut stdout = std::io::stdout().lock();
                let _ = stdout.write_all(&output.stdout);
                let mut stderr = std::io::stderr().lock();
                let _ = stderr.write_all(&output.stderr);
            }
            exit_code_for(output.status)
        }
        Err(err) => {
            if json {
                println!("{}", serde_json::json!({ "error": err.to_string() }));
            } else {
                eprintln!("sandboxrs: {err}");
            }
            ExitCode::from(1)
        }
    }
}

fn exit_code_for(status: std::process::ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) => ExitCode::from(code.clamp(0, u8::MAX as i32) as u8),
        None => ExitCode::from(1),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn parse_args<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let Some(first) = args.first() else {
        return Err("missing subcommand".into());
    };
    match first.as_str() {
        "doctor" => Ok(Command::Doctor {
            json: args.iter().any(|arg| arg == "--json"),
        }),
        "exec" => parse_exec(&args[1..]),
        other => Err(format!("unknown subcommand '{other}'")),
    }
}

fn parse_exec(args: &[String]) -> Result<Command, String> {
    let mut json = false;
    let mut workspace = None;
    let mut read_only = Vec::new();
    let mut read_write = Vec::new();
    let mut timeout_ms = None;
    let mut max_memory_bytes = None;
    let mut max_processes = None;
    let mut argv = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--workspace" => {
                i += 1;
                workspace = Some(PathBuf::from(next_value(args, i, "--workspace")?));
            }
            "--ro" => {
                i += 1;
                read_only.push(PathBuf::from(next_value(args, i, "--ro")?));
            }
            "--rw" => {
                i += 1;
                read_write.push(PathBuf::from(next_value(args, i, "--rw")?));
            }
            "--timeout-ms" => {
                i += 1;
                timeout_ms = Some(
                    next_value(args, i, "--timeout-ms")?
                        .parse()
                        .map_err(|_| "invalid --timeout-ms value".to_string())?,
                );
            }
            "--max-memory-bytes" => {
                i += 1;
                max_memory_bytes = Some(
                    next_value(args, i, "--max-memory-bytes")?
                        .parse()
                        .map_err(|_| "invalid --max-memory-bytes value".to_string())?,
                );
            }
            "--max-processes" => {
                i += 1;
                max_processes = Some(
                    next_value(args, i, "--max-processes")?
                        .parse()
                        .map_err(|_| "invalid --max-processes value".to_string())?,
                );
            }
            "--" => {
                argv = args[i + 1..].to_vec();
                break;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
        i += 1;
    }

    let workspace = workspace.ok_or_else(|| "exec requires --workspace".to_string())?;
    if argv.is_empty() {
        return Err("exec requires a command after '--'".into());
    }
    Ok(Command::Exec {
        json,
        workspace,
        read_only,
        read_write,
        timeout_ms,
        max_memory_bytes,
        max_processes,
        argv,
    })
}

fn next_value(args: &[String], index: usize, option: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}
