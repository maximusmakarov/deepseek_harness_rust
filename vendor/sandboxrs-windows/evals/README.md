# sandboxrs-windows evals

Deterministic OS-level sandbox benchmark, no LLM required.

## Run locally on Windows

```powershell
cargo build --bin sandboxrs-eval --bin sandboxrs-test-attacker
cargo run --bin sandboxrs-eval -- --report sandboxrs-eval.json
```

The report includes:

- backend probe: OS, modern API, AppContainer, admin status
- control checks proving forbidden operations are possible outside the sandbox
- security contract: workspace/readonly/secret read-write-delete matrix plus
  root/child/grandchild escape attempts
- path attacks: dot-dot traversal, absolute paths, case changes, `\\?\` paths,
  junctions, symlinks, nested junctions
- lifecycle containment: kill terminates descendants, timeout, process bomb
  limit, memory limit
- developer compatibility: cmd, git, node, python, cargo inside the sandbox
- environment leakage: `env_clear()` removes a fake secret

Scores are emitted as JSON and uploaded as a GitHub Actions artifact by
`.github/workflows/security-evals.yml`.

## Agentic evals

Agent evals need an LLM API credential and are intentionally separate from the
deterministic benchmark. See `agent-scenarios.md` for the scenario catalog and
`run-agent-evals.ps1` for the runner scaffold.
