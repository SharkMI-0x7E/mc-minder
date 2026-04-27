# MC-Minder 开发路线图

> 本文档由蓝图拆解而来，每个任务独立可交付。
> 标记：✅ 已完成 | 🔧 进行中 | 📋 待开始 | 💤 远期

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
- 版本: msp v0.1.0-alpha.4

### ✅ P0-5: MC 状态集成
- `config.rs`: `[mc_status]` 配置段 + `discover_minecraft_port()`
- `api/mod.rs`: `/status` 返回 MC 状态 (online/players/version/latency/motd)
- `/command` 端点 RCON 可用性预检
- 后台 `tokio::spawn` 周期性 ping，`PingGuard` 防死锁
- TUI `draw_status_view` 显示 MC 状态
- 版本: v0.6.0-alpha.2

### ✅ P0-6: 多平台 CI/CD
- Linux x86_64 + ARM64 (aarch64-unknown-linux-gnu)
- Termux/Android ARM64 (aarch64-linux-android)
- Windows x86_64 (x86_64-pc-windows-gnu)
- 版本: v0.5.2 + v0.6.0-alpha.1

### ✅ P0-7: Windows TUI 按键修复
- 过滤 `KeyEventKind::Release` 避免双事件
- 小键盘 2/8 映射为 Down/Up
- 动态菜单长度 `items().len()`
- 版本: v0.6.0-alpha.1.1

---

## 🟡 Phase 1 — TUI 交互体验升级

### 📋 P1-1: 状态栏定时刷新
- 后台定时器驱动界面刷新（当前是按键驱动）
- 每 2 秒自动刷新状态面板的 MC 状态、进程状态
- 不阻塞用户输入
- 依赖: P0-5

### 📋 P1-2: 弹窗式交互系统
- 封装 `ModalDialog` 通用组件
- 支持: 确认对话框 / 输入框 / 选择列表
- 所有辅助操作用弹窗替代当前的消息提示
- 携带标题、操作按钮、`Esc` 关闭

### 📋 P1-3: 全局快捷键
- 绑定常用功能: `F5` 刷新, `F7` 备份, `r` 重启, `c` 控制台
- 状态栏显示当前可用快捷键
- `Ctrl+C` 保持为退出

### 📋 P1-4: 分栏布局
- 左右分栏: 服务器列表(左) + 详情/操作(右)
- 顶部标题栏或标签页
- 控制台视图独立全屏
- 依赖: P1-1

### 📋 P1-5: 操作反馈与进度条
- 耗时操作 (启动/备份/更新) 显示进度遮罩
- 防止重复操作
- 完成自动刷新界面

---

## 🟠 Phase 2 — 多服务器管理

### 📋 P2-1: 服务器自动发现
- 扫描根目录子文件夹，识别 `fabric-server.jar` / `server.properties`
- 自动构建服务器实例列表
- "拖入即用"体验保留

### 📋 P2-2: 多服务器数据结构
- `config.toml` 支持 `[[servers]]` 数组
- 每个实例独立配置 (内存/端口/RCON/JDK)
- 向后兼容单服务器配置

### 📋 P2-3: TUI 服务器列表侧边栏
- 左侧可滚动列表，上下键切换选中
- 显示每个服务器名 + 在线状态小图标
- 操作按钮跟随当前选中服务器

### 📋 P2-4: 独立配置文件管理
- TUI 内编辑各实例的 `server.properties`
- 支持保存，部分配置需重启生效
- 一键复制配置模板到新实例

### 📋 P2-5: 实例隔离与并发
- 每个服务器在独立 tokio task 中管理
- 崩溃不影响其他实例
- 并发启动/停止/重启

---

## 🟠 Phase 3 — 核心与 Mod 管理

### 📋 P3-1: FabricMC 核心自动下载
- API: `https://meta.fabricmc.net/v2/versions/loader/{ver}/0.17.2/1.1.0/server/jar`
- TUI "新建服务器"向导选择 MC 版本 + Loader 版本
- 自动下载 + 安装 + 生成启动配置
- 来源: FabricMC Meta API v2

### 📋 P3-2: PaperMC 核心自动下载
- API: `https://api.papermc.io/v2/projects/paper/versions/{ver}/builds/{build}/downloads/{name}`
- 自动获取最新 build
- 来源: PaperMC API v2

### 📋 P3-3: 多核心类型选择
- 向导支持: Fabric / Paper / Purpur / Vanilla / Forge
- 每种核心有自己的 API 端点
- 显示版本列表供选择

### 📋 P3-4: Mod 平台集成 (Modrinth)
- API: `https://api.modrinth.com/v2/search?query={keyword}`
- 搜索 Mod/插件/资源包
- 显示搜索结果列表: 名称/下载量/兼容版本
- 一键下载到 `mods/` 文件夹
- 来源: Modrinth API v2

### 📋 P3-5: Mod 更新检测
- API: `GET /v2/project/{id}/version` 获取最新版本
- 列出已安装 Mod，对比在线版本
- 高亮可更新的 Mod
- 批量更新

### 📋 P3-6: 拖入即用保留
- 手动放入的 jar 文件自动识别
- 向导式安装可选，不强制

---

## 🔵 Phase 4 — 自动化与进程管理

