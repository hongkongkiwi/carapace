//! Mattermost Channel
//!
//! Mattermost self-hosted messaging platform integration.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Mattermost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    /// Mattermost server URL
    pub server_url: String,
    /// Access token
    pub access_token: String,
}

/// Mattermost channel implementation
pub struct MattermostChannel {
    config: MattermostConfig,
}

impl MattermostChannel {
    /// Create a new Mattermost channel
    pub fn new(config: MattermostConfig) -> Self {
        Self { config }
    }

    /// Start the channel
    pub async fn start(&self) -> Result<(), MattermostError> {
        tracing::info!(server_url = %self.config.server_url, "Connecting to Mattermost");
        Ok(())
    }

    /// Stop the channel
    pub async fn stop(&self) {
        tracing::info!("Disconnecting from Mattermost");
    }
}

/// Mattermost errors
#[derive(Debug, Error)]
pub enum MattermostError {
    #[error("Connection error: {0}")]
    Connection(String),
}
