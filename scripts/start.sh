#!/bin/bash

CONFIG_FILE="config.toml"
LOG_FILE="logs/latest.log"
RUST_BIN="./mc-minder"
RUST_PID_FILE="/tmp/mc-minder.pid"
SESSION_NAME="mc_server"

# ==================== 颜色定义 ====================
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

# ==================== 二进制文件检测 ====================
find_rust_binary() {
    if [ -f "$RUST_BIN" ]; then
        return 0
    fi

    for candidate in mc-minder-termux-aarch64 mc-minder-x86_64-linux; do
        if [ -f "./$candidate" ]; then
            log_info "Found binary: $candidate, creating symlink..."
            ln -sf "$candidate" "$RUST_BIN"
            return 0
        fi
    done

    return 1
}

# ==================== 配置读取函数（纯 awk 解析 TOML）====================
# 使用 awk 解析 config.toml，不依赖 mc-minder 二进制
get_config_value() {
    local key="$1"
    local default="$2"

    if [ ! -f "$CONFIG_FILE" ]; then
        echo "$default"
        return
    fi

    # 使用 awk 解析 TOML 配置文件
    local value=$(awk -F '=' -v key="$key" '
    BEGIN { in_server = 0 }
    /^\[server\]/ { in_server = 1; next }
    /^\[/ && !/^\[server\]/ { in_server = 0; next }
    in_server && $1 ~ key {
        # 获取等号右边的值
        val = $2
        # 移除行内注释
        sub(/#.*/, "", val)
        # 去除首尾空格
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", val)
        # 去除引号
        gsub(/^["'"'"']|["'"'"']$/, "", val)
        if (val != "") {
            print val
            exit
        }
    }
    ' "$CONFIG_FILE")

    if [ -n "$value" ]; then
        echo "$value"
    else
        echo "$default"
    fi
}

load_config() {
    if [ -f "$CONFIG_FILE" ]; then
        JAR=$(get_config_value "jar" "fabric-server.jar")
        MIN_MEM=$(get_config_value "min_mem" "512M")
        MAX_MEM=$(get_config_value "max_mem" "1G")
        SESSION=$(get_config_value "session_name" "mc_server")
    else
        JAR="fabric-server.jar"
        MIN_MEM="512M"
        MAX_MEM="1G"
        SESSION="mc_server"
    fi
}

is_termux() {
    [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]
}

check_dependencies() {
    local missing=()
    
    command -v java >/dev/null 2>&1 || missing+=("java")
    command -v tmux >/dev/null 2>&1 || missing+=("tmux")
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "缺少依赖: ${missing[*]}"
        
        if is_termux; then
            log_info "检测到 Termux 环境"
            read -p "是否安装缺少的依赖? (Y/n): " choice
            case "$choice" in
                n|N )
                    log_error "缺少依赖，无法继续"
                    exit 1
                    ;;
                * )
                    log_info "正在安装依赖..."
                    pkg install -y openjdk-17 tmux
                    ;;
            esac
        else
            log_info "请安装缺少的依赖:"
            log_info "  Ubuntu/Debian: sudo apt install openjdk-17-jre tmux"
            log_info "  CentOS/RHEL:   sudo yum install java-17-openjdk tmux"
            log_info "  macOS:         brew install openjdk tmux"
            exit 1
        fi
    fi
}

check_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        log_error "配置文件不存在: $CONFIG_FILE"
        log_info "请运行 './mc-minder init' 创建配置文件"
        exit 1
    fi
}

check_server_jar() {
    if [ ! -f "$JAR" ]; then
        log_error "服务器核心不存在: $JAR"
        log_info "请下载 fabric-server.jar 并放置在当前目录"
        log_info "访问: https://fabricmc.net/use/installer/"
        exit 1
    fi
}

