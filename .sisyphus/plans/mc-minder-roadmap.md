# MC-Minder 开发路线图

> 本文档由蓝图拆解而来，每个任务独立可交付。
> 标记：✅ 已完成 | 🔧 进行中 | ⚠️ 部分 | ❌ 未做 | 💤 远期

---

## 🟢 Phase 0 — 核心基础 (已完成)

### ✅ P0-1: 彻底移除 LLM/AI 功能
- 删除 `src/ai/` 和 `src/context/` 模块
- 移除 `server_run.rs` 中 AI 处理逻辑
- 清理 API `/history` 端点
- 更新 `config.rs` 移除 `AiConfig` / `OllamaConfig`
- 版本: v0.5.0

### ✅ P0-2: Java 管理增强
- `init.rs`: 自动检测 Java 安装，平台专属安装指引
- TUI Java 菜单: 版本检测、一键安装、交互式切换
- `switch_java_version`: 多版本列表，选中后写 `config.toml`
- 版本: v0.5.5

### ✅ P0-3: 语言选择
- 首次启动 TUI 时英文提示选择语言
- `load_language()` / `save_language()` 持久化到 `lang.conf` + `config.toml`
- 版本: v0.5.4

### ✅ P0-4: mc-status-probe 原创库
- 独立 Rust 库，纯 wiki.vg 协议实现 MC Ping
- VarInt 编解码、Handshake、Status Request/Response
- MOTD ChatComponent JSON 提取
- 12/12 单元+集成测试
- 发布到 crates.io: `mc-status-probe = "0.1.0-alpha.4"`
- 单独仓库: https://github.com/SharkMI-0x7E/mc-status-probe
- 版本: msp v0.1.0-alpha.4

### ✅ P0-5: MC 状态集成
- `config.rs`: `[mc_status]` 配置段 + `discover_minecraft_port()`
- `api/mod.rs`: `/status` 返回 MC 状态 (online/players/version/latency/motd)
- `/command` 端点 RCON 可用性预检
- 后台 `tokio::spawn` 周期性 ping，`PingGuard` 防死锁
- TUI `draw_status_view` 显示 MC 状态，在线绿色边框，离线红色边框
- 版本: v0.6.0-alpha.2

### ✅ P0-6: 多平台 CI/CD
- Linux x86_64 + ARM64 (aarch64-unknown-linux-gnu)
- Termux/Android ARM64 (aarch64-linux-android)
- Windows x86_64 (x86_64-pc-windows-gnu)
- install.sh 多平台检测
- 版本: v0.5.2 + v0.6.0-alpha.1

### ✅ P0-7: Windows TUI 按键修复
- 过滤 `KeyEventKind::Release` 避免双事件
- 小键盘 2/8 映射为 Down/Up
- 动态菜单长度 `items().len()`
- 版本: v0.6.0-alpha.1.1

---

## 🟡 Phase 1 — TUI 交互体验升级

### ✅ P1-1: 状态栏定时刷新
- MC 状态每 200ms 从共享缓存自动刷新
- F5 手动刷新快捷键 + `refresh_status()` 方法

### ✅ P1-2: 弹窗式交互系统
- `ConfirmAction::Modal` 变体 + `show_modal()` 通用弹窗
- 支持自定义标题和消息
- Esc 关闭

### ✅ P1-3: 全局快捷键
- `F5` 刷新状态, `F7` 备份提示, `c` 控制台
- Busy 状态下阻断其他输入
- 状态栏快捷键提示

### ✅ P1-4: 分栏布局
- 主菜单左右分栏：菜单(左) + 状态面板(右)
- MC 状态 (在线/版本/玩家/延迟/MOTD/TPS) + 进程状态
- 控制台视图独立全屏

### ✅ P1-5: 操作反馈与进度条
- `AppState::Busy(String)` + `set_busy()`/`clear_busy()`
- 居中 spinner 遮罩，防止重复操作

---

## 🟠 Phase 2 — 多服务器管理

### ✅ P2-1: 服务器自动发现
- `discover_servers()` 扫描子目录检测 jar + server.properties
- `DiscoveredServer` 结构体
- "拖入即用"体验保留

### ✅ P2-2: 多服务器数据结构
- `ServerInstance` + `[[servers]]` 数组
- `Config::get_servers()` 统一访问
- 向后兼容单服务器配置

### ✅ P2-3: TUI 服务器列表
- 右侧面板显示已发现的服务器列表
- 选中指示器 + 名称显示

### ⚠️ P2-4: 独立配置文件管理
- TUI 内编辑各实例的基本信息（名称/目录）
- `ServerConfigEdit` 状态已添加
- 尚未: 完整的 server.properties 内联编辑

### ❌ P2-5: 实例隔离与并发
- 每个服务器在独立 tokio task 中管理
- 崩溃不影响其他实例

### ...
### ✅ P3-6: 拖入即用保留
- 已保留手动放入 jar 的识别能力
- `create_eula()` 自动创建 eula.txt

### ...
### ✅ P5-3: 备份列表与恢复
- TUI "19. Backup List" 显示备份文件/大小
- `list_backups()` + `restore_backup()` 已实现

### ...
### ✅ P6-3: 告警机制
- `alert` 字段已添加到 McStatusSnapshot
- TUI 状态面板边框颜色跟随告警状态 (ok=green, warning=yellow, critical=red, offline=red)

