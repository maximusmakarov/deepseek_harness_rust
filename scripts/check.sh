#!/usr/bin/env bash
# Reproduce every CI gate locally. CI runs these exact commands, so a green run
# here is the best available signal before pushing.
#
#   scripts/check.sh            # everything
#   scripts/check.sh quick      # fmt + clippy + workspace tests
set -euo pipefail
cd "$(dirname "$0")/.."

step() {
    echo
    echo "== $*"
    "$@"
}

step cargo fmt --all -- --check
step scripts/check-runtime-assets.sh
step cargo clippy --workspace --all-targets -- -D warnings
step cargo test --workspace

if [ "${1:-}" = "quick" ]; then
    echo
    echo "quick run: skipped evals, perf budgets and the browser gates"
    exit 0
fi

step cargo test -p dshr-cli --test evals -- --nocapture
# Perf budgets are wall-clock measurements: a busy host (Spotlight indexing,
# another build, CI co-tenancy) can push p95 over the budget without a code
# regression. Retry once on failure — a real regression fails both runs.
if ! cargo test -p dshr-perf --release -- --nocapture; then
    echo
    echo "perf budgets failed; load: $( (sysctl -n vm.loadavg 2>/dev/null || cat /proc/loadavg 2>/dev/null || echo unknown) 2>/dev/null )"
    echo "retrying perf budgets once (wall-clock gates are load-sensitive)"
    cargo test -p dshr-perf --release -- --nocapture
fi
step python3 scripts/web-e2e.py
step python3 scripts/plugin-smoke.py

echo
echo "all gates passed"
