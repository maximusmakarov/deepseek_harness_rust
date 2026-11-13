#!/usr/bin/env bash
set -euo pipefail

# Render a stable binary Homebrew formula from SHA256SUMS.
#   scripts/render-formula.sh <version> <SHA256SUMS> [output]

version="${1:?usage: render-formula.sh <version> <SHA256SUMS> [output]}"
sums="${2:?usage: render-formula.sh <version> <SHA256SUMS> [output]}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
template="$root/packaging/homebrew/dshr-binary.rb.template"
out="${3:-$root/dist/dshr.rb}"

sha_for() {
  local target="$1"
  local hash
  hash="$(awk -v file="dshr-$version-$target.tar.gz" '$2 == file { print $1 }' "$sums")"
  if [ -z "$hash" ]; then
    echo "render-formula: no checksum for $target in $sums" >&2
    exit 1
  fi
  printf '%s' "$hash"
}

sed \
  -e "s|__VERSION__|$version|g" \
  -e "s|__SHA_ARM64_MACOS__|$(sha_for aarch64-apple-darwin)|g" \
  -e "s|__SHA_X86_64_MACOS__|$(sha_for x86_64-apple-darwin)|g" \
  -e "s|__SHA_ARM64_LINUX__|$(sha_for aarch64-unknown-linux-gnu)|g" \
  -e "s|__SHA_X86_64_LINUX__|$(sha_for x86_64-unknown-linux-gnu)|g" \
  "$template" > "$out"

if command -v ruby >/dev/null 2>&1; then
  ruby -c "$out" >/dev/null
fi
echo "$out"
