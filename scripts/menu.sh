# MC-Minder - Menu library
# Sourced by start-tui.sh
# Contains: main menu, Java menu, language switching

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/config.sh"
source "$(dirname "${BASH_SOURCE[0]}")/java.sh"
source "$(dirname "${BASH_SOURCE[0]}")/server.sh"
source "$(dirname "${BASH_SOURCE[0]}")/log.sh"

# ==================== 语言切换 ====================
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

# ==================== 主菜单 ====================
show_main_menu() {
    # 加载语言设置
    load_lang

    while true; do
        exec 3>&1
        selection=$(dialog --clear \
            --backtitle "MC-Minder - $(T "minecraft 服务器管理套件" "Minecraft Server Management Suite")" \
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
