# AGENTS.md

> 项目上下文文件 - 供 AI 助手参考的开发指南

## 项目概览

**项目名称**: mc-minder  
**描述**: 为 Termux/Android 上的 Minecraft Fabric 服务器设计的智能管理套件  
**语言**: Rust (Edition 2021)  
**版本**: 0.6.0-alpha.1 <!-- ⚠️ 每次发布新版本时请同步更新此处版本号 -->
**仓库**: https://github.com/SharkMI-0x7E/mc-minder

## 核心功能

- 日志监控：使用 notify 库实时监控服务器日志，事件驱动，低 CPU 占用
- 服务器状态查询：基于 wiki.vg 协议的 MC Ping，实时显示玩家数/版本/延迟
- RCON 通信：持久连接池，自动重连，响应返回
- HTTP API：RESTful API (/status + /command)，MC 状态集成
- Java 管理：自动检测、一键安装、交互式切换
- TUI 管理界面：原生 Rust 终端 UI
- 一键安装：交互式初始化配置
- 自动更新：内置 self-update 命令
- 多平台：Linux x64/ARM64、Termux/Android、Windows x64

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

#### 3. Linux ARM64 构建 (`build-linux-aarch64`)
- **Runner**: `ubuntu-latest`
- **工具链**: `dtolnay/rust-toolchain@stable` + `aarch64-unknown-linux-gnu` target
- **安装 cross**: 使用 `cargo-binstall -y cross` 快速安装预编译版本
- **Cross 配置文件**: `Cross.toml`
  ```toml
  [target.aarch64-unknown-linux-gnu]
  image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main"
  ```
- **构建命令**: `cross build --target aarch64-unknown-linux-gnu --release`
- **输出文件**: `mc-minder-aarch64-linux`
- **缓存**: 使用 `actions/cache@v3` 缓存 cargo registry 和 target 目录

#### 4. Windows x86_64 构建 (`build-windows-x86_64`)
- **Runner**: `ubuntu-latest`
- **工具链**: `dtolnay/rust-toolchain@stable` + `x86_64-pc-windows-gnu` target
- **安装 cross**: 使用 `cargo-binstall -y cross` 快速安装预编译版本
- **构建命令**: `cross build --target x86_64-pc-windows-gnu --release`
- **输出文件**: `mc-minder-x86_64-windows.exe`
- **缓存**: 使用 `actions/cache@v3` 缓存 cargo registry 和 target 目录

#### 5. Release 发布 (`release`)
- **依赖**: `needs: [build-linux-x86_64, build-termux-aarch64, build-linux-aarch64, build-windows-x86_64]`
- **发布工具**: `softprops/action-gh-release@v1`
- **附件文件**:
  - `binaries/mc-minder-x86_64-linux/mc-minder-x86_64-linux`
  - `binaries/mc-minder-termux-aarch64/mc-minder-termux-aarch64`
  - `binaries/mc-minder-aarch64-linux/mc-minder-aarch64-linux`
  - `binaries/mc-minder-x86_64-windows/mc-minder-x86_64-windows.exe`
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
│   ├── command_sender.rs     # 命令发送（RCON/Stdin连接池）
│   ├── foreground_process.rs # 前台进程管理（Java进程stdio管道）
│   │
│   ├── mc-status-probe/     # MC 服务器状态查询库（wiki.vg 协议实现）
│   │
│   ├── tui/                 # TUI 模块（替代 Shell 脚本）
│   │   ├── mod.rs           # TUI 入口 + 终端管理 + 前台进程spawn
│   │   └── app.rs           # 应用状态 + UI 渲染 + 事件处理
│   │
│   ├── monitor/             # 日志监控 + 聊天捕获模块
│   │   └── mod.rs           # LogMonitor + ChatCapture trait + 实现
│   ├── ai/                  # AI 聊天机器人
│   │   └── mod.rs           # OpenAI/Ollama 客户端、限流
│   ├── rcon.rs              # RCON 通信协议
│   ├── context.rs           # 对话上下文管理
│   ├── api/                 # HTTP API 服务器
│   │   └── mod.rs           # warp HTTP 服务器、端点路由
│   ├── notification.rs      # 通知服务（Telegram 等）
│   └── update_engine.rs     # 更新引擎（异步下载+安装）
├── scripts/                 # Shell 脚本
│   ├── start-tui.sh         # TUI 启动脚本（极简启动器）
│   └── backup.sh            # 备份脚本
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
| `server_run.rs` | 运行 | 事件循环、AI聊天处理、组件协调 |
| `command_sender.rs` | 命令发送 | PooledRconSender(持久连接+自动重连)、StdinCommandSender、MultiCommandSender |
| `foreground_process.rs` | 前台进程 | Java进程spawn、stdin/stdout/stderr管道、chat解析 |
| `tui/` | TUI | 原生终端UI + RunningForeground状态(进程内前台运行) |
| `monitor/` | 监控 | LogMonitor(notify文件监控)、ChatCapture trait、Tmux/File/Process实现 |
| `ai/` | AI | OpenAI/Ollama 双后端、请求限流 |
| `rcon.rs` | RCON | 异步 RCON 协议、底层包处理 |
| `context.rs` | 上下文 | 按玩家对话历史、过期清理 |
| `api/` | API | warp HTTP 服务器、命令执行(返回RCON响应) |
| `notification.rs` | 通知 | Telegram Bot API |
```

### 脚本部署方式

脚本支持多种部署方式：

**方式 1：自动部署（推荐）**
```bash
# 通过安装脚本自动部署
curl -fsSL https://github.com/SharkMI-0x7E/mc-minder/releases/latest/download/install.sh | bash

