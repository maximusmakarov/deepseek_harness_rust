#!/usr/bin/env bash
set -euo pipefail

# Install the `dshr` binary from the public repository.
cargo install --git https://github.com/maximusmakarov/deepseek_harness_rust \
  --locked --package dshr-cli

echo "installed: $(command -v dshr || echo ~/.cargo/bin/dshr)"
echo "run: dshr doctor"
