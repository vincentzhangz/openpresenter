#!/usr/bin/env bash
set -euo pipefail

# Build and package OpenPresenter as a macOS .app bundle and .dmg disk image.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_NAME="OpenPresenter"
BUNDLE_DIR="${WORKSPACE_ROOT}/target/bundle/osx/${APP_NAME}.app"
CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
DIST_DIR="${WORKSPACE_ROOT}/target/dist"

echo "==> Building OpenPresenter in release mode..."
cd "${WORKSPACE_ROOT}"
cargo build --release --no-default-features

echo "==> Constructing ${APP_NAME}.app bundle..."
rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"

# Copy binary
cp "${WORKSPACE_ROOT}/target/release/openpresenter" "${MACOS_DIR}/openpresenter"
chmod +x "${MACOS_DIR}/openpresenter"

# Copy Info.plist
cp "${WORKSPACE_ROOT}/packaging/Info.plist" "${CONTENTS_DIR}/Info.plist"

# PkgInfo
echo -n "APPLOPRN" > "${CONTENTS_DIR}/PkgInfo"

echo "==> ${APP_NAME}.app created successfully at ${BUNDLE_DIR}"

if command -v hdiutil >/dev/null 2>&1; then
    echo "==> Creating macOS disk image (.dmg)..."
    mkdir -p "${DIST_DIR}"
    DMG_PATH="${DIST_DIR}/${APP_NAME}-0.1.0-macos.dmg"
    rm -f "${DMG_PATH}"

    # Staging directory for DMG creation
    DMG_STAGING="${WORKSPACE_ROOT}/target/dmg_staging"
    rm -rf "${DMG_STAGING}"
    mkdir -p "${DMG_STAGING}"
    cp -R "${BUNDLE_DIR}" "${DMG_STAGING}/"
    ln -s /Applications "${DMG_STAGING}/Applications"

    hdiutil create -volname "${APP_NAME}" -srcfolder "${DMG_STAGING}" -ov -format UDZO "${DMG_PATH}"
    rm -rf "${DMG_STAGING}"
    echo "==> Disk image generated at: ${DMG_PATH}"
fi

echo "==> Packaging complete!"
