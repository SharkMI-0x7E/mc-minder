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

    // Discover Minecraft server port
    let (mc_port, port_warning) = crate::config::discover_minecraft_port(
        &config_path.parent().unwrap_or(std::path::Path::new("."))
    );
    if let Some(w) = port_warning {
        warn!("{}", w);
    }

    // Check if RCON is properly configured
    let rcon_available = !config.rcon.password.is_empty();

    let http_api = Arc::new(HttpApi::new(
        args.http_port,
        rcon_sender.clone(),
        rcon_available,
        mc_port,
        config.mc_status.clone(),
    ));

    // Clone cache BEFORE moving http_api into spawn
    let _mc_status_cache = http_api.mc_status_cache.clone();
    let mc_poll_interval = Duration::from_secs(config.mc_status.ping_interval_secs);
    let poll_api = http_api.clone();
    let mut poll_shutdown = shutdown_tx.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = poll_api.fetch_mc_status().await;
            tokio::select! {
                _ = tokio::time::sleep(mc_poll_interval) => {}
                _ = poll_shutdown.recv() => {
                    log::info!("MC status poller shutting down");
                    break;
                }
            }
        }
    });

    // Schedule runner (P4-3): periodic backup/broadcast/restart
    if !config.schedules.is_empty() {
        let schedules = config.schedules.clone();
        let backup_cfg = config.backup.clone();
        let server_cfg = config.server.clone();
        let schedule_sender = rcon_sender.clone();
        let mut sched_shutdown = shutdown_tx.subscribe();
        tokio::spawn(async move {
            loop {
                for entry in &schedules {
                    let wait = Duration::from_secs(entry.interval_mins * 60);
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {
                            let mut sender = schedule_sender.write().await;
                            match entry.action.as_str() {
                                "backup" => {
                                    let world = PathBuf::from(&server_cfg.jar)
                                        .parent().unwrap_or(std::path::Path::new("."))
                                        .join(&backup_cfg.world_dir);
                                    let dest = PathBuf::from(&backup_cfg.backup_dest);
                                    match crate::backup::create_backup(&world, &dest, &server_cfg.session_name) {
                                        Ok(p) => {
                                            info!("[Scheduler] Backup created: {:?}", p);
                                            crate::backup::apply_retention(&dest, backup_cfg.max_backups, backup_cfg.max_backup_days);
                                        }
                                        Err(e) => warn!("[Scheduler] Backup failed: {}", e),
                                    }
                                }
                                "broadcast" => {
                                    let _ = sender.send_command(&format!("say {}", entry.message)).await;
                                    info!("[Scheduler] Broadcast: {}", entry.message);
                                }
                                "restart" => {
                                    let _ = sender.send_command("say Server restarting...").await;
                                    let _ = sender.send_command("stop").await;
                                    info!("[Scheduler] Server shutdown initiated");
                                }
                                "command" => {
                                    let _ = sender.send_command(&entry.message).await;
                                    info!("[Scheduler] Command: {}", entry.message);
                                }
                                _ => warn!("[Scheduler] Unknown action: {}", entry.action),
                            }
                        }
                        _ = sched_shutdown.recv() => {
                            log::info!("Scheduler shutting down");
                            break;
                        }
                    }
                }
            }
        });
    }

    // Watchdog (P4-1): health check + auto-restart
    if config.watchdog.enabled {
        let wd_config = config.watchdog.clone();
        let wd_sender = rcon_sender.clone();
        let mut wd_shutdown = shutdown_tx.subscribe();
        tokio::spawn(async move {
            let mut restarts = 0u32;
            loop {
                let check = Duration::from_secs(wd_config.check_interval_secs);
                tokio::select! {
                    _ = tokio::time::sleep(check) => {
                        // Health check via RCON "list" command
                        let mut sender = wd_sender.write().await;
                        match sender.send_command("list").await {
                            Ok(_) => {
                                restarts = 0; // Server is alive, reset counter
                            }
                            Err(e) => {
                                warn!("[Watchdog] Health check failed: {}", e);
                                restarts += 1;
                                if wd_config.max_restarts > 0 && restarts > wd_config.max_restarts {
                                    warn!("[Watchdog] Max restarts ({}) reached, giving up", wd_config.max_restarts);
                                    break;
                                }
                                // Wait cooldown, then try restart
                                tokio::time::sleep(Duration::from_secs(wd_config.cooldown_secs)).await;
                                // Reconnect sender
                                let mut sender2 = wd_sender.write().await;
                                let _ = sender2.send_command("stop").await;
                                info!("[Watchdog] Auto-restart triggered (attempt {}/{})", restarts,
                                    if wd_config.max_restarts == 0 { "unlimited" } else { "" });
                            }
                        }
                    }
                    _ = wd_shutdown.recv() => {
                        log::info!("Watchdog shutting down");
                        break;
                    }
                }
            }
        });
    }

    // Lazy Start (P4-2): TCP listener that auto-starts server on connect
    if config.lazy_start.enabled {
        let ls_settings = crate::lazy_start::LazyStartSettings {
            enabled: true,
            listen_port: config.lazy_start.listen_port,
            idle_timeout_mins: config.lazy_start.idle_timeout_mins,
            jar: config.server.jar.clone(),
            min_mem: config.server.min_mem.clone(),
            max_mem: config.server.max_mem.clone(),
            jdk_path: config.jvm.jdk_path.clone(),
        };
        let ls_sender = rcon_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::lazy_start::run_lazy_start(ls_settings, ls_sender).await {
                log::error!("[LazyStart] Fatal error: {}", e);
            }
        });
    }

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