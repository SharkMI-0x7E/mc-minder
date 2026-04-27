use mc_status_probe::ping;
use mc_status_probe::protocol;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A mock Minecraft server that responds to the Server List Ping protocol.
async fn mock_minecraft_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Simple response: status JSON with expected structure
            let status_json = r#"{"version":{"name":"Mock 1.21","protocol":767},"players":{"max":20,"online":3,"sample":[]},"description":{"text":"A Mock Server"},"favicon":"data:image/png;base64,test"}"#;

            let json_bytes = status_json.as_bytes();
            let json_len = json_bytes.len() as i32;

            // Build status response packet
            let mut response = Vec::new();
            // Packet ID + JSON string length
            let inner_len: i32 = 1 + varint_size(json_len) as i32 + json_len;
            mc_status_probe::varint::encode_varint(inner_len, &mut response);
            // Packet ID (0x00)
            mc_status_probe::varint::encode_varint(0x00, &mut response);
            // JSON string
            mc_status_probe::varint::encode_varint(json_len, &mut response);
            response.extend_from_slice(json_bytes);

            // Read all incoming data (discard - just need to drain)
            let mut buf = [0u8; 512];
            let _ = stream.read(&mut buf).await;
            // Send response
            let _ = stream.write_all(&response).await;
        }
    });

    port
}

fn varint_size(value: i32) -> usize {
    let mut buf = Vec::new();
    mc_status_probe::varint::encode_varint(value, &mut buf);
    buf.len()
}

#[tokio::test]
async fn test_ping_mock_server() {
    let port = mock_minecraft_server().await;
    // Small delay to ensure server is ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = ping("127.0.0.1", port, Duration::from_secs(3), None).await.unwrap();

    assert_eq!(result.version_name, "Mock 1.21");
    assert_eq!(result.version_protocol, 767);
    assert_eq!(result.players_online, 3);
    assert_eq!(result.players_max, 20);
    assert!(result.description.contains("Mock Server"));
    assert!(result.latency_ms < 1000);
}

#[tokio::test]
async fn test_ping_invalid_address() {
    let result = ping("127.0.0.1", 19999, Duration::from_millis(500), None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handshake_packet_format() {
    let packet = protocol::build_handshake_packet(767, "localhost", 25565, 1);
    assert!(!packet.is_empty());
}

#[tokio::test]
async fn test_empty_response() {
    let result = protocol::parse_status_response(&[]);
    assert!(result.is_err());
}
