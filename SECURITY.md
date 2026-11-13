# Security

## Supported versions

Only the current `main` / latest release receives security fixes.

## Reporting

Please use GitHub **private security advisories** for this repository. Do not open a public issue with exploit details.

Include: affected version/commit, reproduction, impact, and whether a sandbox boundary is involved.

## Trust model

- Model output is untrusted; tools still execute with the user's OS permissions unless a sandbox is active.
- Approvals gate shell and out-of-workspace writes; `--danger` disables them by explicit choice.
- Sandbox: macOS Seatbelt, Linux Landlock + seccomp. Windows is **fail-closed** by default; AppContainer is experimental opt-in and not considered a security boundary yet.
- MCP servers and sidecars are separate processes and are **not** sandboxed by `dshr`.
- WASM plugins have no host imports and a fuel limit, but no memory cap and no signature verification.
- ACP runs with local tools unless the editor asks for permissions; fs/terminal capabilities are delegated to the client.

## Known gaps

See [docs/TECH-DEBT.md](docs/TECH-DEBT.md) and [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Active items: Windows sandbox, Windows network, ACP image/client MCP, WASM component model.
