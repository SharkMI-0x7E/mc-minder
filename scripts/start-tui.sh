#!/bin/bash

# MC-Minder TUI 启动脚本
# 基于 dialog 工具的图形化管理界面
# 兼容 Termux 和普通 Linux 系统

set -euo pipefail

# ==================== 调试日志 ====================
DEBUG_DIR="./mc-minder_log"
DEBUG_FILE="$DEBUG_DIR/start-tui-debug.log"
mkdir -p "$DEBUG_DIR"

debug_log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$DEBUG_FILE"
}
debug_log "========== Script started =========="

# ==================== 全局配置 ====================
CONFIG_FILE="config.toml"
LOG_FILE="logs/latest.log"
RUST_BIN="./mc-minder"

# PID 文件路径（使用用户目录，避免 /tmp 权限问题）
PID_DIR="$HOME/.mc-minder/tmp"
mkdir -p "$PID_DIR"
RUST_PID_FILE="$PID_DIR/mc-minder.pid"
WATCHDOG_PID_FILE="$PID_DIR/mc-minder-watchdog.pid"

SESSION_NAME="mc_server"

debug_log "CONFIG_FILE=$CONFIG_FILE"
debug_log "PID_DIR=$PID_DIR"
debug_log "RUST_PID_FILE=$RUST_PID_FILE"

# ==================== 颜色定义（用于非 dialog 输出）====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ==================== 语言系统 ====================
# 语言配置文件
LANG_FILE="$HOME/.mc-minder/lang.conf"

# 默认语言
CURRENT_LANG="zh"

# 加载语言设置
load_lang() {
    if [ -f "$LANG_FILE" ]; then
        CURRENT_LANG=$(cat "$LANG_FILE" | tr -d '[:space:]')
    fi
}

# 保存语言设置
save_lang() {
    mkdir -p "$(dirname "$LANG_FILE")"
    echo "$CURRENT_LANG" > "$LANG_FILE"
}

# 语言切换函数
# 用法: T "中文" "English"
T() {
    if [ "$CURRENT_LANG" = "en" ]; then
        echo "$2"
    else
        echo "$1"
    fi
}

# 切换语言
switch_language() {
    local choice
    choice=$(dialog --clear \
        --backtitle "MC-Minder" \
        --title "$(T "语言设置" "Language Settings")" \
        --menu "\n$(T "选择语言 / Select Language:" "Select Language:")" \
        12 50 3 \
        "zh" "$(T "中文" "Chinese")" \
        "en" "$(T "英文" "English")" \
        2>&1 >/dev/tty)
    
    case $? in
        0)
            CURRENT_LANG="$choice"
            save_lang
            dialog --msgbox "\n$(T "语言已切换，重新进入菜单后生效。" "Language changed. Changes take effect after re-entering the menu.")" 7 50
            ;;
    esac
}

# ==================== 辅助函数：更新 TOML 配置 ====================
update_toml_value() {
    local section="$1"
    local key="$2"
    local value="$3"
    local file="$CONFIG_FILE"
    
    # 如果 section 不存在，则创建
    if ! grep -q "^\\[$section\\]" "$file" 2>/dev/null; then
        echo "" >> "$file"
        echo "[$section]" >> "$file"
    fi
    
    # 如果 key 存在，则替换；否则追加
    if grep -q "^[[:space:]]*${key}[[:space:]]*=" "$file" 2>/dev/null; then
        sed -i "s/^\\([[:space:]]*${key}[[:space:]]*=\\).*/\\1 \"$value\"/" "$file"
    else
        echo "$key = \"$value\"" >> "$file"
    fi
}

