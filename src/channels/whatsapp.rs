//! WhatsApp Channel Implementation
//!
//! Provides messaging support via Twilio WhatsApp API.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// WhatsApp channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhatsAppConfig {
    /// Twilio Account SID
    pub account_sid: String,
    /// Twilio Auth Token
    pub auth_token: String,
    /// Twilio Phone Number (for WhatsApp, format: whatsapp:+1234567890)
    pub from_number: String,
    /// Webhook URL for incoming messages
    pub webhook_url: Option<String>,
    /// Media URL base (for media messages)
    pub media_base_url: Option<String>,
}

/// WhatsApp channel error
#[derive(Debug, thiserror::Error)]
pub enum WhatsAppError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Authentication error: {0}")]
    Authentication(String),
}

/// WhatsApp channel struct
#[derive(Debug)]
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    client: reqwest::Client,
    event_tx: mpsc::Sender<MessageContent>,
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            config,
            client,
            event_tx,
        }
    }

    /// Send a WhatsApp message via Twilio
    pub async fn send_message(
        &self,
        to: &str,
        body: &str,
        media_url: Option<&str>,
    ) -> Result<String, WhatsAppError> {
        let to_value = format!("whatsapp:{}", to);
        let from_value = self.config.from_number.clone();
        let body_value = body.to_string();
        let media_url_value = media_url.map(|s| s.to_string());

        let mut form = reqwest::multipart::Form::new();
        form = form.text("To", to_value);
        form = form.text("From", from_value);
        form = form.text("Body", body_value);

        if let Some(url) = media_url_value {
            form = form.text("MediaUrl", url);
        }

        let response = self
            .client
            .post(format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
                self.config.account_sid
            ))
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| WhatsAppError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(WhatsAppError::Api(error_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WhatsAppError::Parse(e.to_string()))?;
        json.get("sid")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| WhatsAppError::Parse("No message SID".to_string()))
    }

    /// Connect to WhatsApp
    pub async fn connect(&mut self) -> Result<(), WhatsAppError> {
        info!("Connecting to WhatsApp (Twilio)...");

        // Verify credentials by making a test request
        let response = self
            .client
            .get(format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}.json",
                self.config.account_sid
            ))
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .send()
            .await
            .map_err(|e| WhatsAppError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(WhatsAppError::Authentication(
                "Invalid Twilio credentials".to_string(),
            ));
        }

        info!("WhatsApp connected successfully");
        Ok(())
    }

    /// Disconnect from WhatsApp
    pub async fn disconnect(&mut self) -> Result<(), WhatsAppError> {
        info!("Disconnecting from WhatsApp...");
        Ok(())
    }
}
