# MC-Minder 开发路线图

> 标记：✅ 已完成 | 🔧 基本完成 | ⚠️ 部分 | ❌ 未做 | 💤 远期

---

## P0 核心基础 ✅ 100%
P0-1 移除LLM ✅ | P0-2 Java管理 ✅ | P0-3 语言选择 ✅ | P0-4 msp库 ✅ | P0-5 MC状态 ✅ | P0-6 多平台CI ✅ | P0-7 按键修复 ✅

## P1 TUI交互 ✅ 100%
P1-1 自动刷新 ✅ | P1-2 弹窗系统 ✅ | P1-3 快捷键 ✅ | P1-4 分栏布局 ✅ | P1-5 遮罩 ✅

## P2 多服务器 🔧 60%
P2-1 自动发现 ✅ | P2-2 数据结构 ✅ | P2-3 侧边栏 ✅ | P2-4 配置编辑 ❌ | P2-5 实例隔离 ❌

## P3 核心/Mod 🔧 67%
P3-1 Fabric下载 ✅ | P3-2 Paper下载 ✅ | P3-3 版本向导 ✅ | P3-4 Modrinth ✅ | P3-5 更新检测 ❌ | P3-6 EULA自动 ❌

## P4 自动化 ✅ 100%
P4-1 看门狗 ✅ | P4-2 懒启动 ✅ | P4-3 计划任务 ✅

## P5 备份 ✅ 100%
P5-1 自动备份 ✅ | P5-2 保留策略 ✅ | P5-3 备份恢复 ✅

## P6 高级监控 🔧 50%
P6-1 TPS显示 ✅ | P6-2 历史图表 ❌ | P6-3 告警机制 ⚠️(字段已加) | P6-4 玩家日志 ❌

## P7 工具箱 ❌ 0%
❌ 全部未做

## P8 架构 💤
远期

---

## API速查
| 平台 | 端点 |
|------|------|
| FabricMC 下载 | `meta.fabricmc.net/v2/versions/loader/{ver}/{loader}/1.1.0/server/jar` |
| PaperMC 版本 | `api.papermc.io/v2/projects/paper` |
| Modrinth 搜索 | `api.modrinth.com/v2/search?query={q}` |
| MC 协议版本 | `wiki.vg/Protocol_version_numbers` |
