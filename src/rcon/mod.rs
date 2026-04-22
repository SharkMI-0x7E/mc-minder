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
        // RCON 协议：length = 4(id) + 4(type) + payload_len + 1(null)
        let length = 4 + 4 + payload_bytes.len() as i32 + 1;

        let mut buf = Vec::with_capacity(4 + length as usize);
        // RCON 使用小端序 (Little Endian)
        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&packet.id.to_le_bytes());
        buf.extend_from_slice(&packet.packet_type.to_le_bytes());
        buf.extend_from_slice(payload_bytes);
        buf.push(0);  // 只有 1 个 null 字节

        timeout(WRITE_TIMEOUT, stream.write_all(&buf))
            .await
            .context("RCON write timeout")?
            .context("Failed to write to RCON stream")?;

        let _ = timeout(WRITE_TIMEOUT, stream.flush())
            .await
            .context("RCON flush timeout")?;

        debug!("Sent RCON packet: id={}, type={}, length={}", packet.id, packet.packet_type, length);
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
        
        // 读取长度字段（小端序）
        let mut length_buf = [0u8; 4];
        timeout(READ_TIMEOUT, stream.read_exact(&mut length_buf))
            .await
            .context("RCON read length timeout")?
            .context("Failed to read packet length")?;
        let length = i32::from_le_bytes(length_buf);
        
        if !(10..=4096).contains(&length) {
            bail!("Invalid RCON packet length: {}", length);
        }
        
        // 读取剩余数据（id + type + payload + null）
        let mut data = vec![0u8; length as usize];
        timeout(READ_TIMEOUT, stream.read_exact(&mut data))
            .await
            .context("RCON read packet data timeout")?
            .context("Failed to read packet data")?;
        
        // 解析字段（小端序）
        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let packet_type = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        
        // payload 长度 = total - 4(id) - 4(type) - 1(null)
        let payload_length = (length - 4 - 4 - 1) as usize;
        let payload = if payload_length > 0 {
            String::from_utf8_lossy(&data[8..8 + payload_length])
                .trim_end_matches('\0')
                .to_string()
        } else {
            String::new()
        };
        
        debug!("Received RCON packet: id={}, type={}, payload_len={}, payload='{}'", 
               id, packet_type, payload.len(), 
               if payload.len() > 50 { &payload[..50] } else { &payload });
        
        Ok(Packet {
            id,
            packet_type,
            payload,
        })
    }

    #[allow(dead_code)]
    pub async fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
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
