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

One command runs every gate exactly like CI:

```sh
scripts/check.sh          # everything (fmt, clippy, runtime assets, tests, evals, perf, browser)
scripts/check.sh quick    # fmt, clippy, workspace tests
```

Gates that need their own invocation:

```sh
cargo test -p dshr-cli --test evals -- --nocapture   # 10-task eval suite, budgets in target/evals/report.json
cargo test -p dshr-perf --release -- --nocapture     # ARCHITECTURE §20 budgets (debug runs print SKIP)
python3 scripts/web-e2e.py                           # chat -> approval -> tool -> journal in headless Chrome
python3 scripts/plugin-smoke.py                      # client-half bridge: eager registry, regional slot frames, plugin-shipped runtime (`dsh.client.runtime`), shared host runtime (`dsh.client.host_modules`), C-lite React slot components with host hooks/`ctx.get('sessions')` (fixture `hooks`) and the React contract (component types from `React.memo`/`forwardRef`, keyed ids, `ctx.locale.register`/`t()`, `ctx.interval`/`ctx.timeout`), bridge services `ctx.get('slots')`/`ctx.get('locale')`/`ctx.inject` and `__DSHR_LAST_ERROR__`, design-system primitives shim (fixture `ui-kit`; since round 85 (C2) 30+ exports — 24 components/functions including `Input`/`Switch`/`Checkbox`/`Tag`/`Pill`/`Menu`/`MenuSurface`/`Modal`/`Toast`/`SegmentedControl`/`SegmentedTabs`/`DisclosureRow`/`PathLabel`/`TextShimmer`/`writeClipboard`/`useAnchoredPosition`/`useDismissOnOutsidePointer`, plus 18 named `Icon*` with a Proxy fallback; the smoke probes render the whole modelled tree inside a host-React frame with portal Menu/Modal/Toast and check real plugin contributions in the Plugins dialog), and the eleven real conformance fixtures `@kt11/dsh-deepseek-balance`, `dsh-remote`, `@ruoyang/dsh-plugin-deepseek-balance`, `dsh-genui`, `dsh-session-superagent` (composer dock), `dsh-mnemon` (large multi-region plugin; exercises the reference slots surface `ctx.slots.subscribe`/`getVersion`/`entriesOfSlot`), `dsh-plugin-market` (sidebar entry), `dsh-plugin-codegraph-panel` (settings-section panel), `dsh-plugin-notify` (plugins-page rows), `dsh-plugin-debugging` and `dsh-plugin-studio` (primitives-heavy halves that activate cleanly but keep their host-service-gated UI unmounted); C1 host HTTP routes are proxied end-to-end (the real `deepseek-balance` host half answers its own JSON through the plugin frame: 500 `not-configured`, 200 JSON for the usage route with a query string, unknown path → dshr 404; exact and prefix routes; live `routes` table in `GET /v1/plugins/status`; MCP pseudo-tools filtered out; LAN exposure is opt-in via `--allow-lan-plugin-routes`); C1.1 session bridge: the server publishes `<home>/cordis/host.json` (`{url, token}`) and serves `GET /v1/internal/session-snapshot` (recent ≤50 sessions with usage events only, 404 without a token, 401 on a wrong one), the sidecar reads it through `DSHR_HOST_BRIDGE`, rebuilds DSH-shaped `sessions`/`sessionProjections` from a snapshot refreshed every 5 s, and the smoke polls the real balance plugin's usage route for `sessions` ≥ 1 plus totals (80 checks)
scripts/check-runtime-assets.sh                      # vendored shared runtime (React/ReactDOM UMD + bootstrap + the partial primitives shim with 30+ exports; 176,725 bytes total) matches crates/server/assets/runtime/checksums.txt
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

The Tauri shell (`crates/desktop`) is also outside the workspace, so Linux CI does not
need WebKitGTK. Build it with
`CARGO_TARGET_DIR=target cargo build --release --manifest-path crates/desktop/Cargo.toml`;
macOS CI runs its tests separately.

## CI

`.github/workflows/ci.yml` runs on macOS/Ubuntu/Windows: fmt, clippy, nextest, doc tests; separate `cargo-deny` and `coverage` jobs, plus `evals`, `perf` (release) and `web-e2e` (headless Chrome + client-half smoke) gates. The Linux test job also runs `scripts/check-runtime-assets.sh` (checksum gate for the vendored shared runtime). The experimental Windows AppContainer check is `continue-on-error`: it reports into the job summary but does not block trunk while the feature is deferred (see `docs/TECH-DEBT.md`). Failed nextest/clippy output is annotated and uploaded on failure.

`release.yml` builds tag releases (`v*`) for the three OSes.

## Tests

- Unit/integration tests: `cargo test`.
- Plugin host routes: `cargo test -p dshr-server --test plugins_live` — 16 checks: exact/prefix proxying, query/POST/preflight, 404 for unknown and reserved paths, route removal on stop, per-plugin `routes` in `GET /v1/plugins/status`, the MCP pseudo-tool filter (`list_resources`/`read_resource`/`list_prompts`/`get_prompt`), the loopback-only LAN policy (`403` unless `allow_plugin_routes_lan`), and the C1.1 internal session snapshot (`internal_session_snapshot_feeds_the_host_bridge`: usage fields, 401 on a wrong token, 404 when no internal token is configured).
- Ignored/live: DeepSeek smoke, Chrome CUA, search benchmark.
- Eval suite and perf budgets are separate CI jobs; the eval report is uploaded as an artifact.
- Windows ACP duplex tests are skipped because they hang on the GitHub Windows runner; the code still builds there.
- Windows sandbox tests are skipped while AppContainer setup is broken.

## Layout

See README “Структура”. New crates are `dshr-*` and added to the root `[workspace] members`; Windows-only vendored code lives under `vendor/` and is excluded from the workspace.

## Debugging

- `dshr doctor` — config/provider/tool checks.
- `RUST_LOG=debug` — JSON logs to stderr.
- `OTEL_EXPORTER_OTLP_ENDPOINT=...` — OTLP traces.
- `DSHR_WINDOWS_SANDBOX=1` — experimental AppContainer (fail-closed by default).
