#!/bin/bash

# MC-Minder TUI 启动脚本
# 基于 dialog 工具的图形化管理界面
# 兼容 Termux 和普通 Linux 系统

set -euo pipefail

# ==================== 全局配置 ====================
CONFIG_FILE="config.toml"
LOG_FILE="logs/latest.log"
RUST_BIN="./mc-minder"
RUST_PID_FILE="/tmp/mc-minder.pid"
SERVER_PID_FILE="/tmp/mc-server.pid"
SESSION_NAME="mc_server"

# ==================== 颜色定义（用于非 dialog 输出）====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ==================== 日志函数 ====================
log_info() {
    echo -e "${GREEN}[信息]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[警告]${NC} $1"
}

log_error() {
    echo -e "${RED}[错误]${NC} $1"
}

# ==================== 环境检测函数 ====================
is_termux() {
    [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]
}

check_dialog_installed() {
    if ! command -v dialog >/dev/null 2>&1; then
        return 1
    fi
    return 0
}

install_dialog() {
    if is_termux; then
        log_info "检测到 Termux 环境，正在安装 dialog..."
        pkg install -y dialog
    else
        if command -v apt-get >/dev/null 2>&1; then
            log_info "检测到 Debian/Ubuntu 系统..."
            sudo apt-get update && sudo apt-get install -y dialog
        elif command -v yum >/dev/null 2>&1; then
            log_info "检测到 CentOS/RHEL 系统..."
            sudo yum install -y dialog
        elif command -v dnf >/dev/null 2>&1; then
            log_info "检测到 Fedora 系统..."
            sudo dnf install -y dialog
        elif command -v pacman >/dev/null 2>&1; then
            log_info "检测到 Arch Linux 系统..."
            sudo pacman -S --noconfirm dialog
        else
            log_error "无法自动安装 dialog"
            echo ""
            echo "请手动安装 dialog 工具:"
            echo "  Ubuntu/Debian: sudo apt install dialog"
            echo "  CentOS/RHEL:   sudo yum install dialog"
            echo "  Fedora:        sudo dnf install dialog"
            echo "  Arch Linux:    sudo pacman -S dialog"
            exit 1
        fi
    fi
    
    if ! check_dialog_installed; then
        log_error "dialog 安装失败"
        exit 1
    fi
    
    log_info "dialog 安装成功!"
}

# ==================== 配置读取函数 ====================
get_config() {
    local key="$1"
    local default="$2"

    if [ -f "$RUST_BIN" ]; then
        local value=$("$RUST_BIN" config get "$key" 2>/dev/null)
        if [ -n "$value" ] && [ "$value" != "Error" ] && [ "$value" != "" ]; then
            echo "$value"
            return
        fi
    fi

    get_config_value "$key" "$default"
}

