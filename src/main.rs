use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Input, Confirm};
use log::{info, warn, error};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

mod config;
mod monitor;
mod ai;
mod rcon;
mod context;
mod api;

use config::Config;
use monitor::{LogMonitor, LogEvent};
use ai::{AiClient, ChatResult};
use rcon::RconClient;
use context::ContextManager;
use api::HttpApi;

const DEFAULT_CONFIG_PATH: &str = "config.toml";
const LOG_FILE_PATH: &str = "logs/mc-minder.log";
const LOG_FILE_MAX_SIZE: u64 = 50 * 1024 * 1024;

#[derive(Parser, Debug)]
#[command(name = "mc-minder")]
#[command(author = "SharkMI-0x7E")]
#[command(version = "0.3.0")]
#[command(about = "A smart management suite for Minecraft Fabric servers")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, value_name = "PATH", default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "8080")]
    http_port: u16,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(about = "Initialize configuration interactively")]
    Init,
    
    #[command(about = "Generate default config file")]
    GenConfig,
    
    #[command(about = "Generate start.sh script")]
    GenStart,
    
    #[command(about = "Generate backup.sh script")]
    GenBackup,
    
    #[command(about = "Update to the latest version")]
    SelfUpdate,
    
    #[command(about = "Show current configuration")]
    Config,
    
    #[command(about = "Get a config value by key")]
    ConfigGet {
        #[arg(required = true)]
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(ref cmd) = args.command {
        return handle_command(cmd.clone(), &args).await;
    }

    init_logger(args.verbose)?;
    
    print_banner();
    
    run_server(args).await
}

fn print_banner() {
    println!("{}", "MC-Minder v0.3.0".green().bold());
    println!("{}", "A smart management suite for Minecraft Fabric servers".dimmed());
    println!();
}

