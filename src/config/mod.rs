use serde::Deserialize;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rcon: RconConfig,
    pub ai: Option<AiConfig>,
    pub ollama: Option<OllamaConfig>,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
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
pub struct AiConfig {
    pub api_url: String,
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_trigger")]
    pub trigger: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_model() -> String { "gpt-3.5-turbo".to_string() }
fn default_trigger() -> String { "!".to_string() }
fn default_max_tokens() -> u32 { 150 }
fn default_temperature() -> f32 { 0.7 }

#[derive(Debug, Deserialize, Clone)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

fn default_ollama_enabled() -> bool { false }
fn default_ollama_url() -> String { "http://localhost:11434/api/generate".to_string() }
fn default_ollama_model() -> String { "qwen:0.5b".to_string() }

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

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        Self::load_from_str(&content)
    }

    pub fn load_from_str(content: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(content)
            .with_context(|| "Failed to parse config file")?;
        
        if let Some(ref ai) = config.ai {
            if ai.api_key.is_empty() || ai.api_url.is_empty() {
                config.ai = None;
            }
        }
        
        Ok(config)
    }

    pub fn generate_template() -> String {
        r#"# MC-Minder Configuration File
# MC-Minder 配置文件

# Server Configuration
# 服务器配置
[server]
jar = "fabric-server.jar"
min_mem = "512M"
max_mem = "1G"
session_name = "mc_server"
log_file = "logs/latest.log"

# RCON Configuration - Required for MC-Minder to communicate with Minecraft server
# RCON 配置 - MC-Minder 与 Minecraft 服务器通信必需
[rcon]
host = "127.0.0.1"
port = 25575
password = "your_rcon_password"

# AI Configuration - Leave empty or remove this section to disable AI features
# AI 配置 - 留空或删除此部分可禁用 AI 功能
[ai]
api_url = ""
api_key = ""
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7

# Ollama Configuration - Set enabled = true to use local AI
# Ollama 配置 - 设置 enabled = true 使用本地 AI
[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"

# Backup Configuration
# 备份配置
[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

# Notification Configuration - Leave empty to disable notifications
# 通知配置 - 留空禁用通知功能
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
"#.to_string()
    }
}
