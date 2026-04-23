# MC-Minder - Java version management library
# Sourced by start-tui.sh and other scripts
# Contains: Java detection, switching, installation, menu

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/config.sh"

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