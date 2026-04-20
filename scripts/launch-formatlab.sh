#!/usr/bin/env bash
# One-click launcher for FormatLab dev builds.
#
# Put this file on your Desktop (or wherever), mark executable once, and
# double-click to run. It will pick the best available binary in this
# order:
#   1. ./src-tauri/target/release/formatlab   (after `npm run tauri:build`)
#   2. ./src-tauri/target/debug/formatlab     (after `npm run tauri:dev`)
#   3. The first .AppImage found in src-tauri/target/release/bundle/appimage

set -euo pipefail

# Resolve the FormatLab project directory regardless of where this script
# lives or is invoked from.
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &>/dev/null && pwd )"
PROJECT_DIR="$( cd -- "${SCRIPT_DIR}/.." &>/dev/null && pwd )"
cd "${PROJECT_DIR}"

RELEASE_BIN="${PROJECT_DIR}/src-tauri/target/release/formatlab"
DEBUG_BIN="${PROJECT_DIR}/src-tauri/target/debug/formatlab"
APPIMAGE_DIR="${PROJECT_DIR}/src-tauri/target/release/bundle/appimage"

if [[ -x "${RELEASE_BIN}" ]]; then
    exec "${RELEASE_BIN}" "$@"
elif [[ -x "${DEBUG_BIN}" ]]; then
    exec "${DEBUG_BIN}" "$@"
elif [[ -d "${APPIMAGE_DIR}" ]]; then
    APPIMAGE="$( find "${APPIMAGE_DIR}" -maxdepth 1 -type f -name '*.AppImage' -print -quit )"
    if [[ -n "${APPIMAGE}" ]]; then
        chmod +x "${APPIMAGE}"
        exec "${APPIMAGE}" "$@"
    fi
fi

# Fall back to dev mode if nothing is built yet.
echo "No FormatLab binary found yet. Starting in dev mode (first run may take a few minutes)…"
exec npm run tauri:dev
