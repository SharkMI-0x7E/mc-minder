# MC-Minder

[![Build and Release](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml/badge.svg)](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[English](./README_en.md) | 中文

一个为 Minecraft Fabric 服务器设计的智能管理套件，支持 MacOS、Linux 和 Termux/Android 环境。

> 本项目诞生于在 Termux 上更方便地管理 Minecraft 服务器的需求。

> 本项目由 AI 辅助编写。

## 功能特性

- **日志监控**：使用 notify 库实时监控服务器日志，事件驱动，低 CPU 占用
- **RCON 通信**：持久 RCON 连接池，自动重连，命令响应可见
- **聊天捕获**：ChatCapture trait 统一接口，支持 Tmux/File/Process 三种模式
- **前台进程**：TUI 内直接运行 Java 进程，stdio 管道实时捕获
- **HTTP API**：RESTful API 用于状态查询和命令执行（返回 RCON 响应）
- **Java 管理**：自动检测 Java 版本，平台专属安装指引，自定义 JDK 路径
- **一键安装**：交互式初始化，自动检测环境依赖
- **自动更新**：内置 self-update 命令

## 快速开始

### 一键安装

```bash
curl -fsSL https://github.com/SharkMI-0x7E/mc-minder/releases/latest/download/install.sh | bash
```

### 初始化配置

```bash
./mc-minder init
```

这将引导你完成配置：
1. Java 自动检测和安装指引
2. 设置 RCON 密码
3. 配置服务器内存和会话名称
4. 自定义 JDK 路径（可选）

### 启动服务器

```bash
./start-tui.sh
```

## 安装方式

### 方式一：预编译二进制（推荐）

```bash
# 下载安装脚本
curl -fsSL https://github.com/SharkMI-0x7E/mc-minder/releases/latest/download/install.sh | bash

# 初始化配置
./mc-minder init
```

### 方式二：从 crates.io 安装

```bash
cargo install mc-minder
```

### 方式三：从源码编译

```bash
git clone https://github.com/SharkMI-0x7E/mc-minder.git
cd mc-minder
cargo build --release
```

### Termux/Android (aarch64)

```bash
cargo build --target aarch64-linux-android --release
```

### Linux ARM64（树莓派等）

```bash
cargo build --target aarch64-unknown-linux-gnu --release
```

或者从 [Releases](https://github.com/SharkMI-0x7E/mc-minder/releases/latest) 下载预编译二进制文件 (`mc-minder-aarch64-linux`)。

## 使用方法

### 目录结构

```
MC_server/                      # 服务器根目录
├── fabric-server.jar           # 服务端核心
├── server.properties           # 服务器配置
├── mc-minder                   # MC-Minder 二进制
├── start-tui.sh                # TUI 启动脚本（图形化界面）
├── backup.sh                   # 备份脚本
├── config.toml                 # MC-Minder 配置文件
├── logs/
│   ├── latest.log              # 服务器日志
│   └── mc-minder.log           # MC-Minder 日志
└── world/
```

### 配置

**服务器配置**（端口、服务器名、IP等）请在 `server.properties` 中配置。

**MC-Minder 配置**：编辑 `config.toml`：

```toml
# 服务器配置
[server]
jar = "fabric-server.jar"
min_mem = "512M"
max_mem = "1G"
session_name = "mc_server"
log_file = "logs/latest.log"

# RCON 配置 - MC-Minder 与 Minecraft 服务器通信必需
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

# JVM 配置
[jvm]
gc = "G1GC"
extra_flags = ""
# jdk_path = "/usr/lib/jvm/java-17-openjdk/bin/java"  # 可选：自定义 JDK 路径

# 备份配置
[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

# 通知配置
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
```

### 启动脚本命令

```bash
./start-tui.sh           # 启动 TUI 管理界面（推荐）
```

### MC-Minder 命令行

```bash
./mc-minder tui           # 启动原生 TUI 管理界面（推荐）
./mc-minder init          # 交互式初始化
./mc-minder gen-config    # 生成默认配置文件
./mc-minder gen-start     # 生成 start-tui.sh
./mc-minder gen-backup    # 生成 backup.sh
./mc-minder self-update   # 更新到最新版本
./mc-minder config        # 显示当前配置
./mc-minder config get <key>  # 获取配置值（如 backup_dest）
./mc-minder --help        # 显示帮助
```

### TUI 管理界面（v0.4.2+）

MC-Minder 现在提供原生 Rust TUI，无需 Shell 脚本和 dialog 依赖：

```bash
./mc-minder tui           # 直接启动 TUI
./start-tui.sh            # 通过启动脚本（自动查找二进制）
```

**TUI 功能**：
- **主菜单**（13 个选项）：上下键/数字键导航，Enter 确认
- **服务器控制**：后台启动、前台启动、停止、重启、状态查看
- **实时控制台视图**：tmux capture-pane 自动刷新服务器输出，支持手动/自动刷新
- **日志查看**：服务器日志和 MC-Minder 日志，支持滚动
- **原生更新流程**：TUI 内置异步下载和安装，显示实时进度，无需退出 TUI
- **Java 管理**：版本检测、安装指引（支持 Termux/Linux/macOS）、自定义 JDK 路径
- **配置向导**：交互式配置服务器参数
- **语言切换**：中英文即时切换，持久化保存
- **状态监控**：tmux 会话、mc-minder 进程、看门狗状态

**键盘快捷键**：
- 上下箭头 / j/k：导航
- 1-9：快速选择
- Enter：确认
- Esc/q：返回/退出
- r：手动刷新（控制台视图中）
- a：切换自动刷新（控制台视图中）

### Java 管理

MC-Minder 提供完整的 Java 管理功能：

**自动检测**：
- `init` 命令自动运行 `java -version` 检测
- TUI Java 菜单扫描系统默认、自定义路径和常见安装目录

**平台安装指引**：
- **Termux**：`pkg install openjdk-17`
- **Linux**：`apt install openjdk-17-jre` / `dnf install java-17-openjdk`
- **macOS**：`brew install openjdk@17`

**自定义 JDK**：
- 在 `config.toml` 的 `[jvm]` 部分设置 `jdk_path`
- TUI 配置向导支持直接编辑 JDK 路径

## HTTP API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/status` | GET | 获取服务器状态和运行时间 |
| `/command` | POST | 执行 RCON 命令 |

示例：

```bash
# 获取状态
curl http://localhost:8080/status

# 执行命令
curl -X POST http://localhost:8080/command \
  -H "Content-Type: application/json" \
  -d '{"command": "list"}'
```

## 命令行选项

```
mc-minder [OPTIONS] [COMMAND]

Commands:
  init         交互式初始化配置
  gen-config   生成默认配置文件
  gen-start    生成 start.sh 脚本
  gen-backup   生成 backup.sh 脚本
  self-update  更新到最新版本
  config       显示当前配置

Options:
  -c, --config <PATH>  配置文件路径 [默认: config.toml]
  -v, --verbose        启用详细日志
      --http-port      HTTP API 端口 [默认: 8080]
  -h, --help           显示帮助
  -V, --version        显示版本
```

## 备份

```bash
./backup.sh create   # 创建备份
./backup.sh list     # 列出备份
./backup.sh restore <file>  # 从备份恢复
./backup.sh clean    # 清理旧备份
```

## 系统要求

- Rust 1.70+（仅编译时需要）
- Java 17+（用于 Minecraft 服务器）
- tmux（用于会话管理）

## Windows 换行符问题

如果在 Windows 上编辑脚本后在 Linux/Termux 上运行报错 `$'\r': command not found`，请转换换行符：

```bash
# 方法一：使用 dos2unix
dos2unix *.sh

# 方法二：使用 sed
sed -i 's/\r$//' *.sh
```

本项目已在 `.gitattributes` 中配置 `*.sh text eol=lf`，Git 检出时会自动使用 LF 换行符。

## 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 许可证

本项目采用 MIT 许可证 - 详情请见 [LICENSE](LICENSE) 文件。

## 致谢

- 本项目99.2%的内容由 AI 编写(v0.3.15起从Trae CN换到了Opencode)
- 我自己不是很懂Rust。v0.3.14(包括0.3.14)前的代码，都是由 "Trae CN" 的智能体编写。v0.3.15(包括0.3.15)之后的代码，由opencode和Trae编写。
- 灵感来源于移动设备上轻量级 Minecraft 服务器管理的需求

## 作者

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)