# AGENTS.md

> 项目上下文文件 - 供 AI 助手参考的开发指南

## 项目概览

**项目名称**: mc-minder  
**描述**: 为 Termux/Android 上的 Minecraft Fabric 服务器设计的智能管理套件  
**语言**: Rust (Edition 2021)  
**版本**: 0.4.0 <!-- ⚠️ 每次发布新版本时请同步更新此处版本号 -->
**仓库**: https://github.com/SharkMI-0x7E/mc-minder

## 核心功能

- 日志监控：使用 notify 库实时监控服务器日志，事件驱动，低 CPU 占用
- AI 聊天机器人：支持 OpenAI API 和 Ollama，玩家使用 `!` 前缀触发
- RCON 通信：异步 RCON 协议实现，自动重连
- 上下文记忆：按玩家保存对话历史
- HTTP API：RESTful API 用于状态查询和命令执行
- 一键安装：交互式初始化配置
- 自动更新：内置 self-update 命令

## 国际化文档同步

**重要**: 本项目维护中英双语文档，更新时必须同步：

| 中文文档 | 英文文档 |
|----------|----------|
| README.md | README_en.md |

**规则**：
- 每次更新 README.md 时，必须同步更新 README_en.md
- 保持两份文档的结构、功能描述、命令示例一致
- 英文文档不需要逐字翻译，但必须包含相同的信息

## 构建配置

### Release Profile (Cargo.toml)
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 依赖项注意事项 (重要!)
- **reqwest**: 使用 `rustls-tls` feature + `default-features = false`（避免 OpenSSL 交叉编译问题）
- **reqwest 版本**: 使用 `0.12` 版本（默认使用 `aws-lc-rs` 后端，纯 Rust 实现）
- tokio: 完整异步运行时，包含 signal 和 process 功能
- notify: 文件系统事件监控
- dialoguer: 交互式 CLI 提示
- colored: 终端彩色输出
- warp: HTTP 服务器

### Cargo.toml 关键配置
```toml
# 正确的 reqwest 配置（用于交叉编译）
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

## CI/CD 工作流 (GitHub Actions)

### 工作流文件
`.github/workflows/release.yml` 和 `Cross.toml`

### 触发条件
- Push tags 匹配 `v*` 模式
- 手动触发 (`workflow_dispatch`)

### 构建矩阵

#### 1. Linux x86_64 构建 (`build-linux-x86_64`)
- **Runner**: `ubuntu-latest`
- **工具链**: `dtolnay/rust-toolchain@stable`
- **构建命令**: `cargo build --release`
- **输出文件**: `mc-minder-x86_64-linux`
- **缓存**: 使用 `actions/cache@v3` 缓存 cargo registry 和 target 目录

#### 2. Termux ARM64 构建 (`build-termux-aarch64`) - 最终成功方案!
- **Runner**: `ubuntu-latest`
- **工具链**: `dtolnay/rust-toolchain@stable` + `aarch64-linux-android` target
- **安装 cross**: 使用 `cargo-binstall -y cross` 快速安装预编译版本
- **Cross 配置文件**: `Cross.toml`
  ```toml
  [build.env]
  passthrough = [
      "RUSTFLAGS",
  ]

  [target.aarch64-linux-android]
  image = "ghcr.io/cross-rs/aarch64-linux-android:main"
  ```
- **构建命令**: `cross build --target aarch64-linux-android --release`
- **输出文件**: `mc-minder-termux-aarch64`
- **缓存**: 使用 `actions/cache@v3` 缓存 cargo registry 和 target 目录

#### 3. Release 发布 (`release`)
- **依赖**: `needs: [build-linux-x86_64, build-termux-aarch64]`
- **发布工具**: `softprops/action-gh-release@v1`
- **附件文件**:
  - `binaries/mc-minder-x86_64-linux/mc-minder-x86_64-linux`
  - `binaries/mc-minder-termux-aarch64/mc-minder-termux-aarch64`
  - `install.sh`

### CI/CD 坑点总结 (血泪史!)

#### 问题 1: OpenSSL 交叉编译失败
- **错误信息**: `Could not find openssl via pkg-config`
- **原因**: `reqwest` 默认依赖系统 OpenSSL，交叉编译需要目标平台的 OpenSSL
- **解决方案**: 使用 `rustls-tls` feature 替代默认的 `native-tls`

#### 问题 2: ring crate 需要 libunwind
- **错误信息**: `cannot find -lunwind`
- **原因**: `ring` 是 rustls 的加密后端，包含平台相关汇编代码
- **解决方案**: 升级 reqwest 到 0.12，使用 `aws-lc-rs` 后端（纯 Rust 实现）

#### 问题 3: cross-rs/cross action 不存在
- **错误信息**: `Can't find 'action.yml' for action 'cross-rs/cross@main'`
- **原因**: cross 没有官方 GitHub Action
- **解决方案**: 使用 `cargo-binstall` 安装预编译的 cross 二进制文件

