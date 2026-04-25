use anyhow::{Context, Result};
use log::{info, debug, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use std::collections::HashSet;
use std::sync::Arc;
use parking_lot::Mutex;

// ============================================================
// Shared Types
// ============================================================

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub player: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LogEvent {
    // Chat events now come exclusively from ChatCapture implementations.
    // LogMonitor no longer emits this variant, but it's kept for
    // backward compatibility with server_run.rs event processing.
    Chat(ChatMessage),
    PlayerJoin(String),
    PlayerLeave(String),
    PlayerDeath(String),
    ServerStart,
    ServerStop,
}

#[derive(Debug, Clone, PartialEq, Copy)]
struct FileId {
    size: u64,
    modified_secs: i64,
}

impl FileId {
    fn from_metadata(metadata: &std::fs::Metadata) -> Option<Self> {
        let size = metadata.len();
        let modified = metadata.modified().ok()?;
        let modified_secs = modified.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
        Some(Self { size, modified_secs })
    }
}

#[derive(Debug, Clone)]
pub enum ChatCaptureMode {
    Tmux { session: String },
    Process,
    File,
}

// ============================================================
// ChatCapture Trait - Unified interface for chat sources
// ============================================================

/// Trait for chat message capture. Each implementation provides
/// a single source of truth for chat messages, eliminating the
/// double-processing bug where both LogMonitor and ChatCapture
/// detected the same messages.
pub trait ChatCapture: Send {
    /// Capture recent chat messages since the last call.
    /// Implementations should use deduplication to avoid processing
    /// the same message twice.
    fn capture_recent_messages(&mut self) -> Vec<ChatMessage>;

    /// Returns the name of this capture implementation for logging.
    fn name(&self) -> &'static str;
}

// ============================================================
// LogMonitor - Watches log file for non-chat events ONLY
// ============================================================

/// Monitors a log file for server lifecycle events (join, leave, death,
/// start, stop). Chat messages are NOT emitted by LogMonitor — they
/// come from ChatCapture implementations instead.
pub struct LogMonitor {
    log_path: PathBuf,
    join_pattern: Regex,
    leave_pattern: Regex,
    death_pattern: Regex,
}

impl LogMonitor {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        // Join pattern: handles vanilla player names
        let join_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: ([a-zA-Z0-9_]+) joined the game")
            .context("Failed to compile join pattern")?;

        // Leave pattern: handles vanilla player names
        let leave_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: ([a-zA-Z0-9_]+) left the game")
            .context("Failed to compile leave pattern")?;

        // Death pattern: handles various death messages
        let death_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: ([a-zA-Z0-9_]+) .*(died|was|fell|drowned|blew up|burned|froze|suffocated|starved)")
            .context("Failed to compile death pattern")?;

        Ok(Self {
            log_path,
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
                "Log file not found: {:?}. Please start the Minecraft server first to generate the log file.",
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
        patterns: &(Regex, Regex, Regex),
    ) -> Result<Vec<LogEvent>> {
        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let metadata = std::fs::metadata(log_path)?;
        let current_file_id = FileId::from_metadata(&metadata);

        if let (Some(current), Some(last)) = (current_file_id, *last_file_id) {
            if current != last {
                debug!("File rotation detected, resetting offset");
                *last_offset = 0;
            }
        }

        let current_size = metadata.len();

        if current_size < *last_offset {
            debug!("File size decreased, resetting offset");
            *last_offset = 0;
        }

        if current_size == *last_offset {
            return Ok(Vec::new());
        }

        let new_content = Self::read_from_offset(log_path, *last_offset, current_size)?;

        *last_offset = current_size;
        *last_file_id = current_file_id;

        let events = Self::parse_lines(&new_content, patterns);
        Ok(events)
    }

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

    /// Parse log lines for non-chat events only.
    /// Chat events are handled by ChatCapture implementations.
    fn parse_lines(content: &str, patterns: &(Regex, Regex, Regex)) -> Vec<LogEvent> {
        let (join_pattern, leave_pattern, death_pattern) = patterns;
        let mut events = Vec::new();

        for line in content.lines() {
            if let Some(caps) = join_pattern.captures(line) {
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

// ============================================================
// TmuxChatCapture - Captures chat from tmux session pane
// ============================================================

pub struct TmuxChatCapture {
    session: String,
    chat_pattern: Regex,
    seen_positions: Arc<Mutex<HashSet<u64>>>,
}

impl TmuxChatCapture {
    pub fn new(session: String) -> Result<Self> {
        // Handle both vanilla: <Player> message
        // and Fabric: [Not Secure] <Player> message
        let chat_pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)")
            .context("Failed to compile chat pattern")?;

        Ok(Self {
            session,
            chat_pattern,
            seen_positions: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn mode(&self) -> ChatCaptureMode {
        ChatCaptureMode::Tmux { session: self.session.clone() }
    }

    pub fn capture_pane_output(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &self.session])
            .output()
            .context("Failed to execute tmux capture-pane")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "tmux capture-pane failed with exit code: {:?}",
                output.status.code()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn hash_line(line: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        line.hash(&mut hasher);
        hasher.finish()
    }
}

impl ChatCapture for TmuxChatCapture {
    fn capture_recent_messages(&mut self) -> Vec<ChatMessage> {
        let output = match self.capture_pane_output() {
            Ok(o) => o,
            Err(e) => {
                warn!("[TmuxChatCapture] Failed to capture tmux pane: {}", e);
                return Vec::new();
            }
        };

        let mut messages = Vec::new();
        let mut seen = self.seen_positions.lock();

        for line in output.lines().rev() {
            let line_hash = Self::hash_line(line);

            if seen.contains(&line_hash) {
                continue;
            }

            seen.insert(line_hash);

            if seen.len() > 10000 {
                let to_remove: Vec<_> = seen.iter().take(1000).cloned().collect();
                for r in to_remove {
                    seen.remove(&r);
                }
            }

            if let Some(caps) = self.chat_pattern.captures(line) {
                if let (Some(player), Some(content)) = (caps.get(1), caps.get(2)) {
                    debug!("[TmuxChatCapture] Parsed chat: player='{}', content='{}'", player.as_str(), content.as_str());
                    messages.push(ChatMessage {
                        player: player.as_str().to_string(),
                        content: content.as_str().to_string(),
                        timestamp: chrono::Local::now(),
                    });
                }
            }
        }

        messages.reverse();
        messages
    }

    fn name(&self) -> &'static str {
        "TmuxChatCapture"
    }
}

// ============================================================
// FileChatCapture - Captures chat from log file
// ============================================================

pub struct FileChatCapture {
    log_path: PathBuf,
    chat_pattern: Regex,
    seen_positions: Arc<Mutex<HashSet<u64>>>,
}

impl FileChatCapture {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        // Handle both vanilla and [Not Secure] prefixed chat messages in log files
        let chat_pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)")
            .context("Failed to compile chat pattern")?;

        Ok(Self {
            log_path,
            chat_pattern,
            seen_positions: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn mode(&self) -> ChatCaptureMode {
        ChatCaptureMode::File
    }

    fn hash_line(line: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        line.hash(&mut hasher);
        hasher.finish()
    }
}

impl ChatCapture for FileChatCapture {
    fn capture_recent_messages(&mut self) -> Vec<ChatMessage> {
        let content = match std::fs::read_to_string(&self.log_path) {
            Ok(c) => c,
            Err(e) => {
                warn!("[FileChatCapture] Failed to read log file: {}", e);
                return Vec::new();
            }
        };

        let mut messages = Vec::new();
        let mut seen = self.seen_positions.lock();

        for line in content.lines().rev().take(100) {
            let line_hash = Self::hash_line(line);

            if seen.contains(&line_hash) {
                continue;
            }

            seen.insert(line_hash);

            if let Some(caps) = self.chat_pattern.captures(line) {
                if let (Some(player), Some(content)) = (caps.get(2), caps.get(3)) {
                    debug!("[FileChatCapture] Parsed chat: player='{}', content='{}'", player.as_str(), content.as_str());
                    messages.push(ChatMessage {
                        player: player.as_str().to_string(),
                        content: content.as_str().to_string(),
                        timestamp: chrono::Local::now(),
                    });
                }
            }
        }

        messages.reverse();
        messages
    }

    fn name(&self) -> &'static str {
        "FileChatCapture"
    }
}

// ============================================================
// ProcessChatCapture - Parses chat from process stdout lines
// (Used by ForegroundProcess in TUI mode)
// ============================================================

pub struct ProcessChatCapture {
    chat_pattern: Regex,
    #[allow(dead_code)]
    seen_positions: Arc<Mutex<HashSet<u64>>>,
}

impl ProcessChatCapture {
    pub fn new() -> Result<Self> {
        // Handle both vanilla: <Player> message
        // and Fabric: [Not Secure] <Player> message
        let chat_pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)")
            .context("Failed to compile chat pattern")?;

        Ok(Self {
            chat_pattern,
            seen_positions: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    /// Parse a single line for chat messages.
    /// Used by ForegroundProcess to parse stdout lines.
    pub fn parse_line(&self, line: &str) -> Option<ChatMessage> {
        if let Some(caps) = self.chat_pattern.captures(line) {
            if let (Some(player), Some(content)) = (caps.get(1), caps.get(2)) {
                return Some(ChatMessage {
                    player: player.as_str().to_string(),
                    content: content.as_str().to_string(),
                    timestamp: chrono::Local::now(),
                });
            }
        }
        None
    }
}

impl ChatCapture for ProcessChatCapture {
    fn capture_recent_messages(&mut self) -> Vec<ChatMessage> {
        // ProcessChatCapture doesn't poll; messages are pushed to it
        // by the ForegroundProcess via parse_line(). This method
        // returns empty since we don't have a file/tmux to poll.
        Vec::new()
    }

    fn name(&self) -> &'static str {
        "ProcessChatCapture"
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_chat_pattern_vanilla() {
        let pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        let caps = pattern.captures("<Steve> hello world").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Steve");
        assert_eq!(caps.get(2).unwrap().as_str(), "hello world");
    }

    #[test]
    fn test_tmux_chat_pattern_not_secure() {
        let pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        let caps = pattern.captures("[Not Secure] <Player_1> !help").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Player_1");
        assert_eq!(caps.get(2).unwrap().as_str(), "!help");
    }

    #[test]
    fn test_file_chat_pattern_vanilla() {
        let pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        let line = "[12:34:56] [Server thread/INFO]: <Steve> hello";
        let caps = pattern.captures(line).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "Steve");
        assert_eq!(caps.get(3).unwrap().as_str(), "hello");
    }

    #[test]
    fn test_file_chat_pattern_not_secure() {
        let pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: (?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        let line = "[12:34:56] [Server thread/INFO]: [Not Secure] <Player_1> !help";
        let caps = pattern.captures(line).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "Player_1");
        assert_eq!(caps.get(3).unwrap().as_str(), "!help");
    }

    #[test]
    fn test_process_chat_capture_parse_line() {
        let capture = ProcessChatCapture::new().unwrap();
        
        // Vanilla
        let msg = capture.parse_line("<Steve> hello world").unwrap();
        assert_eq!(msg.player, "Steve");
        assert_eq!(msg.content, "hello world");

        // Not Secure prefix
        let msg = capture.parse_line("[Not Secure] <Player_1> !help").unwrap();
        assert_eq!(msg.player, "Player_1");
        assert_eq!(msg.content, "!help");

        // Non-chat line
        assert!(capture.parse_line("Server started on port 25565").is_none());
    }

    #[test]
    fn test_log_monitor_join_pattern() {
        let pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: ([a-zA-Z0-9_]+) joined the game").unwrap();
        let line = "[12:34:56] [Server thread/INFO]: Steve joined the game";
        let caps = pattern.captures(line).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "Steve");
    }

    #[test]
    fn test_log_monitor_leave_pattern() {
        let pattern = Regex::new(r"\[(\d{1,2}:\d{2}:\d{2})\] \[[^\]]+\]: ([a-zA-Z0-9_]+) left the game").unwrap();
        let line = "[12:34:56] [Server thread/INFO]: Player_1 left the game";
        let caps = pattern.captures(line).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "Player_1");
    }
}