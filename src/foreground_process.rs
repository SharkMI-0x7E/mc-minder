// ForegroundProcess - manages a Minecraft server process with piped stdio
// Used by TUI mode for real-time console view and chat capture
#![allow(dead_code)]

use anyhow::{Context, Result};
use log::{debug, info, warn};
use regex::Regex;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};

use crate::monitor::ChatMessage;

/// Lines captured from the server process stdout/stderr
#[derive(Debug, Clone)]
pub enum ProcessOutput {
    Stdout(String),
    Stderr(String),
}

/// A Minecraft server process with piped stdio for real-time interaction
pub struct ForegroundProcess {
    child: Option<Child>,
    stdin: Arc<Mutex<ChildStdin>>,
    chat_rx: mpsc::Receiver<ChatMessage>,
    console_rx: mpsc::Receiver<ProcessOutput>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    chat_pattern: Regex,
    // Patterns for log-format lines from stdout (for events)
    join_pattern: Regex,
    leave_pattern: Regex,
}

impl ForegroundProcess {
    /// Spawn a Minecraft server process
    pub async fn spawn(
        jar: &str,
        min_mem: &str,
        max_mem: &str,
        jvm_flags: Option<&str>,
        jdk_path: Option<&str>,
    ) -> Result<Self> {
        let java_cmd = if let Some(path) = jdk_path {
            if !path.is_empty() { path.to_string() } else { "java".to_string() }
        } else {
            "java".to_string()
        };

        let mut args = vec![
            format!("-Xms{}", min_mem),
            format!("-Xmx{}", max_mem),
        ];

        if let Some(flags) = jvm_flags {
            if !flags.is_empty() {
                for flag in flags.split_whitespace() {
                    args.push(flag.to_string());
                }
            }
        }

        args.extend(vec!["-jar".to_string(), jar.to_string(), "nogui".to_string()]);

        info!("[ForegroundProcess] Spawning: {} {}", java_cmd, args.join(" "));

        let mut child = Command::new(&java_cmd)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn Minecraft server process")?;

        let stdin = child.stdin.take().context("Failed to get stdin")?;
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        let stdin = Arc::new(Mutex::new(stdin));

        // Chat pattern: handles both vanilla and [Not Secure] prefixed
        let chat_pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)")
            .context("Failed to compile chat pattern")?;
        let join_pattern = Regex::new(r"([a-zA-Z0-9_]+) joined the game")
            .context("Failed to compile join pattern")?;
        let leave_pattern = Regex::new(r"([a-zA-Z0-9_]+) left the game")
            .context("Failed to compile leave pattern")?;