fn init_logger(verbose: bool) -> Result<()> {
    let log_dir = PathBuf::from("logs");
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }

    let log_file = PathBuf::from(LOG_FILE_PATH);
    if log_file.exists() {
        let metadata = fs::metadata(&log_file)?;
        if metadata.len() > LOG_FILE_MAX_SIZE {
            let backup_path = format!("{}.old", LOG_FILE_PATH);
            let _ = fs::rename(&log_file, &backup_path);
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    let log_level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format(|buf, record| {
            use std::io::Write;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(buf, "[{} {}] {}", timestamp, record.level(), record.args())
        })
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    Ok(())
}

async fn handle_command(cmd: Commands, args: &Args) -> Result<()> {
    match cmd {
        Commands::Init => run_init().await,
        Commands::GenConfig => generate_config(&args.config),
        Commands::GenStart => generate_start_script(),
        Commands::GenBackup => generate_backup_script(),
        Commands::SelfUpdate => run_self_update().await,
        Commands::Config => show_config(&args.config),
        Commands::ConfigGet { key } => get_config_value(&args.config, &key),
    }
}

async fn run_init() -> Result<()> {
    println!("{}", "MC-Minder Initialization".green().bold());
    println!("This will help you set up MC-Minder for your Minecraft server.\n");

    let rcon_password: String = Input::new()
        .with_prompt("RCON password (from server.properties)")
        .default(String::new())
        .interact()?;

    if rcon_password.is_empty() {
        println!("{}", "Warning: RCON password is empty. Please set it in server.properties first.".yellow());
    }

    let use_ai = Confirm::new()
        .with_prompt("Enable AI chat features?")
        .default(false)
        .interact()?;

    let (api_url, api_key, use_ollama) = if use_ai {
        let use_ollama = Confirm::new()
            .with_prompt("Use local Ollama instead of OpenAI-compatible API?")
            .default(false)
            .interact()?;

        if use_ollama {
            (String::new(), String::new(), true)
        } else {
            let api_url: String = Input::new()
                .with_prompt("AI API URL (e.g., https://api.openai.com/v1/chat/completions)")
                .default("https://api.openai.com/v1/chat/completions".to_string())
                .interact()?;
            
            let api_key: String = Input::new()
                .with_prompt("API Key")
                .interact()?;
            
            (api_url, api_key, false)
        }
    } else {
        (String::new(), String::new(), false)
    };

    let min_mem: String = Input::new()
        .with_prompt("Minimum memory for Minecraft server")
        .default("512M".to_string())
        .interact()?;

    let max_mem: String = Input::new()
        .with_prompt("Maximum memory for Minecraft server")
        .default("1G".to_string())
        .interact()?;

    let session_name: String = Input::new()
        .with_prompt("tmux session name")
        .default("mc_server".to_string())
        .interact()?;

    let config_content = generate_config_content(
        &rcon_password,
        &api_url,
        &api_key,
        use_ai && !use_ollama,
        use_ai && use_ollama,
        &min_mem,
        &max_mem,
        &session_name,
    );

    let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
    fs::write(&config_path, &config_content)?;
    println!("\n{} Created {}", "✓".green(), config_path.display());

    generate_start_script()?;
    generate_backup_script()?;

    println!("\n{}", "Initialization complete!".green().bold());
    println!("\nNext steps:");
    println!("  1. Ensure RCON is enabled in server.properties:");
    println!("     enable-rcon=true");
    println!("     rcon.port=25575");
    println!("     rcon.password=<your_password>");
    println!("  2. Place fabric-server.jar in the current directory");
    println!("  3. Run: ./start.sh start");

    Ok(())
}

fn generate_config_content(
    rcon_password: &str,
    api_url: &str,
    api_key: &str,
    enable_ai: bool,
    enable_ollama: bool,
    min_mem: &str,
    max_mem: &str,
    session_name: &str,
) -> String {
    let ai_section = if enable_ai {
        format!(
r#"[ai]
api_url = "{}"
api_key = "{}"
model = "gpt-3.5-turbo"
trigger = "!"
max_tokens = 150
temperature = 0.7
"#, api_url, api_key)
    } else {
        String::new()
    };

    let ollama_section = if enable_ollama {
        String::from(
r#"[ollama]
enabled = true
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"
"#)
    } else {
        String::from(
r#"[ollama]
enabled = false
url = "http://localhost:11434/api/generate"
model = "qwen:0.5b"
"#)
    };

    format!(
r#"# MC-Minder Configuration File
# MC-Minder 配置文件

# Server Configuration
# 服务器配置
[server]
jar = "fabric-server.jar"
min_mem = "{}"
max_mem = "{}"
session_name = "{}"
log_file = "logs/latest.log"

# RCON Configuration
# RCON 配置
[rcon]
host = "127.0.0.1"
port = 25575
password = "{}"

{}
{}
# Backup Configuration
# 备份配置
[backup]
world_dir = "world"
backup_dest = "../backups"
retain_days = 7

# Notification Configuration
# 通知配置
[notification]
telegram_bot_token = ""
telegram_chat_id = ""
termux_notify = true
"#, min_mem, max_mem, session_name, rcon_password, ai_section, ollama_section)
}

fn generate_config(path: &PathBuf) -> Result<()> {
    let content = Config::generate_template();
    fs::write(path, &content)?;
    println!("{} Generated config file: {}", "✓".green(), path.display());
    Ok(())
}

fn generate_start_script() -> Result<()> {
    let script = include_str!("../scripts/start.sh");
    let script_path = PathBuf::from("start.sh");
    fs::write(&script_path, script)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }
    
    println!("{} Generated start.sh", "✓".green());
    Ok(())
}

fn generate_backup_script() -> Result<()> {
    let script = include_str!("../scripts/backup.sh");
    let script_path = PathBuf::from("backup.sh");
    fs::write(&script_path, script)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }
    
    println!("{} Generated backup.sh", "✓".green());
    Ok(())
}

async fn run_self_update() -> Result<()> {
    println!("Checking for updates...");
    
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/SharkMI-0x7E/mc-minder/releases/latest")
        .header("User-Agent", "mc-minder")
        .send()
        .await
        .context("Failed to check for updates. Please check your network connection.")?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to check for updates: HTTP {}", response.status());
    }
    
    let release: serde_json::Value = response.json().await?;
    let latest_version = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v');
    
    let current_version = env!("CARGO_PKG_VERSION");
    
    println!("Current version: {}", current_version);
    println!("Latest version: {}", latest_version);
    
    if latest_version == current_version {
        println!("You are already on the latest version!");
        return Ok(());
    }
    
    let confirm = Confirm::new()
        .with_prompt("Update to the latest version?")
        .default(true)
        .interact()?;
    
    if !confirm {
        println!("Update cancelled.");
        return Ok(());
    }
    
    let target = if cfg!(target_os = "android") {
        "aarch64-linux-android"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-musl"
    } else {
        "aarch64-unknown-linux-musl"
    };
    
    let download_url = format!(
        "https://github.com/SharkMI-0x7E/mc-minder/releases/download/v{}/mc-minder-{}",
        latest_version, target
    );
    
    println!("Downloading from: {}", download_url);
    
    let binary_response = client
        .get(&download_url)
        .send()
        .await
        .context("Failed to download update")?;
    
    if !binary_response.status().is_success() {
        anyhow::bail!("Failed to download binary: HTTP {}", binary_response.status());
    }
    
    let binary_data = binary_response.bytes().await?;
    
    let exe_path = std::env::current_exe()?;
    let backup_path = format!("{}.old", exe_path.display());
    
    fs::rename(&exe_path, &backup_path)?;
    fs::write(&exe_path, &binary_data)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe_path, fs::Permissions::from_mode(0o755))?;
    }
    
    println!("{} Updated to version {}!", "✓".green(), latest_version);
    println!("Please restart MC-Minder to use the new version.");
    
    Ok(())
}

fn show_config(path: &PathBuf) -> Result<()> {
    let config = Config::load(path)?;
    println!("Configuration loaded from: {}", path.display());
    println!("{:#?}", config);
    Ok(())
}

