//! LINE Channel
//!
//! LINE Messaging API integration for carapace.
//! Supports text messages, rich messages, webhook handling, and signature verification.

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;

use crate::channels::ChannelConfig;

/// LINE API base URL
const LINE_API_BASE: &str = "https://api.line.me/v2";

/// LINE Messaging API endpoint
const LINE_SEND_MESSAGE_ENDPOINT: &str = "/bot/message/push";

/// LINE channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    /// Channel access token from LINE Developer Console
    pub channel_access_token: String,
    /// Channel secret for webhook verification
    pub channel_secret: String,
    /// Webhook URL (must be HTTPS)
    pub webhook_url: Option<String>,
    /// Allowed user IDs (empty = all)
    pub allowed_users: Vec<String>,
    /// Enable rich menu
    pub rich_menu_enabled: bool,
    /// Enable TLS verification
    pub verify_tls: bool,
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            channel_access_token: String::new(),
            channel_secret: String::new(),
            webhook_url: None,
            allowed_users: Vec::new(),
            rich_menu_enabled: false,
            verify_tls: true,
        }
    }
}

/// LINE channel implementation
#[derive(Clone)]
pub struct LineChannel {
    config: Arc<LineConfig>,
    client: reqwest::Client,
}

impl LineChannel {
    /// Create new channel
    pub fn new(config: LineConfig) -> Self {
        let client = reqwest::ClientBuilder::new().build();

        Self {
            config: Arc::new(config),
            client: client.unwrap_or_default(),
        }
    }

    /// Create from ChannelConfig
    pub fn from_config(channel_config: &ChannelConfig) -> Option<Self> {
        match channel_config {
            ChannelConfig::Line(config) => {
                // Extract available fields from channels::config::LineConfig
                let token = config.channel_access_token.clone();
                let secret = config.channel_secret.clone();

                // Create line::LineConfig with defaults for missing fields
                let line_config = LineConfig {
                    channel_access_token: token,
                    channel_secret: secret,
                    webhook_url: None,
                    allowed_users: Vec::new(),
                    rich_menu_enabled: false,
                    verify_tls: true,
                };
                Some(Self::new(line_config))
            }
            _ => None,
        }
    }

    /// Get channel name
    pub fn name(&self) -> String {
        "LINE".to_string()
    }

    /// Start the bot
    pub async fn start(&self) -> Result<(), LineError> {
        tracing::info!("Starting LINE bot");

        // Validate configuration
        if self.config.channel_access_token.is_empty() {
            return Err(LineError::Config(
                "channel_access_token is required".to_string(),
            ));
        }
        if self.config.channel_secret.is_empty() {
            return Err(LineError::Config("channel_secret is required".to_string()));
        }

        // Verify API credentials by making a test request
        self.verify_credentials().await?;

        tracing::info!("LINE bot started successfully");
        Ok(())
    }

    /// Stop the bot
    pub async fn stop(&self) -> Result<(), LineError> {
        tracing::info!("Stopping LINE bot");
        Ok(())
    }

    /// Verify credentials with LINE API
    async fn verify_credentials(&self) -> Result<(), LineError> {
        let response = self
            .client
            .get(format!("{}/bot/info", LINE_API_BASE))
            .header(
                "Authorization",
                format!("Bearer {}", self.config.channel_access_token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(LineError::Api(format!(
                "Failed to verify credentials: {}",
                error
            )));
        }

        Ok(())
    }

    /// Send text message
    pub async fn send_text(&self, to: &str, text: &str) -> Result<(), LineError> {
        tracing::info!(recipient = %to, text = %text, "Sending LINE text message");

        let payload = SendMessagePayload {
            to: to.to_string(),
            messages: vec![Message {
                type_: "text".to_string(),
                text: text.to_string(),
                quick_reply: None,
            }],
        };

        self.send_message(&payload).await?;

        tracing::info!(recipient = %to, "LINE text message sent successfully");
        Ok(())
    }

    /// Send rich message with quick replies
    pub async fn send_rich_message(
        &self,
        to: &str,
        text: &str,
        quick_replies: Vec<QuickReply>,
    ) -> Result<(), LineError> {
        tracing::info!(
            recipient = %to,
            text = %text,
            reply_count = quick_replies.len(),
            "Sending LINE rich message"
        );

        let quick_reply = if !quick_replies.is_empty() {
            let items: Vec<QuickReplyItem> = quick_replies
                .into_iter()
                .map(|qr| QuickReplyItem {
                    type_: "action".to_string(),
                    action: qr.action,
                })
                .collect();

            Some(QuickReplyContainer { items })
        } else {
            None
        };

        let payload = SendMessagePayload {
            to: to.to_string(),
            messages: vec![Message {
                type_: "text".to_string(),
                text: text.to_string(),
                quick_reply,
            }],
        };

        self.send_message(&payload).await?;

        tracing::info!(recipient = %to, "LINE rich message sent successfully");
        Ok(())
    }

    /// Send a message using the LINE API
    async fn send_message(&self, payload: &SendMessagePayload) -> Result<(), LineError> {
        let response = self
            .client
            .post(format!("{}{}", LINE_API_BASE, LINE_SEND_MESSAGE_ENDPOINT))
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", self.config.channel_access_token),
            )
            .json(payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(%status, error = %error_body, "LINE API error");
            return Err(LineError::Api(format!(
                "API request failed with status {}: {}",
                status, error_body
            )));
        }

        Ok(())
    }

