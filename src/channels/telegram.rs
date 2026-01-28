//! Telegram Channel
//!
//! Telegram bot integration for carapace.

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
}

/// Telegram channel implementation
pub struct TelegramChannel {
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
        Ok(())
    }
}

/// Telegram errors
#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}
