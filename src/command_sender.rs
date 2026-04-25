use anyhow::{Context, Result};
use log::{debug, warn, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

// ============================================================
// CommandSenderMode - Enum for different transport modes
// ============================================================

#[derive(Clone)]
pub enum CommandSenderMode {
    Rcon { host: String, port: u16, password: String },
    PooledRcon { host: String, port: u16, password: String },
    Stdin { stdin: std::sync::Arc<Mutex<ChildStdin>> },
}

pub struct CommandSender {
    mode: CommandSenderMode,
}

impl CommandSender {
    pub fn new(mode: CommandSenderMode) -> Self {
        Self { mode }
    }

    pub fn rcon(host: String, port: u16, password: String) -> Self {
        Self { mode: CommandSenderMode::Rcon { host, port, password } }
    }

    /// Create a pooled RCON sender that maintains a persistent connection.
    /// This is more efficient than creating a new connection per command.
    pub fn pooled_rcon(host: String, port: u16, password: String) -> Self {
        Self { mode: CommandSenderMode::PooledRcon { host, port, password } }
    }

    pub fn stdin(stdin: ChildStdin) -> Self {
        Self { mode: CommandSenderMode::Stdin { stdin: std::sync::Arc::new(Mutex::new(stdin)) } }
    }

    /// Send a command and return the response text (if available).
    /// For RCON, this returns the server's response to the command.
    /// For Stdin, this returns a confirmation string.
    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        match &mut self.mode {
            CommandSenderMode::Rcon { host, port, password } => {
                RconCommandSender::new(host.clone(), *port, password.clone())
                    .send_command(command)
                    .await
            }
            CommandSenderMode::PooledRcon { host, port, password } => {
                PooledRconSender::new(host.clone(), *port, password.clone())
                    .send_command(command)
                    .await
            }
            CommandSenderMode::Stdin { stdin } => {
                StdinCommandSender::new(stdin.clone()).send_command(command).await
            }
        }
    }

    /// Send a command without caring about the response.
    /// Useful for fire-and-forget commands like `say` and `tellraw`.
    pub async fn send_command_ignore_response(&mut self, command: &str) -> Result<()> {
        match self.send_command(command).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn name(&self) -> &'static str {
        match &self.mode {
            CommandSenderMode::Rcon { .. } => "RCON",
            CommandSenderMode::PooledRcon { .. } => "PooledRCON",
            CommandSenderMode::Stdin { .. } => "Stdin",
        }
    }
}

// ============================================================
// RconCommandSender - Single-shot RCON connection per command
// ============================================================

pub struct RconCommandSender {
    host: String,
    port: u16,
    password: String,
}

impl RconCommandSender {
    pub fn new(host: String, port: u16, password: String) -> Self {
        Self { host, port, password }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        const MAX_RETRIES: u32 = 3;

        for attempt in 1..=MAX_RETRIES {
            match self.execute_with_response(command).await {
                Ok(response) => {
                    debug!("[RconSender] Command sent successfully: {}", command);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("[RconSender] Attempt {}/{} failed: {}", attempt, MAX_RETRIES, e);
                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "[RconSender] Failed to send command after {} attempts: {}",
            MAX_RETRIES, command
        ))
    }

    async fn execute_with_response(&mut self, command: &str) -> Result<String> {
        use tokio::net::TcpStream;

        const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

        let addr = format!("{}:{}", self.host, self.port);
        debug!("[RconSender] Connecting to RCON at {}", addr);

        let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
            .await
            .context("RCON connection timeout")?
            .context(format!(
                "[RconSender] Failed to connect to RCON at {}. Please check:\n\
                 - enable-rcon=true\n\
                 - rcon.port={}\n\
                 - rcon.password=<password>",
                addr, self.port
            ))?;

        self.authenticate(&mut stream).await?;
        let response = self.send_packet(&mut stream, 1, 2, command).await?;

        Ok(response)
    }

    async fn authenticate(&self, stream: &mut tokio::net::TcpStream) -> Result<()> {
        const PACKET_TYPE_AUTH: i32 = 3;

        let payload_bytes = self.password.as_bytes();
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&1_i32.to_le_bytes());
        buf.extend_from_slice(&PACKET_TYPE_AUTH.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);

        stream.write_all(&buf).await.context("[RconSender] Auth write failed")?;
        stream.flush().await.context("[RconSender] Auth flush failed")?;

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.context("[RconSender] Auth read length failed")?;
        let resp_len = i32::from_le_bytes(len_buf);

        if resp_len < 10 {
            return Err(anyhow::anyhow!("[RconSender] Invalid auth response length: {}", resp_len));
        }

        let mut data = vec![0u8; resp_len as usize];
        stream.read_exact(&mut data).await.context("[RconSender] Auth read data failed")?;

        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if id == -1 {
            return Err(anyhow::anyhow!("[RconSender] RCON authentication rejected"));
        }

        debug!("[RconSender] Authenticated successfully");
        Ok(())
    }

