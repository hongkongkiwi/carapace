//! LINE Channel
//!
//! LINE Messaging API integration for carapace.

use serde::{Deserialize, Serialize};

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
}

/// LINE channel implementation
pub struct LineChannel {
    config: LineConfig,
}

impl LineChannel {
    /// Create new channel
    pub fn new(config: LineConfig) -> Self {
        Self { config }
    }

    /// Start the bot
    pub async fn start(&self) -> Result<(), LineError> {
        tracing::info!("Starting LINE bot");
        Ok(())
    }

    /// Stop the bot
    pub async fn stop(&self) -> Result<(), LineError> {
        Ok(())
    }

    /// Send text message
    pub async fn send_text(&self, to: &str, text: &str) -> Result<(), LineError> {
        tracing::info!(recipient = %to, text = %text, "Sending LINE text message");
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
        Ok(())
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
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum LineSource {
    /// User source
    #[serde(rename = "user")]
    User { user_id: String },
    /// Group source
    #[serde(rename = "group")]
    Group { group_id: String, user_id: Option<String> },
    /// Room source
    #[serde(rename = "room")]
    Room { room_id: String, user_id: Option<String> },
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
}
