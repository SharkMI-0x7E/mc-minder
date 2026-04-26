use anyhow::Result;
use log::info;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;
use serde::{Serialize, Deserialize};

use crate::command_sender::MultiCommandSender;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub uptime: u64,
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
}

impl HttpApi {
    pub fn new(
        port: u16,
        sender: Arc<RwLock<MultiCommandSender>>,
    ) -> Self {
        Self {
            port,
            sender,
            start_time: std::time::Instant::now(),
        }
    }

    pub async fn start<S>(self: Arc<Self>, shutdown: S) -> Result<()>
    where
        S: Future<Output = ()> + Send + 'static,
    {
        let start_time = self.start_time;
        let port = self.port;

        let status_route = warp::path("status")
            .and(warp::get())
            .map(move || {
                let response = StatusResponse {
                    status: "running".to_string(),
                    uptime: start_time.elapsed().as_secs(),
                };
                warp::reply::json(&response)
            });

        let sender = self.sender.clone();
        let command_route = warp::path("command")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |req: CommandRequest| {
                let sender = sender.clone();
                async move {
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

        info!("Starting HTTP API server on port {}", port);

        warp::serve(routes)
            .bind_with_graceful_shutdown(([0, 0, 0, 0], port), shutdown)
            .1
            .await;

        Ok(())
    }
}