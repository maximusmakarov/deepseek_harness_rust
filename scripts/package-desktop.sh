#!/usr/bin/env bash
set -euo pipefail

# Build a macOS .app bundle for the Tauri desktop shell.
# Usage: scripts/package-desktop.sh [version]
# Requires: cargo build --release -p dshr-cli -p dshr-desktop (run by this script).

version="${1:-0.1.0}"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app="$root/dist/dshr.app"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "package-desktop: macOS only" >&2
  exit 1
fi

cargo build --release -p dshr-cli -p dshr-desktop

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$root/target/release/dshr-desktop" "$app/Contents/MacOS/dshr-desktop"
cp "$root/target/release/dshr" "$app/Contents/MacOS/dshr"

iconset="$(mktemp -d)/dshr.iconset"
mkdir -p "$iconset"
source_icon="$root/crates/desktop/icons/icon.png"
sips -z 16 16 "$source_icon" --out "$iconset/icon_16x16.png" >/dev/null
sips -z 32 32 "$source_icon" --out "$iconset/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$source_icon" --out "$iconset/icon_32x32.png" >/dev/null
sips -z 64 64 "$source_icon" --out "$iconset/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$source_icon" --out "$iconset/icon_128x128.png" >/dev/null
sips -z 256 256 "$source_icon" --out "$iconset/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$source_icon" --out "$iconset/icon_256x256.png" >/dev/null
sips -z 512 512 "$source_icon" --out "$iconset/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$source_icon" --out "$iconset/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$source_icon" --out "$iconset/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$iconset" -o "$app/Contents/Resources/dshr.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>dshr</string>
  <key>CFBundleDisplayName</key><string>dshr</string>
  <key>CFBundleIdentifier</key><string>dev.dshr.desktop</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleExecutable</key><string>dshr-desktop</string>
  <key>CFBundleIconFile</key><string>dshr</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$app" >/dev/null
plutil -lint "$app/Contents/Info.plist" >/dev/null
codesign --verify --deep --strict "$app"
echo "$app"
