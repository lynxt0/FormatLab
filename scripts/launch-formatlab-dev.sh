#!/usr/bin/env bash
# Dev-mode launcher for FormatLab.
#
# Opens a terminal window, runs `npm run tauri:dev`, and leaves it open
# so you can see the dev server logs. Ideal when you're editing code and
# want hot reload. For regular use (just open the app), prefer
# launch-formatlab.sh — it's instant once built.

set -euo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &>/dev/null && pwd )"
PROJECT_DIR="$( cd -- "${SCRIPT_DIR}/.." &>/dev/null && pwd )"
cd "${PROJECT_DIR}"

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

DEV_CMD=(
    bash -c "
        cd '${PROJECT_DIR}'
        if [ ! -d node_modules ]; then
            echo '- installing node packages (first-time only)' ;
            npm install --no-audit --no-fund || exit 1 ;
        fi
        echo '== FormatLab dev mode (hot reload enabled) ==' ;
        echo '   Close this terminal or press Ctrl+C to stop the app.' ;
        echo ;
        exec npm run tauri:dev
    "
)

TERM="$(find_terminal)" || {
    echo "No terminal emulator available. Run 'npm run tauri:dev' manually." >&2
    exit 1
}

case "${TERM}" in
    gnome-terminal|mate-terminal|tilix)
        setsid "${TERM}" --title="FormatLab (dev)" -- "${DEV_CMD[@]}" &
        ;;
    konsole)
        setsid konsole --new-tab -p 'tabtitle=FormatLab (dev)' -e "${DEV_CMD[@]}" &
        ;;
    xfce4-terminal)
        setsid xfce4-terminal --title="FormatLab (dev)" --command="$(printf '%q ' "${DEV_CMD[@]}")" &
        ;;
    *)
        setsid "${TERM}" -T "FormatLab (dev)" -e "${DEV_CMD[@]}" &
        ;;
esac
disown 2>/dev/null || true
