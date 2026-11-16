#!/usr/bin/env bash
set -euo pipefail

# Offline test for scripts/sync-homebrew-tap.sh: renders a bottle formula from a
# synthetic SHA256SUMS, installs it into a throwaway tap checkout and checks the
# rejection paths. Runs in CI without network access.
cd "$(dirname "$0")/.."

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

version="9.9.9"
sums="$work/SHA256SUMS"
: > "$sums"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
  printf '%064d  dshr-%s-%s.tar.gz\n' 0 "$version" "$target" >> "$sums"
done
cat > "$work/BOTTLES" <<'EOF'
arm64_sequoia 1111111111111111111111111111111111111111111111111111111111111111
x86_64_linux 2222222222222222222222222222222222222222222222222222222222222222
EOF
BOTTLES_FILE="$work/BOTTLES" BOTTLE_ROOT_URL="https://ghcr.io/v2/local/repo" \
  scripts/render-formula.sh "$version" "$sums" "$work/dshr-ghcr.rb" >/dev/null
grep -q 'root_url "https://ghcr.io/v2/local/repo"' "$work/dshr-ghcr.rb"
grep -q "releases/download/v$version/" "$work/dshr-ghcr.rb"

tap="$work/tap"
git init --quiet "$tap"
mkdir -p "$tap/Formula"
git -C "$tap" -c user.name=test -c user.email=test@local commit --allow-empty --quiet -m init

scripts/sync-homebrew-tap.sh "$tap" --formula "$work/dshr-ghcr.rb" --dry-run |
  grep -q "would install"
[ ! -f "$tap/Formula/dshr.rb" ]

scripts/sync-homebrew-tap.sh "$tap" --formula "$work/dshr-ghcr.rb" >/dev/null
cmp -s "$work/dshr-ghcr.rb" "$tap/Formula/dshr.rb"
[ "$(git -C "$tap" log -1 --pretty=%s)" = "dshr $version" ]

# A second sync of the same formula must not create an empty commit.
before="$(git -C "$tap" rev-parse HEAD)"
scripts/sync-homebrew-tap.sh "$tap" --formula "$work/dshr-ghcr.rb" >/dev/null
[ "$(git -C "$tap" rev-parse HEAD)" = "$before" ]

# Formulas without bottle checksums are rejected before they reach the tap.
sed '/^    sha256 /d' "$work/dshr-ghcr.rb" > "$work/no-bottles.rb"
if scripts/sync-homebrew-tap.sh "$tap" --formula "$work/no-bottles.rb" 2>"$work/err"; then
  echo "sync-homebrew-tap accepted a formula without bottle checksums" >&2
  exit 1
fi
grep -q "no bottle checksums" "$work/err"

# The release version helper rejects garbage instead of naming artifacts after it.
out="$(DSHR_VERSION="v1.2.3" GITHUB_REF_NAME=ignored scripts/release-version.sh)"
[ "$out" = "1.2.3" ]
if DSHR_VERSION="not-a-version" scripts/release-version.sh 2>/dev/null; then
  echo "release-version accepted an invalid version" >&2
  exit 1
fi
out="$(DSHR_VERSION= GITHUB_REF_NAME=v0.4.0 scripts/release-version.sh)"
[ "$out" = "0.4.0" ]

echo "homebrew tap sync smoke ok"