# ==================== Java 版本管理 ====================
# 检测已安装的 Java 版本
detect_java_versions() {
    JAVA_VERSIONS=()
    JAVA_PATHS=()
    
    # 检查 java 命令
    if command -v java >/dev/null 2>&1; then
        local java_ver
        java_ver=$(java -version 2>&1 | head -1 | awk -F '"' '{print $2}')
        local java_path
        java_path=$(command -v java)
        JAVA_VERSIONS+=("$java_ver")
        JAVA_PATHS+=("$java_path")
        debug_log "detect_java_versions: found java $java_ver at $java_path"
    fi
    
    # 检查常见的 Java 安装路径
    local java_dirs=(
        "/usr/lib/jvm"
        "/usr/java"
        "/opt/java"
        "/Library/Java/JavaVirtualMachines"  # macOS
        "$PREFIX/lib/jvm"  # Termux
    )
    
    for dir in "${java_dirs[@]}"; do
        if [ -d "$dir" ]; then
            for java_home in "$dir"/{java,openjdk,jdk}*; do
                if [ -d "$java_home" ] && [ -x "$java_home/bin/java" ]; then
                    local ver
                    ver=$("$java_home/bin/java" -version 2>&1 | head -1 | awk -F '"' '{print $2}')
                    # 避免重复
                    local exists=0
                    for existing in "${JAVA_VERSIONS[@]}"; do
                        if [ "$existing" = "$ver" ]; then
                            exists=1
                            break
                        fi
                    done
                    if [ $exists -eq 0 ]; then
                        JAVA_VERSIONS+=("$ver")
                        JAVA_PATHS+=("$java_home/bin/java")
                        debug_log "detect_java_versions: found java $ver at $java_home/bin/java"
                    fi
                fi
            done
        fi
    done
    
    # Termux 特殊处理
    if is_termux; then
        # 检查 pkg 安装的 openjdk
        for ver in 8 11 17 21; do
            local jvm_path="$PREFIX/lib/jvm/openjdk-$ver/bin/java"
            if [ -x "$jvm_path" ]; then
                local java_ver
                java_ver=$("$jvm_path" -version 2>&1 | head -1 | awk -F '"' '{print $2}')
                local exists=0
                for existing in "${JAVA_VERSIONS[@]}"; do
                    if [ "$existing" = "$java_ver" ]; then
                        exists=1
                        break
                    fi
                done
                if [ $exists -eq 0 ]; then
                    JAVA_VERSIONS+=("$java_ver")
                    JAVA_PATHS+=("$jvm_path")
                    debug_log "detect_java_versions: found termux openjdk-$ver"
                fi
            fi
        done
    fi
    
    # 如果没有找到任何版本，尝试通过 PATH 查找
    if [ ${#JAVA_VERSIONS[@]} -eq 0 ]; then
        # 检查常见的 java 路径
        local path_dirs=$(echo "$PATH" | tr ':' '\n')
        for dir in $path_dirs; do
            if [ -x "$dir/java" ]; then
                local ver
                ver=$("$dir/java" -version 2>&1 | head -1 | awk -F '"' '{print $2}')
                JAVA_VERSIONS+=("$ver")
                JAVA_PATHS+=("$dir/java")
                debug_log "detect_java_versions: found java $ver in PATH at $dir/java"
                break
            fi
        done
    fi
}

# Java 版本切换
switch_java_version() {
    detect_java_versions
    
    if [ ${#JAVA_VERSIONS[@]} -eq 0 ]; then
        dialog --title "$(T "错误" "Error")" \
            --msgbox "\n$(T "未检测到已安装的 Java 版本。" "No Java versions detected.")\n\n$(T "请先安装 Java:" "Please install Java first:")\n  $(T "Termux: pkg install openjdk-17" "Termux: pkg install openjdk-17")\n  $(T "Ubuntu: sudo apt install openjdk-17-jdk" "Ubuntu: sudo apt install openjdk-17-jdk")" 10 60
        return 1
    fi
    
    # 构建菜单选项
    local menu_items=()
    local i=1
    for idx in "${!JAVA_VERSIONS[@]}"; do
        menu_items+=("$i" "Java ${JAVA_VERSIONS[$idx]}")
        ((i++))
    done
    
    local choice
    choice=$(dialog --clear \
        --backtitle "MC-Minder" \
        --title "$(T "Java 版本管理" "Java Version Management")" \
        --menu "\n$(T "选择要使用的 Java 版本:" "Select Java version to use:")\n\n$(T "已检测到 ${#JAVA_VERSIONS[@]} 个版本" "Detected ${#JAVA_VERSIONS[@]} version(s)")" \
        15 60 ${#JAVA_VERSIONS[@]} \
        "${menu_items[@]}" \
        2>&1 >/dev/tty)
    
    case $? in
        0)
            local selected_idx=$((choice - 1))
            local selected_java="${JAVA_PATHS[$selected_idx]}"
            local selected_ver="${JAVA_VERSIONS[$selected_idx]}"
            
            # 如果选择的不是默认 java，创建符号链接或提示用户
            if [ "$selected_java" != "$(command -v java 2>/dev/null)" ]; then
                dialog --yesno "\n$(T "已选择 Java $selected_ver" "Selected Java $selected_ver")\n\n$(T "路径: $selected_java" "Path: $selected_java")\n\n$(T "是否将其设置为当前会话的默认 Java?" "Set as default Java for current session?")" 12 60
                
                case $? in
                    0)
                        # 更新 PATH 使选中的 java 优先
                        local java_dir
                        java_dir=$(dirname "$selected_java")
                        export PATH="$java_dir:$PATH"
                        debug_log "switch_java_version: added $java_dir to PATH"
                        
                        # 更新 JAVA_HOME
                        export JAVA_HOME=$(dirname "$java_dir")
                        debug_log "switch_java_version: set JAVA_HOME=$JAVA_HOME"
                        
                        dialog --msgbox "\n$(T "Java 版本已切换!" "Java version switched!")\n\n$(T "当前版本: $(java -version 2>&1 | head -1)" "Current version: $(java -version 2>&1 | head -1)")" 8 60
                        ;;
                esac
            else
                dialog --msgbox "\n$(T "当前已使用 Java $selected_ver" "Already using Java $selected_ver")" 6 40
            fi
            ;;
    esac
}

# 安装 Java 版本
install_java_version() {
    if is_termux; then
        local choice
        choice=$(dialog --clear \
            --backtitle "MC-Minder" \
            --title "$(T "安装 Java" "Install Java")" \
            --menu "\n$(T "选择要安装的 Java 版本:" "Select Java version to install:")" \
            14 50 5 \
            "8" "Java 8 $(T "(轻量级/旧服务器)" "(Lightweight/Old servers)")" \
            "11" "Java 11 $(T "(稳定版)" "(Stable)")" \
            "17" "Java 17 $(T "(推荐/最新MC)" "(Recommended/Latest MC)")" \
            "21" "Java 21 $(T "(最新版)" "(Latest)")" \
            "0" "$(T "返回" "Back")" \
            2>&1 >/dev/tty)
        
        case $? in
            0)
                if [ "$choice" = "0" ]; then
                    return
                fi
                
                dialog --infobox "\n$(T "正在安装 Java $choice..." "Installing Java $choice...")" 5 40
                debug_log "install_java_version: installing openjdk-$choice"
                
                if pkg install -y "openjdk-$choice" 2>&1; then
                    dialog --msgbox "\n$(T "Java $choice 安装成功!" "Java $choice installed successfully!")" 6 40
                else
                    dialog --msgbox "\n$(T "Java $choice 安装失败!" "Java $choice installation failed!")" 6 40
                fi
                ;;
        esac
    else
        dialog --title "$(T "安装 Java" "Install Java")" \
            --msgbox "\n$(T "请根据您的系统手动安装 Java:" "Please install Java manually for your system:")\n\n$(T "Ubuntu/Debian:" "Ubuntu/Debian:")\n  sudo apt install openjdk-17-jdk\n\n$(T "CentOS/RHEL:" "CentOS/RHEL:")\n  sudo yum install java-17-openjdk\n\n$(T "macOS:" "macOS:")\n  brew install openjdk@17\n\n$(T "Arch Linux:" "Arch Linux:")\n  sudo pacman -S jdk17-openjdk" 16 60
    fi
}

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

# ==================== 二进制文件检测（直接使用，不创建软链接，静默）====================
find_rust_binary() {
    # 如果已经找到有效二进制，直接返回
    if [ -f "$RUST_BIN" ] && [ -x "$RUST_BIN" ]; then
        debug_log "find_rust_binary: using existing $RUST_BIN"
        return 0
    fi

    # 遍历候选文件名，找到第一个可执行的
    for candidate in mc-minder-termux-aarch64 mc-minder-x86_64-linux; do
        if [ -f "./$candidate" ]; then
            debug_log "find_rust_binary: found candidate $candidate, using it directly"
            RUST_BIN="./$candidate"
            chmod +x "$RUST_BIN" 2>/dev/null || true
            return 0
        fi
    done

    debug_log "find_rust_binary: no binary found"
    return 1
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

# ==================== 配置读取函数（支持段名）====================
# 用法: get_config_value <段名> <键名> [默认值]
get_config_value() {
    local section="$1"
    local key="$2"
    local default="$3"

    if [ ! -f "$CONFIG_FILE" ]; then
        echo "$default"
        return
    fi

    local raw_line
    raw_line=$(sed -n "/^\[$section\]/,/^\[/p" "$CONFIG_FILE" \
        | grep -E "^[[:space:]]*${key}[[:space:]]*=" \
        | head -1)
    debug_log "get_config_value: section='$section', key='$key', raw_line='$raw_line'"

    if [ -z "$raw_line" ]; then
        echo "$default"
        return
    fi

    local value
    value=$(echo "$raw_line" \
        | cut -d'=' -f2- \
        | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
              -e 's/^"//' -e 's/"$//' \
              -e 's/[[:space:]]*#.*$//')
    debug_log "get_config_value: extracted value='$value'"

    if [ -n "$value" ]; then
        echo "$value"
    else
        echo "$default"
    fi
}

# 加载服务器配置（从 [server] 段）
load_server_config() {
    debug_log "load_server_config: loading..."
    JAR=$(get_config_value "server" "jar" "fabric-server.jar")
    MIN_MEM=$(get_config_value "server" "min_mem" "512M")
    MAX_MEM=$(get_config_value "server" "max_mem" "1G")
    SESSION=$(get_config_value "server" "session_name" "mc_server")
    LOG_FILE=$(get_config_value "server" "log_file" "logs/latest.log")
    debug_log "load_server_config: JAR='$JAR', MIN_MEM='$MIN_MEM', MAX_MEM='$MAX_MEM', SESSION='$SESSION', LOG_FILE='$LOG_FILE'"
}

# 加载 RCON 配置（从 [rcon] 段）
load_rcon_config() {
    RCON_PORT=$(get_config_value "rcon" "port" "25575")
    RCON_PASS=$(get_config_value "rcon" "password" "password")
    debug_log "load_rcon_config: RCON_PORT='$RCON_PORT', RCON_PASS='$RCON_PASS'"
}

# 加载 JVM 配置（从 [jvm] 段）
load_jvm_config() {
    JDK_PATH=$(get_config_value "jvm" "jdk_path" "")
    debug_log "load_jvm_config: JDK_PATH='$JDK_PATH'"
}

# 解析 Java 命令路径
resolve_java_command() {
    load_jvm_config
    if [ -n "$JDK_PATH" ]; then
        if [ -x "$JDK_PATH" ]; then
            JAVA_CMD="$JDK_PATH"
            debug_log "resolve_java_command: using configured JDK_PATH=$JDK_PATH"
        else
            debug_log "resolve_java_command: JDK_PATH='$JDK_PATH' is not executable, falling back to java"
            log_warn "配置的 JDK 路径不可用: $JDK_PATH，使用系统默认 java"
            JAVA_CMD="java"
        fi
    else
        JAVA_CMD="java"
        debug_log "resolve_java_command: no jdk_path configured, using system java"
    fi
}

# 加载 AI 配置（从 [ai] 段）
load_ai_config() {
    AI_PROVIDER="openai"
    API_KEY=$(get_config_value "ai" "api_key" "")
    MODEL=$(get_config_value "ai" "model" "gpt-4o-mini")
    debug_log "load_ai_config: API_KEY='${API_KEY:0:10}...', MODEL='$MODEL'"
}

HTTP_PORT="8080"

# ==================== 依赖检查函数 ====================
check_dependencies() {
    local missing=()
    resolve_java_command
    if [ "$JAVA_CMD" = "java" ]; then
        command -v java >/dev/null 2>&1 || missing+=("java")
    fi
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
    load_server_config
    debug_log "check_server_jar: JAR='$JAR'"
    if [ ! -f "$JAR" ]; then
        dialog --title "服务器核心缺失" --msgbox "\n服务器核心不存在: $JAR\n\n请检查 config.toml 中的 jar 设置是否正确。\n当前目录下存在的文件:\n$(ls -1 | head -20)" 15 60
        return 1
    fi
    return 0
}

check_rust_binary() {
    if find_rust_binary; then
        return 0
    fi
    dialog --title "二进制文件缺失" --msgbox "\nMC-Minder 二进制文件不存在: $RUST_BIN\n\n请从 GitHub Releases 下载或从源码编译:\nhttps://github.com/SharkMI-0x7E/mc-minder/releases\n\n或运行一键安装脚本:\ncurl -fsSL https://raw.githubusercontent.com/SharkMI-0x7E/mc-minder/main/install.sh | bash" 11 60
    return 1
}

# ==================== 核心操作函数 ====================
# 后台启动
start_background() {
    load_server_config

    if tmux has-session -t "$SESSION" 2>/dev/null; then
        dialog --title "会话已存在" --yesno "\n会话 '$SESSION' 已存在。\n\n是否终止现有会话并重启?" 9 50
        case $? in
            0) tmux kill-session -t "$SESSION" ;;
            *) return 1 ;;
        esac
    fi

    dialog --infobox "\n正在启动 Minecraft 服务器（后台模式）...\n\n  核心文件: $JAR\n  内存范围: $MIN_MEM - $MAX_MEM\n  会话名称: $SESSION" 10 60
    sleep 1

    mkdir -p logs

    tmux new-session -d -s "$SESSION" -x 120 -y 30
    tmux send-keys -t "$SESSION" "cd '$(pwd)'" Enter
    resolve_java_command
    tmux send-keys -t "$SESSION" "$JAVA_CMD -Xms$MIN_MEM -Xmx$MAX_MEM -jar $JAR nogui" Enter

    sleep 5

    if check_rust_binary; then
        dialog --infobox "\n正在启动 MC-Minder（后台）..." 5 40

        nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
        RUST_PID=$!
        echo $RUST_PID > "$RUST_PID_FILE"

        (
            while true; do
                if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                    kill -TERM $(cat "$RUST_PID_FILE" 2>/dev/null) 2>/dev/null
                    rm -f "$RUST_PID_FILE"
                    exit 0
                fi

                if [ -f "$RUST_PID_FILE" ]; then
                    RUST_PID=$(cat "$RUST_PID_FILE")
                    if ! kill -0 "$RUST_PID" 2>/dev/null; then
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
        echo $WATCHDOG_PID > "$WATCHDOG_PID_FILE"
    fi

    dialog --msgbox "\n启动完成!\n\n服务器已在 tmux 会话 '$SESSION' 中启动\n附加命令: tmux attach -t $SESSION" 10 60
}

# 前台启动
start_foreground() {
    load_server_config

    if tmux has-session -t "$SESSION" 2>/dev/null; then
        dialog --title "会话已存在" --yesno "\n检测到后台会话 '$SESSION' 正在运行。\n\n前台启动前是否先停止后台会话？\n（选择“是”将停止后台服务，选择“否”将退出前台模式）" 10 60
        case $? in
            0)
                tmux send-keys -t "$SESSION" "stop" Enter
                sleep 5
                tmux kill-session -t "$SESSION" 2>/dev/null
                if [ -f "$WATCHDOG_PID_FILE" ]; then
                    kill -TERM $(cat "$WATCHDOG_PID_FILE") 2>/dev/null
                    rm -f "$WATCHDOG_PID_FILE"
                fi
                if [ -f "$RUST_PID_FILE" ]; then
                    kill -TERM $(cat "$RUST_PID_FILE") 2>/dev/null
                    rm -f "$RUST_PID_FILE"
                fi
                ;;
            *)
                dialog --msgbox "前台启动已取消。" 6 30
                return 1
                ;;
        esac
    fi

    clear
    echo -e "${GREEN}[信息] 正在以【前台模式】启动 Minecraft 服务器...${NC}"
    echo -e "${GREEN}[信息] 核心文件: $JAR${NC}"
    echo -e "${GREEN}[信息] 内存范围: $MIN_MEM - $MAX_MEM${NC}"
    echo -e "${YELLOW}[提示] 按 Ctrl+C 可停止服务器。${NC}"
    echo ""

    resolve_java_command
    $JAVA_CMD -Xms"$MIN_MEM" -Xmx"$MAX_MEM" -jar "$JAR" nogui

    echo ""
    log_info "Minecraft 服务器已停止。"
    read -p "按回车键返回主菜单..." dummy
}

stop_server() {
    dialog --infobox "\n正在停止服务器..." 5 30
    sleep 1

    if [ -f "$WATCHDOG_PID_FILE" ]; then
        WATCHDOG_PID=$(cat "$WATCHDOG_PID_FILE")
        kill "$WATCHDOG_PID" 2>/dev/null
        rm -f "$WATCHDOG_PID_FILE"
    fi

    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        kill -TERM "$RUST_PID" 2>/dev/null
        rm -f "$RUST_PID_FILE"
    fi

    load_server_config

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
    load_server_config
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

    if [ -f "$WATCHDOG_PID_FILE" ]; then
        WATCHDOG_PID=$(cat "$WATCHDOG_PID_FILE")
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
    load_server_config
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

view_server_log() {
    local log_file="$LOG_FILE"
    if [ ! -f "$log_file" ]; then
        dialog --title "错误" --msgbox "\n服务器日志文件不存在: $log_file\n\n请先启动服务器（后台模式）以生成日志。" 8 50
        return 1
    fi

    # 使用用户临时目录，避免 /tmp 权限问题
    local temp_log="$PID_DIR/server_log.tmp"
    local line_count=$(wc -l < "$log_file")
    local display_lines=200

    if [ "$line_count" -gt "$display_lines" ]; then
        tail -n "$display_lines" "$log_file" > "$temp_log"
        dialog --title "服务器日志（最后 $display_lines 行）" --textbox "$temp_log" 25 90
        rm -f "$temp_log"
    else
        dialog --title "服务器日志（共 $line_count 行）" --textbox "$log_file" 25 90
    fi
}

show_minder_logs() {
    local minder_log="logs/mc-minder.log"
    if [ -f "$minder_log" ]; then
        local line_count=$(wc -l < "$minder_log")
        local display_lines=200
        local temp_log="/tmp/mc-minder_minder_log.tmp"
        if [ "$line_count" -gt "$display_lines" ]; then
            tail -n "$display_lines" "$minder_log" > "$temp_log"
            dialog --title "MC-Minder 日志（最后 $display_lines 行）" --textbox "$temp_log" 25 90
            rm -f "$temp_log"
        else
            dialog --title "MC-Minder 日志（共 $line_count 行）" --textbox "$minder_log" 25 90
        fi
    else
        dialog --title "错误" --msgbox "\nMC-Minder 日志文件不存在!" 6 40
    fi
}

init_config() {
    if ! check_rust_binary; then
        return 1
    fi

    load_server_config
    load_rcon_config
    load_ai_config

    exec 3>&1
    VALUES=$(dialog --ok-label "$(T "保存配置" "Save Config")" \
        --cancel-label "$(T "取消" "Cancel")" \
        --form "\n$(T "MC-Minder 初始化配置" "MC-Minder Initial Configuration")\n\n$(T "请填写以下信息:" "Please fill in the following information:")" \
        18 70 0 \
        "$(T "服务器 JAR 文件:" "Server JAR file:")"     1 1 "${JAR}"       1 25 45 0 \
        "$(T "最小内存:" "Min memory:")"                 2 1 "${MIN_MEM}"   2 25 10 0 \
        "$(T "最大内存:" "Max memory:")"                 3 1 "${MAX_MEM}"   3 25 10 0 \
        "$(T "Tmux 会话名称:" "Tmux session name:")"    4 1 "${SESSION}"   4 25 20 0 \
        "$(T "RCON 端口:" "RCON port:")"                5 1 "${RCON_PORT}" 5 25 10 0 \
        "$(T "RCON 密码:" "RCON password:")"            6 1 "${RCON_PASS}" 6 25 30 0 \
        "$(T "AI 提供商:" "AI provider:")"              7 1 "openai"       7 25 15 0 \
        "$(T "API 密钥:" "API key:")"                   8 1 "${API_KEY}"   8 25 40 0 \
        "$(T "API 模型:" "API model:")"                 9 1 "${MODEL}"     9 25 25 0 \
        "$(T "HTTP API 端口:" "HTTP API port:")"       10 1 "8080"        10 25 10 0 \
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
log_file = "logs/latest.log"

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
    if ! find_rust_binary; then
        dialog --title "更新失败" --msgbox "\nMC-Minder 二进制文件不存在，无法执行更新。\n\n请从 GitHub Releases 手动下载最新版本:\nhttps://github.com/SharkMI-0x7E/mc-minder/releases\n\n或运行一键安装脚本:\ncurl -fsSL https://raw.githubusercontent.com/SharkMI-0x7E/mc-minder/main/install.sh | bash" 13 60
        return 1
    fi

    dialog --infobox "\n正在更新 MC-Minder...\n这可能需要一些时间，请耐心等待。" 8 50

    local update_output
    if update_output=$("$RUST_BIN" self-update 2>&1); then
        dialog --msgbox "\nMC-Minder 更新成功!\n\n建议重启服务以应用更新。" 8 50
    else
        dialog --title "更新失败" --msgbox "\nMC-Minder 更新失败!\n\n输出信息:\n$update_output\n\n可能的原因:\n1. 网络连接问题\n2. GitHub API 访问受限\n3. 权限不足\n\n请尝试手动下载:\nhttps://github.com/SharkMI-0x7E/mc-minder/releases" 16 60
        return 1
    fi
}

# ==================== 主菜单函数 ====================
show_main_menu() {
    # 加载语言设置
    load_lang
    
    while true; do
        exec 3>&1
        selection=$(dialog --clear \
            --backtitle "MC-Minder - $(T "Minecraft 服务器管理套件" "Minecraft Server Management Suite")" \
            --title "$(T "主菜单" "Main Menu")" \
            --menu "\n$(T "选择要执行的操作:" "Select an operation:")" \
            22 60 14 \
            1 "$(T "启动服务器（后台模式）" "Start Server (Background)")" \
            2 "$(T "启动服务器（前台模式）" "Start Server (Foreground)")" \
            3 "$(T "停止服务器" "Stop Server")" \
            4 "$(T "重启服务器" "Restart Server")" \
            5 "$(T "查看服务器状态" "View Server Status")" \
            6 "$(T "附加到服务器控制台" "Attach to Server Console")" \
            7 "$(T "查看服务器日志" "View Server Log")" \
            8 "$(T "查看 MC-Minder 日志" "View MC-Minder Log")" \
            9 "$(T "初始化配置" "Initialize Config")" \
            10 "$(T "更新 MC-Minder" "Update MC-Minder")" \
            11 "$(T "Java 版本管理" "Java Version Management")" \
            12 "$(T "语言设置" "Language Settings")" \
            13 "$(T "退出" "Exit")" \
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
                    2)
                        if check_dependencies && check_config && check_server_jar; then
                            start_foreground
                        fi
                        ;;
                    3) stop_server ;;
                    4) restart_server ;;
                    5) status_server ;;
                    6) attach_server ;;
                    7) view_server_log ;;
                    8) show_minder_logs ;;
                    9) init_config ;;
                    10) update_mcminder ;;
                    11) show_java_menu ;;
                    12) switch_language ;;
                    13) clear
                        log_info "$(T "感谢使用 MC-Minder!" "Thank you for using MC-Minder!")"
                        exit 0
                        ;;
                esac
                ;;
            1|255)
                clear
                log_info "$(T "感谢使用 MC-Minder!" "Thank you for using MC-Minder!")"
                exit 0
                ;;
        esac
    done
}

