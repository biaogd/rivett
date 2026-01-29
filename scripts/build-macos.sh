#!/usr/bin/env bash
set -euo pipefail

APP_NAME_DEFAULT="Rivett"
HELPER_NAME="Rivett Settings"
BIN_NAME="rivett"
HELPER_BIN_NAME="rivett-settings"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE_DIR="${ROOT_DIR}/target/release/bundle/osx"

cargo bundle --release

MAIN_APP=""
if [[ -d "${BUNDLE_DIR}/${APP_NAME_DEFAULT}.app" ]]; then
  MAIN_APP="${BUNDLE_DIR}/${APP_NAME_DEFAULT}.app"
else
  MAIN_APP="$(find "${BUNDLE_DIR}" -maxdepth 1 -name "*.app" -print -quit 2>/dev/null || true)"
fi

if [[ -z "${MAIN_APP}" || ! -d "${MAIN_APP}" ]]; then
  echo "Main app bundle not found in: ${BUNDLE_DIR}" >&2
  exit 1
fi

APP_NAME="$(basename "${MAIN_APP}" .app)"
HELPER_APP="${BUNDLE_DIR}/${HELPER_NAME}.app"

rm -rf "${HELPER_APP}"
cp -R "${MAIN_APP}" "${HELPER_APP}"

/usr/libexec/PlistBuddy -c "Set :CFBundleName ${HELPER_NAME}" \
  "${HELPER_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName ${HELPER_NAME}" \
  "${HELPER_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable ${HELPER_BIN_NAME}" \
  "${HELPER_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSUIElement bool true" \
  "${HELPER_APP}/Contents/Info.plist" >/dev/null 2>&1 || true

rm -f "${HELPER_APP}/Contents/MacOS/${BIN_NAME}"
cp "${ROOT_DIR}/target/release/${BIN_NAME}" \
  "${HELPER_APP}/Contents/MacOS/${HELPER_BIN_NAME}"
chmod +x "${HELPER_APP}/Contents/MacOS/${HELPER_BIN_NAME}"

echo "Built helper app: ${HELPER_APP}"