    async fn send_packet(&self, stream: &mut tokio::net::TcpStream, packet_id: i32, packet_type: i32, payload: &str) -> Result<String> {
        const READ_TIMEOUT: Duration = Duration::from_secs(10);

        let payload_bytes = payload.as_bytes();
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&packet_id.to_le_bytes());
        buf.extend_from_slice(&packet_type.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);

        let write_result = timeout(Duration::from_secs(10), stream.write_all(&buf))
            .await
            .context("[RconSender] Write timeout")?;
        write_result.context("[RconSender] Write failed")?;

        stream.flush().await.context("[RconSender] Flush failed")?;

        let mut len_buf = [0u8; 4];
        let read_result = timeout(READ_TIMEOUT, stream.read_exact(&mut len_buf))
            .await
            .context("[RconSender] Read timeout")?;
        read_result.context("[RconSender] Read length failed")?;

        let resp_len = i32::from_le_bytes(len_buf);
        let mut response = String::new();

        if resp_len >= 10 {
            let mut data = vec![0u8; resp_len as usize];
            let read_result = timeout(READ_TIMEOUT, stream.read_exact(&mut data))
                .await
                .context("[RconSender] Read timeout")?;
            read_result.context("[RconSender] Read data failed")?;

            // Extract response text (skip 8 bytes: request_id + packet_type, remove trailing null)
            if data.len() > 9 {
                response = String::from_utf8_lossy(&data[8..data.len().saturating_sub(1)]).into_owned();
            }
        }

        debug!("[RconSender] Packet sent: id={}, type={}, cmd={}", packet_id, packet_type, payload);
        Ok(response)
    }
}

// ============================================================
// PooledRconSender - Persistent RCON connection with auto-reconnect
// ============================================================

pub struct PooledRconSender {
    host: String,
    port: u16,
    password: String,
    stream: Option<tokio::net::TcpStream>,
}

impl PooledRconSender {
    pub fn new(host: String, port: u16, password: String) -> Self {
        Self {
            host,
            port,
            password,
            stream: None,
        }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        const MAX_RETRIES: u32 = 3;

        for attempt in 1..=MAX_RETRIES {
            // Try existing connection first
            if self.stream.is_some() {
                match self.execute_command(command).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        // Connection lost, reconnect
                        self.stream = None;
                        warn!("[PooledRconSender] Connection lost, reconnect attempt {}/{}: {}", attempt, MAX_RETRIES, e);
                        // Fall through to reconnect attempt
                    }
                }
            }

            // No connection or lost connection, establish new one
            if let Err(e) = self.connect().await {
                warn!("[PooledRconSender] Connect attempt {}/{} failed: {}", attempt, MAX_RETRIES, e);
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                continue;
            }

