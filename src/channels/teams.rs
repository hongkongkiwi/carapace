//! Microsoft Teams Channel
//!
//! Microsoft Teams integration via Bot Framework.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Teams configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Bot Application ID
    pub app_id: String,
    /// Bot Application password
    pub app_password: String,
}

/// Teams channel implementation
pub struct TeamsChannel {
    config: TeamsConfig,
}

impl TeamsChannel {
    /// Create a new Teams channel
    pub fn new(config: TeamsConfig) -> Self {
        Self { config }
    }

    /// Start the channel
    pub async fn start(&self) -> Result<(), TeamsError> {
        tracing::info!("Connecting to Microsoft Teams");
        Ok(())
    }

    /// Stop the channel
    pub async fn stop(&self) {
        tracing::info!("Disconnecting from Microsoft Teams");
    }
}

/// Teams errors
#[derive(Debug, Error)]
pub enum TeamsError {
    #[error("Connection error: {0}")]
    Connection(String),
}
