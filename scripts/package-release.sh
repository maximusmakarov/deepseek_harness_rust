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

tar -C "$stage_root" -czf "$dist/$name.tar.gz" "$name"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$dist" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
else
  (cd "$dist" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
fi
echo "$dist/$name.tar.gz"
