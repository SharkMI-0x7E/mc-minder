#!/bin/bash
# MC-Minder TUI 启动脚本（精简入口）
# 现在只负责加载 lib/*.sh 并启动主菜单
# 所有逻辑已拆分到 scripts/*.sh

set -euo pipefail

# ==================== 智能路径检测 ====================
# 支持多种部署方式：
#   1. start-tui.sh 和 scripts/ 在同一目录
#   2. start-tui.sh 在 scripts/ 目录内部
#   3. 通过环境变量 MC_MINDER_SCRIPTS 指定
#   4. 脚本在 PATH 中（通过符号链接）

find_script_dir() {
    local script_name="${BASH_SOURCE[0]}"
    
    # 如果是符号链接，解析真实路径
    if [ -L "$script_name" ]; then
        script_name="$(readlink -f "$script_name")"
    fi
    
    local script_dir="$(cd "$(dirname "$script_name")" && pwd)"
    
    # 情况 1: 脚本在 scripts/ 目录内部
    if [ -f "$script_dir/common.sh" ]; then
        echo "$script_dir"
        return
    fi
    
    # 情况 2: scripts/ 目录在脚本所在目录的上级或同级
    if [ -f "$script_dir/scripts/common.sh" ]; then
        echo "$script_dir/scripts"
        return
    fi
    
    # 情况 3: 从环境变量读取
    if [ -n "${MC_MINDER_SCRIPTS:-}" ] && [ -f "$MC_MINDER_SCRIPTS/common.sh" ]; then
        echo "$MC_MINDER_SCRIPTS"
        return
    fi
    
    # 情况 4: 检查常见的安装路径
    local common_paths=(
        "$HOME/.mc-minder/scripts"
        "/usr/local/share/mc-minder/scripts"
        "/opt/mc-minder/scripts"
        "$HOME/mc-minder/scripts"
    )
    
    for path in "${common_paths[@]}"; do
        if [ -f "$path/common.sh" ]; then
            echo "$path"
            return
        fi
    done
    
    # 错误：找不到脚本目录
    echo ""
    echo "ERROR: Cannot find MC-Minder script directory"
    echo ""
    echo "Solutions:"
    echo "  1. Ensure scripts/*.sh files are in the same directory as start-tui.sh"
    echo "  2. Set environment variable: export MC_MINDER_SCRIPTS=/path/to/scripts"
    echo "  3. Run from the directory containing start-tui.sh"
    echo ""
    exit 1
}

# 设置脚本目录
SCRIPTS_DIR="$(find_script_dir)"
if [ -z "$SCRIPTS_DIR" ]; then
    exit 1
fi

# 设置 LIB_DIR 为脚本目录（向后兼容）
LIB_DIR="$SCRIPTS_DIR"

# 导出变量供其他脚本使用
export MC_MINDER_SCRIPTS="$SCRIPTS_DIR"
export MC_MINDER_BIN="${MC_MINDER_BIN:-./mc-minder}"

# ==================== 加载脚本模块 ====================
source "$LIB_DIR/common.sh"
source "$LIB_DIR/config.sh"
source "$LIB_DIR/java.sh"
source "$LIB_DIR/server.sh"
source "$LIB_DIR/log.sh"
source "$LIB_DIR/menu.sh"

# ==================== 主程序入口 ====================
main() {
    debug_log "main: starting"
    debug_log "SCRIPTS_DIR=$SCRIPTS_DIR"
    debug_log "MC_MINDER_BIN=$MC_MINDER_BIN"

    # 加载语言设置
    load_lang

    # 清理可能残留的旧进程
    if [ -f "$RUST_PID_FILE" ]; then
        local old_pid=$(cat "$RUST_PID_FILE" 2>/dev/null)
        if kill -0 "$old_pid" 2>/dev/null; then
            log_warn "$(T "检测到残留的 MC-Minder 进程 (PID: $old_pid)，正在终止..." "Detected residual MC-Minder process (PID: $old_pid), terminating...")"
            kill -TERM "$old_pid" 2>/dev/null
            sleep 1
        fi
        rm -f "$RUST_PID_FILE"
    fi

    if [ -f "$WATCHDOG_PID_FILE" ]; then
        local old_watchdog=$(cat "$WATCHDOG_PID_FILE" 2>/dev/null)
        if kill -0 "$old_watchdog" 2>/dev/null; then
            kill -TERM "$old_watchdog" 2>/dev/null
        fi
        rm -f "$WATCHDOG_PID_FILE"
    fi

    # 确保 dialog 可用
    if ! check_dialog_installed; then
        echo ""
        echo "========================================="
        echo "  MC-Minder TUI $(T "启动器" "Launcher")"
        echo "========================================="
        echo ""
        echo "$(T "未检测到 dialog 工具，正在尝试安装..." "dialog tool not found, attempting to install...")"
        echo ""
        install_dialog
    fi

    export LANG=${LANG:-zh_CN.UTF-8}

    # 欢迎对话框
    dialog --backtitle "MC-Minder - $(T "Minecraft 服务器管理套件" "Minecraft Server Management Suite")" \
        --title "$(T "欢迎使用" "Welcome")" \
        --msgbox "\n$(T "欢迎使用 MC-Minder TUI 管理界面!" "Welcome to MC-Minder TUI Management Interface!")\n\n$(T "这是一个基于 dialog 的图形化管理工具," "This is a dialog-based graphical management tool,")\n$(T "可以方便地管理你的 Minecraft Fabric 服务器。" "making it easy to manage your Minecraft Fabric server.")\n\n$(T "按 Enter 进入主菜单..." "Press Enter to enter the main menu...")" 13 55

    show_main_menu
}

main "$@"