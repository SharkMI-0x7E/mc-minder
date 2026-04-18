use anyhow::{Result, Context};
use log::{info, debug};
use std::io::{Read, Write};
use std::net::TcpStream;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::time::Duration;

pub struct RconClient {
    host: String,
    port: u16,
    password: String,
    stream: Option<TcpStream>,
}

const PACKET_TYPE_AUTH: i32 = 3;
const PACKET_TYPE_EXEC_COMMAND: i32 = 2;

struct Packet {
    id: i32,
    packet_type: i32,
    payload: String,
}

impl RconClient {
    pub fn new(host: String, port: u16, password: String) -> Self {
        Self {
            host,
            port,
            password,
            stream: None,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        info!("Connecting to RCON at {}", addr);
        
        let stream = TcpStream::connect_timeout(
            &addr.parse().context("Invalid RCON address")?,
            Duration::from_secs(5),
        ).context("Failed to connect to RCON server")?;

        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        
        self.stream = Some(stream);
        
        self.authenticate()?;
        
        info!("RCON authenticated successfully");
        Ok(())
    }

    fn authenticate(&mut self) -> Result<()> {
        let packet = Packet {
            id: 1,
            packet_type: PACKET_TYPE_AUTH,
            payload: self.password.clone(),
        };
        
        self.send_packet(&packet)?;
        let response = self.read_packet()?;
        
        if response.id == -1 {
            anyhow::bail!("RCON authentication failed");
        }
        
        Ok(())
    }

    pub fn execute(&mut self, command: &str) -> Result<String> {
        if self.stream.is_none() {
            self.connect()?;
        }

        let packet = Packet {
            id: 1,
            packet_type: PACKET_TYPE_EXEC_COMMAND,
            payload: command.to_string(),
        };
        
        self.send_packet(&packet)?;
        let response = self.read_packet()?;
        
        Ok(response.payload)
    }

    fn send_packet(&mut self, packet: &Packet) -> Result<()> {
        let stream = self.stream.as_mut().context("Not connected to RCON")?;
        
        let payload_bytes = packet.payload.as_bytes();
        let length = 10 + payload_bytes.len() as i32;
        
        stream.write_i32::<BigEndian>(length)?;
        stream.write_i32::<BigEndian>(packet.id)?;
        stream.write_i32::<BigEndian>(packet.packet_type)?;
        stream.write_all(payload_bytes)?;
        stream.write_all(&[0, 0])?;
        stream.flush()?;
        
        debug!("Sent RCON packet: id={}, type={}", packet.id, packet.packet_type);
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Packet> {
        let stream = self.stream.as_mut().context("Not connected to RCON")?;
        
        let length = stream.read_i32::<BigEndian>()?;
        let id = stream.read_i32::<BigEndian>()?;
        let packet_type = stream.read_i32::<BigEndian>()?;
        
        let payload_length = (length - 10) as usize;
        if payload_length > 4096 {
            anyhow::bail!("RCON packet too large");
        }
        
        let mut payload = vec![0u8; payload_length];
        stream.read_exact(&mut payload)?;
        
        let mut padding = [0u8; 2];
        stream.read_exact(&mut padding)?;
        
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

    pub fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            info!("Disconnected from RCON");
        }
    }

    pub fn say(&mut self, message: &str) -> Result<()> {
        let command = format!("say {}", message);
        self.execute(&command)?;
        Ok(())
    }

    pub fn tell(&mut self, player: &str, message: &str) -> Result<()> {
        let command = format!("tellraw {} {{\"text\":\"{}\"}}", player, message);
        self.execute(&command)?;
        Ok(())
    }
}

impl Drop for RconClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
