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
            if [ -n "$TERMUX_VERSION" ] || [ -f "/system/bin/app_process" ]; then
                echo "termux-aarch64"
            elif [ "$arch" = "aarch64" ] || [ "$arch" = "arm64" ]; then
                echo "aarch64-linux"
            elif [ "$arch" = "x86_64" ]; then
                echo "x86_64-linux"
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
    command -v sha256sum >/dev/null 2>&1 || missing+=("sha256sum")
    
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
    local version=""
    local retry_count=0
    local max_retries=3
    
    while [ $retry_count -lt $max_retries ]; do
        version=$(curl -s \
            --connect-timeout 10 \
            --max-time 30 \
            --retry 2 \
            --retry-delay 3 \
            "https://api.github.com/repos/$REPO/releases/latest" | \
            grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
        
        if [ -n "$version" ] && [ "$version" != "" ] && [ "$version" != "null" ]; then
            echo "$version"
            return 0
        fi
        
        retry_count=$((retry_count + 1))
        
        if [ $retry_count -lt $max_retries ]; then
            log_warn "Failed to get version (attempt $retry_count/$max_retries), retrying..."
            sleep 2
        fi
    done
    
    echo ""
    return 1
}

download_with_retry() {
    local url="$1"
    local output="$2"
    local description="$3"
    local retry_count=0
    local max_retries=3
    
    while [ $retry_count -lt $max_retries ]; do
        log_info "Downloading $description (attempt $((retry_count + 1))/$max_retries)..."
        
        if curl -L \
            --connect-timeout 15 \
            --max-time 120 \
            --retry 2 \
            --retry-delay 3 \
            --progress-bar \
            -o "$output" \
            "$url"; then
            
            if [ -f "$output" ] && [ -s "$output" ]; then
                return 0
            else
                log_warn "Downloaded file is empty or missing"
                rm -f "$output"
            fi
        else
            log_warn "Download failed"
            rm -f "$output"
        fi
        
        retry_count=$((retry_count + 1))
        
        if [ $retry_count -lt $max_retries ]; then
            sleep 3
        fi
    done
    
    return 1
}

verify_checksum() {
    local file="$1"
    local expected_sha256="$2"
    
    if ! command -v sha256sum >/dev/null 2>&1; then
        log_warn "sha256sum not found, skipping verification"
        return 0
    fi
    
    if [ -z "$expected_sha256" ]; then
        log_warn "No checksum provided, skipping verification"
        return 0
    fi
    
    log_info "Verifying integrity of $(basename $file)..."
    
    local actual_sha256=$(sha256sum "$file" | awk '{print $1}')
    
    if [ "$actual_sha256" = "$expected_sha256" ]; then
        log_info "Checksum verified: ${actual_sha256:0:16}...✓"
        return 0
    else
        log_error "Checksum mismatch!"
        log_error "Expected: ${expected_sha256:0:16}..."
        log_error "Actual:   ${actual_sha256:0:16}..."
        return 1
    fi
}

download_binary() {
    local target="$1"
    local version="$2"
    
    local binary_name="$BINARY_NAME-$target"
    local url="https://github.com/$REPO/releases/download/v$version/$binary_name"
    local sha256_url="https://github.com/$REPO/releases/download/v$version/$binary_name.sha256"
    
    log_info "Downloading MC-Minder v$version for $target..."
    
    if ! download_with_retry "$url" "$BINARY_NAME" "binary ($target)"; then
        return 1
    fi
    
    chmod +x "$BINARY_NAME"
    
    local sha256_file="$BINARY_NAME.sha256"
    if download_with_retry "$sha256_url" "$sha256_file" "checksum file"; then
        local expected_sha256=$(cat "$sha256_file" | awk '{print $1}')
        rm -f "$sha256_file"
        
        if ! verify_checksum "$BINARY_NAME" "$expected_sha256"; then
            log_error "Binary verification failed! The file may be corrupted."
            rm -f "$BINARY_NAME"
            return 1
        fi
    else
        log_warn "Could not download checksum file, skipping verification"
        rm -f "$sha256_file"
    fi
    
    return 0
}

