//! LINE Channel
//!
//! LINE messaging platform integration.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// LINE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    /// LINE Messaging API channel access token
    pub channel_access_token: String,
    /// Channel secret
    pub channel_secret: String,
}

/// LINE channel implementation
pub struct LineChannel {
    config: LineConfig,
}

impl LineChannel {
    /// Create a new LINE channel
    pub fn new(config: LineConfig) -> Self {
        Self { config }
    }

    /// Start the channel
    pub async fn start(&self) -> Result<(), LineError> {
        tracing::info!("Connecting to LINE");
        Ok(())
    }

    /// Stop the channel
    pub async fn stop(&self) {
        tracing::info!("Disconnecting from LINE");
    }
}

/// LINE errors
#[derive(Debug, Error)]
pub enum LineError {
    #[error("Connection error: {0}")]
    Connection(String),
}
