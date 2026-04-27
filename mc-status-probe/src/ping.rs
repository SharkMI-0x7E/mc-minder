/// Core ping function — the main entry point for the library.
///
/// Opens a TCP connection, performs the Minecraft Server List Ping
/// handshake, and returns structured server status information.

use crate::error::PingError;
use crate::protocol;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Structured result of a successful Minecraft server ping.
#[derive(Debug, Clone)]
pub struct PingResult {
    /// Clean text representation of the MOTD / server description.
    pub description: String,
    /// Number of players currently online.
    pub players_online: i32,
    /// Maximum number of players allowed.
    pub players_max: i32,
    /// Server version name string (e.g. "1.21").
    pub version_name: String,
    /// Server protocol version number (e.g. 767 for 1.21).
    pub version_protocol: i32,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Raw JSON response from the server (useful for debugging).
    pub raw_json: String,
}

/// Minecraft Server List Ping JSON response structure.
/// See: https://wiki.vg/Server_List_Ping#Response
#[derive(Debug, Deserialize)]
struct StatusResponse {
    version: VersionInfo,
    players: PlayersInfo,
    description: serde_json::Value,
    #[serde(default)]
    #[allow(dead_code)]
    favicon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    name: String,
    protocol: i32,
}

#[derive(Debug, Deserialize)]
struct PlayersInfo {
    max: i32,
    online: i32,
    #[serde(default)]
    #[allow(dead_code)]
    sample: Vec<PlayerSample>,
}

#[derive(Debug, Deserialize)]
struct PlayerSample {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    id: String,
}

/// Extract a human-readable string from a Minecraft chat component JSON value.
/// Handles: plain strings, {"text": "..."}, and {"extra": [{"text": "..."}, ...]}
fn extract_motd_text(description: &serde_json::Value) -> String {
    match description {
        // Plain string
        serde_json::Value::String(s) => s.clone(),
        // Object with "text" field
        serde_json::Value::Object(obj) => {
            // Try "extra" array (modern servers)
            if let Some(extra) = obj.get("extra") {
                if let Some(arr) = extra.as_array() {
                    let parts: Vec<String> = arr
                        .iter()
                        .filter_map(|e| e.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
                        .collect();
                    if !parts.is_empty() {
                        return parts.join("");
                    }
                }
            }
            // Fall back to "text"
            obj.get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| serde_json::to_string(description).unwrap_or_default())
        }
        _ => serde_json::to_string(description).unwrap_or_default(),
    }
}

/// Ping a Minecraft Java server using the Server List Ping protocol.
///
/// # Arguments
/// * `address` - Server hostname or IP address
/// * `port` - Server port (default Minecraft port is 25565)
/// * `timeout` - Maximum time to wait for the entire ping operation
/// * `protocol_version` - Optional Minecraft protocol version (defaults to 767 for 1.21.x)
///
/// # Returns
/// * `Ok(PingResult)` with structured server status
/// * `Err(PingError)` with the specific failure reason
pub async fn ping(
    address: &str,
    port: u16,
    timeout: Duration,
    protocol_version: Option<i32>,
) -> Result<PingResult, PingError> {
    let protocol_ver = protocol_version.unwrap_or(protocol::DEFAULT_PROTOCOL_VERSION);
    let start = Instant::now();

    // Connect with timeout
    let addr = format!("{}:{}", address, port);
    let mut stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| PingError::Timeout)?
        .map_err(PingError::Connection)?;

    // Send Handshake packet (next state = 1 for status)
    let handshake = protocol::build_handshake_packet(protocol_ver, address, port, 1);
    tokio::time::timeout(timeout, stream.write_all(&handshake))
        .await
        .map_err(|_| PingError::Timeout)?
        .map_err(PingError::Connection)?;

    // Send Status Request packet
    let status_req = protocol::build_status_request_packet();
    tokio::time::timeout(timeout, stream.write_all(&status_req))
        .await
        .map_err(|_| PingError::Timeout)?
        .map_err(PingError::Connection)?;

    // Read response: read ALL available data in one go
    // The Minecraft protocol sends the response as a single TCP segment
    // for the small status JSON, so one read is usually sufficient.
    let mut response_buf = vec![0u8; 65536];
    let total_read = tokio::time::timeout(timeout, stream.read(&mut response_buf))
        .await
        .map_err(|_| PingError::Timeout)?
        .map_err(PingError::Connection)?;

    if total_read == 0 {
        return Err(PingError::Protocol("connection closed with no response".to_string()));
    }

    let latency_ms = start.elapsed().as_millis() as u64;

    // Parse the full status response (including packet length prefix)
    let (json_str, _consumed) = protocol::parse_status_response(&response_buf[..total_read])?;

    // Parse JSON
    let status: StatusResponse = serde_json::from_str(&json_str)?;

    let description = extract_motd_text(&status.description);

    Ok(PingResult {
        description,
        players_online: status.players.online,
        players_max: status.players.max,
        version_name: status.version.name,
        version_protocol: status.version.protocol,
        latency_ms,
        raw_json: json_str,
    })
}
