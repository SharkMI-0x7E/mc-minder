use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::RwLock;
use chrono::{DateTime, Local};
use crate::ai::Message;

const MAX_CONTEXT_MESSAGES: usize = 20;
const CONTEXT_EXPIRY_HOURS: i64 = 2;

pub struct ContextManager {
    messages: Arc<RwLock<VecDeque<ContextEntry>>>,
    max_messages: usize,
    expiry_hours: i64,
}

#[derive(Debug, Clone)]
pub struct ContextEntry {
    pub message: Message,
    pub timestamp: DateTime<Local>,
    pub player: Option<String>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_CONTEXT_MESSAGES))),
            max_messages: MAX_CONTEXT_MESSAGES,
            expiry_hours: CONTEXT_EXPIRY_HOURS,
        }
    }

    pub fn add_message(&self, role: &str, content: &str, player: Option<&str>) {
        let entry = ContextEntry {
            message: Message {
                role: role.to_string(),
                content: content.to_string(),
            },
            timestamp: Local::now(),
            player: player.map(|s| s.to_string()),
        };

        let mut messages = self.messages.write();
        messages.push_back(entry);

        while messages.len() > self.max_messages {
            messages.pop_front();
        }
    }

    pub fn add_user_message(&self, content: &str, player: &str) {
        self.add_message("user", content, Some(player));
    }

    #[allow(dead_code)]
    pub fn add_assistant_message(&self, content: &str) {
        self.add_message("assistant", content, None);
    }

    pub fn add_assistant_message_for_player(&self, content: &str, player: &str) {
        self.add_message("assistant", content, Some(player));
    }

    pub fn add_system_message(&self, content: &str) {
        self.add_message("system", content, None);
    }

    pub fn get_messages(&self) -> Vec<Message> {
        let messages = self.messages.read();
        let now = Local::now();
        
        messages
            .iter()
            .filter(|entry| {
                let elapsed = now.signed_duration_since(entry.timestamp);
                elapsed.num_hours() < self.expiry_hours
            })
            .map(|entry| entry.message.clone())
            .collect()
    }

    pub fn get_messages_for_player(&self, player: &str) -> Vec<Message> {
        let messages = self.messages.read();
        let now = Local::now();
        
        messages
            .iter()
            .filter(|entry| {
                let elapsed = now.signed_duration_since(entry.timestamp);
                elapsed.num_hours() < self.expiry_hours
            })
            .filter(|entry| {
                entry.player.as_deref() == Some(player) || entry.player.is_none()
            })
            .map(|entry| entry.message.clone())
            .collect()
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut messages = self.messages.write();
        messages.clear();
    }

    pub fn len(&self) -> usize {
        self.messages.read().len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.messages.read().is_empty()
    }

    #[allow(dead_code)]
    pub fn get_recent_players(&self, count: usize) -> Vec<String> {
        let messages = self.messages.read();
        let mut players = Vec::new();
        
        for entry in messages.iter().rev() {
            if let Some(ref player) = entry.player {
                if !players.contains(player) {
                    players.push(player.clone());
                    if players.len() >= count {
                        break;
                    }
                }
            }
        }
        
        players
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
