#!/bin/bash

CONFIG_FILE="config.toml"

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
            fi
            echo "$value"
            return
        fi
    fi
    echo "$default"
}

# 从 config.toml 读取配置，若未配置则使用默认值
WORLD_DIR=$(get_config_value "world_dir" "world")
BACKUP_DEST=$(get_config_value "backup_dest" "../backups")
RETAIN_DAYS=$(get_config_value "retain_days" "7")
BACKUP_NAME="mc-backup-$(date +%Y%m%d-%H%M%S).tar.gz"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

create_backup() {
    log_info "Creating backup..."
    
    mkdir -p "$BACKUP_DEST"
    
    if [ ! -d "$WORLD_DIR" ]; then
        log_error "World directory not found: $WORLD_DIR"
        exit 1
    fi
    
    local items_to_backup=""
    
    if [ -d "$WORLD_DIR" ]; then
        items_to_backup="$items_to_backup $WORLD_DIR"
    fi
    
    if [ -d "world_nether" ]; then
        items_to_backup="$items_to_backup world_nether"
    fi
    
    if [ -d "world_the_end" ]; then
        items_to_backup="$items_to_backup world_the_end"
    fi
    
    if [ -f "server.properties" ]; then
        items_to_backup="$items_to_backup server.properties"
    fi
    
    if [ -f "config.toml" ]; then
        items_to_backup="$items_to_backup config.toml"
    fi
    
    tar -czf "$BACKUP_DEST/$BACKUP_NAME" $items_to_backup 2>/dev/null
    
    if [ $? -eq 0 ]; then
        log_info "Backup created: $BACKUP_DEST/$BACKUP_NAME"
        log_info "Size: $(du -h "$BACKUP_DEST/$BACKUP_NAME" | cut -f1)"
    else
        log_error "Backup failed!"
        exit 1
    fi
}

clean_old_backups() {
    log_info "Cleaning backups older than $RETAIN_DAYS days..."
    
    local count=0
    while IFS= read -r file; do
        rm -f "$file"
        ((count++))
    done < <(find "$BACKUP_DEST" -name "mc-backup-*.tar.gz" -type f -mtime +$RETAIN_DAYS)
    
    if [ $count -gt 0 ]; then
        log_info "Removed $count old backup(s)"
    else
        log_info "No old backups to remove"
    fi
}

list_backups() {
    echo ""
    echo "Available Backups"
    echo "================="
    
    if [ -d "$BACKUP_DEST" ]; then
        ls -lh "$BACKUP_DEST"/mc-backup-*.tar.gz 2>/dev/null || echo "No backups found"
    else
        echo "Backup directory not found: $BACKUP_DEST"
    fi
    
    echo ""
}

restore_backup() {
    if [ -z "${2:-}" ]; then
        log_error "Please specify backup file to restore"
        echo "Usage: $0 restore <backup-file>"
        list_backups
        exit 1
    fi
    
    local backup_file="$2"
    
    if [ ! -f "$backup_file" ]; then
        log_error "Backup file not found: $backup_file"
        exit 1
    fi
    
    log_warn "This will overwrite your current world data!"
    read -p "Are you sure? (yes/N): " confirm
    
    if [ "$confirm" != "yes" ]; then
        log_info "Aborted"
        exit 0
    fi
    
    log_info "Restoring from $backup_file..."
    
    tar -xzf "$backup_file"
    
    if [ $? -eq 0 ]; then
        log_info "Restore complete!"
        log_warn "Please restart the server to apply changes"
    else
        log_error "Restore failed!"
        exit 1
    fi
}

case "${1:-}" in
    create)
        create_backup
        clean_old_backups
        ;;
    clean)
        clean_old_backups
        ;;
    list)
        list_backups
        ;;
    restore)
        restore_backup "$@"
        ;;
    *)
        echo "MC-Minder Backup Tool"
        echo ""
        echo "Usage: $0 {create|clean|list|restore <file>}"
        echo ""
        echo "Commands:"
        echo "  create        Create a new backup and clean old ones"
        echo "  clean         Remove backups older than $RETAIN_DAYS days"
        echo "  list          List all available backups"
        echo "  restore <file> Restore from specified backup file"
        ;;
esac
