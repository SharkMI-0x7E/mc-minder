# MC-Minder 🎮

[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

A smart management suite for Minecraft Fabric servers on Termux/Android. 一个为 Termux/Android 上的 Minecraft Fabric 服务器设计的智能管理套件。

> **Note**: This project was written with AI assistance. 本项目由 AI 辅助编写。

## Features 功能特性

- **Log Monitoring 日志监控**: Real-time monitoring of server logs, parsing chat/join/leave/death events
- **AI Chatbot AI 聊天机器人**: Support for OpenAI API and Ollama, triggered by `!` prefix
- **RCON Communication RCON 通信**: Native RCON protocol implementation for sending commands and messages
- **Context Memory 上下文记忆**: Per-player conversation history with automatic expiration
- **HTTP API**: RESTful API for status queries, history, and command execution
- **Shell Scripts Shell 脚本**: Integrated start/stop/monitor/backup management

## Installation 安装

### From crates.io

```bash
cargo install mc-minder
```

### From Source

```bash
git clone https://github.com/SharkMI-0x7E/mc-minder.git
cd mc-minder
cargo build --release
```

### For Termux/Android (aarch64)

```bash
cargo build --target aarch64-linux-android --release
```

## Usage 使用方法

### 1. Directory Structure 目录结构

```
MC_server/                      # Server root directory
├── fabric-server.jar           # Server core
├── start.sh                    # Startup script (copy from scripts/)
├── config.toml                 # Configuration file
├── logs/
│   └── latest.log
├── world/
└── mc-minder/                  # This project
    ├── Cargo.toml
    ├── src/
    └── target/release/mc-minder
```

### 2. Configuration 配置

Copy `config.example.toml` to your server root directory and rename to `config.toml`:

```bash
cp mc-minder/config.example.toml ./config.toml
```

Edit `config.toml`:

```toml
[server]
jar = "fabric-server.jar"
min_mem = "512M"
max_mem = "1G"
session_name = "mc_server"
log_file = "logs/latest.log"

[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

[ai]
api_url = "https://api.openai.com/v1/chat/completions"
api_key = "sk-xxx"
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7

[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"

[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
```

### 3. Start the Server 启动服务器

```bash
# Copy scripts to server root
cp mc-minder/scripts/start.sh ./
cp mc-minder/scripts/backup.sh ./

# Start
./start.sh start

# Stop
./start.sh stop

# Status
./start.sh status

# Attach to console
./start.sh attach
```

### 4. AI Chat Usage AI 聊天使用

Players can trigger AI responses by prefixing their message with `!`:

```
!hello
!help
!how to make a diamond sword?
```

## HTTP API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/status` | GET | Get server status and uptime |
| `/history` | GET | Get conversation history |
| `/command` | POST | Execute RCON command |

Example:

```bash
# Get status
curl http://localhost:8080/status

# Execute command
curl -X POST http://localhost:8080/command \
  -H "Content-Type: application/json" \
  -d '{"command": "list"}'
```

## Project Structure 项目结构

```
mc-minder/
├── Cargo.toml              # Rust project configuration
├── config.example.toml     # Configuration example
├── README.md               # This file
├── LICENSE                 # MIT License
├── .gitignore
├── scripts/
│   ├── start.sh            # Server startup script
│   └── backup.sh           # Backup utility
└── src/
    ├── main.rs             # Main entry point
    ├── lib.rs              # Library exports
    ├── config.rs           # Configuration parsing
    ├── log_monitor.rs      # Log file monitoring
    ├── ai_client.rs        # AI API client
    ├── rcon_client.rs      # RCON protocol client
    ├── context.rs          # Conversation context manager
    └── http_api.rs         # HTTP API server
```

## Command Line Options 命令行选项

```
mc-minder [OPTIONS]

Options:
  -c, --config <PATH>  Configuration file path [default: ../config.toml]
  -v, --verbose        Enable verbose logging
      --http-port      HTTP API port [default: 8080]
  -h, --help           Show help
  -V, --version        Show version
```

## Backup 备份

```bash
# Create backup
./backup.sh create

# List backups
./backup.sh list

# Restore from backup
./backup.sh restore ../backups/mc-backup-20240101-120000.tar.gz

# Clean old backups
./backup.sh clean
```

## Requirements 系统要求

- Rust 1.70+
- Java (for Minecraft server)
- tmux (for session management)
- Optional: Ollama (for local AI)

## Contributing 贡献

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License 许可证

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments 致谢

- This project was written with AI assistance
- Inspired by the need for lightweight Minecraft server management on mobile devices

## Author 作者

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)
