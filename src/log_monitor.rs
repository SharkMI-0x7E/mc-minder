use anyhow::{Result, Context};
use log::{info, debug};
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub player: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

pub struct LogMonitor {
    log_path: PathBuf,
    chat_pattern: Regex,
    join_pattern: Regex,
    leave_pattern: Regex,
    death_pattern: Regex,
}

impl LogMonitor {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        let chat_pattern = Regex::new(r"\[(\d{2}:\d{2}:\d{2})\] \[Server thread/INFO\]: <([^>]+)> (.+)")
            .context("Failed to compile chat pattern")?;
        
        let join_pattern = Regex::new(r"\[(\d{2}:\d{2}:\d{2})\] \[Server thread/INFO\]: (\w+) joined the game")
            .context("Failed to compile join pattern")?;
        
        let leave_pattern = Regex::new(r"\[(\d{2}:\d{2}:\d{2})\] \[Server thread/INFO\]: (\w+) left the game")
            .context("Failed to compile leave pattern")?;
        
        let death_pattern = Regex::new(r"\[(\d{2}:\d{2}:\d{2})\] \[Server thread/INFO\]: (\w+) .*(died|was|fell|drowned|blew up|burned|froze|suffocated|starved)")
            .context("Failed to compile death pattern")?;

        Ok(Self {
            log_path,
            chat_pattern,
            join_pattern,
            leave_pattern,
            death_pattern,
        })
    }

    pub async fn start_monitoring(
        self,
        tx: mpsc::Sender<LogEvent>,
        shutdown: Arc<tokio::sync::Notify>,
    ) -> Result<()> {
        info!("Started monitoring log file: {:?}", self.log_path);

        let mut last_size = std::fs::metadata(&self.log_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let mut poll_interval = interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    info!("Log monitor shutting down");
                    break;
                }
                
                _ = poll_interval.tick() => {
                    match self.check_for_new_content(&mut last_size) {
                        Ok(Some(events)) => {
                            for log_event in events {
                                if tx.send(log_event).await.is_err() {
                                    debug!("Receiver dropped, stopping monitor");
                                    return Ok(());
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            debug!("Error checking log content: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_for_new_content(&self, last_size: &mut u64) -> Result<Option<Vec<LogEvent>>> {
        let metadata = std::fs::metadata(&self.log_path)?;
        let current_size = metadata.len();

        if current_size < *last_size {
            *last_size = 0;
            return Ok(None);
        }

        if current_size == *last_size {
            return Ok(None);
        }

        let new_bytes = current_size - *last_size;
        let file = std::fs::File::open(&self.log_path)?;
        use std::io::{Read, Seek, SeekFrom};
        let mut reader = std::io::BufReader::new(file);
        reader.seek(SeekFrom::End(-(new_bytes as i64)))?;

        let mut new_content = String::new();
        reader.read_to_string(&mut new_content)?;
        *last_size = current_size;

        let events = self.parse_lines(&new_content);
        if events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(events))
        }
    }

    fn parse_lines(&self, content: &str) -> Vec<LogEvent> {
        let mut events = Vec::new();
        
        for line in content.lines() {
            if let Some(caps) = self.chat_pattern.captures(line) {
                if let (Some(player), Some(content)) = (caps.get(2), caps.get(3)) {
                    events.push(LogEvent::Chat(ChatMessage {
                        player: player.as_str().to_string(),
                        content: content.as_str().to_string(),
                        timestamp: chrono::Local::now(),
                    }));
                }
            } else if let Some(caps) = self.join_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerJoin(player.as_str().to_string()));
                }
            } else if let Some(caps) = self.leave_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerLeave(player.as_str().to_string()));
                }
            } else if let Some(caps) = self.death_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerDeath(player.as_str().to_string()));
                }
            }
        }
        
        events
    }
}

#[derive(Debug, Clone)]
pub enum LogEvent {
    Chat(ChatMessage),
    PlayerJoin(String),
    PlayerLeave(String),
    PlayerDeath(String),
    ServerStart,
    ServerStop,
}
