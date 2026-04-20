#!/usr/bin/env bash
# One-click launcher for FormatLab.
#
# Behaviour:
#   1. If a release binary is built, run it directly. The app opens in
#      about a second and needs no dev server or node processes.
#   2. If nothing is built yet, build the release once (takes ~5-10 min
#      the first time on a fresh clone) then launch it.
#   3. If a build fails, fall back to `npm run tauri:dev` in a terminal
#      so you can see what went wrong.
#
# Designed for "double-click and it just works" usage via the .desktop
# entry installed by scripts/install-desktop-entry.sh.

set -euo pipefail

# ---- locate project ---------------------------------------------------------

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &>/dev/null && pwd )"
PROJECT_DIR="$( cd -- "${SCRIPT_DIR}/.." &>/dev/null && pwd )"
cd "${PROJECT_DIR}"

RELEASE_BIN="${PROJECT_DIR}/src-tauri/target/release/formatlab"
APPIMAGE_DIR="${PROJECT_DIR}/src-tauri/target/release/bundle/appimage"
LOG_DIR="${PROJECT_DIR}/.launcher-logs"
mkdir -p "${LOG_DIR}"

# ---- helpers ----------------------------------------------------------------

# Find a terminal emulator we can use to surface build progress / errors.
# Tries a bunch of common ones and returns the first one found.
find_terminal() {
    for t in \
        gnome-terminal \
        x-terminal-emulator \
        tilix \
        konsole \
        xfce4-terminal \
        mate-terminal \
        lxterminal \
        alacritty \
        kitty \
        xterm; do
        if command -v "${t}" >/dev/null 2>&1; then
            echo "${t}"
            return 0
        fi
    done
    return 1
}

# Run a command in a new terminal window, detached from this launcher so
# the parent Nemo "Run" click doesn't keep a zombie process around.
# Args: $1 = window title, remaining args = command + args
run_in_terminal() {
    local title="$1"; shift
    local term
    term="$(find_terminal)" || {
        # No terminal available — run in background and tee to a log.
        nohup "$@" >"${LOG_DIR}/launch.log" 2>&1 &
        disown
        return 0
    }

    case "${term}" in
        gnome-terminal|mate-terminal|tilix)
            setsid "${term}" --title="${title}" -- bash -c "$(printf '%q ' "$@"); echo; read -rp 'Press Enter to close…'" &
            ;;
        konsole)
            setsid konsole --new-tab -p "tabtitle=${title}" -e bash -c "$(printf '%q ' "$@"); echo; read -rp 'Press Enter to close…'" &
            ;;
        xfce4-terminal)
            setsid xfce4-terminal --title="${title}" --command="bash -c \"$(printf '%q ' "$@"); echo; read -rp 'Press Enter to close…'\"" &
            ;;
        x-terminal-emulator|lxterminal|alacritty|kitty|xterm)
            setsid "${term}" -T "${title}" -e bash -c "$(printf '%q ' "$@"); echo; read -rp 'Press Enter to close…'" &
            ;;
    esac
    disown 2>/dev/null || true
}

# Best-guess test: is the release binary fresh enough to run, or is it
# older than the source? If older, we should rebuild before launching.
needs_rebuild() {
    [[ ! -x "${RELEASE_BIN}" ]] && return 0
    # Any source file newer than the binary means it's stale.
    if find \
        "${PROJECT_DIR}/src" \
        "${PROJECT_DIR}/src-tauri/src" \
        "${PROJECT_DIR}/src-tauri/Cargo.toml" \
        "${PROJECT_DIR}/src-tauri/tauri.conf.json" \
        "${PROJECT_DIR}/index.html" \
        "${PROJECT_DIR}/package.json" \
        -newer "${RELEASE_BIN}" \
        -print -quit 2>/dev/null | grep -q .; then
        return 0
    fi
    return 1
}

# ---- main -------------------------------------------------------------------

# Fast path: already built and source is unchanged.
if ! needs_rebuild; then
    exec "${RELEASE_BIN}" "$@"
fi

# Slow path: something's changed or nothing is built yet. Build in a
# terminal so the user can see progress, then launch the fresh binary.
# We don't use `exec` here because we want to control the sequence.
BUILD_CMD=(
    bash -c "
        cd '${PROJECT_DIR}'
        echo '== FormatLab: preparing build ==' ;
        if [ ! -d node_modules ]; then
            echo '- installing node packages (first-time only)' ;
            npm install --no-audit --no-fund || exit 1 ;
        fi ;
        echo '- building release binary (this takes a few minutes the first time)' ;
        # --no-bundle skips .deb / .AppImage / .rpm creation and, critically,
        # the signed-updater step that needs TAURI_SIGNING_PRIVATE_KEY. The
        # launcher only ever runs the resulting binary, so those artifacts
        # are redundant here. For a full signed release, use the GitHub
        # Actions workflow (it has the private key) or run
        # \`npm run tauri:build\` manually with the env var set.
        npm run tauri:build -- --no-bundle || exit 1 ;
        echo ;
        echo '== Build complete, launching FormatLab ==' ;
        exec '${RELEASE_BIN}'
    "
)

if find_terminal >/dev/null; then
    run_in_terminal "FormatLab — first-time build" "${BUILD_CMD[@]}"
else
    # Headless fallback (rare). Build silently, then exec.
    (
        cd "${PROJECT_DIR}"
        [[ ! -d node_modules ]] && npm install --no-audit --no-fund >"${LOG_DIR}/npm-install.log" 2>&1
        npm run tauri:build -- --no-bundle >"${LOG_DIR}/build.log" 2>&1 || exit 1
        exec "${RELEASE_BIN}"
    ) &
    disown
fi
