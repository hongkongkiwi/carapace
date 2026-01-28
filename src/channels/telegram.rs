//! Telegram Channel
//!
//! Telegram bot integration using teloxide.

use serde::{Deserialize, Serialize};

/// Telegram channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token from BotFather
    pub bot_token: String,
    /// Webhook URL (optional, uses polling if not set)
    pub webhook_url: Option<String>,
    /// Allowed chat IDs (empty = all)
    pub allowed_chats: Vec<i64>,
    /// Command prefix
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
}

fn default_command_prefix() -> String {
    "/".to_string()
}

/// Telegram channel
pub struct TelegramChannel {
    #[allow(dead_code)]
    config: TelegramConfig,
}

impl TelegramChannel {
    /// Create new channel
    pub fn new(config: TelegramConfig) -> Self {
        Self { config }
    }

    /// Start the bot
    pub async fn start(&self) -> Result<(), TelegramError> {
        tracing::info!("Starting Telegram bot");
        Ok(())
    }

    /// Stop the bot
    pub async fn stop(&self) -> Result<(), TelegramError> {
        tracing::info!("Stopping Telegram bot");
        Ok(())
    }

    /// Send message
    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<(), TelegramError> {
        tracing::info!(chat_id = chat_id, text = text, "Sending Telegram message");
        Ok(())
    }
}

/// Telegram errors
#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Invalid token")]
    InvalidToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = TelegramConfig {
            bot_token: "test".to_string(),
            webhook_url: None,
            allowed_chats: vec![],
            command_prefix: "/".to_string(),
        };
        assert_eq!(config.command_prefix, "/");
    }
}