check_rust_binary() {
    if find_rust_binary; then
        return 0
    fi
    log_warn "MC-Minder 二进制文件不存在: $RUST_BIN"
    log_info "请从 GitHub Releases 下载或从源码编译:"
    log_info "  https://github.com/SharkMI-0x7E/mc-minder/releases"
    log_info ""
    log_info "或运行一键安装脚本:"
    log_info "  curl -fsSL https://raw.githubusercontent.com/SharkMI-0x7E/mc-minder/main/install.sh | bash"
    return 1
}

start_background() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_warn "会话 '$SESSION' 已存在"
        read -p "是否终止现有会话并重启? (y/N): " choice
        case "$choice" in
            y|Y )
                tmux kill-session -t "$SESSION"
                ;;
            * )
                log_info "已取消"
                return 1
                ;;
        esac
    fi
    
    log_info "正在启动 Minecraft 服务器..."
    log_info "  核心文件: $JAR"
    log_info "  内存范围: $MIN_MEM - $MAX_MEM"
    log_info "  会话名称: $SESSION"
    
    mkdir -p logs
    
    tmux new-session -d -s "$SESSION" -x 120 -y 30
    
    tmux send-keys -t "$SESSION" "cd '$(pwd)'" Enter
    tmux send-keys -t "$SESSION" "java -Xms$MIN_MEM -Xmx$MAX_MEM -jar $JAR nogui" Enter

    log_info "服务器已在 tmux 会话 '$SESSION' 中启动"
    log_info "附加到会话: tmux attach -t $SESSION"
    
    sleep 5
    
    if check_rust_binary; then
        log_info "正在启动 MC-Minder..."
        
        nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
        RUST_PID=$!
        echo $RUST_PID > "$RUST_PID_FILE"
        
        log_info "MC-Minder 已启动 (PID: $RUST_PID)"
        
        (
            while true; do
                if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                    log_warn "服务器会话已终止，正在停止 MC-Minder..."
                    kill -TERM $(cat "$RUST_PID_FILE" 2>/dev/null) 2>/dev/null
                    rm -f "$RUST_PID_FILE"
                    exit 0
                fi
                
                if [ -f "$RUST_PID_FILE" ]; then
                    RUST_PID=$(cat "$RUST_PID_FILE")
                    if ! kill -0 "$RUST_PID" 2>/dev/null; then
                        log_warn "MC-Minder 已退出，正在重启..."
                        if check_rust_binary; then
                            nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
                            echo $! > "$RUST_PID_FILE"
                        fi
                    fi
                fi
                
                sleep 10
            done
        ) &
        WATCHDOG_PID=$!
        echo $WATCHDOG_PID > /tmp/mc-minder-watchdog.pid
        log_info "看门狗已启动 (PID: $WATCHDOG_PID)"
    fi
    
    log_info "启动完成!"
}

stop_server() {
    log_info "正在停止服务器..."
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        kill "$WATCHDOG_PID" 2>/dev/null
        rm -f /tmp/mc-minder-watchdog.pid
        log_info "看门狗已停止"
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        kill -TERM "$RUST_PID" 2>/dev/null
        rm -f "$RUST_PID_FILE"
        log_info "MC-Minder 已停止"
    fi
    
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_info "正在发送 'stop' 命令到服务器..."
        tmux send-keys -t "$SESSION" "stop" Enter
        
        for i in {1..30}; do
            if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                break
            fi
            sleep 1
        done
        
        if tmux has-session -t "$SESSION" 2>/dev/null; then
            log_warn "服务器未正常停止，正在强制终止..."
            tmux kill-session -t "$SESSION"
        fi
        
        log_info "服务器已停止"
    else
        log_warn "未找到运行中的会话"
    fi
    
    rm -f "$RUST_PID_FILE"
}

