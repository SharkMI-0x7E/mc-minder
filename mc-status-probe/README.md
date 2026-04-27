# mc-status-probe

A zero-dependency, protocol-level Minecraft Java server status probe. Built from scratch following the [wiki.vg Server List Ping](https://wiki.vg/Server_List_Ping) protocol specification.

## Features

- **Pure protocol implementation** — no Minecraft protocol library dependencies
- **Async** — built on `tokio` for non-blocking I/O
- **Type-safe** — structured `PingResult` with version, players, MOTD, latency
- **Configurable** — adjustable protocol version, timeout
- **Lightweight** — only `tokio`, `serde_json`, `thiserror` as dependencies

## Usage

```rust
use mc_status_probe::ping;
use std::time::Duration;

#[tokio::main]
async fn main() {
    match ping("mc.hypixel.net", 25565, Duration::from_secs(3), None).await {
        Ok(result) => {
            println!("Server: {}", result.description);
            println!("Players: {}/{}", result.players_online, result.players_max);
            println!("Version: {}", result.version_name);
            println!("Latency: {}ms", result.latency_ms);
        }
        Err(e) => println!("Ping failed: {}", e),
    }
}
```

## How It Works

1. Opens a TCP connection to the Minecraft server
2. Sends a Handshake packet (protocol version + server address + "status" next state)
3. Sends a Status Request packet
4. Reads and parses the Status Response JSON
5. Calculates round-trip latency

## License

MIT
