use serde::Deserialize;
use std::path::PathBuf;
use anyhow::{Result, Context};
use log::{warn, debug};

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
pub struct AiConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
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

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            api_key: String::new(),
            model: default_model(),
            trigger: default_trigger(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
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
        let mut config: Config = toml::from_str(content)
            .with_context(|| "Failed to parse config file")?;
        
        // 检查 AI 配置
        let using_ollama = config.ollama.as_ref().map(|o| o.enabled).unwrap_or(false);
        
        if let Some(ref mut ai) = config.ai {
            if using_ollama {
                // Ollama 模式：确保 ollama URL 有效
                if let Some(ref mut ollama) = config.ollama {
                    if ollama.url.is_empty() {
                        ollama.url = "http://localhost:11434".to_string();
                    }
                }
                debug!("AI mode: Ollama (model: {})", 
                    config.ollama.as_ref().map(|o| o.model.clone()).unwrap_or_default());
            } else {
                // OpenAI 模式：检查必需字段
                if ai.api_key.is_empty() || ai.api_url.is_empty() {
                    warn!("AI configuration incomplete for OpenAI mode (api_key or api_url is empty). AI features will be disabled.");
                    config.ai = None;
                } else {
                    debug!("AI mode: OpenAI-compatible (model: {})", ai.model);
                }
            }
        } else if using_ollama {
            // 用户启用了 Ollama 但没有 [ai] 部分，创建默认的 AiConfig
            debug!("Ollama enabled but no [ai] section found, creating default AI config");
            config.ai = Some(AiConfig::default());
        }
        
        Ok(config)
    }

    pub fn generate_template() -> String {
        // Minimal, safe template with jdk_path option (non-raw string for stability)
        let s = "# MC-Minder Configuration File\n[jvm]\ngc = \"G1GC\"\nextra_flags = \"\"\njdk_path = \"\"  # Optional: Custom JDK path\n# xmx = \"2G\"  # Uncomment to override server.max_mem\n# xms = \"512M\"  # Uncomment to override server.min_mem\n";
        s.to_string()
    }
}
