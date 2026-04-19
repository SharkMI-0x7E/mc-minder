# MC-Minder 稳定性和健壮性改进 Spec

## Why
当前 MC-Minder v0.3.0 存在若干稳定性和健壮性问题：日志监控在文件轮转时可能重复解析或丢失数据；HTTP API 缺乏优雅关闭机制；RCON 重连可能导致命令重复发送；配置解析对特殊字符处理不当；备份脚本路径硬编码；Windows 换行符问题未完全解决；缺少 CI/CD 自动构建发布。

## What Changes
- 日志监控改用字节偏移量 + inode 检测，避免重复解析
- HTTP API 集成优雅关闭信号
- RCON 重连逻辑优化，避免命令重复发送
- 配置解析增强，支持特殊字符
- 备份脚本从 config.toml 读取路径
- 添加 .gitattributes 强制 LF 换行符
- 添加 GitHub Actions CI/CD 自动构建发布

## Impact
- Affected specs: 日志监控、HTTP API、RCON 通信、配置解析、备份系统
- Affected code: src/monitor/mod.rs, src/api/mod.rs, src/rcon/mod.rs, src/main.rs, scripts/start.sh, scripts/backup.sh

## ADDED Requirements

### Requirement: 日志监控偏移量管理
日志监控 SHALL 使用字节偏移量方式读取增量内容，并监控 inode 变化检测文件轮转。

#### Scenario: 正常增量读取
- **WHEN** 日志文件被追加内容
- **THEN** 从上次偏移量开始读取到 EOF，更新偏移量

#### Scenario: 文件轮转检测
- **WHEN** 日志文件 inode 改变（文件被轮转）
- **THEN** 重置偏移量为 0，重新开始读取新文件

### Requirement: HTTP API 优雅关闭
HTTP API SHALL 支持优雅关闭，确保程序退出时请求不被强制中断。

#### Scenario: 收到关闭信号
- **WHEN** 程序收到 Ctrl+C 或其他关闭信号
- **THEN** HTTP 服务等待当前请求完成后关闭

### Requirement: RCON 重连安全
RCON 重连 SHALL 确保命令不会重复发送。

#### Scenario: 发送失败重连
- **WHEN** RCON 发送失败后重连
- **THEN** 重试一次，若仍失败则返回错误，不重复发送

### Requirement: 配置解析增强
配置解析 SHALL 正确处理包含特殊字符（如 `#`、空格）的值。

#### Scenario: 特殊字符值
- **WHEN** 配置值包含特殊字符且用双引号包围
- **THEN** 正确提取完整值，不截断

### Requirement: 备份路径灵活性
备份脚本 SHALL 从 config.toml 读取备份路径，支持用户自定义。

#### Scenario: 自定义备份路径
- **WHEN** config.toml 中配置了 backup_dest
- **THEN** 备份脚本使用配置的路径

### Requirement: 强制 LF 换行符
仓库 SHALL 强制所有 .sh 文件使用 LF 换行符。

#### Scenario: Git 检出
- **WHEN** 用户从 Git 检出代码
- **THEN** .sh 文件自动使用 LF 换行符

### Requirement: CI/CD 自动构建
项目 SHALL 在打 tag 时自动构建并发布预编译二进制。

#### Scenario: 发布新版本
- **WHEN** 推送新的 tag（如 v0.3.1）
- **THEN** 自动构建 aarch64 和 x86_64 二进制并上传到 GitHub Releases

## MODIFIED Requirements

### Requirement: mc-minder config get 子命令
系统 SHALL 提供 `mc-minder config get <key>` 子命令，供脚本读取配置值。

#### Scenario: 获取配置值
- **WHEN** 执行 `mc-minder config get backup_dest`
- **THEN** 输出配置值（如 `../backups`）

## REMOVED Requirements

无移除的需求。