status_server() {
    load_config
    
    echo ""
    echo "MC-Minder 状态"
    echo "==============="
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        echo -e "服务器:    ${GREEN}运行中${NC} (tmux 会话: $SESSION)"
    else
        echo -e "服务器:    ${RED}已停止${NC}"
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        if kill -0 "$RUST_PID" 2>/dev/null; then
            echo -e "MC-Minder: ${GREEN}运行中${NC} (PID: $RUST_PID)"
        else
            echo -e "MC-Minder: ${RED}已退出${NC} (PID 文件残留)"
        fi
    else
        echo -e "MC-Minder: ${RED}未运行${NC}"
    fi
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        if kill -0 "$WATCHDOG_PID" 2>/dev/null; then
            echo -e "看门狗:    ${GREEN}运行中${NC} (PID: $WATCHDOG_PID)"
        else
            echo -e "看门狗:    ${RED}已退出${NC}"
        fi
    else
        echo -e "看门狗:    ${RED}未运行${NC}"
    fi
    
    echo ""
}

attach_server() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_info "正在附加到服务器会话..."
        log_info "按 Ctrl+B 然后按 D 可分离会话"
        tmux attach -t "$SESSION"
    else
        log_error "未找到运行中的会话"
    fi
}

show_logs() {
    if [ -f "$LOG_FILE" ]; then
        log_info "显示服务器日志最后 50 行..."
        tail -n 50 "$LOG_FILE"
    else
        log_error "日志文件不存在: $LOG_FILE"
        log_info "请先启动服务器以生成日志"
    fi
}

show_minder_logs() {
    local minder_log="logs/mc-minder.log"
    if [ -f "$minder_log" ]; then
        log_info "显示 MC-Minder 日志最后 50 行..."
        tail -n 50 "$minder_log"
    else
        log_error "MC-Minder 日志文件不存在"
    fi
}

case "${1:-}" in
    start)
        check_dependencies
        check_config
        check_server_jar
        start_background
        ;;
    stop)
        stop_server
        ;;
    restart)
        stop_server
        sleep 2
        check_dependencies
        check_config
        check_server_jar
        start_background
        ;;
    status)
        status_server
        ;;
    attach|a)
        attach_server
        ;;
    logs|l)
        show_logs
        ;;
    minder-logs|ml)
        show_minder_logs
        ;;
    init)
        if find_rust_binary; then
            "$RUST_BIN" init
        else
            log_error "MC-Minder 二进制文件不存在"
            exit 1
        fi
        ;;
    update)
        if find_rust_binary; then
            log_info "正在更新 MC-Minder..."
            if "$RUST_BIN" self-update; then
                log_info "MC-Minder 更新成功!"
                log_info "建议重启服务以应用更新。"
            else
                log_error "MC-Minder 更新失败!"
                log_info "可能的原因:"
                log_info "  1. 网络连接问题"
                log_info "  2. GitHub API 访问受限"
                log_info "  3. 权限不足"
                log_info ""
                log_info "请尝试手动下载:"
                log_info "  https://github.com/SharkMI-0x7E/mc-minder/releases"
                exit 1
            fi
        else
            log_error "MC-Minder 二进制文件不存在"
            log_info "请从 GitHub Releases 下载或从源码编译:"
            log_info "  https://github.com/SharkMI-0x7E/mc-minder/releases"
            log_info ""
            log_info "或运行一键安装脚本:"
            log_info "  curl -fsSL https://raw.githubusercontent.com/SharkMI-0x7E/mc-minder/main/install.sh | bash"
            exit 1
        fi
        ;;
    *)
        echo "MC-Minder - Minecraft 服务器管理器"
        echo ""
        echo "用法: $0 {start|stop|restart|status|attach|logs|init|update}"
        echo ""
        echo "命令:"
        echo "  start        启动服务器和 MC-Minder"
        echo "  stop         停止所有服务"
        echo "  restart      重启所有服务"
        echo "  status       查看状态"
        echo "  attach (a)   附加到服务器控制台"
        echo "  logs (l)     查看服务器日志"
        echo "  minder-logs  查看 MC-Minder 日志"
        echo "  init         初始化配置"
        echo "  update       更新 MC-Minder 到最新版本"
        ;;
esac
