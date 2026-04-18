use anyhow::{Result, Context};
use clap::Parser;
use log::{info, warn, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify, RwLock};

mod config;
mod log_monitor;
mod ai_client;
mod rcon_client;
mod context;
mod http_api;

use config::Config;
use log_monitor::{LogMonitor, LogEvent};
use ai_client::AiClient;
use rcon_client::RconClient;
use context::ContextManager;
use http_api::HttpApi;

#[derive(Parser, Debug)]
#[command(name = "mc-minder")]
#[command(author = "MC-Minder Team")]
#[command(version = "0.1.0")]
#[command(about = "A smart management suite for Minecraft Fabric servers")]
struct Args {
    #[arg(short, long, value_name = "PATH", default_value = "../config.toml")]
    config: PathBuf,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long, default_value = "8080")]
    http_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

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
        match rcon_client.connect() {
            Ok(_) => {
                *rcon_guard = Some(rcon_client);
                info!("RCON connection established");
            }
            Err(e) => {
                warn!("Failed to connect to RCON: {}. Will retry later.", e);
            }
        }
    }

    let (event_tx, mut event_rx) = mpsc::channel::<LogEvent>(100);
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    let monitor_handle = tokio::spawn(async move {
        if let Err(e) = log_monitor.start_monitoring(event_tx, shutdown_clone).await {
            error!("Log monitor error: {}", e);
        }
    });

    let http_api = Arc::new(HttpApi::new(args.http_port, context.clone(), rcon.clone()));
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_api.start().await {
            error!("HTTP API error: {}", e);
        }
    });

    info!("MC-Minder is running. Press Ctrl+C to stop.");

    let trigger = ai_client.as_ref().map(|a| a.get_trigger().to_string());

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
                            
                            match ai.chat(messages).await {
                                Ok(response) => {
                                    context.add_assistant_message(&response);
                                    
                                    let mut rcon_guard = rcon.write().await;
                                    if let Some(ref mut rcon_client) = *rcon_guard {
                                        let tell_msg = format!("[AI] {}", response);
                                        if let Err(e) = rcon_client.tell(&msg.player, &tell_msg) {
                                            warn!("Failed to send AI response: {}", e);
                                        }
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
                    let _ = rcon_client.say(&format!("Welcome {}!", player));
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

    shutdown.notify_waiters();
    let _ = tokio::try_join!(monitor_handle, http_handle);

    info!("MC-Minder stopped");
    Ok(())
}
