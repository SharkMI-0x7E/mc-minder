use anyhow::Result;
use log::info;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use warp::Filter;
use serde::{Serialize, Deserialize};

use crate::command_sender::MultiCommandSender;

/// Snapshot of MC server status at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McStatusSnapshot {
    pub online: bool,
    pub players_online: i32,
    pub players_max: i32,
    pub version: String,
    pub latency_ms: u64,
    pub motd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl McStatusSnapshot {
    pub fn offline(error: &str) -> Self {
        Self {
            online: false,
            players_online: 0,
            players_max: 0,
            version: String::new(),
            latency_ms: 0,
            motd: String::new(),
            error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub uptime: u64,
    pub mc_status: McStatusSnapshot,
    pub rcon_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub result: String,
}

pub struct HttpApi {
    port: u16,
    sender: Arc<RwLock<MultiCommandSender>>,
    start_time: std::time::Instant,
    pub mc_status_cache: Arc<RwLock<Option<(McStatusSnapshot, std::time::Instant)>>>,
    rcon_available: bool,
    mc_port: u16,
    mc_status_config: crate::config::McStatusConfig,
    ping_lock: AtomicBool,
}

impl HttpApi {
    pub fn new(
        port: u16,
        sender: Arc<RwLock<MultiCommandSender>>,
        rcon_available: bool,
        mc_port: u16,
        mc_status_config: crate::config::McStatusConfig,
    ) -> Self {
        Self {
            port,
            sender,
            start_time: std::time::Instant::now(),
            mc_status_cache: Arc::new(RwLock::new(None)),
            rcon_available,
            mc_port,
            mc_status_config,
            ping_lock: AtomicBool::new(false),
        }
    }

    pub async fn fetch_mc_status(&self) -> McStatusSnapshot {
        // Check cache validity
        {
            let cache = self.mc_status_cache.read().await;
            if let Some((ref snapshot, ref time)) = *cache {
                let age = time.elapsed().as_secs();
                if age < self.mc_status_config.ping_interval_secs {
                    return snapshot.clone();
                }
            }
        }

        // Prevent concurrent pings
        if self.ping_lock.swap(true, Ordering::Acquire) {
            let cache = self.mc_status_cache.read().await;
            if let Some((ref snapshot, _)) = *cache {
                return snapshot.clone();
            }
            return McStatusSnapshot::offline("status probe busy, retry later");
        }

        // Use a simple struct to guarantee lock release on cancel/drop
        struct PingGuard<'a>(&'a AtomicBool);
        impl<'a> Drop for PingGuard<'a> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = PingGuard(&self.ping_lock);

        let timeout = Duration::from_secs(self.mc_status_config.ping_timeout_secs);
        let result = mc_status_probe::ping("127.0.0.1", self.mc_port, timeout, None).await;

        // Guard auto-releases on return/cancel/drop — no manual store(false) needed

        let snapshot = match result {
            Ok(r) => McStatusSnapshot {
                online: true,
                players_online: r.players_online,
                players_max: r.players_max,
                version: r.version_name,
                latency_ms: r.latency_ms,
                motd: r.description,
                error: None,
            },
            Err(e) => McStatusSnapshot::offline(&e.to_string()),
        };

        // Update cache
        {
            let mut cache = self.mc_status_cache.write().await;
            *cache = Some((snapshot.clone(), std::time::Instant::now()));
        }

        snapshot
    }

    pub async fn start<S>(self: Arc<Self>, shutdown: S) -> Result<()>
    where
        S: Future<Output = ()> + Send + 'static,
    {
        let start_time = self.start_time;
        let preferred_port = self.port;
        let cache = self.mc_status_cache.clone();
        let rcon_available = self.rcon_available;

        // Status route
        let status_cache = cache.clone();
        let status_route = warp::path("status")
            .and(warp::get())
            .and_then(move || {
                let cache = status_cache.clone();
                let st = start_time;
                let rcon_ok = rcon_available;
                async move {
                    let mc_status = {
                        let cached = cache.read().await;
                        match *cached {
                            Some((ref snapshot, _)) => snapshot.clone(),
                            None => McStatusSnapshot::offline("no status data yet"),
                        }
                    };
                    let response = StatusResponse {
                        status: "running".to_string(),
                        uptime: st.elapsed().as_secs(),
                        mc_status,
                        rcon_available: rcon_ok,
                    };
                    Ok::<_, warp::Rejection>(warp::reply::json(&response))
                }
            });

        // Command route — with RCON availability precheck
        let sender = self.sender.clone();
        let command_route = warp::path("command")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: CommandRequest| {
                let sender = sender.clone();
                let rcon_ok = rcon_available;
                async move {
                    if !rcon_ok {
                        let response = CommandResponse {
                            success: false,
                            result: "RCON is not configured or not available".to_string(),
                        };
                        return Ok(warp::reply::json(&response));
                    }
                    let mut sender_guard = sender.write().await;
                    match sender_guard.send_command(&req.command).await {
                        Ok(response_text) => {
                            let response = CommandResponse {
                                success: true,
                                result: response_text.trim().to_string(),
                            };
                            Ok::<_, warp::Rejection>(warp::reply::json(&response))
                        }
                        Err(e) => {
                            let response = CommandResponse {
                                success: false,
                                result: e.to_string(),
                            };
                            Ok(warp::reply::json(&response))
                        }
                    }
                }
            });

        let routes = status_route
            .or(command_route)
            .with(warp::cors().allow_any_origin());

        info!("Starting HTTP API server on port {}", preferred_port);

        warp::serve(routes)
            .bind_with_graceful_shutdown(([0, 0, 0, 0], preferred_port), async { shutdown.await })
            .1
            .await;

        Ok(())
    }
}
