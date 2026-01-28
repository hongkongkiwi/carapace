//! Signal Channel
//!
//! Signal integration via signal-cli or libsignal.

use serde::{Deserialize, Serialize};

/// Signal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Phone number for this account
    pub phone_number: String,
    /// Device ID
    #[serde(default = "default_device_id")]
    pub device_id: i32,
    /// Data directory
    pub data_dir: Option<String>,
}

fn default_device_id() -> i32 {
    1
}

/// Signal channel
pub struct SignalChannel {
    #[allow(dead_code)]
    config: SignalConfig,
}

impl SignalChannel {
    /// Create new Signal channel
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    /// Start the connection
    pub async fn start(&self) -> Result<(), SignalError> {
        tracing::info!("Starting Signal connection");
        Ok(())
    }

    /// Stop the connection
    pub async fn stop(&self) -> Result<(), SignalError> {
        tracing::info!("Stopping Signal connection");
        Ok(())
    }

    /// Send message
    pub async fn send_message(
        &self,
        to: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        tracing::info!(to = to, text = text, "Sending Signal message");
        Ok(())
    }
}

/// Signal errors
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Not registered")]
    NotRegistered,
    #[error("Send error: {0}")]
    SendError(String),
}
