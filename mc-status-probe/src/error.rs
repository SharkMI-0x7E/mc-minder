use thiserror::Error;

/// Errors that can occur during a Minecraft server ping.
#[derive(Error, Debug)]
pub enum PingError {
    /// TCP connection failed or timed out.
    #[error("connection failed: {0}")]
    Connection(std::io::Error),

    /// Connection timed out before handshake could complete.
    #[error("connection timed out")]
    Timeout,

    /// Received invalid or malformed data from the server.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Failed to parse the JSON response from the server.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// The server returned a response with unexpected structure.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}