### 📋 P4-1: 看门狗增强
- "假死"检测: 长时间 TPS=0 自动重启
- 自定义重试策略: 最大次数/冷却时间
- 崩溃日志自动收集

### 📋 P4-2: 懒启动 (Lazy Start)
- 配置项启用轻量 TCP 监听器
- 收到连接请求 → 唤醒服务器
- 启动期间客户端收到等待提示
- 无玩家在线 N 分钟后自动停服
- 省资源模式

### 📋 P4-3: 计划任务调度器
- 配置 `[[schedules]]` 数组
- 支持: 定时备份 / 广播消息 / 重启 / 执行命令
- Cron 式时间表达式
- TUI 内可视化编辑

---

## 🔵 Phase 5 — 备份管理

### 📋 P5-1: 自动化备份策略
- 触发方式: 定时 / 关服前 / 玩家全退 / 手动
- 调用 `save-all` 确保数据一致性
- 压缩归档

### 📋 P5-2: 备份保留与清理
- 最大备份数 / 最大天数
- 自动清理旧备份
- 保留策略可在 TUI 中配置

### 📋 P5-3: 备份列表与恢复
- TUI 备份列表: 文件名/时间/大小
- 选中一键恢复，弹出确认
- 恢复后自动重启

---

## 🟢 Phase 6 — 高级状态监控

### 📋 P6-1: 实时性能指标 (TPS)
- 通过 RCON 执行 `/tps` 或 `/mspt`
- 在状态面板显示 TPS/内存/CPU
- 颜色编码: 绿色>18, 黄色>10, 红色<10

### 📋 P6-2: 历史趋势图
- 保留最近 N 分钟 TPS 数据
- TUI 中 ASCII 柱状图/折线图
- 帮助诊断性能瓶颈

### 📋 P6-3: 告警机制
- 触发条件: 离线 / TPS过低 / 内存超阈值
- TUI 高亮提示
- 可选: 桌面通知 / Discord Webhook

### 📋 P6-4: 玩家行为日志
- 增强日志监控: 进出时间戳/死亡/成就
- TUI 查询: 玩家在线时长/最后登录
- 依赖: 现有 `monitor/` 模块

---

## 🟣 Phase 7 — 实用工具箱

### 📋 P7-1: 快捷指令面板
- TUI 中显示自定义按钮:
  - "切换到和平" → `/difficulty peaceful`
  - "查看TPS" → 跳转监控面板
  - "全部传送" → 快捷指令列表
- 用户可在配置中自定义按钮

### 📋 P7-2: 崩溃报告查看器
- 扫描 `crash-reports/` 文件夹
- 按时间排序列表
- 点击查看: 高亮关键堆栈行
- 辅助排查崩溃原因

### 📋 P7-3: 游戏内公告
- TUI 直接向在线玩家广播
- 支持动态倒计时: "服务器5分钟后重启"

---

## 🌐 Phase 8 — 架构扩展

### 💤 P8-1: Web 管理面板
- 基于现有 HTTP API 扩展
- 前后端分离: Rust 后端 + 轻量前端框架
- 实时 WebSocket 推送状态

### 💤 P8-2: 插件系统
- 允许第三方 Rust crate 扩展功能
- 定义 Plugin trait
- 动态加载 (abi_stable / wasm)

### 💤 P8-3: 移动端专用优化
- Termux 通知集成 (`termux-notification`)
- 电池优化: 降低轮询频率
- 小屏适配 TUI 布局

---

## 📊 进度总览

| Phase | 任务数 | 已完成 | 状态 |
|-------|--------|--------|------|
| P0 核心基础 | 7 | 7 | ✅ 100% |
| P1 TUI 体验 | 5 | 0 | 📋 0% |
| P2 多服务器 | 5 | 0 | 📋 0% |
| P3 核心/Mod | 6 | 0 | 📋 0% |
| P4 自动化 | 3 | 0 | 📋 0% |
| P5 备份 | 3 | 0 | 📋 0% |
| P6 监控 | 4 | 0 | 📋 0% |
| P7 工具箱 | 3 | 0 | 📋 0% |
| P8 架构 | 3 | 0 | 💤 远期 |

---

## 🔗 API 速查

| 平台/功能 | API 端点 | 备注 |
|-----------|---------|------|
| FabricMC 服务端 | `https://meta.fabricmc.net/v2/versions/loader/{ver}/{loader}/1.1.0/server/jar` | 无鉴权 |
| PaperMC 版本列表 | `https://api.papermc.io/v2/projects/paper` | 无鉴权 |
| PaperMC build | `https://api.papermc.io/v2/projects/paper/versions/{ver}/builds/{build}` | 需先查 builds |
| PaperMC 下载 | `{build_url}/downloads/{name}` | name 从 builds 响应获取 |
| Modrinth 搜索 | `https://api.modrinth.com/v2/search?query={q}` | 无鉴权 |
| Modrinth 项目版本 | `https://api.modrinth.com/v2/project/{id}/version` | 无鉴权 |
| Modrinth 下载 | `https://cdn.modrinth.com/data/{id}/versions/{ver}/{file}` | 直链 |
| wiki.vg 协议版本 | `https://wiki.vg/Protocol_version_numbers` | MC Ping 版本号 |
| Minecraft 命令参考 | `https://minecraft.wiki/w/Commands` | 官方 Wiki |
