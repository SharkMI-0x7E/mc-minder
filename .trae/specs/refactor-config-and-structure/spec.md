# 重构配置和目录结构 Spec

## Why
当前项目的配置文件包含冗余配置项（server.properties 已有），README 文件混合中英文，src 目录结构缺乏模块化设计，且 Windows/Linux 换行符问题未处理。

## What Changes
- 简化 config.toml，移除 server.properties 已有的配置项（端口、服务器名、IP等）
- 分离 README 文件：README.md（中文）、README_en.md（英文）
- 重构 src 目录结构，按模块分类组织代码
- 在文档中添加换行符处理说明

## Impact
- Affected specs: 配置系统、文档结构、代码组织
- Affected code: src/ 目录所有文件、config.example.toml、README.md

## ADDED Requirements

### Requirement: 简化配置文件
系统配置 SHALL 只包含 MC-Minder 特有的配置项，不与 server.properties 重复。

#### Scenario: 配置加载
- **WHEN** 用户配置 config.toml
- **THEN** 只需配置 AI、RCON、备份、通知等 MC-Minder 特有功能

### Requirement: 分离文档语言
文档 SHALL 按语言分离为独立文件。

#### Scenario: 文档访问
- **WHEN** 用户查看 README.md
- **THEN** 显示中文文档
- **WHEN** 用户查看 README_en.md
- **THEN** 显示英文文档

### Requirement: 模块化目录结构
src 目录 SHALL 按功能模块分类组织代码文件。

#### Scenario: 目录结构
- **WHEN** 开发者查看 src 目录
- **THEN** 看到按模块分类的文件夹结构

### Requirement: 换行符兼容性说明
文档 SHALL 包含 Windows/Linux 换行符处理说明。

#### Scenario: 跨平台使用
- **WHEN** 用户在 Windows 上编辑脚本后在 Linux 运行
- **THEN** 文档提供 dos2unix 或 sed 命令解决方案

## MODIFIED Requirements

### Requirement: 配置文件结构
配置文件结构修改为：
```toml
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

[ai]
api_url = ""
api_key = ""
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

### Requirement: src 目录结构
src 目录结构修改为：
```
src/
├── main.rs
├── lib.rs
├── config/
│   └── mod.rs
├── monitor/
│   └── mod.rs
├── ai/
│   └── mod.rs
├── rcon/
│   └── mod.rs
├── context/
│   └── mod.rs
└── api/
    └── mod.rs
```

## REMOVED Requirements

### Requirement: server 配置块
**Reason**: server.properties 已包含 jar、内存、会话名、日志文件等配置
**Migration**: 用户直接修改 server.properties 配置服务器参数
