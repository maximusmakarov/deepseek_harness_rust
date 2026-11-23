#!/usr/bin/env bash
# Reproduce every CI gate locally. CI runs these exact commands, so a green run
# here is the best available signal before pushing.
#
#   scripts/check.sh            # everything
#   scripts/check.sh quick      # everything except evals, perf and browser gates
set -euo pipefail
cd "$(dirname "$0")/.."

step() {
    echo
    echo "== $*"
    "$@"
}

# cargo-deny is a separately installed binary (not a workspace dependency), so a
# missing install would otherwise surface as a bare "command not found" and fail a
# clean machine. CI runs the same advisory set through cargo-deny-action, so skip
# loudly here instead of pretending the gate ran.
deny_check() {
    if command -v cargo-deny >/dev/null 2>&1; then
        cargo deny check
        return
    fi
    echo "cargo-deny not installed: skipping the advisory/license gate." >&2
    echo "  install it with: cargo install cargo-deny --locked" >&2
    echo "  CI still enforces it through cargo-deny-action (see .github/workflows/ci.yml)." >&2
}

# CI runs the workspace suite through nextest; plain cargo test is the fallback.
# Running both would only double the time, so pick the same runner CI uses and say
# which one ran.
workspace_tests() {
    if command -v cargo-nextest >/dev/null 2>&1; then
        cargo nextest run --workspace --no-fail-fast
        return
    fi
    echo "cargo-nextest not installed: running the workspace suite through cargo test instead." >&2
    echo "  install it with: cargo install cargo-nextest --locked" >&2
    cargo test --workspace
}

# The web crate is a separate workspace member exclusion (Cargo.toml) because its
# tests need the host target and its build needs wasm32, so neither
# `cargo test --workspace` nor a CI nextest run reaches it. Its bundle is compiled
# into the server through include_bytes!, so a broken crate or a stale dist breaks
# the shipped binary rather than a test. Gate both halves explicitly.
web_check() {
    CARGO_INCREMENTAL=0 cargo test --manifest-path crates/web/Cargo.toml
    CARGO_INCREMENTAL=0 cargo check --manifest-path crates/web/Cargo.toml --target wasm32-unknown-unknown
}

# The Tauri shell is its own crate with its own lockfile and target directory, so
# `cargo test --workspace` and `cargo clippy --workspace` never reach it; CI runs
# it as a separate macOS-only step. Mirror that here, loudly skipping elsewhere
# because the Linux build needs webkit2gtk.
desktop_check() {
    if [ "$(uname -s)" != "Darwin" ]; then
        echo "skip: the desktop shell is checked on macOS only"
        return 0
    fi
    CARGO_INCREMENTAL=0 cargo test --manifest-path crates/desktop/Cargo.toml
    rustfmt --edition 2024 --config-path rustfmt.toml --check \
        crates/desktop/src/*.rs crates/desktop/tests/*.rs
}

step cargo fmt --all -- --check
step python3 scripts/check-platform-branches.py
step scripts/check-runtime-assets.sh
step scripts/test-sync-homebrew-tap.sh
step deny_check
step cargo clippy --workspace --all-targets -- -D warnings
step workspace_tests
step cargo test --workspace --doc
step web_check
step desktop_check

if [ "${1:-}" = "quick" ]; then
    echo
    echo "quick run: skipped evals, perf budgets and the browser gates"
    exit 0
fi

step cargo test -p dshr-cli --test evals -- --nocapture
# Perf budgets are wall-clock measurements: a busy host (Spotlight indexing,
# another build, CI co-tenancy) can push p95 over the budget without a code
# regression. On failure, wait (bounded) for the 1-minute load to fall below the
# CPU count and retry once; a real regression fails both runs.
load_one() {
    if command -v sysctl >/dev/null 2>&1 && sysctl -n vm.loadavg >/dev/null 2>&1; then
        sysctl -n vm.loadavg | awk '{print $2}'
    elif [ -r /proc/loadavg ]; then
        awk '{print $1}' /proc/loadavg
    else
        echo 0
    fi
}
cpu_count() {
    if command -v sysctl >/dev/null 2>&1 && sysctl -n hw.ncpu >/dev/null 2>&1; then
        sysctl -n hw.ncpu
    elif command -v nproc >/dev/null 2>&1; then
        nproc
    else
        echo 1
    fi
}
if ! cargo test -p dshr-perf --release -- --nocapture; then
    threshold=$(cpu_count)
    echo
    echo "perf budgets failed; 1-min load $(load_one) (threshold ${threshold}); waiting up to 180s for the host to settle"
    waited=0
    while [ "$waited" -lt 180 ] && awk -v l="$(load_one)" -v t="$threshold" 'BEGIN { exit !(l > t) }'; do
        sleep 10
        waited=$((waited + 10))
    done
    echo "retrying perf budgets once (load $(load_one))"
    cargo test -p dshr-perf --release -- --nocapture
fi
step python3 scripts/web-e2e.py
step python3 scripts/plugin-smoke.py

echo
echo "all gates passed"
