#!/bin/bash

CONFIG_FILE="config.toml"
LOG_FILE="logs/latest.log"
RUST_BIN="./mc-minder"
RUST_PID_FILE="/tmp/mc-minder.pid"
SERVER_PID_FILE="/tmp/mc-server.pid"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
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
    JAR=$(get_config_value "jar" "fabric-server.jar")
    MIN_MEM=$(get_config_value "min_mem" "512M")
    MAX_MEM=$(get_config_value "max_mem" "1G")
    SESSION=$(get_config_value "session_name" "mc_server")
}

is_termux() {
    [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]
}

check_dependencies() {
    local missing=()
    
    command -v java >/dev/null 2>&1 || missing+=("java")
    command -v tmux >/dev/null 2>&1 || missing+=("tmux")
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        
        if is_termux; then
            log_info "Detected Termux environment"
            read -p "Install missing dependencies? (Y/n): " choice
            case "$choice" in
                n|N )
                    log_error "Cannot continue without dependencies"
                    exit 1
                    ;;
                * )
                    log_info "Installing dependencies..."
                    pkg install -y openjdk-17 tmux
                    ;;
            esac
        else
            log_info "Please install missing dependencies:"
            log_info "  Ubuntu/Debian: sudo apt install openjdk-17-jre tmux"
            log_info "  CentOS/RHEL:   sudo yum install java-17-openjdk tmux"
            log_info "  macOS:         brew install openjdk tmux"
            exit 1
        fi
    fi
}

check_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        log_error "Config file not found: $CONFIG_FILE"
        log_info "Run './mc-minder init' to create one"
        exit 1
    fi
}

check_server_jar() {
    if [ ! -f "$JAR" ]; then
        log_error "Server jar not found: $JAR"
        log_info "Please download fabric-server.jar and place it in the current directory"
        log_info "Visit: https://fabricmc.net/use/installer/"
        exit 1
    fi
}

check_rust_binary() {
    if [ ! -f "$RUST_BIN" ]; then
        log_warn "MC-Minder binary not found at $RUST_BIN"
        log_info "Please download it from GitHub Releases or compile from source:"
        log_info "  https://github.com/SharkMI-0x7E/mc-minder/releases"
        return 1
    fi
    return 0
}

start_background() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_warn "Session '$SESSION' already exists"
        read -p "Kill existing session and restart? (y/N): " choice
        case "$choice" in
            y|Y )
                tmux kill-session -t "$SESSION"
                ;;
            * )
                log_info "Aborted"
                return 1
                ;;
        esac
    fi
    
    log_info "Starting Minecraft server..."
    log_info "  JAR: $JAR"
    log_info "  Memory: $MIN_MEM - $MAX_MEM"
    log_info "  Session: $SESSION"
    
    mkdir -p logs
    
    tmux new-session -d -s "$SESSION" -x 120 -y 30
    
    tmux send-keys -t "$SESSION" "cd '$(pwd)'" Enter
    tmux send-keys -t "$SESSION" "java -Xms$MIN_MEM -Xmx$MAX_MEM -jar $JAR nogui" Enter
    
    echo $! > "$SERVER_PID_FILE"
    
    log_info "Server started in tmux session '$SESSION'"
    log_info "Attach with: tmux attach -t $SESSION"
    
    sleep 5
    
    if check_rust_binary; then
        log_info "Starting MC-Minder..."
        
        nohup "$RUST_BIN" --config "$CONFIG_FILE" >> logs/mc-minder.log 2>&1 &
        RUST_PID=$!
        echo $RUST_PID > "$RUST_PID_FILE"
        
        log_info "MC-Minder started (PID: $RUST_PID)"
        
        (
            while true; do
                if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                    log_warn "Server session died, stopping MC-Minder..."
                    kill -TERM $(cat "$RUST_PID_FILE" 2>/dev/null) 2>/dev/null
                    rm -f "$RUST_PID_FILE" "$SERVER_PID_FILE"
                    exit 0
                fi
                
                if [ -f "$RUST_PID_FILE" ]; then
                    RUST_PID=$(cat "$RUST_PID_FILE")
                    if ! kill -0 "$RUST_PID" 2>/dev/null; then
                        log_warn "MC-Minder died, restarting..."
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
        log_info "Watchdog started (PID: $WATCHDOG_PID)"
    fi
    
    log_info "Startup complete!"
}