#### 问题 4: 目标架构错误
- **错误目标**: `aarch64-unknown-linux-musl` (musl libc)
- **正确目标**: `aarch64-linux-android` (Bionic libc)
- **原因**: Termux 运行在 Android 上，使用 Bionic libc

#### 问题 5: NDK 手动配置复杂
- **问题**: 使用 `nttld/setup-ndk` + 手动配置链接器容易出错
- **解决方案**: 使用 `cross` 工具自动处理 Docker 环境和 NDK 配置

### 最终成功配置要点
1. **reqwest 0.12** + **rustls-tls** feature（避免原生依赖）
2. **cross** 通过 **cargo-binstall** 安装（快速）
3. **Cross.toml** 使用官方 Docker 镜像（自动处理 NDK）
4. **target**: `aarch64-linux-android`（正确的 Android 目标）

### 版本管理
- 版本号定义在 `Cargo.toml` 的 `[package].version` 字段
- Git tag 格式: `v{version}` (如 `v0.3.2`)
- 发布 Release 时，tag 会触发 CI/CD 工作流

## 开发注意事项

### Windows 开发环境
- Python 命令使用 `py` 而不是 `python`（环境变量问题）
- PowerShell 中使用 `;` 分隔命令，而不是 `&&`

### 代码风格
- 代码中**不**使用 emoji
- 终端输出保持简洁，不要大量使用 `=` 等分隔符
- 回复用户时可以使用 emoji，但代码保持专业

### 测试与验证
- 交叉编译的二进制文件需要在实际 Termux 环境中测试
- GitHub Actions 构建失败时，检查 job 日志获取详细错误信息

## 文件结构

### 核心源代码结构
```
mc-minder/
├── src/
│   ├── main.rs              # 入口点 - 薄层分发器
│   ├── cli.rs               # 命令行参数解析
│   ├── config.rs            # 配置加载和管理
│   ├── banner.rs            # 横幅打印和日志初始化
│   ├── init.rs              # 交互式初始化配置
│   ├── self_update.rs       # 自动更新功能
│   ├── server_run.rs        # 服务器运行主循环
│   │
│   ├── monitor.rs           # 日志监控模块
│   ├── ai.rs                # AI 聊天机器人
│   ├── rcon.rs              # RCON 通信协议
│   ├── context.rs           # 对话上下文管理
│   ├── api.rs               # HTTP API 服务器
│   └── notification.rs      # 通知服务（Telegram 等）
├── scripts/                 # Shell 脚本库
│   ├── start-tui.sh         # TUI 启动脚本
│   ├── backup.sh            # 备份脚本
│   ├── install_oh_my_opencode.sh  # oh-my-opencode 安装脚本
│   └── lib/                 # 脚本库
│       ├── common.sh        # 通用函数
│       ├── config.sh        # 配置读取
│       ├── java.sh          # Java 版本管理
│       ├── server.sh        # 服务器控制
│       ├── log.sh           # 日志查看
│       └── menu.sh          # TUI 菜单
├── .github/
│   └── workflows/
│       └── release.yml      # CI/CD 发布工作流
├── Cargo.toml               # 项目配置和依赖
├── Cross.toml               # Cross 交叉编译配置
├── install.sh               # 一键安装脚本
├── AGENTS.md                # 本文件 - AI 上下文
├── README.md                # 中文说明文档
└── README_en.md             # 英文说明文档
```