        let (chat_tx, chat_rx) = mpsc::channel(100);
        let (console_tx, console_rx) = mpsc::channel(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Clone console_tx for stderr task before moving tx into stdout task
        let console_tx_stderr = console_tx.clone();

        // Spawn stdout reader task
        let chat_pattern_clone = chat_pattern.clone();
        let join_pattern_clone = join_pattern.clone();
        let leave_pattern_clone = leave_pattern.clone();
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let _ = console_tx.send(ProcessOutput::Stdout(line.clone())).await;

                                // Parse for chat messages
                                if let Some(caps) = chat_pattern_clone.captures(&line) {
                                    if let (Some(player), Some(content)) = (caps.get(1), caps.get(2)) {
                                        let msg = ChatMessage {
                                            player: player.as_str().to_string(),
                                            content: content.as_str().to_string(),
                                            timestamp: chrono::Local::now(),
                                        };
                                        let _ = chat_tx.send(msg).await;
                                    }
                                }

                                // Parse for join/leave events (logging only)
                                if let Some(caps) = join_pattern_clone.captures(&line) {
                                    if let Some(player) = caps.get(1) {
                                        info!("[ForegroundProcess] Player {} joined", player.as_str());
                                    }
                                }
                                if let Some(caps) = leave_pattern_clone.captures(&line) {
                                    if let Some(player) = caps.get(1) {
                                        info!("[ForegroundProcess] Player {} left", player.as_str());
                                    }
                                }
                            }
                            Ok(None) => {
                                info!("[ForegroundProcess] stdout EOF");
                                break;
                            }
                            Err(e) => {
                                warn!("[ForegroundProcess] stdout read error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("[ForegroundProcess] stdout reader shutting down");
                        break;
                    }
                }
            }
        });

        // Spawn stderr reader task
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let _ = console_tx_stderr.send(ProcessOutput::Stderr(line)).await;
                    }
                    Ok(None) => {
                        debug!("[ForegroundProcess] stderr EOF");
                        break;
                    }
                    Err(e) => {
                        warn!("[ForegroundProcess] stderr read error: {}", e);
                        break;
                    }
                }
            }
        });

        info!("[ForegroundProcess] Server process spawned successfully");

        Ok(Self {
            child: Some(child),
            stdin,
            chat_rx,
            console_rx,
            shutdown_tx: Some(shutdown_tx),
            chat_pattern,
            join_pattern,
            leave_pattern,
        })
    }

    /// Send a command to the server via stdin
    pub async fn send_command(&self, command: &str) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(command.as_bytes()).await
            .context("[ForegroundProcess] Failed to write command to stdin")?;
        stdin.write_all(b"\n").await
            .context("[ForegroundProcess] Failed to write newline to stdin")?;
        stdin.flush().await
            .context("[ForegroundProcess] Failed to flush stdin")?;
        debug!("[ForegroundProcess] Sent command: {}", command);
        Ok(())
    }

    /// Receive the next chat message from the server (non-blocking)
    pub fn recv_chat_message(&mut self) -> Option<ChatMessage> {
        self.chat_rx.try_recv().ok()
    }

    /// Receive the next console output line (non-blocking)
    pub fn recv_console_output(&mut self) -> Option<ProcessOutput> {
        self.console_rx.try_recv().ok()
    }

    /// Receive the next chat message (async, waits for one)
    pub async fn next_chat_message(&mut self) -> Option<ChatMessage> {
        self.chat_rx.recv().await
    }

    /// Receive the next console output (async, waits for one)
    pub async fn next_console_output(&mut self) -> Option<ProcessOutput> {
        self.console_rx.recv().await
    }

    /// Check if the server process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,   // Still running
                Ok(Some(_)) => false, // Exited
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Kill the server process
    pub async fn kill(&mut self) -> Result<()> {
        // Signal shutdown to reader tasks
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        if let Some(ref mut child) = self.child {
            child.kill().await
                .context("[ForegroundProcess] Failed to kill server process")?;
            info!("[ForegroundProcess] Server process killed");
        }
        Ok(())
    }

    /// Gracefully stop the server by sending "stop" command
    pub async fn graceful_stop(&self) -> Result<()> {
        self.send_command("stop").await
    }

    /// Wait for the server process to exit and return the exit status
    pub async fn wait(mut self) -> Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            let status = child.wait().await
                .context("[ForegroundProcess] Failed to wait for server process")?;
            info!("[ForegroundProcess] Server exited with status: {}", status);
            Ok(status)
        } else {
            Err(anyhow::anyhow!("[ForegroundProcess] No process to wait for"))
        }
    }

    /// Parse a line for chat messages (useful for testing)
    pub fn parse_chat_line(&self, line: &str) -> Option<ChatMessage> {
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

impl Drop for ForegroundProcess {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // Best-effort kill on drop
            let _ = child.start_kill();
            warn!("[ForegroundProcess] Process killed on drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_pattern_vanilla() {
        let pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        
        // Vanilla chat
        let caps = pattern.captures("<Steve> hello world").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Steve");
        assert_eq!(caps.get(2).unwrap().as_str(), "hello world");
    }

    #[test]
    fn test_chat_pattern_not_secure() {
        let pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        
        // Fabric with [Not Secure] prefix
        let caps = pattern.captures("[Not Secure] <Player_1> !help").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Player_1");
        assert_eq!(caps.get(2).unwrap().as_str(), "!help");
    }

    #[test]
    fn test_chat_pattern_with_timestamp() {
        let pattern = Regex::new(r"(?:\[Not Secure\] )?<([a-zA-Z0-9_]+)> (.+)").unwrap();
        
        // With timestamp prefix (tmux output might have this)
        let line = "[12:34:56] [Server thread/INFO]: <Admin> test message";
        // The pattern without timestamp prefix won't match the full line,
        // but will match a stripped version
        let stripped = line.split("]: ").last().unwrap_or(line);
        let caps = pattern.captures(stripped).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Admin");
        assert_eq!(caps.get(2).unwrap().as_str(), "test message");
    }
}