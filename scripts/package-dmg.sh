#!/usr/bin/env bash
set -euo pipefail

# Build a macOS DMG from dist/dshr.app.
#   scripts/package-dmg.sh [version]

version="${1:-0.1.0}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app="$root/dist/dshr.app"
out="$root/dist/dshr-$version.dmg"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "package-dmg: macOS only" >&2
  exit 1
fi
if [ ! -d "$app" ]; then
  echo "package-dmg: $app not found; run package-desktop.sh first" >&2
  exit 1
fi

stage="$(mktemp -d)"
ditto "$app" "$stage/dshr.app"
ln -s /Applications "$stage/Applications"
rm -f "$out"
hdiutil create -volname "dshr" -srcfolder "$stage" -ov -format UDZO "$out" >/dev/null
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$out" | awk '{print $1}' > "$out.sha256"
else
  shasum -a 256 "$out" | awk '{print $1}' > "$out.sha256"
fi
echo "$out"