get_config_value() {
    local key="$1"
    local default="$2"

    if [ -f "$CONFIG_FILE" ]; then
        local line=$(grep -E "^${key}\s*=" "$CONFIG_FILE" 2>/dev/null | head -1)
        if [ -n "$line" ]; then
            local value="${line#*=}"
            value="${value#"${value%%[![:space:]]*}"}"
            value="${value%"${value##*[![:space:]]}"}"

            if [[ "$value" == \"*\" ]]; then
                value="${value#\"}"
                value="${value%\"}"
                echo "$value"
                return
            fi

            value="${value%%#*}"
            value="${value%"${value##*[![:space:]]}"}"

            if [ -n "$value" ]; then
                echo "$value"
                return
            fi
        fi
    fi
    echo "$default"
}

load_config() {
    JAR=$(get_config "jar" "fabric-server.jar")
    MIN_MEM=$(get_config "min_mem" "512M")
    MAX_MEM=$(get_config "max_mem" "1G")
    SESSION=$(get_config "session_name" "mc_server")
}

# ==================== 依赖检查函数 ====================
check_dependencies() {
    local missing=()
    
    command -v java >/dev/null 2>&1 || missing+=("java")
    command -v tmux >/dev/null 2>&1 || missing+=("tmux")
    
    if [ ${#missing[@]} -ne 0 ]; then
        dialog --title "依赖检查失败" --msgbox "\n缺少以下依赖:\n\n  ${missing[*]}\n\n请先安装缺少的依赖后再继续。" 12 60
        return 1
    fi
    
    return 0
}

check_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        dialog --title "配置文件缺失" --msgbox "\n配置文件不存在: $CONFIG_FILE\n\n请运行「初始化配置」创建配置文件。" 10 50
        return 1
    fi
    return 0
}

check_server_jar() {
    if [ ! -f "${JAR:-fabric-server.jar}" ]; then
        dialog --title "服务器核心缺失" --msgbox "\n服务器核心不存在: ${JAR:-fabric-server.jar}\n\n请下载 fabric-server.jar 并放置在当前目录。\n访问: https://fabricmc.net/use/installer/" 12 60
        return 1
    fi
    return 0
}

check_rust_binary() {
    if [ ! -f "$RUST_BIN" ]; then
        dialog --title "二进制文件缺失" --msgbox "\nMC-Minder 二进制文件不存在: $RUST_BIN\n\n请从 GitHub Releases 下载或从源码编译:\nhttps://github.com/SharkMI-0x7E/mc-minder/releases" 11 60
        return 1
    fi
    return 0
}

# ==================== 核心操作函数 ====================
start_background() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        dialog --title "会话已存在" --yesno "\n会话 '$SESSION' 已存在。\n\n是否终止现有会话并重启?" 9 50
        case $? in
            0) tmux kill-session -t "$SESSION" ;;
            *) return 1 ;;
        esac
    fi
    
    dialog --infobox "\n正在启动 Minecraft 服务器...\n\n  核心文件: $JAR\n  内存范围: $MIN_MEM - $MAX_MEM\n  会话名称: $SESSION" 10 60
    sleep 1
    
    mkdir -p logs
    
    tmux new-session -d -s "$SESSION" -x 120 -y 30
    tmux send-keys -t "$SESSION" "cd '$(pwd)'" Enter
    tmux send-keys -t "$SESSION" "java -Xms$MIN_MEM -Xmx$MAX_MEM -jar $JAR nogui" Enter
    
    echo $! > "$SERVER_PID_FILE"
    
    sleep 5
    
    if check_rust_binary; then
        dialog --infobox "\n正在启动 MC-Minder..." 5 40
        
        nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
        RUST_PID=$!
        echo $RUST_PID > "$RUST_PID_FILE"
        
        (
            while true; do
                if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                    kill -TERM $(cat "$RUST_PID_FILE" 2>/dev/null) 2>/dev/null
                    rm -f "$RUST_PID_FILE" "$SERVER_PID_FILE"
                    exit 0
                fi
                
                if [ -f "$RUST_PID_FILE" ]; then
                    RUST_PID=$(cat "$RUST_PID_FILE")
                    if ! kill -0 "$RUST_PID" 2>/dev/null; then
                        if check_rust_binary; then
                            nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
                            echo ! > "$RUST_PID_FILE"
                        fi
                    fi
                fi
                
                sleep 10
            done
        ) &
        WATCHDOG_PID=$!
        echo $WATCHDOG_PID > /tmp/mc-minder-watchdog.pid
    fi
    
    dialog --msgbox "\n启动完成!\n\n服务器已在 tmux 会话 '$SESSION' 中启动\n附加命令: tmux attach -t $SESSION" 10 60
}

stop_server() {
    dialog --infobox "\n正在停止服务器..." 5 30
    sleep 1
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        kill "$WATCHDOG_PID" 2>/dev/null
        rm -f /tmp/mc-minder-watchdog.pid
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        kill -TERM "$RUST_PID" 2>/dev/null
        rm -f "$RUST_PID_FILE"
    fi
    
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        tmux send-keys -t "$SESSION" "stop" Enter
        
        for i in {1..30}; do
            if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                break
            fi
            sleep 1
        done
        
        if tmux has-session -t "$SESSION" 2>/dev/null; then
            tmux kill-session -t "$SESSION"
        fi
    fi
    
    rm -f "$SERVER_PID_FILE"
    
    dialog --msgbox "\n服务器已成功停止!" 7 40
}

restart_server() {
    stop_server
    sleep 2
    
    if check_dependencies && check_config && check_server_jar; then
        start_background
    fi
}

status_server() {
    load_config
    
    local status_text=""
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        status_text+="服务器:    运行中 (tmux 会话: $SESSION)\n"
    else
        status_text+="服务器:    已停止\n"
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        if kill -0 "$RUST_PID" 2>/dev/null; then
            status_text+="MC-Minder: 运行中 (PID: $RUST_PID)\n"
        else
            status_text+="MC-Minder: 已退出 (PID 文件残留)\n"
        fi
    else
        status_text+="MC-Minder: 未运行\n"
    fi
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        if kill -0 "$WATCHDOG_PID" 2>/dev/null; then
            status_text+="看门狗:    运行中 (PID: $WATCHDOG_PID)\n"
        else
            status_text+="看门狗:    已退出\n"
        fi
    else
        status_text+="看门狗:    未运行\n"
    fi
    
    dialog --title "MC-Minder 状态" --msgbox "\n$status_text" 14 55
}

attach_server() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        clear
        log_info "正在附加到服务器会话..."
        log_info "按 Ctrl+B 然后按 D 可分离会话"
        sleep 1
        tmux attach -t "$SESSION"
    else
        dialog --title "错误" --msgbox "\n未找到运行中的服务器会话!" 7 40
    fi
}

show_logs() {
    if [ -f "$LOG_FILE" ]; then
        tail -n 50 "$LOG_FILE" | dialog --title "服务器日志 (最后50行)" --textbox - 20 80
    else
        dialog --title "错误" --msgbox "\n日志文件不存在: $LOG_FILE\n\n请先启动服务器以生成日志。" 8 50
    fi
}

show_minder_logs() {
    local minder_log="logs/mc-minder.log"
    if [ -f "$minder_log" ]; then
        tail -n 50 "$minder_log" | dialog --title "MC-Minder 日志 (最后50行)" --textbox - 20 80
    else
        dialog --title "错误" --msgbox "\nMC-Minder 日志文件不存在!" 6 40
    fi
}

# ==================== 初始化配置函数 ====================
init_config() {
    if ! check_rust_binary; then
        return 1
    fi
    
    exec 3>&1
    VALUES=$(dialog --ok-label "保存配置" \
        --cancel-label "取消" \
        --form "\nMC-Minder 初始化配置\n\n请填写以下信息:" \
        18 70 0 \
        "服务器 JAR 文件:"     1 1 "$(get_config 'jar' 'fabric-server.jar')"       1 25 45 0 \
        "最小内存:"             2 1 "$(get_config 'min_mem' '512M')"                 2 25 10 0 \
        "最大内存:"             3 1 "$(get_config 'max_mem' '1G')"                   3 25 10 0 \
        "Tmux 会话名称:"        4 1 "$(get_config 'session_name' 'mc_server')"        4 25 20 0 \
        "RCON 端口:"            5 1 "$(get_config 'rcon_port' '25575')"              5 25 10 0 \
        "RCON 密码:"            6 1 "$(get_config 'rcon_password' 'password')"       6 25 30 0 \
        "AI 提供商:"            7 1 "$(get_config 'ai_provider' 'openai')"           7 25 15 0 \
        "API 密钥:"             8 1 "$(get_config 'api_key' '')"                      8 25 40 0 \
        "API 模型:"             9 1 "$(get_config 'model' 'gpt-4o-mini')"           9 25 25 0 \
        "HTTP API 端口:"       10 1 "$(get_config 'http_port' '8080')"              10 25 10 0 \
        2>&1 1>&3)
    exit_code=$?
    exec 3>&-
    
    case $exit_code in
        0)
            IFS=$'\n' read -rd '' -a lines <<<"$VALUES" || true
            
            JAR_VAL="${lines[0]:-fabric-server.jar}"
            MIN_MEM_VAL="${lines[1]:-512M}"
            MAX_MEM_VAL="${lines[2]:-1G}"
            SESSION_VAL="${lines[3]:-mc_server}"
            RCON_PORT_VAL="${lines[4]:-25575}"
            RCON_PASS_VAL="${lines[5]:-password}"
            AI_PROVIDER_VAL="${lines[6]:-openai}"
            API_KEY_VAL="${lines[7]:-}"
            MODEL_VAL="${lines[8]:-gpt-4o-mini}"
            HTTP_PORT_VAL="${lines[9]:-8080}"
            
            cat > "$CONFIG_FILE" <<EOF
# MC-Minder 配置文件
# 由 TUI 初始化向导生成

[server]
jar = "$JAR_VAL"
min_mem = "$MIN_MEM_VAL"
max_mem = "$MAX_MEM_VAL"
session_name = "$SESSION_VAL"

[rcon]
host = "127.0.0.1"
port = $RCON_PORT_VAL
password = "$RCON_PASS_VAL"

[ai]
api_url = ""
api_key = "$API_KEY_VAL"
model = "$MODEL_VAL"
trigger = "!"
max_tokens = 150
temperature = 0.7

[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"

[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true

[jvm]
gc = "G1GC"
extra_flags = ""
EOF
            
            dialog --msgbox "\n配置已保存到 $CONFIG_FILE" 7 50
            ;;
        *)
            dialog --msgbox "\n配置已取消!" 6 30
            ;;
    esac
}

update_mcminder() {
    if ! check_rust_binary; then
        return 1
    fi
    
    dialog --infobox "\n正在更新 MC-Minder...\n这可能需要一些时间，请耐心等待。" 8 50
    
    if "$RUST_BIN" self-update; then
        dialog --msgbox "\nMC-Minder 更新成功!\n\n建议重启服务以应用更新。" 8 50
    else
        dialog --msgbox "\n更新失败!\n\n请检查网络连接或手动从 GitHub 下载最新版本。" 8 50
    fi
}

# ==================== 主菜单函数 ====================
show_main_menu() {
    while true; do
        exec 3>&1
        selection=$(dialog --clear \
            --backtitle "MC-Minder v0.3.4 - Minecraft 服务器管理套件" \
            --title "主菜单" \
            --menu "\n选择要执行的操作:" \
            18 60 10 \
            1 "启动服务器" \
            2 "停止服务器" \
            3 "重启服务器" \
            4 "查看服务器状态" \
            5 "附加到服务器控制台" \
            6 "查看服务器日志" \
            7 "查看 MC-Minder 日志" \
            8 "初始化配置" \
            9 "更新 MC-Minder" \
            10 "退出" \
            2>&1 1>&3)
        exit_code=$?
        exec 3>&-
        
        case $exit_code in
            0)
                case $selection in
                    1)
                        if check_dependencies && check_config && check_server_jar; then
                            start_background
                        fi
                        ;;
                    2) stop_server ;;
                    3) restart_server ;;
                    4) status_server ;;
                    5) attach_server ;;
                    6) show_logs ;;
                    7) show_minder_logs ;;
                    8) init_config ;;
                    9) update_mcminder ;;
                    10) clear
                        log_info "感谢使用 MC-Minder!"
                        exit 0
                        ;;
                esac
                ;;
            1|255)
                clear
                log_info "感谢使用 MC-Minder!"
                exit 0
                ;;
        esac
    done
}

# ==================== 主程序入口 ====================
main() {
    if ! check_dialog_installed; then
        echo ""
        echo "========================================="
        echo "  MC-Minder TUI 启动器"
        echo "========================================="
        echo ""
        echo "未检测到 dialog 工具，正在尝试安装..."
        echo ""
        
        install_dialog
    fi
    
    export LANG=${LANG:-zh_CN.UTF-8}
    
    dialog --backtitle "MC-Minder v0.3.4" \
        --title "欢迎使用" \
        --msgbox "\n欢迎使用 MC-Minder TUI 管理界面!\n\n这是一个基于 dialog 的图形化管理工具,\n可以方便地管理你的 Minecraft Fabric 服务器。\n\n按 Enter 进入主菜单..." 13 55
    
    show_main_menu
}

main "$@"
