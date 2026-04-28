// Lazy Start module (P4-2)
// Lightweight TCP listener that auto-starts the Minecraft server
// when a client connects, and auto-stops when idle.

use anyhow::Result;
use log::{info, warn};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::command_sender::MultiCommandSender;
use crate::foreground_process::ForegroundProcess;

/// Configuration for lazy start.
#[derive(Debug, Clone)]
pub struct LazyStartSettings {
    pub enabled: bool,
    pub listen_port: u16,       // Port to listen on (typically the MC port)
    pub idle_timeout_mins: u64, // Minutes of no players before auto-stop
    pub jar: String,
    pub min_mem: String,
    pub max_mem: String,
    pub jdk_path: Option<String>,
}

/// Run the lazy start listener. Returns when the process shuts down.
pub async fn run_lazy_start(
    settings: LazyStartSettings,
    rcon_sender: Arc<RwLock<MultiCommandSender>>,
) -> Result<()> {
    let addr = format!("0.0.0.0:{}", settings.listen_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("[LazyStart] Listening on {} for wake-up connections", addr);

    let mut server_proc: Option<ForegroundProcess> = None;
    let mut last_player_time = tokio::time::Instant::now();

    loop {
        tokio::select! {
            // Accept new connections to wake/check server
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut stream, addr)) => {
                        info!("[LazyStart] Connection from {}", addr);

                        if server_proc.is_none() {
                            // Spawn the server
                            info!("[LazyStart] Waking server...");
                            match ForegroundProcess::spawn(
                                &settings.jar,
                                &settings.min_mem,
                                &settings.max_mem,
                                None,
                                settings.jdk_path.as_deref(),
                            ).await {
                                Ok(proc) => {
                                    server_proc = Some(proc);
                                    // Tell client server is starting
                                    let _ = stream.write_all(b"Server is starting, please wait...\n").await;
                                    let _ = stream.shutdown().await;
                                }
                                Err(e) => {
                                    warn!("[LazyStart] Failed to spawn server: {}", e);
                                    let _ = stream.write_all(b"Server failed to start\n").await;
                                    let _ = stream.shutdown().await;
                                }
                            }
                        } else {
                            // Server is running, notify client
                            let _ = stream.write_all(b"Server is running\n").await;
                            let _ = stream.shutdown().await;
                        }
                    }
                    Err(e) => warn!("[LazyStart] Accept error: {}", e),
                }
            }

            // Idle check: stop server when no players for timeout period
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if let Some(ref proc) = server_proc {
                    // Check player count via RCON
                    let mut sender = rcon_sender.write().await;
                    match sender.send_command("list").await {
                        Ok(resp) => {
                            // MC "list" returns "There are X of a max of Y players online: ..."
                            if resp.contains("There are 0 of") || resp.contains("There are 0 ") {
                                let elapsed = last_player_time.elapsed().as_secs();
                                let timeout_secs = settings.idle_timeout_mins * 60;
                                if elapsed >= timeout_secs {
                                    info!("[LazyStart] No players for {} min, stopping server", settings.idle_timeout_mins);
                                    let _ = proc.graceful_stop();
                                    server_proc = None;
                                }
                            } else {
                                // Players online, reset timer
                                last_player_time = tokio::time::Instant::now();
                            }
                        }
                        Err(_) => {
                            // RCON not available yet, server might still be starting
                            warn!("[LazyStart] RCON not available during idle check");
                        }
                    }
                }
            }
        }
    }
}
