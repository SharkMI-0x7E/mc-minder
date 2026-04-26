use anyhow::{Result, Context};
use log::{info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, Duration};

use crate::config::Config;
use crate::monitor::{LogMonitor, LogEvent, TmuxChatCapture, FileChatCapture, ChatCapture};
use crate::api::HttpApi;
use crate::notification::send_telegram_notification;
use crate::cli::Args;
use crate::command_sender::{CommandSender, MultiCommandSender};

#[allow(dead_code)]
pub enum ServerMode {
    Background,
    Foreground,
}

pub async fn run_server(args: Args, mode: ServerMode) -> Result<()> {
    info!("MC-Minder starting up...");

    let config_path = args.config.clone();
    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {:?}", config_path))?;

    info!("Configuration loaded successfully");

    let http_client = reqwest::Client::new();

    let log_path = PathBuf::from(&config.server.log_file);
    let log_monitor = LogMonitor::new(log_path.clone())?;

    // Use pooled RCON for persistent connection with auto-reconnect
    let mut command_sender = MultiCommandSender::new();
    command_sender.add_sender(CommandSender::pooled_rcon(
        config.rcon.host.clone(),
        config.rcon.port,
        config.rcon.password.clone(),
    ));

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let rcon_sender = Arc::new(RwLock::new(command_sender));

    let http_api = Arc::new(HttpApi::new(
        args.http_port,
        rcon_sender.clone(),
    ));
    let mut shutdown_rx = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_api.start(async move { shutdown_rx.recv().await.ok(); }).await {
            log::error!("HTTP API error: {}", e);
        }
    });

    info!("MC-Minder is running. Press Ctrl+C to stop.");

    let mut event_rx = log_monitor.start_monitoring()?;

    let mut tmux_capture: Option<TmuxChatCapture> = None;
    let mut file_capture: Option<FileChatCapture> = None;
    let capture_log_path = log_path.clone();

    match mode {
        ServerMode::Background => {
            info!("Running in background mode (tmux session)");
            tmux_capture = TmuxChatCapture::new(config.server.session_name.clone()).ok();
            if tmux_capture.is_none() {
                warn!("Failed to create TmuxChatCapture, falling back to FileChatCapture");
                file_capture = FileChatCapture::new(capture_log_path).ok();
            }
        }
        ServerMode::Foreground => {
            info!("Running in foreground mode (file monitoring)");
            file_capture = FileChatCapture::new(capture_log_path).ok();
        }
    }

    let capture_interval = Duration::from_secs(2);
    let mut capture_timer = interval(capture_interval);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            let _ = shutdown_tx.send(());
        },
        _ = async {
            loop {
                tokio::select! {
                    _ = capture_timer.tick() => {
                        if let Some(ref mut cap) = tmux_capture {
                            let messages = cap.capture_recent_messages();
                            for msg in messages {
                                log::info!("[Chat] {}: {}", msg.player, msg.content);
                            }
                        } else if let Some(ref mut cap) = file_capture {
                            let messages = cap.capture_recent_messages();
                            for msg in messages {
                                log::info!("[Chat] {}: {}", msg.player, msg.content);
                            }
                        }
                    }
                    Some(event) = event_rx.recv() => {
                        match event {
                            LogEvent::Chat(msg) => {
                                log::info!("[Chat] {}: {}", msg.player, msg.content);
                            }
                            LogEvent::PlayerJoin(player) => {
                                info!("[Join] {} joined the game", player);
                                let mut sender = rcon_sender.write().await;
                                if let Err(e) = sender.send_command_ignore_response(&format!("say Welcome {}!", player)).await {
                                    warn!("Failed to send welcome message: {}", e);
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
                }
            }
        } => {}
    }

    let _ = tokio::try_join!(http_handle);

    info!("MC-Minder stopped");
    Ok(())
}

pub async fn run_server_bg(args: Args) -> Result<()> {
    run_server(args, ServerMode::Background).await
}

#[allow(dead_code)]
pub async fn run_server_fg(args: Args) -> Result<()> {
    run_server(args, ServerMode::Foreground).await
}