    /// Verify LINE webhook signature
    pub fn verify_signature(&self, body: &[u8], signature: &str) -> bool {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.channel_secret.as_bytes())
            .expect("Invalid channel secret length");

        mac.update(body);
        mac.verify_slice(
            &STANDARD
                .decode(signature)
                .expect("Invalid signature format"),
        )
        .is_ok()
    }

    /// Parse and validate incoming webhook event
    pub fn parse_webhook_event(
        &self,
        body: &[u8],
        signature: &str,
    ) -> Result<LineWebhookEvent, LineError> {
        // Verify signature
        if !self.verify_signature(body, signature) {
            tracing::warn!("Invalid LINE webhook signature");
            return Err(LineError::InvalidSignature);
        }

        // Parse event
        let event: LineWebhookEvent =
            serde_json::from_slice(body).map_err(|e| LineError::Parse(e.to_string()))?;

        // Check if user is allowed
        for line_event in &event.events {
            let user_id: Option<&String> = match &line_event.source {
                LineSource::User { user_id } => Some(user_id),
                LineSource::Group { user_id, .. } => user_id.as_ref(),
                LineSource::Room { user_id, .. } => user_id.as_ref(),
            };

            if let Some(id) = user_id {
                if !self.config.allowed_users.is_empty() && !self.config.allowed_users.contains(id)
                {
                    tracing::warn!(user_id = %id, "User not in allowed list");
                    return Err(LineError::Unauthorized(id.clone()));
                }
            }
        }

        Ok(event)
    }

    /// Get user profile
    pub async fn get_user_profile(&self, user_id: &str) -> Result<UserProfile, LineError> {
        let response = self
            .client
            .get(format!("{}/bot/profile/{}", LINE_API_BASE, user_id))
            .header(
                "Authorization",
                format!("Bearer {}", self.config.channel_access_token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(LineError::Api(format!(
                "Failed to get user profile: {}",
                error
            )));
        }

        let profile: UserProfile = response.json().await?;
        Ok(profile)
    }

    /// Leave group or room
    pub async fn leave(&self, target_type: &str, target_id: &str) -> Result<(), LineError> {
        let endpoint = match target_type {
            "group" => "/bot/group/{}/leave".replace("{}", target_id),
            "room" => "/bot/room/{}/leave".replace("{}", target_id),
            _ => return Err(LineError::Config("Invalid target type".to_string())),
        };

        let response = self
            .client
            .post(format!("{}{}", LINE_API_BASE, endpoint))
            .header(
                "Authorization",
                format!("Bearer {}", self.config.channel_access_token),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(LineError::Api(format!(
                "Failed to leave {}: {}",
                target_type, error
            )));
        }

        Ok(())
    }

    /// Get config reference
    pub fn config(&self) -> &Arc<LineConfig> {
        &self.config
    }
}

/// Quick reply button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickReply {
    /// Button label
    pub label: String,
    /// Button action type
    pub action: QuickReplyAction,
}

/// Quick reply action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QuickReplyAction {
    /// Postback action
    #[serde(rename = "postback")]
    Postback { data: String, display_text: String },
    /// Message action
    #[serde(rename = "message")]
    Message { text: String },
    /// URI action
    #[serde(rename = "uri")]
    Uri { uri: String },
}

