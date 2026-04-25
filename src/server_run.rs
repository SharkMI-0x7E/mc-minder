use anyhow::{Result, Context};
use log::{debug, info, warn, error};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, Duration};

use crate::config::Config;
use crate::monitor::{LogMonitor, LogEvent, ChatMessage, TmuxChatCapture, FileChatCapture, ChatCapture};
use crate::ai::{AiClient, ChatResult};
use crate::context::ContextManager;
use crate::api::HttpApi;
use crate::notification::send_telegram_notification;
use crate::cli::Args;
use crate::command_sender::{CommandSender, MultiCommandSender};

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

    let ai_client = if let Some(ref ai_config) = config.ai {
        Some(AiClient::new(ai_config.clone(), config.ollama.clone())?)
    } else {
        warn!("No AI configuration found, AI features disabled");
        None
    };

    let context = Arc::new(ContextManager::new());
    context.add_system_message(
        "You are a helpful Minecraft server assistant. Respond concisely and helpfully to player questions. Keep responses under 100 characters when possible."
    );

    let trigger = ai_client.as_ref().map(|a| a.get_trigger().to_string());

    let mut command_sender = MultiCommandSender::new();

    command_sender.add_sender(CommandSender::rcon(
        config.rcon.host.clone(),
        config.rcon.port,
        config.rcon.password.clone(),
    ));

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let rcon_sender = Arc::new(RwLock::new(command_sender));

    let http_api = Arc::new(HttpApi::new(
        args.http_port,
        context.clone(),
        rcon_sender.clone(),
    ));
    let mut shutdown_rx = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_api.start(async move { shutdown_rx.recv().await.ok(); }).await {
            error!("HTTP API error: {}", e);
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
                warn!("[AI] Failed to create TmuxChatCapture, falling back to FileChatCapture");
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
                                process_chat_event(
                                    &msg,
                                    ai_client.as_ref(),
                                    trigger.as_ref(),
                                    &context,
                                    &rcon_sender,
                                ).await;
                            }
                        } else if let Some(ref mut cap) = file_capture {
                            let messages = cap.capture_recent_messages();
                            for msg in messages {
                                process_chat_event(
                                    &msg,
                                    ai_client.as_ref(),
                                    trigger.as_ref(),
                                    &context,
                                    &rcon_sender,
                                ).await;
                            }
                        }
                    }
                    Some(event) = event_rx.recv() => {
                        match event {
                            LogEvent::Chat(msg) => {
                                process_chat_event(
                                    &msg,
                                    ai_client.as_ref(),
                                    trigger.as_ref(),
                                    &context,
                                    &rcon_sender,
                                ).await;
                            }
                            LogEvent::PlayerJoin(player) => {
                                info!("[Join] {} joined the game", player);
                                let mut sender = rcon_sender.write().await;
                                if let Err(e) = sender.send_command(&format!("say Welcome {}!", player)).await {
                                    warn!("[AI] Failed to send welcome message: {}", e);
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

async fn process_chat_event(
    msg: &ChatMessage,
    ai_client: Option<&AiClient>,
    trigger: Option<&String>,
    context: &Arc<ContextManager>,
    rcon_sender: &Arc<RwLock<MultiCommandSender>>,
) {
    let player = &msg.player;
    let message = &msg.content;

    info!("[Chat] {}: {}", player, message);

    let Some(ai) = ai_client else {
        debug!("[AI] AI client not configured, ignoring chat message");
        return;
    };

    let Some(trig) = trigger else {
        debug!("[AI] Trigger not configured, ignoring chat message");
        return;
    };

    debug!("[AI] Checking trigger '{}' in message: '{}', starts_with={}", trig, message, message.starts_with(trig));

    if !message.starts_with(trig) {
        debug!("[AI] Message '{}' does not start with trigger '{}'", message, trig);
        return;
    }

    let question = message.trim_start_matches(trig).trim();

    if question.is_empty() {
        debug!("[AI] Question is empty after removing trigger, ignoring");
        return;
    }

    debug!("[AI] Trigger detected! Question: '{}', Player: '{}'", question, player);

    context.add_user_message(question, player);

    let messages = context.get_messages_for_player(player);
    let player_clone = player.clone();

    debug!("[AI] Sending request to AI backend...");

    match ai.chat(messages, &player_clone).await {
        Ok(ChatResult::Success(response)) => {
            debug!("[AI] Received response: '{}'", response);
            context.add_assistant_message_for_player(&response, player);

            let mut sender = rcon_sender.write().await;
            let tell_msg = format!("[AI] {}", response);

            match sender.send_command(&format!("tellraw {} {{\"text\":\"{}\"}}", player, escape_json(&tell_msg))).await {
                Ok(_) => {
                    debug!("[AI] Successfully sent response to player '{}' via tellraw", player);
                }
                Err(e) => {
                    warn!("[AI] Failed to send tellraw to player '{}': {}, trying /say", player, e);
                    if let Err(e2) = sender.send_command(&format!("say {}", tell_msg)).await {
                        warn!("[AI] Failed to send AI response via /say: {}", e2);
                        error!("[AI] All delivery methods failed for player '{}' AI response: {}", player, response);
                    }
                }
            }
        }
        Ok(ChatResult::RateLimited(rate_limit_msg)) => {
            debug!("[AI] Player '{}' rate limited", player);
            let mut sender = rcon_sender.write().await;
            let msg = format!("[AI] {}", rate_limit_msg);
            if let Err(e) = sender.send_command(&format!("tellraw {} {{\"text\":\"{}\"}}", player, escape_json(&msg))).await {
                warn!("[AI] Failed to send rate limit message: {}", e);
                error!("[AI] Rate limit message dropped for player '{}': {}", player, rate_limit_msg);
            }
        }
        Err(e) => {
            warn!("[AI] Chat error for player '{}': {}", player, e);
            error!("[AI] Failed to process AI chat for player '{}': {:?}", player, e);
        }
    }
}

fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

pub async fn run_server_bg(args: Args) -> Result<()> {
    run_server(args, ServerMode::Background).await
}

pub async fn run_server_fg(args: Args) -> Result<()> {
    run_server(args, ServerMode::Foreground).await
}
