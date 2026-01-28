//! iMessage Channel
//!
//! macOS iMessage integration via private APIs.

use serde::{Deserialize, Serialize};

/// iMessage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    /// Enable sending (requires Full Disk Access)
    pub enable_sending: bool,
    /// Handle (phone/email) to use
    pub handle: Option<String>,
}

/// iMessage channel
pub struct IMessageChannel {
    #[allow(dead_code)]
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
        Ok(())
    }

    /// Stop listening
    pub async fn stop(&self) -> Result<(), IMessageError> {
        tracing::info!("Stopping iMessage listener");
        Ok(())
    }

    /// Send message (macOS only)
    pub async fn send_message(
        &self,
        to: &str,
        text: &str,
    ) -> Result<(), IMessageError> {
        tracing::info!(to = to, text = text, "Sending iMessage");
        Ok(())
    }
}

/// iMessage errors
#[derive(Debug, thiserror::Error)]
pub enum IMessageError {
    #[error("macOS only")]
    NotMacOS,
    #[error("Full Disk Access required")]
    FullDiskAccessRequired,
    #[error("Send error: {0}")]
    SendError(String),
}
