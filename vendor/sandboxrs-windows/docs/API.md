# API guide

`sandboxrs-windows` is a Windows-only library for running child process trees
inside a reusable filesystem authority boundary. It deliberately follows the
shape of `std::process::Command`, while making backend selection, filesystem
grants, limits, and lifecycle ownership explicit.

## Build a sandbox

The workspace is always read-write. Add only the extra roots required by the
program, then build once and reuse the resulting `Sandbox` for commands with
the same authority.

```rust,no_run
use std::time::Duration;
use sandboxrs_windows::{BackendPreference, Sandbox};

let sandbox = Sandbox::builder(r"C:\work\project")
    .read_only(r"C:\Users\me\.cargo")
    .read_write(r"C:\work\cache")
    .timeout(Duration::from_secs(60))
    .max_memory(2 * 1024 * 1024 * 1024)
    .max_processes(24)
    .preferred_backend(BackendPreference::Auto)
    .build()?;
# Ok::<(), sandboxrs_windows::SandboxError>(())
```

`build()` performs live backend selection and policy setup. It fails when no
backend is usable; it never falls back to an unsandboxed child process.

`SandboxBuilder::identity()` accepts a diagnostic label only. The library
always appends a unique suffix, so two sandboxes cannot accidentally share an
AppContainer profile or its accumulated filesystem grants.

`BackendPreference::Auto` probes the Windows Sandbox API first and then the
regular AppContainer fallback. Use a specific preference only when deployment
requires that exact backend.

## Run commands

`SandboxCommand` supports arguments, environment changes, working directory,
and standard-stream settings.

```rust,no_run
use sandboxrs_windows::{Sandbox, Stdio};

# let sandbox = Sandbox::builder(r"C:\work\project").build()?;
let output = sandbox
    .command("git")
    .args(["status", "--short"])
    .current_dir(r"C:\work\project")
    .env("GIT_OPTIONAL_LOCKS", "0")
    .stdout(Stdio::piped())
    .output()?;

assert!(output.status.success());
println!("{}", String::from_utf8_lossy(&output.stdout));
# Ok::<(), sandboxrs_windows::SandboxError>(())
```

Stream defaults are intentionally the same as the standard library:

| Method | stdin | stdout | stderr |
| --- | --- | --- | --- |
| `spawn()` | inherit | inherit | inherit |
| `output()` | null | piped | piped |

An explicit `stdin`, `stdout`, or `stderr` setting takes precedence. With
`Stdio::piped()`, `SandboxChild` exposes the corresponding handle. Calling
`wait_with_output()` drains stdout and stderr concurrently, preventing a child
from blocking on a full output pipe.

## Lifecycle and limits

Each child is placed into a Windows Job Object before its main thread resumes.
`kill()` and dropping `SandboxChild` terminate the full descendant tree.
`timeout()` does the same after the configured duration. `max_processes()` and
`max_memory()` are Job Object limits, not cooperative hints.

`SandboxOutput` contains the root exit status, captured byte streams, selected
backend, and elapsed duration. Use `SandboxChild::try_wait()` for polling.

## Filesystem policy

All policy roots must be absolute and normalized. Overlapping rules resolve by
specificity: a more-specific rule takes precedence over a parent rule. Roots
not granted by the policy remain inaccessible to the child where the selected
Windows backend can enforce that distinction.

```text
C:\work\project              read-write (workspace)
C:\work\project\vendor       read-only (explicit child rule)
C:\Users\me\secrets          not granted
```

If a requested rule cannot be faithfully represented, `build()` returns
`SandboxError::UnsupportedPolicy`; it does not widen access silently.

## Errors and diagnostics

Use `Sandbox::probe()` or the `sandboxrs doctor --json` command to inspect
live backend availability. `SandboxError` distinguishes unavailable backends,
invalid paths, unrepresentable policy, process-creation errors, timeout, and
underlying I/O failures. A process creation failure includes the backend and,
when available, the Win32 error code.

The experimental Windows Sandbox API is capability-probed at runtime. On
systems where it is disabled, a usable AppContainer backend is selected by
`Auto`; requesting the unavailable backend explicitly fails.
