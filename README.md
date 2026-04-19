# MC-Minder

[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[English](./README_en.md) | 中文

一个为 Termux/Android 上的 Minecraft Fabric 服务器设计的智能管理套件。

> 本项目由 AI 辅助编写。

## 功能特性

- **日志监控**：实时监控服务器日志，解析聊天/加入/离开/死亡事件
- **AI 聊天机器人**：支持 OpenAI API 和 Ollama，玩家使用 `!` 前缀触发
- **RCON 通信**：原生 RCON 协议实现，发送命令和消息
- **上下文记忆**：按玩家保存对话历史，自动过期清理
- **HTTP API**：RESTful API 用于状态查询、历史记录和命令执行
- **Shell 脚本**：集成启动/停止/监控/备份管理

## 安装

### 从 crates.io 安装

```bash
cargo install mc-minder
```

### 从源码编译

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

### 1. 目录结构

```
MC_server/                      # 服务器根目录
├── fabric-server.jar           # 服务端核心
├── server.properties           # 服务器配置（端口、IP等在此配置）
├── start.sh                    # 启动脚本（从 scripts/ 复制）
├── config.toml                 # MC-Minder 配置文件
├── logs/
│   └── latest.log
├── world/
└── mc-minder/                  # 本项目
    ├── Cargo.toml
    ├── src/
    └── target/release/mc-minder
```

### 2. 配置

**服务器配置**（端口、服务器名、IP等）请在 `server.properties` 中配置，这是 Minecraft 原生配置文件。

**MC-Minder 配置**：将 `config.example.toml` 复制到服务器根目录并重命名为 `config.toml`：

```bash
cp mc-minder/config.example.toml ./config.toml
```

编辑 `config.toml`：

```toml
# RCON 配置 - MC-Minder 与 Minecraft 服务器通信必需
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

# AI 配置 - 留空或删除此部分可禁用 AI 功能
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

# 通知配置 - 留空禁用通知功能
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
```

**注意**：配置项留空表示不启用该功能。例如：
- `[ai]` 部分的 `api_key` 留空将禁用 AI 功能
- `[notification]` 中的 `telegram_bot_token` 留空将禁用 Telegram 通知

### 3. Windows/Linux 换行符问题

如果在 Windows 上编辑脚本后在 Linux/Termux 上运行报错，需要转换换行符：

```bash
# 方法一：使用 dos2unix
dos2unix start.sh
dos2unix backup.sh

# 方法二：使用 sed
sed -i 's/\r$//' start.sh
sed -i 's/\r$//' backup.sh

# 方法三：批量转换
sed -i 's/\r$//' *.sh
```

### 4. 启动服务器

```bash
# 复制脚本到服务器根目录
cp mc-minder/scripts/start.sh ./
cp mc-minder/scripts/start_en.sh ./  # 英文版

# 启动
./start.sh start

# 停止
./start.sh stop

# 查看状态
./start.sh status

# 附加到控制台
./start.sh attach
```

### 5. AI 聊天使用

玩家在游戏中使用 `!` 前缀触发 AI 响应：

```
!你好
!help
!如何制作钻石剑？
```

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

## 项目结构

```
mc-minder/
├── Cargo.toml              # Rust 项目配置
├── config.example.toml     # 配置示例
├── README.md               # 中文文档
├── README_en.md            # 英文文档
├── LICENSE                 # MIT 许可证
├── .gitignore
├── scripts/
│   ├── start.sh            # 启动脚本（中文）
│   ├── start_en.sh         # 启动脚本（英文）
│   └── backup.sh           # 备份工具
└── src/
    ├── main.rs             # 主入口
    ├── lib.rs              # 库导出
    ├── config/             # 配置模块
    ├── monitor/            # 日志监控模块
    ├── ai/                 # AI 客户端模块
    ├── rcon/               # RCON 协议模块
    ├── context/            # 上下文管理模块
    └── api/                # HTTP API 模块
```

## 命令行选项

```
mc-minder [OPTIONS]

选项:
  -c, --config <PATH>  配置文件路径 [默认: ../config.toml]
  -v, --verbose        启用详细日志
      --http-port      HTTP API 端口 [默认: 8080]
      --log-file       日志文件路径 [默认: logs/latest.log]
  -h, --help           显示帮助
  -V, --version        显示版本
```

## 备份

```bash
# 创建备份
./backup.sh create

# 列出备份
./backup.sh list

# 从备份恢复
./backup.sh restore ../backups/mc-backup-20240101-120000.tar.gz

# 清理旧备份
./backup.sh clean
```

## 系统要求

- Rust 1.70+
- Java（用于 Minecraft 服务器）
- tmux（用于会话管理）
- 可选：Ollama（用于本地 AI）

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
