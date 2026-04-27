use serde::Deserialize;
use std::path::PathBuf;
use anyhow::{Result, Context};
// use log::warn;

// Pre-declared types for Config struct
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleEntry {
    pub interval_mins: u64,
    pub action: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WatchdogConfig {
    #[serde(default = "default_watchdog_enabled")]
    pub enabled: bool,
    #[serde(default = "default_watchdog_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_watchdog_cooldown")]
    pub cooldown_secs: u64,
    #[serde(default = "default_watchdog_check_interval")]
    pub check_interval_secs: u64,
}
fn default_watchdog_enabled() -> bool { false }
fn default_watchdog_max_restarts() -> u32 { 5 }
fn default_watchdog_cooldown() -> u64 { 30 }
fn default_watchdog_check_interval() -> u64 { 60 }
impl Default for WatchdogConfig {
    fn default() -> Self {
        Self { enabled: false, max_restarts: 5, cooldown_secs: 30, check_interval_secs: 60 }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LazyStartConfig {
    #[serde(default = "default_lazy_enabled")]
    pub enabled: bool,
    #[serde(default = "default_lazy_port")]
    pub listen_port: u16,
    #[serde(default = "default_lazy_idle")]
    pub idle_timeout_mins: u64,
}
fn default_lazy_enabled() -> bool { false }
fn default_lazy_port() -> u16 { 25565 }
fn default_lazy_idle() -> u64 { 10 }
impl Default for LazyStartConfig {
    fn default() -> Self {
        Self { enabled: false, listen_port: 25565, idle_timeout_mins: 10 }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub servers: Vec<ServerInstance>,
    #[serde(default)]
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
    #[serde(default)]
    pub schedules: Vec<ScheduleEntry>,
    #[serde(default)]
    pub watchdog: WatchdogConfig,
    #[serde(default)]
    pub lazy_start: LazyStartConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RconConfig {
    #[serde(default = "default_rcon_host")]
    pub host: String,
    #[serde(default = "default_rcon_port")]
    pub port: u16,
    #[serde(default)]
    pub password: String,
}

impl Default for RconConfig {
    fn default() -> Self {
        Self {
            host: default_rcon_host(),
            port: default_rcon_port(),
            password: String::new(),
        }
    }
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
    /// Returns the list of active server instances.
    /// If [[servers]] is configured, returns those.
    /// Otherwise, falls back to the legacy single-server config.
    #[allow(dead_code)]
    pub fn get_servers(&self) -> Vec<ServerInstance> {
        if !self.servers.is_empty() {
            return self.servers.clone();
        }
        // Legacy mode: create a single instance from top-level config
        vec![ServerInstance {
            name: "default".to_string(),
            dir: ".".to_string(),
            server: self.server.clone(),
            rcon: Some(self.rcon.clone()),
            jvm: self.jvm.clone(),
        }]
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        Self::load_from_str(&content)
    }

    pub fn load_from_str(content: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(content)
            .with_context(|| "Failed to parse config file")?;

        // Backward compat: if no servers configured and no rcon password,
        // fill in defaults from top-level config
        if config.servers.is_empty() && config.rcon.password.is_empty() {
            config.rcon = RconConfig::default();
        }
        
        Ok(config)
    }

    pub fn generate_template() -> String {
        let s = r#"# MC-Minder Configuration File
# 单服务器配置（传统模式）
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

# 多服务器配置（取消注释启用）
# [[servers]]
# name = "survival"
# dir = "./survival"
# [servers.server]
# jar = "fabric-server.jar"
# min_mem = "1G"
# max_mem = "2G"
# [servers.rcon]
# password = "secret1"
"#;
        s.to_string()
    }
}

/// A managed Minecraft server instance.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ServerInstance {
    /// Display name for this server
    #[serde(default = "default_instance_name")]
    pub name: String,
    /// Directory containing this server's files (relative to config.toml)
    #[serde(default = "default_instance_dir")]
    pub dir: String,
    /// Server configuration (jar, memory, etc.)
    #[serde(default)]
    pub server: ServerConfig,
    /// RCON configuration (optional override)
    #[serde(default)]
    pub rcon: Option<RconConfig>,
    /// JVM configuration
    #[serde(default)]
    pub jvm: JvmConfig,
}

fn default_instance_name() -> String { "server".to_string() }
fn default_instance_dir() -> String { ".".to_string() }

/// Auto-discover server instances in subdirectories.
/// Returns a list of discovered servers with their names and directories.
pub fn discover_servers(base_dir: &std::path::Path) -> Vec<DiscoveredServer> {
    let mut servers = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Check if this directory looks like a Minecraft server
            let has_jar = path.read_dir().map(|d| {
                d.flatten().any(|e| {
                    e.file_name().to_string_lossy().contains("server")
                        || e.file_name().to_string_lossy().ends_with(".jar")
                })
            }).unwrap_or(false);
            let has_props = path.join("server.properties").exists();
            if has_jar || has_props {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                servers.push(DiscoveredServer {
                    name,
                    dir: path.to_string_lossy().to_string(),
                });
            }
        }
    }
    servers
}

/// Result of auto-discovering a server instance.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    pub name: String,
    pub dir: String,
}