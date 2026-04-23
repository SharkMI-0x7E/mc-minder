# MC-Minder - Common utilities library
# Sourced by all other scripts
# Contains: colors, logging, language, environment detection, dialog installation

# ==================== Debug logging ====================
DEBUG_DIR="./mc-minder_log"
DEBUG_FILE="$DEBUG_DIR/start-tui-debug.log"
mkdir -p "$DEBUG_DIR"

debug_log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$DEBUG_FILE"
}

# ==================== Global constants ====================
CONFIG_FILE="config.toml"
LOG_FILE="logs/latest.log"
RUST_BIN="./mc-minder"

# PID file paths (use user home to avoid /tmp permission issues)
PID_DIR="$HOME/.mc-minder/tmp"
mkdir -p "$PID_DIR"
RUST_PID_FILE="$PID_DIR/mc-minder.pid"
WATCHDOG_PID_FILE="$PID_DIR/mc-minder-watchdog.pid"

SESSION_NAME="mc_server"

debug_log "CONFIG_FILE=$CONFIG_FILE"
debug_log "PID_DIR=$PID_DIR"
debug_log "RUST_PID_FILE=$RUST_PID_FILE"

# ==================== Color definitions (for non-dialog output) ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ==================== Language system ====================
# Language config file
LANG_FILE="$HOME/.mc-minder/lang.conf"

# Default language
CURRENT_LANG="zh"

# Load language settings
load_lang() {
    if [ -f "$LANG_FILE" ]; then
        CURRENT_LANG=$(cat "$LANG_FILE" | tr -d '[:space:]')
    fi
}

# Save language settings
save_lang() {
    mkdir -p "$(dirname "$LANG_FILE")"
    echo "$CURRENT_LANG" > "$LANG_FILE"
}

# Language translation function
# Usage: T "Chinese" "English"
T() {
    if [ "$CURRENT_LANG" = "en" ]; then
        echo "$2"
    else
        echo "$1"
    fi
}

# Switch language
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

# ==================== Log functions ====================
log_info() {
    echo -e "${GREEN}[信息]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[警告]${NC} $1"
}

log_error() {
    echo -e "${RED}[错误]${NC} $1"
}

# ==================== Environment detection ====================
is_termux() {
    [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]
}

# ==================== Dialog installation ====================
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