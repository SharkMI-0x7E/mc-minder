# mc-status-probe

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[English](./README.md) | 中文

一个零外部协议依赖的 Minecraft Java 版服务器状态查询库。完全按照 [wiki.vg Server List Ping](https://wiki.vg/Server_List_Ping) 协议规范从头实现。

## 功能特性

- **纯协议实现** — 不依赖任何现有的 Minecraft 协议库
- **异步** — 基于 `tokio` 的非阻塞 I/O
- **类型安全** — 结构化的 `PingResult`，包含版本、玩家数、MOTD、延迟
- **可配置** — 支持自定义协议版本号、超时时间
- **轻量级** — 仅依赖 `tokio`、`serde_json`、`thiserror`

## 使用方式

```rust
use mc_status_probe::ping;
use std::time::Duration;

#[tokio::main]
async fn main() {
    match ping("mc.hypixel.net", 25565, Duration::from_secs(3), None).await {
        Ok(result) => {
            println!("服务器: {}", result.description);
            println!("玩家: {}/{}", result.players_online, result.players_max);
            println!("版本: {}", result.version_name);
            println!("延迟: {}ms", result.latency_ms);
        }
        Err(e) => println!("Ping 失败: {}", e),
    }
}
```

## 工作原理

1. 建立到 Minecraft 服务器的 TCP 连接
2. 发送 Handshake 包（协议版本 + 服务器地址 + 下一状态 = "查询"）
3. 发送 Status Request 包
4. 读取并解析 Status Response JSON
5. 计算往返延迟

## API

### `ping(address, port, timeout, protocol_version)` → `Result<PingResult, PingError>`

| 参数 | 类型 | 说明 |
|------|------|------|
| `address` | `&str` | 服务器主机名或 IP 地址 |
| `port` | `u16` | 服务器端口（默认 Minecraft 端口为 25565） |
| `timeout` | `Duration` | 总超时时间 |
| `protocol_version` | `Option<i32>` | Minecraft 协议版本号（`None` 使用默认值 767） |

### `PingResult`

| 字段 | 类型 | 说明 |
|------|------|------|
| `description` | `String` | 服务器 MOTD / 描述文本 |
| `players_online` | `i32` | 当前在线玩家数 |
| `players_max` | `i32` | 最大玩家数 |
| `version_name` | `String` | 服务器版本名称（如 "1.21"） |
| `version_protocol` | `i32` | 协议版本号（如 767） |
| `latency_ms` | `u64` | 往返延迟（毫秒） |
| `raw_json` | `String` | 服务器返回的原始 JSON |

### `PingError`

| 变体 | 说明 |
|------|------|
| `Connection(std::io::Error)` | TCP 连接失败 |
| `Timeout` | 连接或读取超时 |
| `Protocol(String)` | 协议解析错误 |
| `Json(serde_json::Error)` | JSON 解析失败 |
| `UnexpectedResponse(String)` | 意外的响应格式 |

## 协议版本号参考

| Minecraft 版本 | 协议版本号 |
|----------------|-----------|
| 1.21.11 | 774 |
| 1.21.5 | 767 |
| 1.21 / 1.21.1 | 767 |
| 1.20.5 / 1.20.6 | 766 |
| 1.20.2 / 1.20.4 | 765 |
| 1.19.4 | 762 |
| 1.19.3 | 761 |
| 1.19.1 / 1.19.2 | 760 |
| 1.19 | 759 |
| 1.18.2 | 758 |

完整列表见 <https://wiki.vg/Protocol_version_numbers>。

## 许可证

MIT
