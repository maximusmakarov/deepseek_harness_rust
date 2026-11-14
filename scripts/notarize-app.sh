#!/usr/bin/env bash
set -euo pipefail

# Sign, notarize and staple a macOS .app.
# Requires DSHR_CODESIGN_IDENTITY, APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD.

app="${1:-dist/dshr.app}"
if [ "$(uname -s)" != "Darwin" ]; then
  echo "notarize-app: macOS only" >&2
  exit 1
fi
: "${DSHR_CODESIGN_IDENTITY:?DSHR_CODESIGN_IDENTITY is required}"
: "${APPLE_ID:?APPLE_ID is required}"
: "${APPLE_TEAM_ID:?APPLE_TEAM_ID is required}"
: "${APPLE_APP_PASSWORD:?APPLE_APP_PASSWORD is required}"

codesign --force --deep --options runtime --timestamp \
  --sign "$DSHR_CODESIGN_IDENTITY" "$app"
codesign --verify --deep --strict --verbose=2 "$app"

work="$(mktemp -d)"
zip="$work/dshr.zip"
ditto -c -k --keepParent "$app" "$zip"
xcrun notarytool submit "$zip" \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_PASSWORD" \
  --wait
xcrun stapler staple "$app"
xcrun stapler validate "$app"
spctl --assess --type execute --verbose=2 "$app"
echo "notarized: $app"
