/// A zero-dependency Minecraft Java server status probe.
///
/// Implements the [Server List Ping](https://wiki.vg/Server_List_Ping)
/// protocol from scratch, using only `tokio` for networking,
/// `serde_json` for response parsing, and `thiserror` for errors.

pub mod error;
pub mod ping;
pub mod protocol;
pub mod varint;

pub use error::PingError;
pub use ping::{ping, PingResult};
pub use protocol::DEFAULT_PROTOCOL_VERSION;
