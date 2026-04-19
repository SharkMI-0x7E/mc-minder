#!/bin/bash

REPO="SharkMI-0x7E/mc-minder"
BINARY_NAME="mc-minder"

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

detect_arch() {
    local arch=$(uname -m)
    local os=$(uname -s)
    
    case "$os" in
        Linux)
            if [ -f "/system/bin/app_process" ] || [ -n "$TERMUX_VERSION" ]; then
                echo "aarch64-linux-android"
            elif [ "$arch" = "aarch64" ] || [ "$arch" = "arm64" ]; then
                echo "aarch64-unknown-linux-musl"
            elif [ "$arch" = "x86_64" ]; then
                echo "x86_64-unknown-linux-musl"
            else
                echo "unknown"
            fi
            ;;
        Darwin)
            if [ "$arch" = "arm64" ]; then
                echo "aarch64-apple-darwin"
            else
                echo "x86_64-apple-darwin"
            fi
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

check_dependencies() {
    local missing=()
    
    command -v curl >/dev/null 2>&1 || missing+=("curl")
    command -v tar >/dev/null 2>&1 || missing+=("tar")
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        
        if [ -n "$TERMUX_VERSION" ]; then
            log_info "Installing dependencies..."
            pkg install -y ${missing[*]}
        else
            log_info "Please install: ${missing[*]}"
            exit 1
        fi
    fi
}

get_latest_version() {
    local version=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    echo "$version"
}

download_binary() {
    local target="$1"
    local version="$2"
    local url="https://github.com/$REPO/releases/download/v$version/$BINARY_NAME-$target"
    
    log_info "Downloading MC-Minder v$version for $target..."
    log_info "URL: $url"
    
    if curl -L -o "$BINARY_NAME" "$url"; then
        chmod +x "$BINARY_NAME"
        return 0
    else
        return 1
    fi
}

download_scripts() {
    local version="$1"
    local base_url="https://raw.githubusercontent.com/$REPO/v$version/scripts"
    
    log_info "Downloading start.sh..."
    curl -L -o "start.sh" "$base_url/start.sh"
    chmod +x "start.sh"
    
    log_info "Downloading backup.sh..."
    curl -L -o "backup.sh" "$base_url/backup.sh"
    chmod +x "backup.sh"
}

main() {
    echo ""
    echo -e "${BLUE}MC-Minder Installer${NC}"
    echo "==================="
    echo ""
    
    check_dependencies
    
    local target=$(detect_arch)
    
    if [ "$target" = "unknown" ]; then
        log_error "Unsupported platform: $(uname -s) $(uname -m)"
        log_info "Please compile from source: https://github.com/$REPO"
        exit 1
    fi
    
    log_info "Detected platform: $target"
    
    local version=$(get_latest_version)
    
    if [ -z "$version" ]; then
        log_error "Failed to get latest version"
        log_info "Please check your network connection"
        exit 1
    fi
    
    log_info "Latest version: $version"
    
    if [ -f "$BINARY_NAME" ]; then
        log_warn "MC-Minder already exists"
        read -p "Reinstall? (y/N): " choice
        case "$choice" in
            y|Y )
                rm -f "$BINARY_NAME"
                ;;
            * )
                log_info "Skipping binary download"
                ;;
        esac
    fi
    
    if [ ! -f "$BINARY_NAME" ]; then
        if ! download_binary "$target" "$version"; then
            log_error "Failed to download binary"
            log_info "Try compiling from source: https://github.com/$REPO"
            exit 1
        fi
    fi
    
    if [ ! -f "start.sh" ]; then
        download_scripts "$version"
    else
        log_info "start.sh already exists, skipping"
    fi
    
    if [ ! -f "backup.sh" ]; then
        download_scripts "$version"
    else
        log_info "backup.sh already exists, skipping"
    fi
    
    echo ""
    log_info "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Run: ./$BINARY_NAME init"
    echo "  2. Place fabric-server.jar in this directory"
    echo "  3. Run: ./start.sh start"
    echo ""
    echo "For more information, visit: https://github.com/$REPO"
}

main "$@"
