use anyhow::Result;
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;
use serde::{Serialize, Deserialize};

use crate::context::ContextManager;
use crate::rcon::RconClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub uptime: u64,
    pub context_messages: usize,
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
    context: Arc<ContextManager>,
    rcon: Arc<RwLock<Option<RconClient>>>,
    start_time: std::time::Instant,
}

impl HttpApi {
    pub fn new(
        port: u16,
        context: Arc<ContextManager>,
        rcon: Arc<RwLock<Option<RconClient>>>,
    ) -> Self {
        Self {
            port,
            context,
            rcon,
            start_time: std::time::Instant::now(),
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let context = self.context.clone();
        let start_time = self.start_time;
        let port = self.port;

        let status_route = warp::path("status")
            .and(warp::get())
            .map(move || {
                let response = StatusResponse {
                    status: "running".to_string(),
                    uptime: start_time.elapsed().as_secs(),
                    context_messages: context.len(),
                };
                warp::reply::json(&response)
            });

        let context = self.context.clone();
        let history_route = warp::path("history")
            .and(warp::get())
            .map(move || {
                let messages = context.get_messages();
                warp::reply::json(&messages)
            });

        let rcon_clone = self.rcon.clone();
        let command_route = warp::path("command")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: CommandRequest| {
                let rcon = rcon_clone.clone();
                async move {
                    let mut rcon_guard = rcon.write().await;
                    if let Some(ref mut rcon_client) = *rcon_guard {
                        match rcon_client.execute(&req.command) {
                            Ok(result) => {
                                let response = CommandResponse {
                                    success: true,
                                    result,
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
                    } else {
                        let response = CommandResponse {
                            success: false,
                            result: "RCON not connected".to_string(),
                        };
                        Ok(warp::reply::json(&response))
                    }
                }
            });

        let routes = status_route
            .or(history_route)
            .or(command_route)
            .with(warp::cors().allow_any_origin());

        info!("Starting HTTP API server on port {}", port);

        warp::serve(routes)
            .run(([0, 0, 0, 0], port))
            .await;

        Ok(())
    }
}
