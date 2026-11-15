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
