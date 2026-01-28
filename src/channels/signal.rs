//! Signal Channel
//!
//! Signal integration via signal-cli or libsignal.

use serde::{Deserialize, Serialize};

/// Signal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Phone number for this account
    pub phone_number: String,
    /// Path to signal-cli binary or libsignal
    pub cli_path: Option<String>,
    /// Data directory
    pub data_dir: Option<String>,
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
        recipient: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        tracing::info!(recipient = recipient, text = text, "Sending Signal message");
        Ok(())
    }
}

/// Signal errors
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Registration required")]
    RegistrationRequired,
}