/// Quick reply item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickReplyItem {
    #[serde(rename = "type")]
    pub type_: String,
    pub action: QuickReplyAction,
}

/// Quick reply container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickReplyContainer {
    pub items: Vec<QuickReplyItem>,
}

/// Message for sending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_reply: Option<QuickReplyContainer>,
}

/// Payload for sending messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessagePayload {
    pub to: String,
    pub messages: Vec<Message>,
}

/// LINE webhook event
#[derive(Debug, Clone, Deserialize)]
pub struct LineWebhookEvent {
    /// Events array
    pub events: Vec<LineEvent>,
}

/// LINE event
#[derive(Debug, Clone, Deserialize)]
pub struct LineEvent {
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event timestamp
    pub timestamp: i64,
    /// Reply token (for message/postback events)
    #[serde(default)]
    pub reply_token: Option<String>,
    /// Source (user, group, or room)
    pub source: LineSource,
    /// Message (for message events)
    #[serde(default)]
    pub message: Option<LineMessage>,
    /// Postback data (for postback events)
    #[serde(default)]
    pub postback: Option<LinePostback>,
}

/// LINE message source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LineSource {
    /// User source
    #[serde(rename = "user")]
    User { user_id: String },
    /// Group source
    #[serde(rename = "group")]
    Group {
        group_id: String,
        user_id: Option<String>,
    },
    /// Room source
    #[serde(rename = "room")]
    Room {
        room_id: String,
        user_id: Option<String>,
    },
}

/// LINE message
#[derive(Debug, Clone, Deserialize)]
pub struct LineMessage {
    /// Message ID
    pub id: String,
    /// Message type
    #[serde(rename = "type")]
    pub message_type: String,
    /// Message text (for text messages)
    #[serde(default)]
    pub text: Option<String>,
}

/// LINE postback
#[derive(Debug, Clone, Deserialize)]
pub struct LinePostback {
    /// Postback data
    pub data: String,
    /// Params for date/time postbacks
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// User profile
#[derive(Debug, Clone, Deserialize)]
pub struct UserProfile {
    /// User ID
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Profile image URL
    #[serde(default)]
    pub picture_url: Option<String>,
    /// Status message
    #[serde(default)]
    pub status_message: Option<String>,
}

/// LINE errors
#[derive(Debug, thiserror::Error)]
pub enum LineError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Channel error: {0}")]
    Channel(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_config_default() {
        let config = LineConfig::default();
        assert!(config.channel_access_token.is_empty());
        assert!(config.channel_secret.is_empty());
        assert!(config.allowed_users.is_empty());
    }

    #[test]
    fn test_quick_reply_serialization() {
        let action = QuickReplyAction::Postback {
            data: "action_data".to_string(),
            display_text: "Click me".to_string(),
        };
        let quick_reply = QuickReply {
            label: "Button".to_string(),
            action,
        };

        let json = serde_json::to_string(&quick_reply).unwrap();
        assert!(json.contains("postback"));
        assert!(json.contains("action_data"));
    }

    #[test]
    fn test_quick_reply_container_serialization() {
        let action = QuickReplyAction::Message {
            text: "Hello".to_string(),
        };
        let container = QuickReplyContainer {
            items: vec![QuickReplyItem {
                type_: "action".to_string(),
                action,
            }],
        };

        let json = serde_json::to_string(&container).unwrap();
        assert!(json.contains("\"type\":\"action\""));
        assert!(json.contains("message"));
    }

    #[test]
    fn test_line_source_variants() {
        let user_source = LineSource::User {
            user_id: "U123".to_string(),
        };
        let user_json = serde_json::to_string(&user_source).unwrap();
        assert!(user_json.contains("\"type\":\"user\""));

        let group_source = LineSource::Group {
            group_id: "G123".to_string(),
            user_id: Some("U456".to_string()),
        };
        let group_json = serde_json::to_string(&group_source).unwrap();
        assert!(group_json.contains("\"type\":\"group\""));
    }

    #[test]
    fn test_send_message_payload() {
        let payload = SendMessagePayload {
            to: "U123456".to_string(),
            messages: vec![Message {
                type_: "text".to_string(),
                text: "Hello".to_string(),
                quick_reply: None,
            }],
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"to\":\"U123456\""));
        assert!(json.contains("\"messages\""));
    }
}