fn get_config_value(path: &PathBuf, key: &str) -> Result<()> {
    let config = Config::load(path)?;
    
    let value = match key {
        "rcon.host" => config.rcon.host.clone(),
        "rcon.port" => config.rcon.port.to_string(),
        "rcon.password" => config.rcon.password.clone(),
        "server.jar" => config.server.jar.clone(),
        "server.min_mem" => config.server.min_mem.clone(),
        "server.max_mem" => config.server.max_mem.clone(),
        "server.session_name" => config.server.session_name.clone(),
        "server.log_file" => config.server.log_file.clone(),
        "backup.world_dir" => config.backup.world_dir.clone(),
        "backup.backup_dest" => config.backup.backup_dest.clone(),
        "backup.retain_days" => config.backup.retain_days.to_string(),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    };
    
    println!("{}", value);
    Ok(())
}

async fn run_server(args: Args) -> Result<()> {
    info!("MC-Minder starting up...");

    let config_path = args.config.clone();
    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {:?}", config_path))?;

    info!("Configuration loaded successfully");

    let log_path = PathBuf::from(&config.server.log_file);
    let log_monitor = LogMonitor::new(log_path)?;

    let ai_client = if let Some(ref ai_config) = config.ai {
        Some(AiClient::new(ai_config.clone(), config.ollama.clone())?)
    } else {
        warn!("No AI configuration found, AI features disabled");
        None
    };

    let mut rcon_client = RconClient::new(
        config.rcon.host.clone(),
        config.rcon.port,
        config.rcon.password.clone(),
    );

    let context = Arc::new(ContextManager::new());
    context.add_system_message("You are a helpful Minecraft server assistant. Respond concisely and helpfully to player questions. Keep responses under 100 characters when possible.");

    let rcon = Arc::new(RwLock::new(None));
    {
        let mut rcon_guard = rcon.write().await;
        match rcon_client.connect().await {
            Ok(_) => {
                *rcon_guard = Some(rcon_client);
                info!("RCON connection established");
            }
            Err(e) => {
                warn!("Failed to connect to RCON: {}. Will retry later.", e);
            }
        }
    }

    let mut event_rx = log_monitor.start_monitoring()?;

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let http_api = Arc::new(HttpApi::new(args.http_port, context.clone(), rcon.clone()));
    let mut shutdown_rx = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_api.start(async move { shutdown_rx.recv().await.ok(); }).await {
            error!("HTTP API error: {}", e);
        }
    });

    info!("MC-Minder is running. Press Ctrl+C to stop.");

    let trigger = ai_client.as_ref().map(|a| a.get_trigger().to_string());

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            let _ = shutdown_tx.send(());
        }
        _ = async {
            while let Some(event) = event_rx.recv().await {
                match event {
                    LogEvent::Chat(msg) => {
                        info!("[Chat] {}: {}", msg.player, msg.content);

                        if let (Some(ref ai), Some(ref trig)) = (&ai_client, &trigger) {
                            if msg.content.starts_with(trig) {
                                let question = msg.content.trim_start_matches(trig).trim();
                                if !question.is_empty() {
                                    context.add_user_message(question, &msg.player);

                                    let messages = context.get_messages_for_player(&msg.player);
                                    let player = msg.player.clone();
                                    
                                    match ai.chat(messages, &player).await {
                                        Ok(ChatResult::Success(response)) => {
                                            context.add_assistant_message(&response);
                                            
                                            let mut rcon_guard = rcon.write().await;
                                            if let Some(ref mut rcon_client) = *rcon_guard {
                                                let tell_msg = format!("[AI] {}", response);
                                                if let Err(e) = rcon_client.tell(&msg.player, &tell_msg).await {
                                                    warn!("Failed to send AI response: {}", e);
                                                }
                                            }
                                        }
                                        Ok(ChatResult::RateLimited(rate_limit_msg)) => {
                                            let mut rcon_guard = rcon.write().await;
                                            if let Some(ref mut rcon_client) = *rcon_guard {
                                                let _ = rcon_client.tell(&player, &rate_limit_msg).await;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("AI chat error: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    LogEvent::PlayerJoin(player) => {
                        info!("[Join] {} joined the game", player);
                        let mut rcon_guard = rcon.write().await;
                        if let Some(ref mut rcon_client) = *rcon_guard {
                            let _ = rcon_client.say(&format!("Welcome {}!", player)).await;
                        }
                    }
                    LogEvent::PlayerLeave(player) => {
                        info!("[Leave] {} left the game", player);
                    }
                    LogEvent::PlayerDeath(player) => {
                        info!("[Death] {} died", player);
                    }
                    LogEvent::ServerStart => {
                        info!("[Server] Server started");
                    }
                    LogEvent::ServerStop => {
                        info!("[Server] Server stopped");
                    }
                }
            }
        } => {}
    }

    let _ = tokio::try_join!(http_handle);

    info!("MC-Minder stopped");
    Ok(())
}
