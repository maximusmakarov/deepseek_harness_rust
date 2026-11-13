#!/usr/bin/env bash
set -euo pipefail

# Install `dshr` from a release archive (default) or from source.
#
#   scripts/install.sh                 # latest release, checksum verified
#   DSHR_VERSION=0.2.0 scripts/install.sh
#   scripts/install.sh --from-source   # cargo install from git
#
# Environment: DSHR_VERSION, DSHR_TARGET, DSHR_INSTALL_DIR (default ~/.local/bin),
# DSHR_REPO, DSHR_RELEASE_BASE_URL.

repo="${DSHR_REPO:-maximusmakarov/deepseek_harness_rust}"
install_dir="${DSHR_INSTALL_DIR:-$HOME/.local/bin}"

if [ "${DSHR_FROM_SOURCE:-0}" = "1" ] || [ "${1:-}" = "--from-source" ]; then
  cargo install --git "https://github.com/$repo" --locked --package dshr-cli
  echo "installed: $(command -v dshr || echo ~/.cargo/bin/dshr)"
  echo "run: dshr doctor"
  exit 0
fi

detect_target() {
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) echo aarch64-apple-darwin ;;
    Darwin-x86_64) echo x86_64-apple-darwin ;;
    Linux-x86_64) echo x86_64-unknown-linux-gnu ;;
    Linux-aarch64 | Linux-arm64) echo aarch64-unknown-linux-gnu ;;
    *)
      echo "install: unsupported platform; rerun with --from-source" >&2
      exit 1
      ;;
  esac
}

target="${DSHR_TARGET:-$(detect_target)}"
version="${DSHR_VERSION:-}"
if [ -z "$version" ]; then
  latest="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$repo/releases/latest")"
  version="${latest##*/tag/v}"
fi
version="${version#v}"

base="${DSHR_RELEASE_BASE_URL:-https://github.com/$repo/releases/download/v$version}"
archive="dshr-$version-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$base/$archive" -o "$tmp/$archive"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"
(
  cd "$tmp"
  line="$(grep " $archive\$" SHA256SUMS)" || true
  if [ -z "$line" ]; then
    echo "install: $archive is not listed in SHA256SUMS" >&2
    exit 1
  fi
  printf '%s\n' "$line" | sha256sum -c - 2>/dev/null ||
    printf '%s\n' "$line" | shasum -a 256 -c -
)

tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$install_dir"
cp "$tmp/dshr-$version-$target/dshr" "$install_dir/dshr"
chmod +x "$install_dir/dshr"
echo "installed: $install_dir/dshr"
"$install_dir/dshr" --version
