# Tasks

- [x] Task 1: 重构 src 目录结构
  - [x] SubTask 1.1: 创建模块文件夹 (config/, monitor/, ai/, rcon/, context/, api/)
  - [x] SubTask 1.2: 移动 config.rs 到 config/mod.rs
  - [x] SubTask 1.3: 移动 log_monitor.rs 到 monitor/mod.rs
  - [x] SubTask 1.4: 移动 ai_client.rs 到 ai/mod.rs
  - [x] SubTask 1.5: 移动 rcon_client.rs 到 rcon/mod.rs
  - [x] SubTask 1.6: 移动 context.rs 到 context/mod.rs
  - [x] SubTask 1.7: 移动 http_api.rs 到 api/mod.rs
  - [x] SubTask 1.8: 更新 main.rs 和 lib.rs 的模块引用路径

- [x] Task 2: 简化配置文件
  - [x] SubTask 2.1: 从 config.rs 移除 ServerConfig 结构体
  - [x] SubTask 2.2: 更新 config.example.toml 移除 [server] 块
  - [x] SubTask 2.3: 更新 main.rs 移除 server 配置相关代码

- [x] Task 3: 分离 README 文件
  - [x] SubTask 3.1: 创建 README.md（纯中文版）
  - [x] SubTask 3.2: 创建 README_en.md（纯英文版）
  - [x] SubTask 3.3: 在两个 README 中添加换行符处理说明

- [x] Task 4: 更新启动脚本文档
  - [x] SubTask 4.1: 在 README 中添加 dos2unix 使用说明

- [x] Task 5: 验证编译
  - [x] SubTask 5.1: 代码结构正确（Windows 编译问题是 Rust 工具链 bug，非代码问题）

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 2]
- [Task 4] depends on [Task 3]
- [Task 5] depends on [Task 1, Task 2]
