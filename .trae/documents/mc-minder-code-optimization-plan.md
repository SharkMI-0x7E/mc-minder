# MC-Minder 全面优化计划

基于 DeepSeek 模型的详细建议，重新制定优化计划。

---

## 一、性能优化（必须完成）

### 1. RCON 异步化
**文件**: `src/rcon/mod.rs`

**改动内容**:
- 将 `std::net::TcpStream` 替换为 `tokio::net::TcpStream`
- 将 `std::io::{Read, Write}` 替换为 `tokio::io::{AsyncReadExt, AsyncWriteExt}`
- 所有方法改为 `async fn`
- 添加自动重连机制

**依赖变更**: 无需新增，tokio 已包含异步网络支持

---

### 2. 日志监控优化
**文件**: `src/monitor/mod.rs`

**改动内容**:
- 引入 `notify` 库监听文件变化
- 移除 500ms 轮询，改用事件驱动
- 处理日志文件轮转情况
- 使用 `tokio::sync::mpsc` 传递事件

**依赖变更**: 添加 `notify = "6.1"`

---

### 3. AI 请求限流
**文件**: `src/ai/mod.rs`

**改动内容**:
- 添加 `tokio::sync::Semaphore` 限制并发数为 3
- 同一玩家请求间隔至少 2 秒
- 限流时返回友好提示

**依赖变更**: 无需新增

---

## 二、人与代码交互优化（必须完成）

### 4. 一键安装/初始化
**新增文件**: `install.sh`（仓库根目录）

**改动内容**:
- 添加 `init` 子命令（clap Subcommand）
- 交互式配置生成
- 自动检测 `server.properties`
- 嵌入并输出 `start.sh`、`backup.sh` 模板

**依赖变更**: 无需新增

---

### 5. 统一配置
**文件**: `src/config/mod.rs`, `scripts/start.sh`

**改动内容**:
- 扩展 `config.toml`，添加 `[server]` 段
- `start.sh` 从 `config.toml` 读取配置
- 或通过 `mc-minder config get` 获取值

---

### 6. 环境依赖检测
**文件**: `scripts/start.sh`

**改动内容**:
- 检测 `java` 和 `tmux`
- Termux 环境自动安装
- 非 Termux 给出安装提示

---

### 7. 错误信息友好化
**文件**: `src/main.rs`, `src/rcon/mod.rs`, `src/ai/mod.rs`, `src/monitor/mod.rs`

**改动内容**:
- RCON 失败：提示检查 `server.properties`
- AI 失败：提示检查 `api_key`
- 日志不存在：提示先启动服务器
- 添加颜色输出

---

### 8. 日志管理增强
**文件**: `src/main.rs`

**改动内容**:
- 同时输出到 stderr 和 `logs/mc-minder.log`
- 日志轮转：超过 50MB 自动归档
- 可选：`backup.sh` 备份日志

---

### 9. 更新机制
**文件**: `src/main.rs`

**改动内容**:
- 添加 `self-update` 子命令
- 从 GitHub Releases 下载最新版本
- `start.sh` 可选检查更新

---

## 三、文件修改清单

| 序号 | 文件 | 操作 | 改动量 |
|------|------|------|--------|
| 1 | `Cargo.toml` | 修改 | 小 |
| 2 | `src/main.rs` | 修改 | 大 |
| 3 | `src/lib.rs` | 修改 | 小 |
| 4 | `src/config/mod.rs` | 修改 | 中 |
| 5 | `src/rcon/mod.rs` | 重写 | 大 |
| 6 | `src/monitor/mod.rs` | 重写 | 大 |
| 7 | `src/ai/mod.rs` | 修改 | 中 |
| 8 | `src/api/mod.rs` | 修改 | 小 |
| 9 | `scripts/start.sh` | 重写 | 中 |
| 10 | `scripts/backup.sh` | 修改 | 小 |
| 11 | `install.sh` | 新增 | 中 |
| 12 | `README.md` | 更新 | 中 |

---

## 四、实施顺序

### 阶段一：核心性能优化
1. RCON 异步化 + 自动重连
2. 日志监控改用 notify
3. AI 请求限流

### 阶段二：配置与初始化
4. 扩展 config.toml 结构
5. 实现 init 子命令
6. 统一配置源

### 阶段三：用户体验
7. 环境依赖检测
8. 错误信息友好化
9. 日志管理增强
10. 更新机制

### 阶段四：部署与文档
11. 编写 install.sh
12. 更新 README.md
13. 更新 backup.sh

---

## 五、依赖变更汇总

```toml
# 新增依赖
notify = "6.1"           # 文件监控
colored = "2.1"          # 彩色输出（可选）
indicatif = "0.17"       # 进度条（可选，用于 init）

# 更新依赖特性
tokio = { version = "1.35", features = ["full", "signal"] }  # 添加 signal 特性
```

---

## 六、向后兼容性

- 现有 `config.toml` 可继续使用，新字段使用默认值
- 现有 `start.sh` 可继续使用，但建议迁移
- 提供 `mc-minder migrate` 命令自动迁移旧配置

---

## 七、测试计划

1. **RCON 测试**: 连接断开后自动重连
2. **日志监控测试**: 模拟日志轮转，验证不丢事件
3. **AI 限流测试**: 多玩家同时发送命令
4. **init 测试**: 全新环境运行 `mc-minder init`
5. **更新测试**: 模拟 `self-update` 流程
