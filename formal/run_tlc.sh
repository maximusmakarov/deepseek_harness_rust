#!/usr/bin/env bash
# Runs every TLA+ model/config pair with TLC.
#
# Requires a JDK (17+) and tla2tools.jar. The jar is looked up in:
#   $TLA2TOOLS_JAR, formal/tools/tla2tools.jar, then a few common locations.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

jar="${TLA2TOOLS_JAR:-}"
if [ -z "$jar" ]; then
  for candidate in \
    "$root/tools/tla2tools.jar" \
    "$HOME/.local/share/tla2tools/tla2tools.jar" \
    "/Users/maksim/Orlan/run-test/tools/tla2tools.jar" \
    "$(command -v tla2tools.jar 2>/dev/null || true)"; do
    if [ -n "$candidate" ] && [ -f "$candidate" ]; then jar="$candidate"; break; fi
  done
fi
if [ -z "$jar" ]; then
  echo "tla2tools.jar not found; set TLA2TOOLS_JAR" >&2
  exit 2
fi

java="${JAVA_HOME:+$JAVA_HOME/bin/java}"
if [ -z "$java" ]; then
  if [ -x /opt/homebrew/opt/openjdk@21/bin/java ]; then
    java=/opt/homebrew/opt/openjdk@21/bin/java
  elif command -v java >/dev/null 2>&1; then
    java=java
  else
    echo "java not found; install a JDK 17+ or set JAVA_HOME" >&2
    exit 2
  fi
fi

pairs=(
  "CordisSupervisor.cfg:CordisSupervisor.tla"
  "CordisSupervisorLiveness.cfg:CordisSupervisor.tla"
  "ClientAssets.cfg:ClientAssets.tla"
  "ApprovalGate.cfg:ApprovalGate.tla"
  "ApprovalGateLive.cfg:ApprovalGate.tla"
)

failures=0
for pair in "${pairs[@]}"; do
  cfg="${pair%%:*}"; tla="${pair##*:}"
  echo "=== $cfg"
  log="/tmp/tlc-${cfg%.cfg}.log"
  meta="/tmp/tlc-meta-${cfg%.cfg}"
  rm -rf "$meta"
  "$java" -cp "$jar" tlc2.TLC -workers auto -metadir "$meta" \
      -config "$root/$cfg" "$root/$tla" >"$log" 2>&1 || true
  if grep -qE "No error has been found" "$log"; then
    echo "    ok"
  else
    echo "    FAILED (see $log)"
    failures=$((failures + 1))
  fi
done

if [ "$failures" -gt 0 ]; then
  echo "$failures model(s) failed" >&2
  exit 1
fi
echo "all models checked"
