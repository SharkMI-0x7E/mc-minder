#!/bin/bash
# MC-Minder TUI Launcher
# Launches the native Rust TUI interface
# All TUI functionality is built into the mc-minder binary

set -euo pipefail

# ==================== Find mc-minder binary ====================
find_mcminder() {
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local found_not_executable=""

    # Check common locations (including platform-specific binary names from releases)
    local candidates=(
        "${MC_MINDER_BIN:-}"
        "$script_dir/mc-minder"
        "$script_dir/mc-minder-termux-aarch64"
        "$script_dir/mc-minder-x86_64-linux"
        "$script_dir/../mc-minder"
        "$script_dir/../mc-minder-termux-aarch64"
        "$script_dir/../mc-minder-x86_64-linux"
        "$(dirname "$script_dir")/mc-minder"
        "$(dirname "$script_dir")/mc-minder-termux-aarch64"
        "$(dirname "$script_dir")/mc-minder-x86_64-linux"
        "./mc-minder"
        "./mc-minder-termux-aarch64"
        "./mc-minder-x86_64-linux"
    )

    for bin in "${candidates[@]}"; do
        if [ -n "$bin" ] && [ -f "$bin" ]; then
            if [ -x "$bin" ]; then
                echo "$bin"
                return 0
            else
                found_not_executable="$bin"
            fi
        fi
    done

    # Try PATH
    if command -v mc-minder >/dev/null 2>&1; then
        command -v mc-minder
        return 0
    fi

    if [ -n "$found_not_executable" ]; then
        echo "Error: mc-minder found at $found_not_executable but is not executable"
        echo ""
        echo "Run this command to fix:"
        echo "  chmod +x \"$found_not_executable\""
        exit 1
    fi

    echo "Error: mc-minder binary not found"
    echo ""
    echo "Solutions:"
    echo "  1. Place mc-minder in the same directory as this script"
    echo "  2. Set MC_MINDER_BIN environment variable"
    echo "  3. Add mc-minder to your PATH"
    exit 1
}

BIN=$(find_mcminder)

# Launch TUI
exec "$BIN" tui "$@"
