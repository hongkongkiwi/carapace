//! WhatsApp Channel
//!
//! WhatsApp integration via whatsmeow (Go) or Baileys (JS bridge).

use serde::{Deserialize, Serialize};

/// WhatsApp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Phone number for this account
    pub phone_number: String,
    /// Session name
    pub session_name: String,
    /// Data directory for session storage
    pub data_dir: Option<String>,
}

/// WhatsApp channel
pub struct WhatsAppChannel {
    #[allow(dead_code)]
    config: WhatsAppConfig,
}

impl WhatsAppChannel {
    /// Create new WhatsApp channel
    pub fn new(config: WhatsAppConfig) -> Self {
        Self { config }
    }

    /// Start the connection
    pub async fn start(&self) -> Result<(), WhatsAppError> {
        tracing::info!("Starting WhatsApp connection");
        Ok(())
    }

    /// Stop the connection
    pub async fn stop(&self) -> Result<(), WhatsAppError> {
        tracing::info!("Stopping WhatsApp connection");
        Ok(())
    }

    /// Send message
    pub async fn send_message(
        &self,
        to: &str,
        text: &str,
    ) -> Result<(), WhatsAppError> {
        tracing::info!(to = to, text = text, "Sending WhatsApp message");
        Ok(())
    }
}

/// WhatsApp errors
#[derive(Debug, thiserror::Error)]
pub enum WhatsAppError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("QR code timeout")]
    QrTimeout,
    #[error("Not authenticated")]
    NotAuthenticated,
}
