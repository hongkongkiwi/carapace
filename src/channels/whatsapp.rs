//! WhatsApp Channel
//!
//! WhatsApp integration via whatsmeow library.

use serde::{Deserialize, Serialize};

/// WhatsApp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Session file path for persistence
    #[serde(default = "default_session_path")]
    pub session_path: String,
    /// QR code timeout in seconds
    #[serde(default = "default_qr_timeout")]
    pub qr_timeout: u64,
}

fn default_session_path() -> String {
    "~/.carapace/whatsapp-session.db".to_string()
}

fn default_qr_timeout() -> u64 {
    60
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            session_path: default_session_path(),
            qr_timeout: default_qr_timeout(),
        }
    }
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
        // TODO: Implement whatsmeow client
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
        jid: &str,
        content: &str,
    ) -> Result<(), WhatsAppError> {
        tracing::info!(jid = jid, content = content, "Sending WhatsApp message");
        // TODO: Implement message sending
        Ok(())
    }
}

/// WhatsApp errors
#[derive(Debug, thiserror::Error)]
pub enum WhatsAppError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("QR scan failed")]
    QrFailed,
    #[error("Session error: {0}")]
    Session(String),
}
