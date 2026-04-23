# MC-Minder - Log viewing library
# Sourced by start-tui.sh and other scripts
# Contains: server log viewer, mc-minder log viewer

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

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
