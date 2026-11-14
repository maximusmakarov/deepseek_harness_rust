#!/usr/bin/env bash
set -euo pipefail

# Render a stable binary Homebrew formula from SHA256SUMS.
#   scripts/render-formula.sh <version> <SHA256SUMS> [output]
# Optional BOTTLES_FILE: lines "<bottle-tag> <sha256>".

version="${1:?usage: render-formula.sh <version> <SHA256SUMS> [output]}"
sums="${2:?usage: render-formula.sh <version> <SHA256SUMS> [output]}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
template="$root/packaging/homebrew/dshr-binary.rb.template"
out="${3:-$root/dist/dshr.rb}"
bottles_file="${BOTTLES_FILE:-}"

python3 - "$version" "$sums" "$template" "$out" "$bottles_file" <<'PYTHON'
import os
import sys

version, sums_path, template_path, out_path, bottles_path = sys.argv[1:6]
sums = {}
with open(sums_path) as handle:
    for line in handle:
        parts = line.split()
        if len(parts) >= 2:
            sums[parts[1]] = parts[0]

def sha_for(target):
    name = f"dshr-{version}-{target}.tar.gz"
    if name not in sums:
        raise SystemExit(f"render-formula: no checksum for {target} in {sums_path}")
    return sums[name]

bottle_block = ""
if bottles_path:
    try:
        with open(bottles_path) as handle:
            entries = [line.split() for line in handle if line.strip()]
    except FileNotFoundError:
        entries = []
    if entries:
        root_url = os.environ.get(
            "BOTTLE_ROOT_URL",
            f"https://github.com/maximusmakarov/deepseek_harness_rust/releases/download/v{version}",
        )
        bottle_block = (
            "  bottle do\n"
            f'    root_url "{root_url}"\n'
        )
        for entry in entries:
            bottle_block += f'    sha256 {entry[0]}: "{entry[1]}"\n'
        bottle_block += "  end\n"

template = open(template_path).read()
replacements = {
    "__VERSION__": version,
    "__BOTTLES__": bottle_block,
    "__SHA_ARM64_MACOS__": sha_for("aarch64-apple-darwin"),
    "__SHA_X86_64_MACOS__": sha_for("x86_64-apple-darwin"),
    "__SHA_ARM64_LINUX__": sha_for("aarch64-unknown-linux-gnu"),
    "__SHA_X86_64_LINUX__": sha_for("x86_64-unknown-linux-gnu"),
}
for key, value in replacements.items():
    template = template.replace(key, value)
with open(out_path, "w") as handle:
    handle.write(template)
print(out_path)
PYTHON

if command -v ruby >/dev/null 2>&1; then
  ruby -c "$out" >/dev/null
fi
