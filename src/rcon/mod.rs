use anyhow::{Context, Result, bail};
use log::{info, debug, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use std::sync::atomic::{AtomicBool, Ordering};

const PACKET_TYPE_AUTH: i32 = 3;
const PACKET_TYPE_EXEC_COMMAND: i32 = 2;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

struct Packet {
    id: i32,
    packet_type: i32,
    payload: String,
}

pub struct RconClient {
    host: String,
    port: u16,
    password: String,
    stream: Option<TcpStream>,
    connected: AtomicBool,
}

impl RconClient {
    pub fn new(host: String, port: u16, password: String) -> Self {
        Self {
            host,
            port,
            password,
            stream: None,
            connected: AtomicBool::new(false),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        info!("Connecting to RCON at {}", addr);
        
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr))
            .await
            .context("RCON connection timeout - please check if Minecraft server is running")?
            .context(format!(
                "Failed to connect to RCON at {}. \n\
                 Please check server.properties:\n\
                 - enable-rcon=true\n\
                 - rcon.port=25575\n\
                 - rcon.password=<your_password>",
                addr
            ))?;

        self.stream = Some(stream);
        
        if let Err(e) = self.authenticate().await {
            self.stream = None;
            self.connected.store(false, Ordering::Relaxed);
            bail!(
                "RCON authentication failed. \n\
                 Please check that the password in config.toml matches rcon.password in server.properties.\n\
                 Error: {}", e
            );
        }
        
        self.connected.store(true, Ordering::Relaxed);
        info!("RCON authenticated successfully");
        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        let packet = Packet {
            id: 1,
            packet_type: PACKET_TYPE_AUTH,
            payload: self.password.clone(),
        };

        self.try_send(&packet).await?;
        let response = self.read_packet().await?;

        if response.id == -1 {
            bail!("Authentication rejected by server");
        }

        Ok(())
    }

    async fn ensure_connected(&mut self) -> Result<()> {
        if !self.is_connected() {
            debug!("RCON not connected, attempting to reconnect...");
            self.connect().await?;
        }
        Ok(())
    }

    /// 尝试发送数据包（不包含重连逻辑）
    async fn try_send(&mut self, packet: &Packet) -> Result<()> {
        let stream = self.stream.as_mut().context("Not connected to RCON")?;

        let payload_bytes = packet.payload.as_bytes();
        let length = 10 + payload_bytes.len() as i32;

        let mut buf = Vec::with_capacity(length as usize + 4);
        buf.extend_from_slice(&length.to_be_bytes());
        buf.extend_from_slice(&packet.id.to_be_bytes());
        buf.extend_from_slice(&packet.packet_type.to_be_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.extend_from_slice(&[0, 0]);

        timeout(WRITE_TIMEOUT, stream.write_all(&buf))
            .await
            .context("RCON write timeout")?
            .context("Failed to write to RCON stream")?;

        timeout(WRITE_TIMEOUT, stream.flush())
            .await
            .context("RCON flush timeout")?;

        debug!("Sent RCON packet: id={}, type={}", packet.id, packet.packet_type);
        Ok(())
    }

    pub async fn execute(&mut self, command: &str) -> Result<String> {
        self.ensure_connected().await?;

        let packet = Packet {
            id: 1,
            packet_type: PACKET_TYPE_EXEC_COMMAND,
            payload: command.to_string(),
        };

        // 首次尝试发送
        if let Err(e) = self.try_send(&packet).await {
            warn!("RCON send failed, attempting reconnect: {}", e);
            self.stream = None;
            self.connected.store(false, Ordering::Relaxed);
            // 重连并重试一次
            self.connect().await?;
            self.try_send(&packet).await?;
        }

        // 读取响应
        match self.read_packet().await {
            Ok(response) => Ok(response.payload),
            Err(e) => {
                warn!("RCON read failed: {}", e);
                self.stream = None;
                self.connected.store(false, Ordering::Relaxed);
                bail!("RCON read failed: {}", e);
            }
        }
    }

    async fn read_packet(&mut self) -> Result<Packet> {
        let stream = self.stream.as_mut().context("Not connected to RCON")?;
        
        let mut length_buf = [0u8; 4];
        timeout(READ_TIMEOUT, stream.read_exact(&mut length_buf))
            .await
            .context("RCON read length timeout")?
            .context("Failed to read packet length")?;
        let length = i32::from_be_bytes(length_buf);
        
        let mut header_buf = [0u8; 8];
        timeout(READ_TIMEOUT, stream.read_exact(&mut header_buf))
            .await
            .context("RCON read header timeout")?
            .context("Failed to read packet header")?;
        let id = i32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
        let packet_type = i32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);
        
        let payload_length = (length - 10) as usize;
        if payload_length > 4096 {
            bail!("RCON packet too large: {} bytes", payload_length);
        }
        
        let mut payload = vec![0u8; payload_length];
        if payload_length > 0 {
            timeout(READ_TIMEOUT, stream.read_exact(&mut payload))
                .await
                .context("RCON read payload timeout")?
                .context("Failed to read packet payload")?;
        }
        
        let mut padding = [0u8; 2];
        timeout(READ_TIMEOUT, stream.read_exact(&mut padding))
            .await
            .context("RCON read padding timeout")?
            .context("Failed to read packet padding")?;
        
        let payload = String::from_utf8_lossy(&payload)
            .trim_end_matches('\0')
            .to_string();
        
        debug!("Received RCON packet: id={}, type={}, payload_len={}", id, packet_type, payload.len());
        
        Ok(Packet {
            id,
            packet_type,
            payload,
        })
    }

    pub async fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown().await;
            self.connected.store(false, Ordering::Relaxed);
            info!("Disconnected from RCON");
        }
    }

    pub async fn say(&mut self, message: &str) -> Result<()> {
        let command = format!("say {}", message);
        self.execute(&command).await?;
        Ok(())
    }

    pub async fn tell(&mut self, player: &str, message: &str) -> Result<()> {
        let safe_message = message
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        let command = format!("tellraw {} {{\"text\":\"{}\"}}", player, safe_message);
        self.execute(&command).await?;
        Ok(())
    }
}