download_scripts() {
    local version="$1"
    local base_url="https://raw.githubusercontent.com/$REPO/v$version/scripts"
    
    if [ ! -f "start.sh" ]; then
        log_info "Downloading start.sh..."
        if download_with_retry "$base_url/start.sh" "start.sh" "startup script"; then
            chmod +x "start.sh"
        else
            log_warn "Failed to download start.sh"
        fi
    else
        log_info "start.sh already exists, skipping"
    fi
    
    if [ ! -f "backup.sh" ]; then
        log_info "Downloading backup.sh..."
        if download_with_retry "$base_url/backup.sh" "backup.sh" "backup script"; then
            chmod +x "backup.sh"
        else
            log_warn "Failed to download backup.sh"
        fi
    else
        log_info "backup.sh already exists, skipping"
    fi
    
    if [ ! -f "start-tui.sh" ]; then
        log_info "Downloading start-tui.sh (TUI interface)..."
        if download_with_retry "$base_url/start-tui.sh" "start-tui.sh" "TUI startup script"; then
            chmod +x "start-tui.sh"
            log_info ""
            log_info "TUI script downloaded! You can use it with:"
            log_info "  ./start-tui.sh"
            log_info ""
            
            if ! command -v dialog >/dev/null 2>&1; then
                log_warn "Note: 'dialog' is required for TUI mode"
                log_warn "Install it with: pkg install dialog (Termux) or apt install dialog (Linux)"
            fi
        else
            log_warn "Failed to download start-tui.sh"
        fi
    else
        log_info "start-tui.sh already exists, skipping"
    fi
}

show_post_install_instructions() {
    echo ""
    log_info "Installation complete!"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "  1. Run: ./$BINARY_NAME init          # Initialize configuration"
    echo "  2. Place fabric-server.jar here         # Minecraft server jar"
    echo "  3. Run: ./start.sh start              # Start server (CLI mode)"
    echo ""
    echo -e "${BLUE}Alternative:${NC}"
    echo "  Run: ./start-tui.sh                   # Start with TUI menu (requires dialog)"
    echo ""
    echo -e "${BLUE}Useful commands:${NC}"
    echo "  ./$BINARY_NAME --version               # Show version"
    echo "  ./$BINARY_NAME self-update             # Update to latest version"
    echo "  ./start.sh status                     # Check server status"
    echo ""
    echo -e "${BLUE}For more information:${NC} https://github.com/$REPO"
}

main() {
    echo ""
    echo -e "${BLUE}MC-Minder Installer v0.3.7${NC}"
    echo "========================="
    echo ""
    
    check_dependencies
    
    local target=$(detect_arch)
    
    if [ "$target" = "unknown" ]; then
        log_error "Unsupported platform: $(uname -s) $(uname -m)"
        log_info "Supported platforms:"
        log_info "  - Linux x86_64"
        log_info "  - Linux aarch64"
        log_info "  - Termux/Android ARM64"
        log_info "  - macOS (experimental)"
        echo ""
        log_info "Please compile from source: https://github.com/$REPO"
        exit 1
    fi
    
    log_info "Detected platform: $target"
    
    local version=""
    
    if [ $# -gt 0 ] && [ -n "$1" ]; then
        version="$1"
        log_info "Using specified version: $version"
    else
        log_info "Checking latest version from GitHub..."
        version=$(get_latest_version)
        
        if [ -z "$version" ]; then
            log_error "Failed to get latest version from GitHub API"
            echo ""
            log_info "Possible reasons:"
            log_info "  1. Network connection issue"
            log_info "  2. GitHub API rate limited"
            log_info "  3. No releases available yet"
            echo ""
            log_info "You can specify version manually:"
            log_info "  $0 <version>"
            log_info "Example: $0 0.3.6"
            exit 1
        fi
        
        log_info "Latest version: $version"
    fi
    
    if [ -f "$BINARY_NAME" ]; then
        log_warn "MC-Minder already exists (size: $(du -h $BINARY_NAME | cut -f1))"
        echo ""
        echo "Options:"
        echo "  y) Reinstall (overwrite existing binary)"
        echo "  N) Keep existing binary and continue"
        echo "  C) Cancel installation"
        echo ""
        read -p "Your choice [y/N/C]: " choice
        case "$choice" in
            y|Y )
                log_info "Removing old binary..."
                rm -f "$BINARY_NAME"
                ;;
            c|C )
                log_info "Installation cancelled"
                exit 0
                ;;
            * )
                log_info "Keeping existing binary"
                ;;
        esac
    fi
    
    if [ ! -f "$BINARY_NAME" ]; then
        echo ""
        if ! download_binary "$target" "$version"; then
            log_error "Failed to download MC-Minder v$version"
            echo ""
            log_info "Troubleshooting:"
            log_info "  1. Check your internet connection"
            log_info "  2. Verify the version exists: https://github.com/$REPO/releases/tag/v$version"
            log_info "  3. Try compiling from source: https://github.com/$REPO"
            exit 1
        fi
        
        local size=$(du -h "$BINARY_NAME" | cut -f1)
        log_info "Downloaded successfully ($size)"
    fi
    
    echo ""
    download_scripts "$version"
    
    show_post_install_instructions
}

main "$@"