            // Send command on fresh connection
            match self.execute_command(command).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!("[PooledRconSender] Command failed on attempt {}: {}", attempt, e);
                    self.stream = None;
                }
            }
        }

        Err(anyhow::anyhow!(
            "[PooledRconSender] Failed after {} attempts: {}",
            MAX_RETRIES, command
        ))
    }

    async fn connect(&mut self) -> Result<()> {
        use tokio::net::TcpStream;

        const PACKET_TYPE_AUTH: i32 = 3;
        const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

        let addr = format!("{}:{}", self.host, self.port);
        debug!("[PooledRconSender] Connecting to {}", addr);

        let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
            .await
            .context(format!("[PooledRconSender] Connection timeout to {}", addr))?
            .context(format!("[PooledRconSender] Failed to connect to RCON at {}", addr))?;

        // Authenticate
        let payload_bytes = self.password.as_bytes();
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&1_i32.to_le_bytes());
        buf.extend_from_slice(&PACKET_TYPE_AUTH.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);

        stream.write_all(&buf).await.context("[PooledRconSender] Auth write failed")?;
        stream.flush().await.context("[PooledRconSender] Auth flush failed")?;

        // Read auth response
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.context("[PooledRconSender] Auth read length failed")?;
        let resp_len = i32::from_le_bytes(len_buf);

        if resp_len < 10 {
            return Err(anyhow::anyhow!("[PooledRconSender] Invalid auth response length: {}", resp_len));
        }

        let mut data = vec![0u8; resp_len as usize];
        stream.read_exact(&mut data).await.context("[PooledRconSender] Auth read data failed")?;

        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if id == -1 {
            return Err(anyhow::anyhow!("[PooledRconSender] RCON authentication rejected"));
        }

        debug!("[PooledRconSender] Authenticated successfully to {}", addr);
        self.stream = Some(stream);
        Ok(())
    }

    async fn execute_command(&mut self, command: &str) -> Result<String> {
        const PACKET_TYPE_COMMAND: i32 = 2;

        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow::anyhow!("[PooledRconSender] No connection"))?;

        // Send command packet
        let payload_bytes = command.as_bytes();
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&1_i32.to_le_bytes());
        buf.extend_from_slice(&PACKET_TYPE_COMMAND.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);

        let write_result = timeout(Duration::from_secs(10), stream.write_all(&buf)).await
            .context("[PooledRconSender] Write timeout")?;
        write_result.context("[PooledRconSender] Write failed")?;
        stream.flush().await.context("[PooledRconSender] Flush failed")?;

        // Read response
        let mut len_buf = [0u8; 4];
        let read_result = timeout(Duration::from_secs(10), stream.read_exact(&mut len_buf)).await
            .context("[PooledRconSender] Read timeout")?;
        read_result.context("[PooledRconSender] Read length failed")?;

        let resp_len = i32::from_le_bytes(len_buf);
        let mut response = String::new();

        if resp_len >= 10 {
            let mut data = vec![0u8; resp_len as usize];
            let read_result = timeout(Duration::from_secs(10), stream.read_exact(&mut data)).await
                .context("[PooledRconSender] Read timeout")?;
            read_result.context("[PooledRconSender] Read data failed")?;

            // Extract response text (skip 8 bytes: request_id + packet_type)
            if data.len() > 9 {
                response = String::from_utf8_lossy(&data[8..data.len().saturating_sub(1)]).into_owned();
            }
        }

        debug!("[PooledRconSender] Command sent: {}", command);
        Ok(response)
    }
}

// ============================================================
// StdinCommandSender - Sends commands via process stdin
// ============================================================

pub struct StdinCommandSender {
    stdin: std::sync::Arc<Mutex<ChildStdin>>,
}

impl StdinCommandSender {
    pub fn new(stdin: std::sync::Arc<Mutex<ChildStdin>>) -> Self {
        Self { stdin }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(command.as_bytes()).await.context("[StdinSender] Write failed")?;
        stdin.write_all(b"\n").await.context("[StdinSender] Newline failed")?;
        stdin.flush().await.context("[StdinSender] Flush failed")?;
        debug!("[StdinSender] Command sent: {}", command);
        Ok(format!("Command sent: {}", command))
    }
}

// ============================================================
// MultiCommandSender - Tries multiple senders in order
// ============================================================

pub struct MultiCommandSender {
    senders: Vec<CommandSender>,
}

impl MultiCommandSender {
    pub fn new() -> Self {
        Self { senders: Vec::new() }
    }

    pub fn add_sender(&mut self, sender: CommandSender) {
        self.senders.push(sender);
    }

    /// Send a command through the first available sender.
    /// Returns the response text from the first successful sender.
    pub async fn send_command(&mut self, command: &str) -> Result<String> {
        if self.senders.is_empty() {
            return Err(anyhow::anyhow!("[MultiSender] No command senders available"));
        }

        let mut last_error = None;
        for sender in &mut self.senders {
            match sender.send_command(command).await {
                Ok(response) => {
                    info!("[MultiSender] Command sent via {}: {}", sender.name(), command);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("[MultiSender] {} failed: {}", sender.name(), e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("[MultiSender] All senders failed")))
    }

    /// Send a command without caring about the response text.
    pub async fn send_command_ignore_response(&mut self, command: &str) -> Result<()> {
        self.send_command(command).await.map(|_| ())
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }
}

impl Default for MultiCommandSender {
    fn default() -> Self {
        Self::new()
    }
}