//! iMessage Channel
//!
//! macOS iMessage integration via BlueBubbles API or private APIs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// iMessage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    /// BlueBubbles server URL (recommended)
    pub bluebubbles_url: Option<String>,
    /// BlueBobbles API key
    pub api_key: Option<String>,
    /// Enable sending via private APIs (requires Full Disk Access)
    pub enable_private_api: bool,
    /// Handle (phone/email) to use
    pub handle: Option<String>,
}

/// iMessage channel
pub struct IMessageChannel {
    config: IMessageConfig,
}

impl IMessageChannel {
    /// Create new iMessage channel
    pub fn new(config: IMessageConfig) -> Self {
        Self { config }
    }

    /// Start listening for messages
    pub async fn start(&self) -> Result<(), IMessageError> {
        tracing::info!("Starting iMessage listener");
        // TODO: Connect to BlueBubbles or initialize private API
        Ok(())
    }

    /// Stop listening
    pub async fn stop(&self) -> Result<(), IMessageError> {
        tracing::info!("Stopping iMessage listener");
        Ok(())
    }

    /// Send message
    pub async fn send_message(&self, to: &str, text: &str) -> Result<(), IMessageError> {
        tracing::info!(to = to, text = text, "Sending iMessage");
        // TODO: Implement via BlueBubbles API or private API
        Ok(())
    }
}

/// iMessage errors
#[derive(Debug, Error)]
pub enum IMessageError {
    #[error("macOS only")]
    NotMacOS,
    #[error("Full Disk Access required for private API")]
    FullDiskAccessRequired,
    #[error("BlueBubbles connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send error: {0}")]
    SendError(String),
}
