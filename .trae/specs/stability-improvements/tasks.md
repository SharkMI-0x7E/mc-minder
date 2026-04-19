# Tasks

- [x] Task 1: 日志监控偏移量管理改进
  - [x] SubTask 1.1: 添加 `last_offset` 字段替代 `last_content`
  - [x] SubTask 1.2: 实现 inode 检测逻辑（使用 `std::fs::Metadata` 的文件标识）
  - [x] SubTask 1.3: 修改 `check_file_changes` 使用偏移量读取
  - [x] SubTask 1.4: 处理文件轮转时重置偏移量

- [x] Task 2: HTTP API 优雅关闭
  - [x] SubTask 2.1: 修改 `HttpApi::start` 接收 shutdown Future 参数
  - [x] SubTask 2.2: 使用 `bind_with_graceful_shutdown` 启动服务
  - [x] SubTask 2.3: 在 main.rs 中传递 shutdown 信号

- [x] Task 3: RCON 重连逻辑优化
  - [x] SubTask 3.1: 封装 `try_send` 内部方法
  - [x] SubTask 3.2: 修改 `execute` 方法：失败后重连并重试一次，仍失败则返回错误

- [x] Task 4: 配置解析增强
  - [x] SubTask 4.1: 改进 `get_config_value` 正则表达式，正确处理双引号值
  - [x] SubTask 4.2: 在 README 中说明特殊字符需用双引号包围

- [x] Task 5: mc-minder config get 子命令
  - [x] SubTask 5.1: 在 Commands 枚举中添加 `ConfigGet` 子命令
  - [x] SubTask 5.2: 实现 `config get <key>` 功能，输出配置值

- [x] Task 6: 备份脚本路径灵活性
  - [x] SubTask 6.1: 修改 backup.sh 调用 `mc-minder config get backup_dest`
  - [x] SubTask 6.2: 添加默认值回退逻辑

- [x] Task 7: 强制 LF 换行符
  - [x] SubTask 7.1: 创建 .gitattributes 文件
  - [x] SubTask 7.2: 更新 README 添加换行符说明

- [x] Task 8: CI/CD 自动构建
  - [x] SubTask 8.1: 创建 .github/workflows/release.yml
  - [x] SubTask 8.2: 配置 aarch64-unknown-linux-musl 和 x86_64-unknown-linux-musl 构建
  - [x] SubTask 8.3: 配置自动上传到 GitHub Releases

- [x] Task 9: 更新 README
  - [x] SubTask 9.1: 添加 CI 状态徽章
  - [x] SubTask 9.2: 添加换行符处理说明

# Task Dependencies
- [Task 5] depends on [Task 3] (需要 mc-minder 二进制支持 config get)
- [Task 6] depends on [Task 5] (backup.sh 需要调用 config get)
- [Task 9] depends on [Task 7, Task 8]
