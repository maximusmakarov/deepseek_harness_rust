#!/usr/bin/env bash
set -euo pipefail

# Install the GHCR-bottle formula into a Homebrew tap checkout.
#
#   scripts/sync-homebrew-tap.sh <tap-dir> [--formula dist/dshr-ghcr.rb] [--push] [--dry-run]
#
# <tap-dir> is an existing git checkout of the tap repository (for example
# `homebrew-dshr`, which serves `brew install maximusmakarov/dshr/dshr`). The
# formula is validated before it is copied to Formula/dshr.rb, so a broken
# render never reaches users.

usage() {
  cat >&2 <<'EOF'
usage: sync-homebrew-tap.sh <tap-dir> [--formula <path>] [--push] [--dry-run]
EOF
  exit 2
}

tap=""
formula="dist/dshr-ghcr.rb"
push=0
dry_run=0
while [ $# -gt 0 ]; do
  case "$1" in
    --formula) formula="${2:?--formula needs a path}"; shift 2 ;;
    --push) push=1; shift ;;
    --dry-run) dry_run=1; shift ;;
    -h|--help) usage ;;
    -*) echo "sync-homebrew-tap: unknown flag $1" >&2; usage ;;
    *) [ -z "$tap" ] || usage; tap="$1"; shift ;;
  esac
done
[ -n "$tap" ] || usage

[ -f "$formula" ] || { echo "sync-homebrew-tap: formula not found: $formula" >&2; exit 1; }
[ -d "$tap" ] || { echo "sync-homebrew-tap: tap checkout not found: $tap" >&2; exit 1; }
[ -d "$tap/.git" ] || { echo "sync-homebrew-tap: $tap is not a git checkout" >&2; exit 1; }

# A bottle formula is useless without the GHCR root and at least one bottle
# checksum, and brew rejects a formula that ruby cannot parse.
if ! grep -q 'root_url "https://ghcr.io/v2/' "$formula"; then
  echo "sync-homebrew-tap: $formula has no GHCR root_url" >&2
  exit 1
fi
if ! grep -qE '^    sha256 [A-Za-z0-9_]+: "[0-9a-f]{64}"$' "$formula"; then
  echo "sync-homebrew-tap: $formula has no bottle checksums" >&2
  exit 1
fi
if command -v ruby >/dev/null 2>&1; then
  ruby -c "$formula" >/dev/null
fi

# The formula carries no `version` line (brew scans it from the URL and
# `brew audit --strict` rejects the redundant one), so read it from the release
# URL first and fall back to an explicit version line for older renders.
version="$(sed -n 's#.*/releases/download/v\([^/]*\)/.*#\1#p' "$formula" | head -1)"
if [ -z "$version" ]; then
  version="$(sed -n 's/^  version "\(.*\)"$/\1/p' "$formula" | head -1)"
fi
[ -n "$version" ] || { echo "sync-homebrew-tap: cannot read the version from $formula" >&2; exit 1; }

target="$tap/Formula/dshr.rb"
if [ "$dry_run" -eq 1 ]; then
  echo "would install $formula (version $version) as $target"
  [ "$push" -eq 1 ] && echo "would commit and push $tap"
  exit 0
fi

mkdir -p "$tap/Formula"
cp "$formula" "$target"
git -C "$tap" add Formula/dshr.rb
if git -C "$tap" diff --cached --quiet; then
  echo "sync-homebrew-tap: $target is unchanged"
else
  git -C "$tap" \
    -c user.name="${GIT_AUTHOR_NAME:-dshr release bot}" \
    -c user.email="${GIT_AUTHOR_EMAIL:-dshr@localhost}" \
    commit -m "dshr $version" --quiet
  echo "sync-homebrew-tap: committed dshr $version in $tap"
fi
if [ "$push" -eq 1 ]; then
  git -C "$tap" push --quiet
  echo "sync-homebrew-tap: pushed $tap"
fi
echo "next: brew install maximusmakarov/dshr/dshr"
