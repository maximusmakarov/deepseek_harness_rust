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
step cargo clippy --workspace --all-targets -- -D warnings
step cargo test --workspace

if [ "${1:-}" = "quick" ]; then
    echo
    echo "quick run: skipped evals, perf budgets and the browser gates"
    exit 0
fi

step cargo test -p dshr-cli --test evals -- --nocapture
step cargo test -p dshr-perf --release -- --nocapture
step python3 scripts/web-e2e.py
step python3 scripts/plugin-smoke.py

echo
echo "all gates passed"
