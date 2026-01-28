//! Discord Channel
//!
//! Discord bot integration using serenity.

use serde::{Deserialize, Serialize};

/// Discord configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token
    pub bot_token: String,
    /// Application ID
    pub application_id: u64,
    /// Gateway intents (as bitmask)
    pub intents: u64,
    /// Default prefix for commands
    #[serde(default = "default_prefix")]
    pub prefix: String,
}

fn default_prefix() -> String {
    "!".to_string()
}

/// Discord channel
pub struct DiscordChannel {
    #[allow(dead_code)]
    config: DiscordConfig,
}

impl DiscordChannel {
    /// Create new Discord channel
    pub fn new(config: DiscordConfig) -> Self {
        Self { config }
    }

    /// Start the bot
    pub async fn start(&self) -> Result<(), DiscordError> {
        tracing::info!("Starting Discord bot");
        Ok(())
    }

    /// Stop the bot
    pub async fn stop(&self) -> Result<(), DiscordError> {
        tracing::info!("Stopping Discord bot");
        Ok(())
    }

    /// Send message to a channel
    pub async fn send_message(
        &self,
        channel_id: u64,
        content: &str,
    ) -> Result<(), DiscordError> {
        tracing::info!(channel_id = channel_id, content = content, "Sending Discord message");
        Ok(())
    }
}

/// Discord errors
#[derive(Debug, thiserror::Error)]
pub enum DiscordError {
    #[error("Gateway error: {0}")]
    Gateway(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Invalid token")]
    InvalidToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = DiscordConfig {
            bot_token: "test".to_string(),
            application_id: 123456,
            intents: 0,
            prefix: "!".to_string(),
        };
        assert_eq!(config.prefix, "!");
    }
}
