# Contributing

## Setup

- Rust 1.90+ (stable).
- No Node.js required for the core build.
- Optional live integrations: DeepSeek/OpenAI keys, Chrome for CUA, `tsserver`/`rust-analyzer` for LSP.

```sh
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

## Rules

1. Keep the workspace MSRV green; no new `unsafe` outside platform crates and vendored code.
2. Add tests for behavior changes; live-provider tests stay `#[ignore]`.
3. Update `docs/` and an ADR for architectural changes.
4. Keep CI green on macOS, Ubuntu, and Windows; do not skip a platform test without documenting why.
5. Do not copy DeepSeek Harness branding/UI assets; this is an independent Rust implementation.

## ADRs

New decisions go to `docs/adr/NNN-title.md` and `docs/adr/README.md`. Superseding an ADR updates its status and links the replacement.

## Commit style

Short imperative subject, body explains *why* and lists test evidence.