### 模块职责

| 模块 | 职责 | 关键功能 |
|------|------|----------|
| `main.rs` | 入口点 | 参数解析、模块分发 |
| `cli.rs` | CLI | clap 参数定义、命令处理 |
| `config.rs` | 配置 | toml 加载、配置验证 |
| `banner.rs` | 横幅/日志 | 打印版本、env_logger 初始化 |
| `init.rs` | 初始化 | 交互式配置生成、脚本生成 |
| `self_update.rs` | 更新 | GitHub Release 检查、二进制替换 |
| `server_run.rs` | 运行 | 事件循环、组件协调 |
| `monitor.rs` | 监控 | notify 文件监控、日志解析 |
| `ai.rs` | AI | OpenAI/Ollama 客户端、限流 |
| `rcon.rs` | RCON | 异步 RCON 协议、自动重连 |
| `context.rs` | 上下文 | 对话历史、过期清理 |
| `api.rs` | API | warp HTTP 服务器、端点路由 |
| `notification.rs` | 通知 | Telegram Bot API |
```

## 常用命令

### 本地构建 (Linux x86_64)
```bash
cargo build --release
```

### 交叉编译 (Termux ARM64)
```bash
cross build --target aarch64-linux-android --release
```

### 创建 Release
```bash
git tag v{version}
git push origin v{version}
```

## 更新日志
- 2026-04-21: **v0.3.14 发布!** 修复编译错误、clippy 警告、代码质量优化
- 2026-04-21: **v0.3.13 发布!** 修复 AI 聊天无响应问题、改进日志解析、增强 debug 日志
- 2026-04-21: **v0.3.12 发布!** 改进配置读取、添加前台运行、修复符号链接权限
- 2026-04-21: **v0.3.11 发布!** 修复函数顺序、grep兼容性、PID写入等问题
- 2026-04-21: **v0.3.10 发布!** 修复二进制检测、配置读取等重大BUG
- 2026-04-21: **v0.3.9 发布!** 修复二进制文件命名和SHA256校验问题
- 2026-04-21: **v0.3.8 发布!** 修复PID文件错误、改进macOS支持
- 2026-04-21: **v0.3.7 发布!** TUI启动脚本、JVM配置、Telegram通知
- 2026-04-21: **v0.3.6 发布!** CI/CD 自动生成 Changelog 功能
- 2026-04-21: **v0.3.5 发布!** 修复所有编译警告，零警告构建
- 2026-04-21: **v0.3.4 发布!** 新增 TUI 启动脚本、JVM 配置支持、Telegram 通知功能
- 2026-04-20: **v0.3.3 发布!** 版本统一、依赖升级、代码优化、单元测试
- 2026-04-20: **CI/CD 终于成功!** 使用 reqwest 0.12 + rustls + cross + Cross.toml 方案
- 2026-04-20: 升级 reqwest 到 0.12 解决 libunwind 链接问题
- 2026-04-20: 添加 Cross.toml 配置 Android 交叉编译环境
- 2026-04-20: 切换到 rustls-tls 避免 OpenSSL 交叉编译问题
- 2024-04-20: 初始添加 Termux ARM64 交叉编译支持

---

## Commit 规范（重要！）

### 必须遵循 Conventional Commits 规范

为了 CI/CD 能自动生成专业的 Changelog，**所有 commit message 必须遵循以下格式**：

```
<type>(<scope>): <subject>

