//! Google Chat Channel
//!
//! Google Chat (formerly Hangouts Chat) integration.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Google Chat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    /// Bot token for authentication
    pub bot_token: String,
}

/// Google Chat channel implementation
pub struct GoogleChatChannel {
    config: GoogleChatConfig,
}

impl GoogleChatChannel {
    /// Create a new Google Chat channel
    pub fn new(config: GoogleChatConfig) -> Self {
        Self { config }
    }

    /// Start the channel
    pub async fn start(&self) -> Result<(), GoogleChatError> {
        tracing::info!("Connecting to Google Chat");
        Ok(())
    }

    /// Stop the channel
    pub async fn stop(&self) {
        tracing::info!("Disconnecting from Google Chat");
    }
}

/// Google Chat errors
#[derive(Debug, Error)]
pub enum GoogleChatError {
    #[error("Connection error: {0}")]
    Connection(String),
}
