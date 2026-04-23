# MC-Minder - Server operations library
# Sourced by start-tui.sh and other scripts
# Contains: server start/stop/restart/status/attach, checks, watchdog

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/java.sh"

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
        dialog --title "会话已存在" --yesno "\n检测到后台会话 '$SESSION' 正在运行。\n\n前台启动前是否先停止后台会话？\n（选择"是"将停止后台服务，选择"否"将退出前台模式）" 10 60
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