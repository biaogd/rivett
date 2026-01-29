#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
APP_NAME="Rivett"
APP_PATH="$ROOT_DIR/target/release/bundle/osx/${APP_NAME}.app"
DMG_PATH="$DIST_DIR/Rivett.dmg"

"$ROOT_DIR/scripts/macos/make_icns.sh"

cargo bundle --release

if [ ! -d "$APP_PATH" ]; then
  echo "App bundle not found at $APP_PATH" >&2
  exit 1
fi

mkdir -p "$DIST_DIR"
rm -f "$DMG_PATH"

hdiutil create -volname "$APP_NAME" -srcfolder "$APP_PATH" -ov -format UDZO "$DMG_PATH"

echo "Created $DMG_PATH"
