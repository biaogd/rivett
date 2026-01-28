#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SVG_PATH="$ROOT_DIR/resources/app-icon.svg"
ICNS_PATH="$ROOT_DIR/resources/app-icon.icns"

if ! command -v rsvg-convert >/dev/null 2>&1; then
  echo "rsvg-convert not found. Install it with: brew install librsvg" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
BASE_PNG="$TMP_DIR/app-icon-1024.png"
rsvg-convert -w 1024 -h 1024 "$SVG_PATH" -o "$BASE_PNG"

python3 - <<PY
import struct

png_path = r"$BASE_PNG"
icns_path = r"$ICNS_PATH"

with open(png_path, "rb") as f:
    data = f.read()

chunk_type = b"ic10"  # 1024x1024 PNG
chunk_len = 8 + len(data)
icns_len = 8 + chunk_len

with open(icns_path, "wb") as f:
    f.write(b"icns")
    f.write(struct.pack(">I", icns_len))
    f.write(chunk_type)
    f.write(struct.pack(">I", chunk_len))
    f.write(data)
PY

rm -rf "$TMP_DIR"

echo "Generated $ICNS_PATH"
