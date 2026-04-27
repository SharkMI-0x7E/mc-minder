/// VarInt encoding/decoding as used in the Minecraft protocol.
///
/// A VarInt is a variable-length integer where each byte uses
/// 7 bits for data and the MSB (bit 7) as a continuation flag.
/// If bit 7 is set, more bytes follow. The integer is encoded
/// in little-endian order across the bytes.

use crate::error::PingError;

/// Encode an i32 as a VarInt and write it into the buffer.
pub fn encode_varint(value: i32, buf: &mut Vec<u8>) {
    let mut val = value as u32;
    loop {
        if val & !0x7F == 0 {
            buf.push(val as u8);
            break;
        }
        buf.push((val & 0x7F | 0x80) as u8);
        val >>= 7;
    }
}

/// Read a VarInt from a byte slice. Returns the decoded value
/// and the number of bytes consumed.
pub fn decode_varint(data: &[u8]) -> Result<(i32, usize), PingError> {
    let mut value: u32 = 0;
    let mut position: u32 = 0;

    for (i, &byte) in data.iter().enumerate() {
        value |= ((byte & 0x7F) as u32) << position;

        if byte & 0x80 == 0 {
            return Ok((value as i32, i + 1));
        }

        position += 7;
        if position >= 32 {
            return Err(PingError::Protocol("VarInt too large".to_string()));
        }
        if i >= 4 {
            return Err(PingError::Protocol("VarInt too long (max 5 bytes)".to_string()));
        }
    }

    Err(PingError::Protocol("VarInt truncated".to_string()))
}

/// Encode a Minecraft string (length-prefixed UTF-8).
pub fn encode_string(s: &str, buf: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    encode_varint(bytes.len() as i32, buf);
    buf.extend_from_slice(bytes);
}

/// Read a Minecraft string from a byte slice. Returns (string, bytes_consumed).
pub fn decode_string(data: &[u8]) -> Result<(String, usize), PingError> {
    let (len, offset) = decode_varint(data)?;
    let len = len as usize;
    let end = offset + len;
    if end > data.len() {
        return Err(PingError::Protocol(format!(
            "String length {} exceeds buffer size {}",
            len,
            data.len() - offset
        )));
    }
    let s = String::from_utf8(data[offset..end].to_vec())
        .map_err(|e| PingError::Protocol(format!("Invalid UTF-8: {}", e)))?;
    Ok((s, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_varint_zero() {
        let mut buf = Vec::new();
        encode_varint(0, &mut buf);
        assert_eq!(buf, vec![0x00]);
    }

    #[test]
    fn test_encode_varint_one() {
        let mut buf = Vec::new();
        encode_varint(1, &mut buf);
        assert_eq!(buf, vec![0x01]);
    }

    #[test]
    fn test_encode_varint_127() {
        let mut buf = Vec::new();
        encode_varint(127, &mut buf);
        assert_eq!(buf, vec![0x7F]);
    }

    #[test]
    fn test_encode_varint_128() {
        let mut buf = Vec::new();
        encode_varint(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);
    }

    #[test]
    fn test_encode_varint_25565() {
        let mut buf = Vec::new();
        encode_varint(25565, &mut buf);
        assert_eq!(buf, vec![0xDD, 0xC7, 0x01]);
    }

    #[test]
    fn test_decode_varint_roundtrip() {
        for val in &[0, 1, 127, 128, 255, 25565, 2147483647] {
            let mut buf = Vec::new();
            encode_varint(*val, &mut buf);
            let (decoded, consumed) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, *val, "roundtrip for {}", val);
            assert_eq!(consumed, buf.len(), "all bytes consumed for {}", val);
        }
    }

    #[test]
    fn test_encode_string() {
        let mut buf = Vec::new();
        encode_string("hello", &mut buf);
        // length 5 as VarInt -> 0x05, then "hello"
        let expected: &[u8] = &[0x05, b'h', b'e', b'l', b'l', b'o'];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_string_roundtrip() {
        for s in &["", "a", "hello world", "Minecraft 1.21"] {
            let mut buf = Vec::new();
            encode_string(s, &mut buf);
            let (decoded, consumed) = decode_string(&buf).unwrap();
            assert_eq!(decoded, *s, "roundtrip for '{}'", s);
            assert_eq!(consumed, buf.len(), "all bytes consumed for '{}'", s);
        }
    }
}
