#!/usr/bin/env bash
set -euo pipefail

# Build a Homebrew bottle tarball for one target.
#   scripts/package-bottle.sh <version> <bottle-tag> <binary> [dist]

version="${1:?usage: package-bottle.sh <version> <tag> <binary> [dist]}"
tag="${2:?usage: package-bottle.sh <version> <tag> <binary> [dist]}"
binary="${3:?usage: package-bottle.sh <version> <tag> <binary> [dist]}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="${4:-$root/dist}"
name="dshr--$version.$tag.bottle.tar.gz"

work="$(mktemp -d)"
stage="$work/stage"
mkdir -p "$dist"
mkdir -p "$stage/dshr/$version/bin"
cp "$binary" "$stage/dshr/$version/bin/dshr"
chmod 755 "$stage/dshr/$version/bin/dshr"
case "$tag" in
  arm64_*) arch="arm64"; os="Macintosh"; os_version="macOS 15" ;;
  sonoma) arch="x86_64"; os="Macintosh"; os_version="macOS 14" ;;
  x86_64_linux) arch="x86_64"; os="Linux"; os_version="Ubuntu 24.04" ;;
  aarch64_linux) arch="arm64"; os="Linux"; os_version="Ubuntu 24.04" ;;
  *) arch="arm64"; os="Macintosh"; os_version="macOS 15" ;;
esac
cat > "$stage/dshr/$version/INSTALL_RECEIPT.json" <<RECEIPT
{"homebrew_version":"4.6.0","used_options":[],"unused_options":[],"built_as_bottle":true,"poured_from_bottle":true,"installed_as_dependency":false,"installed_on_request":true,"arch":"$arch","built_on":{"os":"$os","os_version":"$os_version"},"runtime_dependencies":[],"source":{"spec":"stable","versions":{"stable":"$version","head":null,"version_scheme":0,"compatibility_version":null}}}
RECEIPT

python3 - "$stage" "$dist/$name" <<'PYTHON'
import io
import os
import struct
import sys
import tarfile
import zlib

stage, out = sys.argv[1], sys.argv[2]
raw = io.BytesIO()
with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as tar:
    for root, dirs, files in os.walk(stage):
        dirs.sort()
        for file in sorted(files):
            path = os.path.join(root, file)
            info = tar.gettarinfo(path, arcname=os.path.relpath(path, stage))
            info.mtime = 0
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mode = 0o755 if os.access(path, os.X_OK) else 0o644
            info.pax_headers = {}
            with open(path, "rb") as handle:
                tar.addfile(info, handle)
data = raw.getvalue()
compressed = zlib.compressobj(9, zlib.DEFLATED, -15)
body = compressed.compress(data) + compressed.flush()
header = b"\x1f\x8b\x08\x00" + struct.pack("<I", 0) + b"\x02\xff"
footer = struct.pack("<II", zlib.crc32(data) & 0xFFFFFFFF, len(data) & 0xFFFFFFFF)
with open(out, "wb") as handle:
    handle.write(header + body + footer)
PYTHON

if command -v sha256sum >/dev/null 2>&1; then
  sha="$(sha256sum "$dist/$name" | awk '{print $1}')"
else
  sha="$(shasum -a 256 "$dist/$name" | awk '{print $1}')"
fi
echo "$tag $sha" > "$dist/bottles-$tag"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$dist" && sha256sum "$name" > "$name.sha256")
else
  (cd "$dist" && shasum -a 256 "$name" > "$name.sha256")
fi
echo "$dist/$name"
