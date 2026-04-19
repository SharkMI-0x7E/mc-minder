# MC-Minder

[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

English | [中文](./README.md)

A smart management suite for Minecraft Fabric servers on Termux/Android.

> This project was written with AI assistance.

## Features

- **Log Monitoring**: Real-time monitoring of server logs, parsing chat/join/leave/death events
- **AI Chatbot**: Support for OpenAI API and Ollama, triggered by `!` prefix
- **RCON Communication**: Native RCON protocol implementation for sending commands and messages
- **Context Memory**: Per-player conversation history with automatic expiration
- **HTTP API**: RESTful API for status queries, history, and command execution
- **Shell Scripts**: Integrated start/stop/monitor/backup management

## Installation

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

## Usage

### 1. Directory Structure

```
MC_server/                      # Server root directory
├── fabric-server.jar           # Server core
├── server.properties           # Server config (port, IP, etc. configured here)
├── start.sh                    # Startup script (copy from scripts/)
├── config.toml                 # MC-Minder configuration file
├── logs/
│   └── latest.log
├── world/
└── mc-minder/                  # This project
    ├── Cargo.toml
    ├── src/
    └── target/release/mc-minder
```

### 2. Configuration

**Server Configuration** (port, server name, IP, etc.) should be configured in `server.properties`, which is the native Minecraft configuration file.

**MC-Minder Configuration**: Copy `config.example.toml` to your server root directory and rename to `config.toml`:

```bash
cp mc-minder/config.example.toml ./config.toml
```

Edit `config.toml`:

```toml
# RCON Configuration - Required for MC-Minder to communicate with Minecraft server
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

# AI Configuration - Leave empty or remove this section to disable AI features
[ai]
api_url = ""
api_key = ""
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7

# Ollama Configuration - Set enabled = true to use local AI
[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"

# Backup Configuration
[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

# Notification Configuration - Leave empty to disable notifications
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
```

**Note**: Leave configuration items empty to disable that feature. For example:
- Leaving `api_key` empty in `[ai]` section will disable AI features
- Leaving `telegram_bot_token` empty in `[notification]` will disable Telegram notifications

### 3. Windows/Linux Line Ending Issue

If you edit scripts on Windows and get errors when running on Linux/Termux, you need to convert line endings:

```bash
# Method 1: Using dos2unix
dos2unix start.sh
dos2unix backup.sh

# Method 2: Using sed
sed -i 's/\r$//' start.sh
sed -i 's/\r$//' backup.sh

# Method 3: Batch conversion
sed -i 's/\r$//' *.sh
```

### 4. Start the Server

```bash
# Copy scripts to server root
cp mc-minder/scripts/start.sh ./
cp mc-minder/scripts/start_en.sh ./  # English version

# Start
./start.sh start

# Stop
./start.sh stop

# Status
./start.sh status

# Attach to console
./start.sh attach
```

### 5. AI Chat Usage

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

## Project Structure

```
mc-minder/
├── Cargo.toml              # Rust project configuration
├── config.example.toml     # Configuration example
├── README.md               # Chinese documentation
├── README_en.md            # English documentation
├── LICENSE                 # MIT License
├── .gitignore
├── scripts/
│   ├── start.sh            # Startup script (Chinese)
│   ├── start_en.sh         # Startup script (English)
│   └── backup.sh           # Backup utility
└── src/
    ├── main.rs             # Main entry point
    ├── lib.rs              # Library exports
    ├── config/             # Configuration module
    ├── monitor/            # Log monitoring module
    ├── ai/                 # AI client module
    ├── rcon/               # RCON protocol module
    ├── context/            # Context management module
    └── api/                # HTTP API module
```

## Command Line Options

```
mc-minder [OPTIONS]

Options:
  -c, --config <PATH>  Configuration file path [default: ../config.toml]
  -v, --verbose        Enable verbose logging
      --http-port      HTTP API port [default: 8080]
      --log-file       Log file path [default: logs/latest.log]
  -h, --help           Show help
  -V, --version        Show version
```

## Backup

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

## Requirements

- Rust 1.70+
- Java (for Minecraft server)
- tmux (for session management)
- Optional: Ollama (for local AI)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- This project was written with AI assistance
- Inspired by the need for lightweight Minecraft server management on mobile devices

## Author

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)
