# MC-Minder

[![Build and Release](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml/badge.svg)](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[English](./README_en.md) | 中文

一个为 Minecraft Fabric 服务器设计的智能管理套件，支持 Linux 和 Termux/Android 环境。

> 本项目诞生于在 Termux 上更方便地管理 Minecraft 服务器的需求。

> 本项目由 AI 辅助编写。

## 功能特性

- **日志监控**：使用 notify 库实时监控服务器日志，事件驱动，低 CPU 占用
- **AI 聊天机器人**：支持 OpenAI API 和 Ollama，玩家使用 `!` 前缀触发，内置请求限流
- **RCON 通信**：异步 RCON 协议实现，自动重连，不阻塞事件循环
- **上下文记忆**：按玩家保存对话历史，自动过期清理
- **HTTP API**：RESTful API 用于状态查询、历史记录和命令执行
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
1. 设置 RCON 密码
2. 选择是否启用 AI 功能
3. 配置服务器内存和会话名称

### 启动服务器

```bash
./start.sh start
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

## 使用方法

### 目录结构

```
MC_server/                      # 服务器根目录
├── fabric-server.jar           # 服务端核心
├── server.properties           # 服务器配置
├── mc-minder                   # MC-Minder 二进制
├── start.sh                    # 启动脚本
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

# AI 配置 - 留空禁用 AI 功能
[ai]
api_url = ""
api_key = ""
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7

# Ollama 配置 - 设置 enabled = true 使用本地 AI
[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"

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
./start.sh start      # 启动服务器和 MC-Minder
./start.sh stop       # 停止所有服务
./start.sh restart    # 重启服务
./start.sh status     # 查看状态
./start.sh attach     # 附加到服务器控制台
./start.sh logs       # 查看服务器日志
./start.sh minder-logs # 查看 MC-Minder 日志
./start.sh init       # 初始化配置
./start.sh update     # 更新 MC-Minder
```

### MC-Minder 命令行

```bash
./mc-minder init          # 交互式初始化
./mc-minder gen-config    # 生成默认配置文件
./mc-minder gen-start     # 生成 start.sh
./mc-minder gen-backup    # 生成 backup.sh
./mc-minder self-update   # 更新到最新版本
./mc-minder config        # 显示当前配置
./mc-minder config get <key>  # 获取配置值（如 backup_dest）
./mc-minder --help        # 显示帮助
```

### AI 聊天使用

玩家在游戏中使用 `!` 前缀触发 AI 响应：

```
!你好
!help
!如何制作钻石剑？
```

**限流机制**：
- 同一玩家请求间隔至少 2 秒
- 最大并发请求数为 3

## HTTP API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/status` | GET | 获取服务器状态和运行时间 |
| `/history` | GET | 获取对话历史 |
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
- 可选：Ollama（用于本地 AI）

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

- 本项目由 AI 辅助编写
- 灵感来源于移动设备上轻量级 Minecraft 服务器管理的需求

## 作者

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)