stop_server() {
    log_info "Stopping server..."
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        kill "$WATCHDOG_PID" 2>/dev/null
        rm -f /tmp/mc-minder-watchdog.pid
        log_info "Watchdog stopped"
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        kill -TERM "$RUST_PID" 2>/dev/null
        rm -f "$RUST_PID_FILE"
        log_info "MC-Minder stopped"
    fi
    
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_info "Sending 'stop' command to server..."
        tmux send-keys -t "$SESSION" "stop" Enter
        
        for i in {1..30}; do
            if ! tmux has-session -t "$SESSION" 2>/dev/null; then
                break
            fi
            sleep 1
        done
        
        if tmux has-session -t "$SESSION" 2>/dev/null; then
            log_warn "Server didn't stop gracefully, killing..."
            tmux kill-session -t "$SESSION"
        fi
        
        log_info "Server stopped"
    else
        log_warn "No running session found"
    fi
    
    rm -f "$SERVER_PID_FILE"
}

status_server() {
    load_config
    
    echo ""
    echo "MC-Minder Status"
    echo "================"
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        echo -e "Server:    ${GREEN}Running${NC} (tmux session: $SESSION)"
    else
        echo -e "Server:    ${RED}Stopped${NC}"
    fi
    
    if [ -f "$RUST_PID_FILE" ]; then
        RUST_PID=$(cat "$RUST_PID_FILE")
        if kill -0 "$RUST_PID" 2>/dev/null; then
            echo -e "MC-Minder: ${GREEN}Running${NC} (PID: $RUST_PID)"
        else
            echo -e "MC-Minder: ${RED}Dead${NC} (stale PID file)"
        fi
    else
        echo -e "MC-Minder: ${RED}Not running${NC}"
    fi
    
    if [ -f /tmp/mc-minder-watchdog.pid ]; then
        WATCHDOG_PID=$(cat /tmp/mc-minder-watchdog.pid)
        if kill -0 "$WATCHDOG_PID" 2>/dev/null; then
            echo -e "Watchdog:  ${GREEN}Running${NC} (PID: $WATCHDOG_PID)"
        else
            echo -e "Watchdog:  ${RED}Dead${NC}"
        fi
    else
        echo -e "Watchdog:  ${RED}Not running${NC}"
    fi
    
    echo ""
}

attach_server() {
    load_config
    
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        log_info "Attaching to server session..."
        log_info "Press Ctrl+B then D to detach"
        tmux attach -t "$SESSION"
    else
        log_error "No running session found"
    fi
}

show_logs() {
    if [ -f "$LOG_FILE" ]; then
        log_info "Showing last 50 lines of server log..."
        tail -n 50 "$LOG_FILE"
    else
        log_error "Log file not found: $LOG_FILE"
        log_info "Start the server first to generate logs"
    fi
}

show_minder_logs() {
    local minder_log="logs/mc-minder.log"
    if [ -f "$minder_log" ]; then
        log_info "Showing last 50 lines of MC-Minder log..."
        tail -n 50 "$minder_log"
    else
        log_error "MC-Minder log file not found"
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
        if [ -f "$RUST_BIN" ]; then
            "$RUST_BIN" init
        else
            log_error "MC-Minder binary not found"
            exit 1
        fi
        ;;
    update)
        if [ -f "$RUST_BIN" ]; then
            "$RUST_BIN" self-update
        else
            log_error "MC-Minder binary not found"
            exit 1
        fi
        ;;
    *)
        echo "MC-Minder - Minecraft Server Manager v0.3.0"
        echo ""
        echo "Usage: $0 {start|stop|restart|status|attach|logs|init|update}"
        echo ""
        echo "Commands:"
        echo "  start        Start the server and MC-Minder"
        echo "  stop         Stop everything"
        echo "  restart      Restart everything"
        echo "  status       Show status"
        echo "  attach (a)   Attach to server console"
        echo "  logs (l)     Show recent server logs"
        echo "  minder-logs  Show MC-Minder logs"
        echo "  init         Initialize configuration"
        echo "  update       Update MC-Minder to latest version"
        ;;
esac