<body (可选)>
```

#### type（必选，必须是小写英文）

| type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(tui): add dialog-based startup script` |
| `fix` | bug 修复 | `fix(rcon): resolve borrow after move error` |
| `docs` | 文档更改 | `docs(readme): update installation instructions` |
| `style` | 代码格式（不影响功能） | `style: format code with cargo fmt` |
| `refactor` | 重构（不是新功能也不是修复） | `refactor(config): simplify config loading logic` |
| `perf` | 性能优化 | `perf(monitor): reduce file I/O operations` |
| `test` | 测试相关 | `test(rcon): add unit tests for packet parsing` |
| `chore` | 构建/工具链/辅助工具 | `chore: update dependencies to latest versions` |
| `ci` | CI/CD 配置更改 | `ci: auto-generate changelog from git commits` |
| `release` | 发布新版本 | `release: v0.3.6 - new features and bug fixes` |

#### scope（可选，建议填写）

表示 commit 影响的模块或文件：

```
常见 scope:
- tui       - TUI 启动脚本相关
- config    - 配置系统相关
- rcon      - RCON 通信相关
- monitor   - 日志监控相关
- ai        - AI 聊天功能相关
- api       - HTTP API 相关
- context   - 对话上下文管理
- ci        - CI/CD 工作流
- release   - 版本发布
```

#### subject（必选）

- 使用中文或英文都可以（建议与项目主要语言一致）
- 首字母小写
- 不以句号结尾
- 使用祈使语气（"添加"而不是"添加了"）

#### 正确示例 ✅

```bash
feat(tui): add dialog-based startup script for better UX
fix(config): resolve borrow after move error in Telegram notification
ci: auto-generate changelog from git commits for each release
release: v0.3.5 - fix remaining dead_code warnings
chore: update .gitignore to exclude AI assistant files
```

#### 错误示例 ❌

```bash
# 缺少 type
add new feature for TUI

# type 大写
Feat(tui): add dialog-based startup script

# subject 以句号结尾
fix(rcon): resolve error.

# 不够具体
fixed some bugs
update code
```

### 为什么需要这个规范？

1. **自动生成 Changelog**: CI/CD 会读取 commit 信息生成 Release 描述
2. **版本追踪**: 可以清楚知道每个版本改了什么
3. **专业形象**: 让项目看起来更专业、更易维护
4. **AI 友好**: 后续 AI 助手可以更好地理解项目历史

### Git 提交流程检查清单

提交前请确认：
- [ ] commit message 符合 Conventional Commits 格式
- [ ] type 正确反映了变更性质
- [ ] scope 准确标识了影响范围
- [ ] subject 简洁明了地描述了变更内容
- [ ] 如果是发布新版本，已更新 Cargo.toml 和 AGENTS.md 的版本号


### 用户个人要求
我是windows10系统，然后python不知道为什么没有上环境变量，所以你要执行python命令的时候不要使用 "python xxx.py" 要使用 "py xxx.py"

回复保持幽默，回复带emoji。但是生成的代码要保持专业，比如，终端打印的内容不要一大堆"="

但是，在实际项目编写中，代码里尽量不出现过多或不出现emoji

说中文
 
### oh-my-opencode 安装脚本
- 项目内新增一个简化安装的脚本：`scripts/install_oh_my_opencode.sh`。
- 作用：按文档要求，自动拼装安装命令并执行，支持非交互模式和简单参数切换。
- 使用方式（示例）：
  - 运行前请确保已安装 Bun（bunx）并可执行 `bunx`。
  - 运行脚本以开始安装：
    ```bash
    bash scripts/install_oh_my_opencode.sh --no-tui --claude=yes --openai=yes --gemini=no --copilot=no --opencode-zen=yes
    ```
- 安装完成后建议执行健康检查：`bunx oh-my-opencode doctor`。
- 如需更多自定义参数，请查看脚本内帮助信息。 
