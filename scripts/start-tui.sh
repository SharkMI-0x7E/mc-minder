#!/bin/bash
# MC-Minder TUI 启动脚本（精简入口）
# 现在只负责加载 lib/*.sh 并启动主菜单
# 所有逻辑已拆分到 scripts/lib/*.sh

set -euo pipefail

# ==================== 路径配置 ====================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"

# 按依赖顺序加载模块
source "$LIB_DIR/common.sh"
source "$LIB_DIR/config.sh"
source "$LIB_DIR/java.sh"
source "$LIB_DIR/server.sh"
source "$LIB_DIR/log.sh"
source "$LIB_DIR/menu.sh"

# ==================== 主程序入口 ====================
main() {
    debug_log "main: starting"

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