use serde::Deserialize;
use std::path::PathBuf;
use anyhow::{Result, Context};
// use log::warn;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rcon: RconConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
    #[serde(default)]
    #[allow(dead_code)]
    pub jvm: JvmConfig,
    #[serde(default)]
    pub mc_status: McStatusConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RconConfig {
    #[serde(default = "default_rcon_host")]
    pub host: String,
    #[serde(default = "default_rcon_port")]
    pub port: u16,
    pub password: String,
}

fn default_rcon_host() -> String { "127.0.0.1".to_string() }
fn default_rcon_port() -> u16 { 25575 }

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_jar")]
    pub jar: String,
    #[serde(default = "default_min_mem")]
    pub min_mem: String,
    #[serde(default = "default_max_mem")]
    pub max_mem: String,
    #[serde(default = "default_session_name")]
    pub session_name: String,
    #[serde(default = "default_log_file")]
    pub log_file: String,
    /// Server type: fabric, paper, vanilla, forge, etc. (reserved for future multi-type support)
    #[serde(default = "default_server_type")]
    #[allow(dead_code)]
    pub server_type: String,
}

fn default_jar() -> String { "fabric-server.jar".to_string() }
fn default_min_mem() -> String { "512M".to_string() }
fn default_max_mem() -> String { "1G".to_string() }
fn default_session_name() -> String { "mc_server".to_string() }
fn default_log_file() -> String { "logs/latest.log".to_string() }
fn default_server_type() -> String { "fabric".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            jar: default_jar(),
            min_mem: default_min_mem(),
            max_mem: default_max_mem(),
            session_name: default_session_name(),
            log_file: default_log_file(),
            server_type: default_server_type(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackupConfig {
    #[serde(default = "default_world_dir")]
    pub world_dir: String,
    #[serde(default = "default_backup_dest")]
    pub backup_dest: String,
    #[serde(default = "default_retain_days")]
    pub retain_days: u32,
}

fn default_world_dir() -> String { "world".to_string() }
fn default_backup_dest() -> String { "../backups".to_string() }
fn default_retain_days() -> u32 { 7 }

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            world_dir: default_world_dir(),
            backup_dest: default_backup_dest(),
            retain_days: default_retain_days(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct NotificationConfig {
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default)]
    pub telegram_chat_id: String,
    #[serde(default = "default_termux_notify")]
    pub termux_notify: bool,
}

fn default_termux_notify() -> bool { true }

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            termux_notify: default_termux_notify(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct JvmConfig {
    #[serde(default = "default_gc")]
    pub gc: String,
    #[serde(default)]
    pub extra_flags: String,
    #[serde(default)]
    pub xmx: Option<String>,
    #[serde(default)]
    pub xms: Option<String>,
    #[serde(default)]
    pub jdk_path: Option<String>,
}

fn default_gc() -> String { "G1GC".to_string() }

#[derive(Debug, Deserialize, Clone)]
pub struct McStatusConfig {
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,
    #[serde(default = "default_ping_timeout")]
    pub ping_timeout_secs: u64,
}

fn default_ping_interval() -> u64 { 60 }
fn default_ping_timeout() -> u64 { 3 }

impl Default for McStatusConfig {
    fn default() -> Self {
        Self {
            ping_interval_secs: default_ping_interval(),
            ping_timeout_secs: default_ping_timeout(),
        }
    }
}

/// Discover the Minecraft server port from server.properties.
/// Falls back to 25565 if the file is missing or unreadable.
pub fn discover_minecraft_port(server_dir: &std::path::Path) -> (u16, Option<String>) {
    let props_path = server_dir.join("server.properties");
    match std::fs::read_to_string(&props_path) {
        Ok(content) => {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    if key.trim() == "server-port" || key.trim() == "query.port" {
                        if let Ok(port) = value.trim().parse::<u16>() {
                            return (port, None);
                        }
                    }
                }
            }
            (25565, Some("server.properties found but no server-port defined, using default 25565".to_string()))
        }
        Err(_) => (25565, Some("server.properties not found, using default port 25565".to_string())),
    }
}

impl Default for JvmConfig {
    fn default() -> Self {
        Self {
            gc: default_gc(),
            extra_flags: String::new(),
            xmx: None,
            xms: None,
            jdk_path: None,
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        Self::load_from_str(&content)
    }

    pub fn load_from_str(content: &str) -> Result<Self> {
        let config: Config = toml::from_str(content)
            .with_context(|| "Failed to parse config file")?;
        
        Ok(config)
    }

    pub fn generate_template() -> String {
        let s = r#"# MC-Minder Configuration File

[server]
jar = "fabric-server.jar"
min_mem = "512M"
max_mem = "1G"
session_name = "mc_server"
log_file = "logs/latest.log"

[rcon]
host = "127.0.0.1"
port = 25575
password = ""

[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true

[jvm]
gc = "G1GC"
extra_flags = ""
# jdk_path = "/usr/lib/jvm/java-17-openjdk/bin/java"
"#;
        s.to_string()
    }
}