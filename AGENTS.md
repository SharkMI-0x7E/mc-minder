# MC-Minder 开发指南

## 项目架构

### 目录结构

```
mc-minder/
├── Cargo.toml              # Rust 项目配置
├── config.example.toml     # 配置文件示例
├── README.md               # 中文文档
├── README_en.md            # 英文文档
├── LICENSE                 # MIT 许可证
├── agents.md               # 本开发指南
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

### 模块说明

| 模块 | 路径 | 职责 |
|------|------|------|
| config | `src/config/` | 配置文件解析，支持 TOML 格式 |
| monitor | `src/monitor/` | 日志文件监控，解析游戏事件 |
| ai | `src/ai/` | AI API 客户端，支持 OpenAI 和 Ollama |
| rcon | `src/rcon/` | RCON 协议实现，与 Minecraft 服务器通信 |
| context | `src/context/` | 对话上下文管理，按玩家存储历史 |
| api | `src/api/` | HTTP API 服务，提供 RESTful 接口 |

## 核心数据流

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Minecraft  │────▶│    Log      │────▶│   Event     │
│   Server    │     │   Monitor   │     │   Channel   │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    RCON     │◀────│   Main      │◀────│    AI       │
│   Client    │     │   Event     │     │   Client    │
└─────────────┘     │   Loop      │     └─────────────┘
                    └─────────────┘
```

## 开发规范

### 代码风格

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 所有公共 API 必须有文档注释
- 错误处理使用 `anyhow::Result`

### 模块开发

创建新模块时：

1. 在 `src/` 下创建模块文件夹
2. 创建 `mod.rs` 文件
3. 在 `src/lib.rs` 中添加 `pub mod module_name;`
4. 在 `src/main.rs` 中添加 `mod module_name;`

### 配置项

配置文件只包含 MC-Minder 特有配置，服务器配置（端口、IP等）在 `server.properties` 中。

### 日志规范

- 使用 `log` crate 的宏：`info!`, `warn!`, `error!`, `debug!`
- 日志输出到 stderr，可重定向到文件
- 使用 `env_logger` 控制日志级别

## 扩展功能

### 添加新的日志事件

1. 在 `src/monitor/mod.rs` 的 `LogEvent` 枚举中添加新变体
2. 在 `parse_lines` 方法中添加解析逻辑
3. 在 `main.rs` 的事件循环中处理新事件

### 添加新的 AI 提供商

1. 在 `src/ai/mod.rs` 中添加新的请求/响应结构体
2. 实现 `chat_xxx` 异步方法
3. 在 `chat` 方法中添加路由逻辑

### 添加新的 HTTP 端点

1. 在 `src/api/mod.rs` 中定义请求/响应结构体
2. 创建新的路由处理函数
3. 在 `start` 方法中注册路由

## 构建与发布

### 开发构建

```bash
cargo build
```

### 生产构建

```bash
cargo build --release
```

### Termux/Android 构建

```bash
cargo build --target aarch64-linux-android --release
```

### 发布到 crates.io

```bash
cargo publish
```

## 测试

```bash
cargo test
```

## 常见问题

### Windows 编译错误

Windows 上可能遇到 `proc-macro2` 编译错误，这是 Rust 工具链的已知问题。建议在 Linux/WSL 环境中编译。

### 换行符问题

Windows 上编辑的脚本在 Linux 上运行需要转换换行符：

```bash
sed -i 's/\r$//' *.sh
# 或
dos2unix *.sh
```

## 依赖说明

| 依赖 | 用途 |
|------|------|
| tokio | 异步运行时 |
| serde / toml | 配置文件解析 |
| reqwest | HTTP 客户端（AI API） |
| warp | HTTP 服务器 |
| regex | 日志解析 |
| chrono | 时间处理 |
| parking_lot | 高性能锁 |
| byteorder | RCON 协议字节序 |
| clap | 命令行参数 |
| anyhow / thiserror | 错误处理 |
| env_logger / log | 日志系统 |

## 贡献指南

1. Fork 仓库
2. 创建功能分支
3. 提交代码（遵循 Conventional Commits）
4. 推送分支
5. 创建 Pull Request

## 联系方式

- GitHub: [@SharkMI-0x7E](https://github.com/SharkMI-0x7E)
