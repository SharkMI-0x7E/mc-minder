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
}

fn default_jar() -> String { "fabric-server.jar".to_string() }
fn default_min_mem() -> String { "512M".to_string() }
fn default_max_mem() -> String { "1G".to_string() }
fn default_session_name() -> String { "mc_server".to_string() }
fn default_log_file() -> String { "logs/latest.log".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            jar: default_jar(),
            min_mem: default_min_mem(),
            max_mem: default_max_mem(),
            session_name: default_session_name(),
            log_file: default_log_file(),
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
        let s = "# MC-Minder Configuration File\n[jvm]\ngc = \"G1GC\"\nextra_flags = \"\"\njdk_path = \"\"  # Optional: Custom JDK path\n# xmx = \"2G\"  # Uncomment to override server.max_mem\n# xms = \"512M\"  # Uncomment to override server.min_mem\n";
        s.to_string()
    }
}