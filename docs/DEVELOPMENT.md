# Development

## Commands

```sh
cargo build -p dshr-cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo deny check
cargo llvm-cov --workspace --summary-only
```

## Web UI

The Leptos CSR app lives in `crates/web` and is excluded from the root workspace
(it targets `wasm32-unknown-unknown`). Build it with trunk:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked        # or a prebuilt release binary
cd crates/web && trunk build --release
```

`crates/web/dist` is committed because `crates/server` embeds it via `include_bytes!`.
Rebuild the UI and commit `dist` whenever the frontend changes; `cargo test -p dshr-server`
checks that the wasm asset is served.

## CI

`.github/workflows/ci.yml` runs on macOS/Ubuntu/Windows: fmt, clippy, nextest, doc tests; separate `cargo-deny` and `coverage` jobs. Failed nextest/clippy output is annotated and uploaded on failure.

`release.yml` builds tag releases (`v*`) for the three OSes.

## Tests

- Unit/integration tests: `cargo test`.
- Ignored/live: DeepSeek smoke, Chrome CUA, search benchmark.
- Windows ACP duplex tests are skipped because they hang on the GitHub Windows runner; the code still builds there.
- Windows sandbox tests are skipped while AppContainer setup is broken.

## Layout

See README “Структура”. New crates are `dshr-*` and added to the root `[workspace] members`; Windows-only vendored code lives under `vendor/` and is excluded from the workspace.

## Debugging

- `dshr doctor` — config/provider/tool checks.
- `RUST_LOG=debug` — JSON logs to stderr.
- `OTEL_EXPORTER_OTLP_ENDPOINT=...` — OTLP traces.
- `DSHR_WINDOWS_SANDBOX=1` — experimental AppContainer (fail-closed by default).
