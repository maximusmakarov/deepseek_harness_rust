# Changelog

All notable changes to `sandboxrs-windows` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-08-11

### Fixed

- Made AppContainer piped stdio work with lower-integrity child tokens by
  replacing anonymous pipes with access-controlled local named pipes.
- Closed parent copies of child pipe endpoints after launch so readers receive
  EOF when the child exits.
- Made `SandboxCommand::output()` match `std::process::Command::output()`:
  stdin defaults to null and stdout/stderr are captured unless explicitly set.
- Made caller-provided identity labels unique per sandbox to prevent accidental
  AppContainer-profile sharing and filesystem grant bleed between policies.

## [0.1.0] - 2026-08-08

### Added

- Reusable `Sandbox` API with `SandboxBuilder`, `SandboxCommand`,
  `SandboxChild`, and `SandboxOutput`.
- Private `FilesystemPlan` normalization, conflict validation, and
  more-specific-wins overlap rules.
- Windows Sandbox API backend using `Experimental_CreateProcessInSandbox`
  with runtime capability probing and a real launch probe.
- AppContainer fallback using `rappct` profiles/ACLs and a private
  `CreateProcessW` launch path with Job Object containment.
- Job Object lifecycle: `KILL_ON_JOB_CLOSE`, process limits, memory limits,
  timeout tree teardown, and explicit kill.
- `sandboxrs` CLI with `exec`, `exec --json`, and `doctor`.
- Deterministic Evals V2.1 suite and standard-user CI benchmark.

[0.1.0]: https://github.com/TarunKurella/sandboxrs-windows/releases/tag/v0.1.0
[0.1.1]: https://github.com/TarunKurella/sandboxrs-windows/releases/tag/v0.1.1
