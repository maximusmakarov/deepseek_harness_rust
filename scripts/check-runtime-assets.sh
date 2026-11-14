#!/usr/bin/env bash
# Verify the vendored shared runtime (served at /v1/runtime/react-<ver>.js) still
# matches the provenance manifest. JS assets are not covered by cargo-deny.
set -euo pipefail
cd "$(dirname "$0")/../crates/server/assets/runtime"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c checksums.txt
else
  shasum -a 256 -c checksums.txt
fi
