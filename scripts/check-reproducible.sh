#!/usr/bin/env bash
set -euo pipefail

# Builds the release binary twice with path remapping and compares checksums.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
export SOURCE_DATE_EPOCH=0
export CARGO_INCREMENTAL=0

build() {
  local target_dir="$1"
  local remap_target="$2"
  RUSTFLAGS="--remap-path-prefix=$root=/dshr --remap-path-prefix=$root/$target_dir=/target -C debuginfo=0" \
    cargo build --release -p dshr-cli --target-dir "$root/$target_dir" >/dev/null
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$root/$target_dir/release/dshr" | awk '{print $1}'
  else
    shasum -a 256 "$root/$target_dir/release/dshr" | awk '{print $1}'
  fi
}

first="$(build target/repro-a target/repro-a)"
second="$(build target/repro-b target/repro-b)"
echo "first:  $first"
echo "second: $second"
if [ "$first" != "$second" ]; then
  echo "reproducible build check failed" >&2
  exit 1
fi
echo "reproducible build ok"
