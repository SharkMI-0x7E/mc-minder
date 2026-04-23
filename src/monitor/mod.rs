use anyhow::{Context, Result};
use log::{info, debug, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
#[allow(dead_code)]
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LogEvent {
    Chat(ChatMessage),
    PlayerJoin(String),
    PlayerLeave(String),
    PlayerDeath(String),
    ServerStart,
    ServerStop,
}

/// 文件标识，用于检测文件轮转
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileId {
    size: u64,
    modified_secs: i64,
}

impl FileId {
    /// 从文件元数据创建文件标识
    fn from_metadata(metadata: &std::fs::Metadata) -> Option<Self> {
        let size = metadata.len();
        let modified = metadata.modified().ok()?;
        let modified_secs = modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
        Some(Self { size, modified_secs })
    }
}

impl LogMonitor {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        // 更灵活的正则表达式，支持多种 Minecraft 版本：
        // - 时间部分：小时可以有或没有前导零 [9:30:45] 或 [09:30:45]
        // - 线程名称：支持 [Server thread/INFO]、[Server thread/INFO] [Minecraft] 等
        // - 玩家消息：<玩家名> 消息内容
        let chat_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: <([^>]+)> (.+)")
            .context("Failed to compile chat pattern")?;
        
        let join_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (\w+) joined the game")
            .context("Failed to compile join pattern")?;
        
        let leave_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (\w+) left the game")
            .context("Failed to compile leave pattern")?;
        
        let death_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (\w+) .*(died|was|fell|drowned|blew up|burned|froze|suffocated|starved)")
            .context("Failed to compile death pattern")?;

        Ok(Self {
            log_path,
            chat_pattern,
            join_pattern,
            leave_pattern,
            death_pattern,
        })
    }

    pub fn start_monitoring(self) -> Result<Receiver<LogEvent>> {
        let (tx, rx) = mpsc::channel(100);
        
        let log_path = self.log_path.clone();
        
        info!("Started monitoring log file: {:?}", log_path);

        let mut last_offset: u64 = 0;
        let mut last_file_id: Option<FileId> = None;

        if log_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&log_path) {
                last_offset = metadata.len();
                last_file_id = FileId::from_metadata(&metadata);
            }
        } else {
            warn!(
                "Log file not found: {:?}. \n\
                 Please start the Minecraft server first to generate the log file.",
                log_path
            );
        }

        let (notify_tx, notify_rx) = std::sync::mpsc::channel();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            },
            Config::default(),
        ).context("Failed to create file watcher")?;

        let parent_dir = log_path
            .parent()
            .context("Log file has no parent directory")?
            .to_path_buf();
        
        let parent_dir_for_unwatch = parent_dir.clone();
        
        watcher
            .watch(&parent_dir, RecursiveMode::NonRecursive)
            .context("Failed to watch log directory")?;

        let patterns = (
            self.chat_pattern,
            self.join_pattern,
            self.leave_pattern,
            self.death_pattern,
        );

        std::thread::spawn(move || {
            loop {
                match notify_rx.recv() {
                    Ok(event) => {
                        if !event.paths.iter().any(|p| p.file_name().map(|n| n == "latest.log").unwrap_or(false)) {
                            continue;
                        }

                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                if let Ok(events) = Self::check_file_changes(
                                    &log_path,
                                    &mut last_offset,
                                    &mut last_file_id,
                                    &patterns,
                                ) {
                                    for log_event in events {
                                        if tx.blocking_send(log_event).is_err() {
                                            debug!("Receiver dropped, stopping monitor");
                                            return;
                                        }
                                    }
                                }
                            }
                            EventKind::Remove(_) => {
                                debug!("Log file removed/rotated, resetting state");
                                last_offset = 0;
                                last_file_id = None;
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {
                        debug!("Notify channel closed, stopping monitor");
                        break;
                    }
                }
            }
            let _ = watcher.unwatch(&parent_dir_for_unwatch);
        });

        Ok(rx)
    }

    fn check_file_changes(
        log_path: &PathBuf,
        last_offset: &mut u64,
        last_file_id: &mut Option<FileId>,
        patterns: &(Regex, Regex, Regex, Regex),
    ) -> Result<Vec<LogEvent>> {
        // 检查文件是否存在
        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let metadata = std::fs::metadata(log_path)?;
        let current_file_id = FileId::from_metadata(&metadata);

        // 检测文件轮转：如果文件标识变化，重置偏移量
        if let (Some(current), Some(last)) = (current_file_id, *last_file_id) {
            if current != last {
                debug!("File rotation detected, resetting offset");
                *last_offset = 0;
            }
        }

        let current_size = metadata.len();

        // 如果文件变小了，说明可能被截断或轮转，重置偏移量
        if current_size < *last_offset {
            debug!("File size decreased, resetting offset");
            *last_offset = 0;
        }

        // 没有新内容
        if current_size == *last_offset {
            return Ok(Vec::new());
        }

        // 从 last_offset 位置读取到文件末尾
        let new_content = Self::read_from_offset(log_path, *last_offset, current_size)?;

        // 更新状态
        *last_offset = current_size;
        *last_file_id = current_file_id;

        let events = Self::parse_lines(&new_content, patterns);
        Ok(events)
    }

    /// 从指定偏移量读取文件内容
    fn read_from_offset(log_path: &PathBuf, offset: u64, end: u64) -> Result<String> {
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom};

        let mut file = File::open(log_path)?;
        file.seek(SeekFrom::Start(offset))?;

        let bytes_to_read = (end - offset) as usize;
        let mut buffer = Vec::with_capacity(bytes_to_read);
        file.take(bytes_to_read as u64).read_to_end(&mut buffer)?;

        String::from_utf8(buffer.clone())
            .map_err(|e| {
                let lossy = String::from_utf8_lossy(&buffer);
                warn!("UTF-8 decode error, using lossy conversion: {}", e);
                anyhow::anyhow!("Failed to convert file content to UTF-8: {}", lossy)
            })
            .or_else(|_| Ok(String::from_utf8_lossy(&buffer).into_owned()))
    }

    fn parse_lines(content: &str, patterns: &(Regex, Regex, Regex, Regex)) -> Vec<LogEvent> {
        let (chat_pattern, join_pattern, leave_pattern, death_pattern) = patterns;
        let mut events = Vec::new();
        
        for line in content.lines() {
            if let Some(caps) = chat_pattern.captures(line) {
                if let (Some(player), Some(content)) = (caps.get(2), caps.get(3)) {
                    // Debug: log parsed chat events for troubleshooting
                    debug!("[Monitor] Parsed chat event: player='{}', content='{}'", player.as_str(), content.as_str());
                    events.push(LogEvent::Chat(ChatMessage {
                        player: player.as_str().to_string(),
                        content: content.as_str().to_string(),
                        timestamp: chrono::Local::now(),
                    }));
                }
            } else if let Some(caps) = join_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerJoin(player.as_str().to_string()));
                }
            } else if let Some(caps) = leave_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerLeave(player.as_str().to_string()));
                }
            } else if let Some(caps) = death_pattern.captures(line) {
                if let Some(player) = caps.get(2) {
                    events.push(LogEvent::PlayerDeath(player.as_str().to_string()));
                }
            }
        }
        
        events
    }
}