# 安装后目录结构
mc-server/
├── mc-minder              # Rust 二进制（包含 TUI）
├── start-tui.sh           # TUI 启动脚本（极简启动器）
├── backup.sh              # 备份脚本
└── ...
```

**方式 2：手动部署**
```bash
# 只需要 mc-minder 二进制
mc-server/
├── mc-minder              # Rust 二进制
├── start-tui.sh           # 可选：TUI 启动脚本
└── ...
```

**方式 3：直接运行**
```bash
# 无需脚本，直接运行
./mc-minder tui
```

**方式 4：环境变量配置**
```bash
# 设置 mc-minder 二进制位置
export MC_MINDER_BIN=/usr/local/bin/mc-minder
```

TUI 启动脚本会自动检测 mc-minder 二进制位置：
1. MC_MINDER_BIN 环境变量
2. 脚本所在目录
3. 当前目录
4. PATH 中查找

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

## Release 内容规范（重要！）

为了确保用户能快速了解每次更新的内容，**Release 页面必须遵循以下格式**：

### 自动生成规则

CI/CD 会根据 commit message 自动生成 Release 内容，**commit message 必须遵循以下格式**：

```
<type>(<scope>): <subject>

<body (可选)>
```

### 分类和类型

Release 页面会自动按以下分类显示更新内容：

| type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(scripts): add smart path detection` |
| `fix` | 问题修复 | `fix(rcon): resolve connection timeout` |
| `perf` | 性能优化 | `perf(monitor): reduce file I/O operations` |
| `refactor` | 代码重构 | `refactor(config): simplify loading logic` |
| `docs` | 文档更新 | `docs(readme): update installation guide` |
| `test` | 测试相关 | `test(rcon): add unit tests` |
| `chore` | 其他改动 | `chore: update dependencies` |

### 最终 Release 页面示例

```markdown
## MC-Minder v0.5.2

### 新功能
- Add intelligent path detection in start-tui.sh
- Enhance binary detection in common.sh

### 问题修复
- Fix path resolution issues in different deployment scenarios

### 性能优化
- Improve script loading with cached path detection

### 文档更新
- Add comprehensive deployment guide
- Document 4 deployment methods

### 其他改动
- Update dependencies to latest versions

---

### 下载
| 平台 | 文件 | 说明 |
|------|------|------|
| Linux x64 | `mc-minder-x86_64-linux` | 适用于桌面 Linux / WSL |
| Linux ARM64 | `mc-minder-aarch64-linux` | 适用于树莓派 / ARM 服务器 |
| Termux/Android ARM64 | `mc-minder-termux-aarch64` | 适用于手机 Termux 环境 |

### 安装
```bash
curl -fsSL https://raw.githubusercontent.com/SharkMI-0x7E/mc-minder/main/install.sh | bash
```
```

### 为什么需要这个规范？

1. **用户体验**：用户打开 Release 页面就能立即看到更新了什么
2. **清晰分类**：按功能、修复、性能等分类，一目了然
3. **自动归档**：每次发布都自动生成结构化的更新日志
4. **专业形象**：让项目看起来更专业、更易维护

### Commit 提交检查清单

提交前请确认：
- [ ] commit message 符合 Conventional Commits 格式
- [ ] type 正确反映了变更性质（feat/fix/docs/refactor/perf/test/chore）
- [ ] scope 准确标识了影响范围
- [ ] subject 简洁明了地描述了变更内容
- [ ] 不要写太长的 commit message，保持在 50 字符以内

## 更新日志
- 2026-04-27: **v0.6.0-alpha.1 发布!** 新增 mc-status-probe 原创库（wiki.vg MC Ping 协议实现）、HTTP API 增强（/status 含 MC 状态、/command RCON 预检）、Windows x86_64 CI/CD 支持、discover_minecraft_port 自动端口发现
- 2026-04-26: **v0.5.6 发布!** 恢复前台模式为终端 exec
- 2026-04-26: **v0.5.5 发布!** Java 交互式版本切换
- 2026-04-26: **v0.5.2 发布!** 添加 Linux ARM64 交叉编译支持（aarch64-unknown-linux-gnu），修复 install.sh 的各种问题（ARM64 检测、脚本库下载、dialog 过期警告）
- 2026-04-26: **v0.5.1 发布!** 消除所有 dead_code 警告，删除未使用的 rcon 模块，更新 README
- 2026-04-26: **v0.5.0 发布!** 彻底移除 LLM/AI 功能，完善初始化配置（Java 自动检测、JDK 路径配置），增强 TUI Java 菜单（版本检测、安装指引）
- 2026-04-25: **v0.4.9 发布!** AI 聊天机器人重大重写：ChatCapture trait 统一捕获、PooledRconSender 持久连接、ForegroundProcess TUI 内前台运行、[Not Secure] regex 修复、[AI] 日志前缀标准化
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
- cmd-sender - 命令发送相关
- fg-proc   - 前台进程相关
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

