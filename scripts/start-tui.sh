#!/bin/bash
# MC-Minder TUI Launcher
# Launches the native Rust TUI interface
# All TUI functionality is built into the mc-minder binary

set -euo pipefail

# ==================== Find mc-minder binary ====================
find_mcminder() {
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    # Check common locations
    local candidates=(
        "${MC_MINDER_BIN:-}"
        "$script_dir/mc-minder"
        "$script_dir/../mc-minder"
        "$(dirname "$script_dir")/mc-minder"
        "./mc-minder"
    )

    for bin in "${candidates[@]}"; do
        if [ -n "$bin" ] && [ -x "$bin" ]; then
            echo "$bin"
            return 0
        fi
    done

    # Try PATH
    if command -v mc-minder >/dev/null 2>&1; then
        command -v mc-minder
        return 0
    fi

    echo ""
    return 1
}

BIN=$(find_mcminder) || {
    echo "Error: mc-minder binary not found"
    echo ""
    echo "Solutions:"
    echo "  1. Place mc-minder in the same directory as this script"
    echo "  2. Set MC_MINDER_BIN environment variable"
    echo "  3. Add mc-minder to your PATH"
    exit 1
}

# Launch TUI
exec "$BIN" tui "$@"