### ⚠️ P6-4: 玩家行为日志
- 现有 monitor 模块已记录 join/leave/death 事件到日志
- 尚未: TUI 查询面板

### ...
### ✅ P7-2: 崩溃报告查看器
- `scan_crash_reports()` 扫描 crash-reports 目录
- 按文件名排序 (最新在前)

### ✅ P7-3: 游戏内公告
- `announce_countdown()` 方法已添加
- Scheduler broadcast 动作支持定时公告

---

## 🔵 Phase 4 — 自动化与进程管理

### ✅ P4-1: 看门狗增强
- RCON `list` 健康检查，失败计数
- 冷却后自动重启，可配置最大重试次数

### ✅ P4-2: 懒启动 (Lazy Start)
- `lazy_start.rs` TCP 监听器
- 收到连接 → `ForegroundProcess::spawn` 唤醒
- RCON idle 检测 → 无人自动停服

### ✅ P4-3: 计划任务调度器
- `[[schedules]]` 配置: broadcast / restart / backup / command
- 间隔分钟，shutdown 信号优雅退出

---

## 🔵 Phase 5 — 备份管理

### ✅ P5-1: 自动化备份策略
- `backup.rs`: `create_backup()` 复制世界目录
- Scheduler `"backup"` 动作集成

### ✅ P5-2: 备份保留与清理
- `BackupConfig`: max_backups + max_backup_days
- `apply_retention()` 自动清理

### ⚠️ P5-3: 备份列表与恢复
- `list_backups()` + `restore_backup()` 已实现
- TUI 备份列表面板未完成

---

## 🟢 Phase 6 — 高级状态监控

### ✅ P6-1: 实时性能指标 (TPS)
- RCON `tps` 查询 (Paper/Purpur)
- McStatusSnapshot 增加 `tps` + `alert` 字段
- 状态面板颜色: 绿色 >18, 黄色 10-18, 红色 <10

### ❌ P6-2: 历史趋势图
- 保留最近 N 分钟 TPS 数据
- ASCII 柱状图/折线图

### ⚠️ P6-3: 告警机制
- `alert` 字段已添加 (ok/warning/critical/offline)
- TUI 边框颜色跟随告警状态
- 尚未: 桌面通知 / Discord Webhook

### ❌ P6-4: 玩家行为日志
- 进出时间戳/死亡/成就记录
- TUI 查询面板

---

## 🟣 Phase 7 — 实用工具箱

### ✅ P7-1: 快捷指令面板
- TUI "18. Quick Commands"
- 预设: 和平/白天/晴天/死亡不掉落/查看状态

### ❌ P7-2: 崩溃报告查看器
- 扫描 `crash-reports/` 文件夹
- 按时间排序，高亮关键堆栈

### ❌ P7-3: 游戏内公告
- TUI 直接广播消息
- 动态倒计时

---

## 🌐 Phase 8 — 架构扩展 (远期)

### 💤 P8-1: Web 管理面板
### 💤 P8-2: 插件系统
### 💤 P8-3: 移动端优化

---

## 📊 进度总览

| Phase | 任务 | ✅ | ⚠️ | ❌ | 完成度 |
|-------|------|----|----|-----|--------|
| P0 核心 | 7 | 7 | 0 | 0 | 100% |
| P1 TUI | 5 | 5 | 0 | 0 | 100% |
| P2 多服 | 5 | 3 | 1 | 1 | 70% |
| P3 核心 | 6 | 5 | 0 | 1 | 83% |
| P4 自动 | 3 | 3 | 0 | 0 | 100% |
| P5 备份 | 3 | 3 | 0 | 0 | 100% |
| P6 监控 | 4 | 2 | 1 | 1 | 63% |
| P7 工具 | 3 | 3 | 0 | 0 | 100% |
| P8 架构 | 3 | 0 | 0 | 3 | 0% 💤 |

总计: 39 任务，26 完成 (67%)

---

## 🔗 API 速查

| 平台/功能 | API 端点 |
|-----------|---------|
| FabricMC 服务端 | `meta.fabricmc.net/v2/versions/loader/{ver}/{loader}/1.1.0/server/jar` |
| PaperMC 版本 | `api.papermc.io/v2/projects/paper` |
| Modrinth 搜索 | `api.modrinth.com/v2/search?query={q}` |
| wiki.vg 协议版本 | `wiki.vg/Protocol_version_numbers` |

---

## 原计划 (蓝图)

### 核心原则
- 保留并强化现有优势：拖入即用、极轻量化、多平台
- 增量迭代，不推倒重来
- 原创性第一：不复制任何第三方代码
- 降低用户门槛：保持拖入即用体验

### 功能蓝图
1. **多服务器管理**：`[[servers]]` 数组、自动发现、TUI 侧边栏、独立配置编辑
2. **TUI 体验**：实时刷新、弹窗交互、全局快捷键、分栏布局、进度遮罩
3. **自动化**：看门狗假死检测、懒启动、计划任务
4. **备份**：多触发方式、保留策略、TUI 恢复
5. **监控**：TPS/延迟/告警/玩家日志
6. **核心/Mod**：多核心下载、Modrinth 集成、自动配置
7. **工具箱**：快捷指令、崩溃查看器、公告
8. **架构**：隔离并发、Web 面板、移动端优化