## 外部依赖

### mc-status-probe

MC 服务器状态查询功能依赖 `mc-status-probe` 库（`https://crates.io/crates/mc-status-probe`）。
该库已在 crates.io 发布，**不需要**将其源代码放在 mc-minder 项目目录中。

**Cargo.toml 配置**：
```toml
mc-status-probe = "0.1.0-alpha.3"
```

如果本地开发需要同时修改 msp，可以临时改为 path 依赖：
```toml
mc-status-probe = { version = "0.1.0-alpha.2", path = "mc-status-probe" }
```

msp 独立仓库：`https://github.com/SharkMI-0x7E/mc-status-probe`

---

## 测试指南

### 本地开发测试

```bash
# 编译检查（不生成二进制，速度快）
cargo check

# 运行所有单元测试
cargo test --lib

# 运行指定模块的测试
cargo test --lib monitor

# 运行所有测试（含集成测试）
cargo test

# 编译并运行 TUI（功能测试）
cargo run -- tui

# 编译 release 版本
cargo build --release
```

### 测试清单（提交前必做）

- [ ] `cargo check` 零错误零警告
- [ ] `cargo test --lib` 全部通过
- [ ] `cargo build --release` 成功
- [ ] 如果改了 TUI：`cargo run -- tui` 启动确认界面正常

### 关于 release

**不要手动 `cargo publish`**。发布由 CI/CD 在打 tag 时自动完成。
只需推送 tag：`git tag v0.x.x && git push origin v0.x.x`

### Release Changelog 生成规则

CI/CD 根据 commit message 自动生成 Release 描述。为了让每个改动都清晰可见：

1. **一个 commit 只做一件事** — 不要把多个不相关的功能写在一个 commit 里
2. **subject 一行写完** — 核心描述写在一行内，不要换行
3. **body 写详细说明** — 需要详细解释的内容写在 subject 后面的空行里

正例 ✅：
```
feat(tui): add interactive Java version picker

- Add JavaSwitch state with list navigation
- Enter to confirm selection, updates config.toml
- Esc to cancel and return to menu
```

反例 ❌：
```
feat: add Java switch, fix foreground mode, update README, bump version
```

---

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

---

## Windows 开发注意事项

### Python 命令
- Python 命令使用 `py` 而不是 `python`（环境变量问题）
- PowerShell 中使用 `;` 分隔命令，而不是 `&&`

### Windows TUI 按键处理（重要！）

Windows 终端（cmd / PowerShell / Windows Terminal）与 Linux 终端在按键事件上有以下区别：

1. **Press/Release 双事件**：Windows 终端每次按键会发送 `KeyEventKind::Press` 和 `KeyEventKind::Release` 两个事件。代码中已在 `tui/mod.rs` 过滤掉 Release 事件，避免状态被重复处理。

2. **小键盘按键**：当 NumLock 开启时，小键盘方向键发送 `KeyCode::Char('8')`（上）、`Char('2')`（下）、`Char('4')`（左）、`Char('6')`（右），而非 `KeyCode::Up/Down/Left/Right`。代码中已在 `app.rs` 的 `normalize_key()` 中做了映射。

3. **新增按键处理时**：必须在 `normalize_key()` 中处理 Windows 特有的按键映射。

### Windows 后台启动不可用

`start_server_background()` 依赖 `tmux` 和 `nohup`，这些是 Unix 专有命令。在 Windows 上后台启动会静默失败。
未来可考虑使用 Windows 的 `start /B` 或 `CreateProcess` 作为替代方案。

### 代码风格
- 代码中**不**使用 emoji
- 终端输出保持简洁，不要大量使用 `=` 等分隔符
- 回复用户时可以使用 emoji，但代码保持专业

### mc-status-probe (msp) 依赖管理

msp 是独立的 Rust 库，有独立仓库 `https://github.com/SharkMI-0x7E/mc-status-probe` 和 crates.io 版本。

**关键规则**：
- msp 更新版本后，mc-minder 的 `Cargo.toml` 中 `mc-status-probe` 版本号**必须同步更新**
- msp 的源代码**不需要**放在 mc-minder 仓库中（已在 .gitignore 排除）
- 本地开发如需同时修改 msp，使用双源依赖：`mc-status-probe = { version = "0.1.0-alpha.3", path = "mc-status-probe" }`
- 发布 mc-minder 前，确保 msp 已发布到 crates.io
