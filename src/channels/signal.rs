//! Signal Channel
//!
//! Signal integration via signal-cli REST API or libsignal.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Signal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Phone number for this account
    pub phone_number: String,
    /// Signal-cli REST API URL
    pub signal_cli_api_url: String,
    /// Optional authorization token for signal-cli REST API
    pub authorization: Option<String>,
}

/// Signal channel
pub struct SignalChannel {
    config: SignalConfig,
}

impl SignalChannel {
    /// Create new Signal channel
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    /// Start the connection
    pub async fn start(&self) -> Result<(), SignalError> {
        tracing::info!(phone_number = %self.config.phone_number, "Starting Signal connection");
        // TODO: Verify connection to signal-cli REST API
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
        // TODO: Implement via signal-cli REST API
        Ok(())
    }
}

/// Signal errors
#[derive(Debug, Error)]
pub enum SignalError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Registration required")]
    RegistrationRequired,
    #[error("Send failed: {0}")]
    SendFailed(String),
}
