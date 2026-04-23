// Server run loop - extracted from main.rs
// Contains: run_server() with log monitoring, AI chat, RCON, Telegram

use anyhow::{Result, Context};
use log::{debug, info, warn, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::monitor::{LogMonitor, LogEvent};
use crate::ai::AiClient;
use crate::ai::ChatResult;
use crate::rcon::RconClient;
use crate::context::ContextManager;
use crate::api::HttpApi;
use crate::notification::send_telegram_notification;
use crate::cli::Args;


pub async fn run_server(args: Args) -> Result<()> {
    info!("MC-Minder starting up...");

    let config_path = args.config.clone();
    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {:?}", config_path))?;

    info!("Configuration loaded successfully");

    // HTTP client for Telegram notifications
    let http_client = reqwest::Client::new();

    // Core components
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
    context.add_system_message(
        "You are a helpful Minecraft server assistant. Respond concisely and helpfully to player questions. Keep responses under 100 characters when possible."
    );

    let rcon = Arc::new(RwLock::new(None));
    {
        let mut rcon_guard = rcon.write().await;
        debug!("[RCON] Attempting to connect to {}:{}...", config.rcon.host, config.rcon.port);
        match rcon_client.connect().await {
            Ok(_) => {
                *rcon_guard = Some(rcon_client);
                info!("RCON connection established");
                debug!("[RCON] Connection successful, rcon_client=Some");
            }
            Err(e) => {
                warn!("Failed to connect to RCON: {}. Will retry later.", e);
                debug!("[RCON] Connection failed, rcon_client=None, error: {}", e);
            }
        }
    }

    let mut event_rx = log_monitor.start_monitoring()?;

    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let http_api = Arc::new(HttpApi::new(
        args.http_port,
        context.clone(),
        rcon.clone(),
    ));
    let mut shutdown_rx = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_api.start(async move { shutdown_rx.recv().await.ok(); }).await {
            error!("HTTP API error: {}", e);
        }
    });

    info!("MC-Minder is running. Press Ctrl+C to stop.");

    let trigger = ai_client.as_ref().map(|a| {
        let t = a.get_trigger().to_string();
        debug!("[AI] Configuration loaded: trigger='{}', ai_client=Some", t);
        t
    });
    if trigger.is_none() {
        debug!("[AI] Configuration loaded: ai_client=None, AI features disabled");
    }

    // last_ai_chat tracking removed - unused

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            let _ = shutdown_tx.send(());
        },
        _ = async {
            while let Some(event) = event_rx.recv().await {
                match event {
                    LogEvent::Chat(msg) => {
                        let player = &msg.player;
                        let message = &msg.content;
                        info!("[Chat] {}: {}", player, message);

                        if let (Some(ai), Some(trig)) = (ai_client.as_ref(), trigger.as_ref()) {
                            debug!("[AI] Checking trigger '{}' in message: '{}', starts_with={}", trig, message, message.starts_with(trig));
                            if message.starts_with(trig) {
                                let question = message.trim_start_matches(trig).trim();
                                debug!("[AI] Trigger detected! Question: '{}', Player: '{}'", question, player);
                                if !question.is_empty() {
                                    context.add_user_message(question, player);

                                    let messages = context.get_messages_for_player(player);
                                    let player_clone = player.clone();
                                    debug!("[AI] Trigger '{}' detected from player '{}', question: '{}'", trig, player_clone, question);

                                    debug!("[AI] Sending request to AI backend...");
                                    match ai.chat(messages, &player_clone).await {
                                        Ok(ChatResult::Success(response)) => {
                                            debug!("[AI] Received response: '{}'", response);
                                            context.add_assistant_message_for_player(&response, player);

                                            let mut rcon_guard = rcon.write().await;
                                            if let Some(rcon_client) = &mut *rcon_guard {
                                                let tell_msg = format!("[AI] {}", response);
                                                if let Err(e) = rcon_client.tell(player, &tell_msg).await {
                                                    warn!("Failed to send AI response to player '{}': {}", player, e);
                                                } else {
                                                    debug!("[AI] Successfully sent response to player '{}'", player);
                                                }
                                            } else {
                                                warn!("[AI] RCON connection not available, cannot send response");
                                            }
                                        }
                                        Ok(ChatResult::RateLimited(rate_limit_msg)) => {
                                            debug!("[AI] Player '{}' rate limited", player);
                                            let mut rcon_guard = rcon.write().await;
                                            if let Some(rcon_client) = &mut *rcon_guard {
                                                if let Err(e) = rcon_client.tell(player, &rate_limit_msg).await {
                                                    warn!("Failed to send rate limit message: {}", e);
                                                }
                                            } else {
                                                warn!("[AI] RCON connection not available, rate limit message dropped");
                                            }
                                        }
                                        Err(e) => {
                                            warn!("[AI] Chat error for player '{}': {}", player, e);
                                        }
                                    }
                                } else {
                                    debug!("[AI] Question is empty after removing trigger, ignoring");
                                }
                            } else {
                                debug!("[AI] Message '{}' does not start with trigger '{}'", message, trig);
                            }
                        } else {
                            debug!("[AI] AI client or trigger not configured, ignoring chat message");
                        }
                    }
                    LogEvent::PlayerJoin(player) => {
                        info!("[Join] {} joined the game", player);
                        let mut rcon_guard = rcon.write().await;
                        if let Some(ref mut rcon_client) = *rcon_guard {
                            let _ = rcon_client.say(&format!("Welcome {}!", player)).await;
                        }
                        let join_message = format!("*MC-Minder Alert*\n\nPlayer *{}* joined the game", player);
                        send_telegram_notification(&http_client, &config, &join_message).await;
                    }
                    LogEvent::PlayerLeave(player) => {
                        info!("[Leave] {} left the game", player);
                        let leave_message = format!("*MC-Minder Alert*\n\nPlayer *{}* left the game", player);
                        send_telegram_notification(&http_client, &config, &leave_message).await;
                    }
                    LogEvent::PlayerDeath(player) => {
                        info!("[Death] {}", player);
                    }
                    LogEvent::ServerStart => {
                        info!("[Server] Server started");
                        let version = env!("CARGO_PKG_VERSION");
                        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        let start_message = format!("*MC-Minder Alert*\n\nServer *Started* successfully!\n\nVersion: `{}`\nTime: `{}`", version, timestamp);
                        send_telegram_notification(&http_client, &config, &start_message).await;
                    }
                    LogEvent::ServerStop => {
                        info!("[Server] Server stopped");
                        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        let stop_message = format!("*MC-Minder Alert*\n\nServer *Stopped*!\n\nTime: `{}`", timestamp);
                        send_telegram_notification(&http_client, &config, &stop_message).await;
                    }
                }
            }
        } => {}
    }

    let _ = tokio::try_join!(http_handle);

    info!("MC-Minder stopped");
    Ok(())
}