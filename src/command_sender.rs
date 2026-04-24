use anyhow::{Context, Result};
use log::{debug, warn, info};
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub enum CommandSenderMode {
    Rcon { host: String, port: u16, password: String },
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

    pub fn stdin(stdin: ChildStdin) -> Self {
        Self { mode: CommandSenderMode::Stdin { stdin: std::sync::Arc::new(Mutex::new(stdin)) } }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<()> {
        match &mut self.mode {
            CommandSenderMode::Rcon { host, port, password } => {
                RconCommandSender::new(host.clone(), *port, password.clone()).send_command(command).await
            }
            CommandSenderMode::Stdin { stdin } => {
                StdinCommandSender::new(stdin.clone()).send_command(command).await
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match &self.mode {
            CommandSenderMode::Rcon { .. } => "RCON",
            CommandSenderMode::Stdin { .. } => "Stdin",
        }
    }
}

pub struct RconCommandSender {
    host: String,
    port: u16,
    password: String,
}

impl RconCommandSender {
    pub fn new(host: String, port: u16, password: String) -> Self {
        Self { host, port, password }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<()> {
        const MAX_RETRIES: u32 = 3;

        for attempt in 1..=MAX_RETRIES {
            match self.execute_with_retry(command).await {
                Ok(_) => {
                    debug!("[RconSender] Command sent successfully: {}", command);
                    return Ok(());
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

    async fn execute_with_retry(&mut self, command: &str) -> Result<()> {
        use tokio::net::TcpStream;

        const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

        let addr = format!("{}:{}", self.host, self.port);
        debug!("[RconSender] Connecting to RCON at {}", addr);

        let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
            .await
            .context("RCON connection timeout")?
            .context(format!(
                "Failed to connect to RCON at {}. Please check:\n\
                 - enable-rcon=true\n\
                 - rcon.port={}\n\
                 - rcon.password=<password>",
                addr, self.port
            ))?;

        self.authenticate(&mut stream).await?;
        self.send_packet(&mut stream, 1, 2, command).await?;

        Ok(())
    }

    async fn authenticate(&self, stream: &mut tokio::net::TcpStream) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        const PACKET_TYPE_AUTH: i32 = 3;

        let payload_bytes = self.password.as_bytes();
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&1_i32.to_le_bytes());
        buf.extend_from_slice(&PACKET_TYPE_AUTH.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);

        stream.write_all(&buf).await.context("RCON auth write failed")?;
        stream.flush().await.context("RCON auth flush failed")?;

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.context("RCON auth read length failed")?;
        let resp_len = i32::from_le_bytes(len_buf);

        if resp_len < 10 {
            return Err(anyhow::anyhow!("Invalid auth response length: {}", resp_len));
        }

        let mut data = vec![0u8; resp_len as usize];
        stream.read_exact(&mut data).await.context("RCON auth read data failed")?;

        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if id == -1 {
            return Err(anyhow::anyhow!("RCON authentication rejected"));
        }

        debug!("[RconSender] Authenticated successfully");
        Ok(())
    }

    async fn send_packet(&self, stream: &mut tokio::net::TcpStream, packet_id: i32, packet_type: i32, payload: &str) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
            .context("RCON write timeout")?;
        write_result.context("RCON write failed")?;

        stream.flush().await.context("RCON flush failed")?;

        let mut len_buf = [0u8; 4];
        let read_result = timeout(READ_TIMEOUT, stream.read_exact(&mut len_buf))
            .await
            .context("RCON read timeout")?;
        read_result.context("RCON read length failed")?;

        let resp_len = i32::from_le_bytes(len_buf);
        if resp_len >= 10 {
            let mut data = vec![0u8; resp_len as usize];
            let read_result = timeout(READ_TIMEOUT, stream.read_exact(&mut data))
                .await
                .context("RCON read timeout")?;
            read_result.context("RCON read data failed")?;
        }

        debug!("[RconSender] Packet sent: id={}, type={}, cmd={}", packet_id, packet_type, payload);
        Ok(())
    }
}

pub struct StdinCommandSender {
    stdin: std::sync::Arc<Mutex<ChildStdin>>,
}

impl StdinCommandSender {
    pub fn new(stdin: std::sync::Arc<Mutex<ChildStdin>>) -> Self {
        Self { stdin }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(command.as_bytes()).await.context("Stdin write failed")?;
        stdin.write_all(b"\n").await.context("Stdin newline failed")?;
        stdin.flush().await.context("Stdin flush failed")?;
        debug!("[StdinSender] Command sent: {}", command);
        Ok(())
    }
}

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

    pub async fn send_command(&mut self, command: &str) -> Result<()> {
        if self.senders.is_empty() {
            return Err(anyhow::anyhow!("No command senders available"));
        }

        let mut last_error = None;
        for sender in &mut self.senders {
            match sender.send_command(command).await {
                Ok(_) => {
                    info!("[MultiSender] Command sent via {}: {}", sender.name(), command);
                    return Ok(());
                }
                Err(e) => {
                    warn!("[MultiSender] {} failed: {}", sender.name(), e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All senders failed")))
    }

    pub fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }
}

impl Default for MultiCommandSender {
    fn default() -> Self {
        Self::new()
    }
}
