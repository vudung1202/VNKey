#!/bin/bash
# bundle.sh - build VNKey.app (+ DMG for release) from the vnkey crate.
#
#   scripts/bundle.sh            release: target/release/bundle/macos/VNKey.app
#                                         target/release/bundle/dmg/VNKey_<version>_aarch64.dmg
#   scripts/bundle.sh --debug    debug:   target/debug/bundle/macos/VNKey.app (no DMG)
#
# Signs ad-hoc ("-") by default. Set VNKEY_SIGN_IDENTITY to a certificate name
# (docs/dev-signing.md) so macOS keeps the Accessibility grant across builds.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."

MODE=release
CARGO_FLAGS=(--release)
if [ "${1:-}" = "--debug" ]; then
  MODE=debug
  CARGO_FLAGS=()
fi

CRATE=crates/vnkey-app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$CRATE/Cargo.toml" | head -1)
OUT="target/$MODE/bundle"
APP="$OUT/macos/VNKey.app"

cargo build -p vnkey ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"}

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "target/$MODE/vnkey" "$APP/Contents/MacOS/vnkey"
cp "$CRATE/resources/icon.icns" "$APP/Contents/Resources/icon.icns"
cp "$CRATE/resources/Info.plist" "$APP/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$VERSION" "$APP/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$VERSION" "$APP/Contents/Info.plist"

# Sign the whole bundle, not just the binary, so a downloaded copy opens
# without stripping xattrs by hand.
codesign --force --deep --sign "${VNKEY_SIGN_IDENTITY:--}" "$APP"
echo "▸ $APP"

if [ "$MODE" = release ]; then
  DMG="$OUT/dmg/VNKey_${VERSION}_aarch64.dmg"
  STAGE=$(mktemp -d)
  trap 'rm -rf "$STAGE"' EXIT
  ditto "$APP" "$STAGE/VNKey.app"
  ln -s /Applications "$STAGE/Applications"
  mkdir -p "$OUT/dmg"
  rm -f "$DMG"
  hdiutil create -quiet -volname VNKey -srcfolder "$STAGE" -format UDZO "$DMG"
  echo "▸ $DMG"
fi