# ==================== Java 管理子菜单 ====================
show_java_menu() {
    while true; do
        local current_java_ver=""
        if command -v java >/dev/null 2>&1; then
            current_java_ver=$(java -version 2>&1 | head -1 | awk -F '"' '{print $2}')
        fi
        
        exec 3>&1
        selection=$(dialog --clear \
            --backtitle "MC-Minder - $(T "Java 版本管理" "Java Version Management")" \
            --title "$(T "Java 管理" "Java Management")" \
            --menu "\n$(T "当前 Java 版本:" "Current Java version:") ${current_java_ver:-$(T "未检测到" "Not detected")}\n\n$(T "选择操作:" "Select operation:")" \
            14 60 5 \
            1 "$(T "切换 Java 版本" "Switch Java Version")" \
            2 "$(T "安装新 Java 版本" "Install New Java Version")" \
            3 "$(T "查看所有已安装版本" "View All Installed Versions")" \
            4 "$(T "返回主菜单" "Back to Main Menu")" \
            2>&1 1>&3)
        exit_code=$?
        exec 3>&-

        case $exit_code in
            0)
                case $selection in
                    1) switch_java_version ;;
                    2) install_java_version ;;
                    3) 
                        detect_java_versions
                        local ver_list=""
                        for idx in "${!JAVA_VERSIONS[@]}"; do
                            ver_list+="Java ${JAVA_VERSIONS[$idx]} - ${JAVA_PATHS[$idx]}\n"
                        done
                        dialog --title "$(T "已安装的 Java 版本" "Installed Java Versions")" \
                            --msgbox "\n${ver_list:-$(T "未检测到 Java 版本" "No Java versions detected")}" 15 60
                        ;;
                    4) return ;;
                esac
                ;;
            1|255) return ;;
        esac
    done
}

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

    dialog --backtitle "MC-Minder - $(T "Minecraft 服务器管理套件" "Minecraft Server Management Suite")" \
        --title "$(T "欢迎使用" "Welcome")" \
        --msgbox "\n$(T "欢迎使用 MC-Minder TUI 管理界面!" "Welcome to MC-Minder TUI Management Interface!")\n\n$(T "这是一个基于 dialog 的图形化管理工具," "This is a dialog-based graphical management tool,")\n$(T "可以方便地管理你的 Minecraft Fabric 服务器。" "making it easy to manage your Minecraft Fabric server.")\n\n$(T "按 Enter 进入主菜单..." "Press Enter to enter the main menu...")" 13 55

    show_main_menu
}

main "$@"