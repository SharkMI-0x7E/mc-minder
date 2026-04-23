# MC-Minder - Configuration library
# Sourced by start-tui.sh and other scripts
# Contains: TOML reading/writing, config loading, init wizard

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

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

# ==================== 初始化配置向导 ====================
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
            dialog --msgbox "\n$(T "配置已保存到 $CONFIG_FILE" "Config saved to $CONFIG_FILE")" 7 50
            ;;
        *)
            dialog --msgbox "\n$(T "配置已取消!" "Config cancelled!")" 6 30
            ;;
    esac
}