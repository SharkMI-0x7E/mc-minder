/// Minecraft protocol packet construction for Server List Ping.
///
/// Handshake packet format (packet ID 0x00, state = handshake):
///   - VarInt: 0x00 (packet ID)
///   - VarInt: protocol version
///   - String: server address
///   - u16: server port
///   - VarInt: next state (1 = status)
///
/// Status Request packet format (packet ID 0x00, state = status):
///   - VarInt: 0x00 (packet ID)
///
/// Status Response packet format (packet ID 0x00, state = status):
///   - VarInt: packet length
///   - VarInt: 0x00 (packet ID)
///   - String: JSON response

use crate::error::PingError;
use crate::varint;

/// Default protocol version for Minecraft 1.21.x.
pub const DEFAULT_PROTOCOL_VERSION: i32 = 767;

/// Build the Handshake packet.
/// state = 1 means "status" (as opposed to 2 = login).
pub fn build_handshake_packet(
    protocol_version: i32,
    address: &str,
    port: u16,
    next_state: i32,
) -> Vec<u8> {
    let mut buf = Vec::new();
    // Packet ID for handshake
    varint::encode_varint(0x00, &mut buf);
    // Protocol version
    varint::encode_varint(protocol_version, &mut buf);
    // Server address (string)
    varint::encode_string(address, &mut buf);
    // Server port (u16 big-endian)
    buf.extend_from_slice(&port.to_be_bytes());
    // Next state (1 = status)
    varint::encode_varint(next_state, &mut buf);

    // Prepend packet length
    let mut framed = Vec::new();
    varint::encode_varint(buf.len() as i32, &mut framed);
    framed.extend_from_slice(&buf);

    framed
}

/// Build the Status Request packet (no payload, just packet ID).
pub fn build_status_request_packet() -> Vec<u8> {
    let mut buf = Vec::new();
    // Packet length = 1 (just the packet ID byte)
    varint::encode_varint(1, &mut buf);
    // Packet ID for status request
    varint::encode_varint(0x00, &mut buf);
    buf
}

/// Parse a full Status Response from raw bytes.
///
/// The Minecraft protocol wraps each packet as:
///   [VarInt: total_length][VarInt: packet_id][data...]
///
/// This function consumes the length prefix, validates the packet ID,
/// and extracts the JSON string.
pub fn parse_status_response(data: &[u8]) -> Result<(String, usize), PingError> {
    if data.is_empty() {
        return Err(PingError::Protocol("empty response".to_string()));
    }

    // Read packet length VarInt
    let (packet_len, offset) = varint::decode_varint(data)?;
    if packet_len < 1 {
        return Err(PingError::Protocol(format!(
            "invalid packet length: {}",
            packet_len
        )));
    }

    let body = &data[offset..];
    if body.len() < (packet_len as usize) {
        return Err(PingError::Protocol(format!(
            "packet body too short: expected {}, got {}",
            packet_len,
            body.len()
        )));
    }

    // Read packet ID from body
    let (packet_id, pid_offset) = varint::decode_varint(body)?;
    if packet_id != 0x00 {
        return Err(PingError::Protocol(format!(
            "unexpected packet ID: 0x{:02X}, expected 0x00",
            packet_id
        )));
    }

    // Read JSON string from remaining body
    let json_data = &body[pid_offset..];
    let (json_str, consumed) = varint::decode_string(json_data)?;

    let total_consumed = offset + pid_offset + consumed;
    Ok((json_str, total_consumed))
}
