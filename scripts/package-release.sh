#!/usr/bin/env bash
set -euo pipefail

# Package one release archive: <version> <target> [binary].
# Writes dist/dshr-<version>-<target>.tar.gz and a matching .sha256 file.

version="${1:?usage: package-release.sh <version> <target> [binary]}"
target="${2:?usage: package-release.sh <version> <target> [binary]}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="${3:-$root/target/$target/release/dshr}"
case "$binary" in
  /*) ;;
  *) binary="$root/$binary" ;;
esac
if [ ! -f "$binary" ]; then
  echo "package-release: binary not found: $binary" >&2
  exit 1
fi

name="dshr-$version-$target"
dist="$root/dist"
stage_root="$root/target/release-stage"
stage="$stage_root/$name"
rm -rf "$stage"
mkdir -p "$stage" "$dist"
cp "$binary" "$stage/"
if [ -f "$root/README.md" ]; then cp "$root/README.md" "$stage/"; fi
for file in LICENSE LICENSE-MIT LICENSE-APACHE; do
  if [ -f "$root/$file" ]; then cp "$root/$file" "$stage/"; fi
done

# Deterministic archive: sorted entries, fixed mtime/uid/gid and a zero gzip
# timestamp, so packaging the same binary twice yields the same checksum.
python3 - "$stage_root" "$name" "$dist/$name.tar.gz" <<'PYTHON'
import gzip
import io
import os
import struct
import sys
import tarfile
import zlib

stage, name, out = sys.argv[1], sys.argv[2], sys.argv[3]
raw = io.BytesIO()
with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as tar:
    for root, dirs, files in os.walk(os.path.join(stage, name)):
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
  (cd "$dist" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
else
  (cd "$dist" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
fi
echo "$dist/$name.tar.gz"
