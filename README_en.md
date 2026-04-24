# MC-Minder

[![Build and Release](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml/badge.svg)](https://github.com/SharkMI-0x7E/mc-minder/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/mc-minder.svg)](https://crates.io/crates/mc-minder)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

English | [中文](./README.md)

A smart management suite for Minecraft Fabric servers, supporting macOS, Linux and Termux/Android environments.

> This project was born from the need to manage Minecraft servers more conveniently on Termux.

> 99.2% of this project was written by AI (v0.3.15 onwards uses Opencode, before that used Trae CN).

## Quick Start

### One-click Install

```bash
curl -fsSL https://github.com/SharkMI-0x7E/mc-minder/releases/latest/download/install.sh | bash
```

### Initialize Configuration

```bash
./mc-minder init
```

This will guide you through:
1. Setting RCON password
2. Choosing whether to enable AI features
3. Configuring server memory and session name

### Start Server

```bash
./start-tui.sh
```

## Installation Methods

### Method 1: Pre-compiled Binary (Recommended)

```bash
# Download install script
curl -fsSL https://github.com/SharkMI-0x7E/mc-minder/releases/latest/download/install.sh | bash

# Initialize configuration
./mc-minder init
```

### Method 2: Install from crates.io

```bash
cargo install mc-minder
```

### Method 3: Build from Source

```bash
git clone https://github.com/SharkMI-0x7E/mc-minder.git
cd mc-minder
cargo build --release
```

### Termux/Android (aarch64)

```bash
cargo build --target aarch64-linux-android --release
```

## Usage

### Directory Structure

```
MC_server/                      # Server root directory
├── fabric-server.jar           # Server core
├── server.properties           # Server configuration
├── mc-minder                   # MC-Minder binary
├── start-tui.sh                # TUI startup script (graphical interface)
├── backup.sh                   # Backup script
├── config.toml                 # MC-Minder configuration file
├── logs/
│   ├── latest.log              # Server log
│   └── mc-minder.log           # MC-Minder log
└── world/
```

### Configuration

**Server Configuration** (port, server name, IP, etc.) should be configured in `server.properties`.

**MC-Minder Configuration**: Edit `config.toml`:

```toml
# Server configuration
[server]
jar = "fabric-server.jar"
min_mem = "512M"
max_mem = "1G"
session_name = "mc_server"
log_file = "logs/latest.log"

# RCON Configuration - Required for MC-Minder to communicate with Minecraft server
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

# AI Configuration - Leave empty to disable AI features
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

# Notification Configuration
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
```

### Startup Script Commands

```bash
./start-tui.sh           # Start TUI management interface (recommended)
```

### MC-Minder Command Line

```bash
./mc-minder tui           # Start native TUI interface (recommended)
./mc-minder init          # Interactive initialization
./mc-minder gen-config    # Generate default config file
./mc-minder gen-start     # Generate start-tui.sh
./mc-minder gen-backup    # Generate backup.sh
./mc-minder self-update   # Update to latest version
./mc-minder config        # Show current configuration
./mc-minder config get <key>  # Get config value (e.g., backup_dest)
./mc-minder --help        # Show help
```

### TUI Management Interface (v0.4.2+)

MC-Minder now provides a native Rust TUI, no Shell scripts or dialog dependency required:

```bash
./mc-minder tui           # Launch TUI directly
./start-tui.sh            # Via launcher script (auto-detects binary)
```

**TUI Features**:
- **Main Menu** (13 items): Arrow keys/number keys navigation, Enter to select
- **Server Control**: Start background/foreground, stop, restart, status view
- **Real-time Console View**: Live server output via tmux capture-pane with auto/manual refresh
- **Log Viewer**: Server log and MC-Minder log with scroll support
- **Native Update Flow**: Async download and install with real-time progress, no need to exit TUI
- **Java Management**: Version detection, switching, installation (Termux)
- **Config Wizard**: Interactive configuration of 10 parameters
- **Language Switching**: Chinese/English instant switch, persistent storage
- **Status Monitor**: tmux session, mc-minder process, watchdog status

**Keyboard Shortcuts**:
- Up/Down arrows / j/k: Navigate
- 1-9: Quick select
- Enter: Confirm
- Esc/q: Back/Quit
- r: Manual refresh (in console view)
- a: Toggle auto-refresh (in console view)

### AI Chat Usage

Players trigger AI responses by prefixing messages with `!`:

```
!hello
!help
!how to make a diamond sword?
```

**AI Backends**:
- **OpenAI-compatible API**: Supports any OpenAI-format API (e.g., OpenAI, DeepSeek)
- **Ollama Local Models**: Set `enabled = true` in config to use local AI models
- Auto-routing: Uses Ollama when enabled, otherwise falls back to remote API

**Chat Capture Modes**:
- **Tmux Mode**: Captures server chat output via tmux capture-pane
- **File Mode**: Parses server log file directly for chat messages

**Rate Limiting**:
- Minimum 2-second interval between requests from the same player
- Maximum 3 concurrent requests

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

## Command Line Options

```
mc-minder [OPTIONS] [COMMAND]

Commands:
  init         Interactive configuration initialization
  gen-config   Generate default config file
  gen-start    Generate start-tui.sh script
  gen-backup   Generate backup.sh script
  self-update  Update to latest version
  config       Show current configuration

Options:
  -c, --config <PATH>  Configuration file path [default: config.toml]
  -v, --verbose        Enable verbose logging
      --http-port      HTTP API port [default: 8080]
  -h, --help           Show help
  -V, --version        Show version
```

## Backup

```bash
./backup.sh create   # Create backup
./backup.sh list     # List backups
./backup.sh restore <file>  # Restore from backup
./backup.sh clean    # Clean old backups
```

## System Requirements

- Rust 1.70+ (only needed for compilation)
- Java 17+ (for Minecraft server)
- tmux (for session management)
- Optional: Ollama (for local AI)

## Windows Line Ending Issues

If you edit scripts on Windows and get errors when running on Linux/Termux (`$'\r': command not found`), convert line endings:

```bash
# Method 1: Using dos2unix
dos2unix *.sh

# Method 2: Using sed
sed -i 's/\r$//' *.sh
```

This project has configured `*.sh text eol=lf` in `.gitattributes`, Git checkout will automatically use LF line endings.

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

- 99.2% of this project was written by AI (v0.3.15 onwards uses Opencode, before that used Trae CN)
- I don't really know Rust. Code before v0.3.14 (including v0.3.14) was written by Trae CN's agent. Code from v0.3.15 (including v0.3.15) onwards was written by Opencode.
- Inspired by the need for lightweight Minecraft server management on mobile devices

## Author

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)
