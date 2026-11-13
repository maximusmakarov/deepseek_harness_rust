#!/usr/bin/env bash
set -euo pipefail

sha="${1:-$(git rev-parse HEAD)}"
repo="${2:-maximusmakarov/deepseek_harness_rust}"

python3 - "$repo" "$sha" <<'PY'
import json
import sys
import urllib.request

repo, sha = sys.argv[1], sys.argv[2]
url = f"https://api.github.com/repos/{repo}/commits/{sha}/check-runs"
request = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
with urllib.request.urlopen(request, timeout=20) as response:
    payload = json.load(response)

runs = payload.get("check_runs", [])
if not runs:
    print(f"no check runs for {sha}")
    raise SystemExit(1)

failed = False
pending = False
for run in sorted(runs, key=lambda item: item["name"]):
    conclusion = run.get("conclusion") or "pending"
    print(f"{run['name']}: {conclusion}")
    if conclusion in {"failure", "timed_out", "cancelled", "action_required"}:
        failed = True
    if conclusion == "pending":
        pending = True

raise SystemExit(1 if failed else 2 if pending else 0)
PY
