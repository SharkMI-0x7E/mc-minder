// Telegram notification module
// Extracted from main.rs for modularity

use log::{info, warn};
use reqwest::Client;
use serde_json::json;

use crate::config::Config;

/// Send Telegram notification
pub async fn send_telegram_notification(
    client: &Client,
    config: &Config,
    message: &str,
) {
    // Check if Telegram configuration is empty
    let bot_token = &config.notification.telegram_bot_token;
    let chat_id = &config.notification.telegram_chat_id;

    if bot_token.is_empty() || chat_id.is_empty() {
        warn!("Telegram notification skipped: bot_token or chat_id is empty");
        return;
    }

    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let payload = json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "Markdown"
    });

    match client.post(&url).json(&payload).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                info!("Telegram notification sent successfully");
            } else {
                if let Ok(text) = response.text().await {
                    warn!("Telegram API error ({}): {}", status, text);
                } else {
                    warn!("Telegram API error: HTTP {}", status);
                }
            }
        }
        Err(e) => {
            warn!("Failed to send Telegram notification: {}", e);
        }
    }
}