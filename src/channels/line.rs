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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> LineConfig {
        LineConfig {
            channel_access_token: "test_token_123".to_string(),
            channel_secret: "test_secret_456".to_string(),
            webhook_url: Some("https://example.com/webhook".to_string()),
            allowed_users: vec!["U123".to_string(), "U456".to_string()],
            rich_menu_enabled: true,
        }
    }

    #[test]
    fn test_line_config_creation() {
        let config = create_test_config();
        assert_eq!(config.channel_access_token, "test_token_123");
        assert_eq!(config.channel_secret, "test_secret_456");
        assert_eq!(config.webhook_url, Some("https://example.com/webhook".to_string()));
        assert!(config.rich_menu_enabled);
        assert_eq!(config.allowed_users.len(), 2);
    }

    #[test]
    fn test_line_channel_new() {
        let config = create_test_config();
        let channel = LineChannel::new(config.clone());
        assert_eq!(channel.config.channel_access_token, config.channel_access_token);
        assert_eq!(channel.config.channel_secret, config.channel_secret);
    }

    #[tokio::test]
    async fn test_line_channel_start() {
        let config = create_test_config();
        let channel = LineChannel::new(config);
        let result = channel.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_line_channel_stop() {
        let config = create_test_config();
        let channel = LineChannel::new(config);
        let result = channel.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_line_channel_send_text() {
        let config = create_test_config();
        let channel = LineChannel::new(config);
        let result = channel.send_text("U123", "Hello, World!").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_line_channel_send_rich_message() {
        let config = create_test_config();
        let channel = LineChannel::new(config);
        let quick_replies = vec![
            QuickReply {
                label: "Yes".to_string(),
                action: QuickReplyAction::Message { text: "yes".to_string() },
            },
            QuickReply {
                label: "No".to_string(),
                action: QuickReplyAction::Message { text: "no".to_string() },
            },
        ];
        let result = channel.send_rich_message("U123", "Do you agree?", quick_replies).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_quick_reply_creation() {
        let reply = QuickReply {
            label: "Test".to_string(),
            action: QuickReplyAction::Message { text: "test message".to_string() },
        };
        assert_eq!(reply.label, "Test");
    }

    #[test]
    fn test_quick_reply_action_message() {
        let action = QuickReplyAction::Message { text: "hello".to_string() };
        match action {
            QuickReplyAction::Message { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected Message action"),
        }
    }

    #[test]
    fn test_quick_reply_action_postback() {
        let action = QuickReplyAction::Postback {
            data: "action=buy".to_string(),
            display_text: "Buy Now".to_string(),
        };
        match action {
            QuickReplyAction::Postback { data, display_text } => {
                assert_eq!(data, "action=buy");
                assert_eq!(display_text, "Buy Now");
            }
            _ => panic!("Expected Postback action"),
        }
    }

    #[test]
    fn test_quick_reply_action_uri() {
        let action = QuickReplyAction::Uri { uri: "https://example.com".to_string() };
        match action {
            QuickReplyAction::Uri { uri } => assert_eq!(uri, "https://example.com"),
            _ => panic!("Expected Uri action"),
        }
    }

    #[test]
    fn test_line_source_user() {
        let source = LineSource::User { user_id: "U123".to_string() };
        match source {
            LineSource::User { user_id } => assert_eq!(user_id, "U123"),
            _ => panic!("Expected User source"),
        }
    }

    #[test]
    fn test_line_source_group() {
        let source = LineSource::Group {
            group_id: "G123".to_string(),
            user_id: Some("U456".to_string()),
        };
        match source {
            LineSource::Group { group_id, user_id } => {
                assert_eq!(group_id, "G123");
                assert_eq!(user_id, Some("U456".to_string()));
            }
            _ => panic!("Expected Group source"),
        }
    }

    #[test]
    fn test_line_source_room() {
        let source = LineSource::Room {
            room_id: "R123".to_string(),
            user_id: None,
        };
        match source {
            LineSource::Room { room_id, user_id } => {
                assert_eq!(room_id, "R123");
                assert_eq!(user_id, None);
            }
            _ => panic!("Expected Room source"),
        }
    }

    #[test]
    fn test_line_error_variants() {
        let api_err = LineError::Api("test error".to_string());
        assert_eq!(api_err.to_string(), "API error: test error");

        let sig_err = LineError::InvalidSignature;
        assert_eq!(sig_err.to_string(), "Invalid signature");

        let chan_err = LineError::Channel("channel closed".to_string());
        assert_eq!(chan_err.to_string(), "Channel error: channel closed");
    }

    #[test]
    fn test_line_message_creation() {
        let message = LineMessage {
            id: "12345".to_string(),
            message_type: "text".to_string(),
            text: Some("Hello".to_string()),
        };
        assert_eq!(message.id, "12345");
        assert_eq!(message.message_type, "text");
        assert_eq!(message.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_line_postback_creation() {
        let postback = LinePostback {
            data: "action=submit".to_string(),
        };
        assert_eq!(postback.data, "action=submit");
    }

    #[test]
    fn test_line_event_creation() {
        let event = LineEvent {
            event_type: "message".to_string(),
            timestamp: 1234567890,
            source: LineSource::User { user_id: "U123".to_string() },
            message: Some(LineMessage {
                id: "12345".to_string(),
                message_type: "text".to_string(),
                text: Some("Hello".to_string()),
            }),
            postback: None,
        };
        assert_eq!(event.event_type, "message");
        assert_eq!(event.timestamp, 1234567890);
    }

    #[test]
    fn test_line_webhook_event_creation() {
        let webhook_event = LineWebhookEvent {
            events: vec![
                LineEvent {
                    event_type: "message".to_string(),
                    timestamp: 1234567890,
                    source: LineSource::User { user_id: "U123".to_string() },
                    message: Some(LineMessage {
                        id: "12345".to_string(),
                        message_type: "text".to_string(),
                        text: Some("Hello".to_string()),
                    }),
                    postback: None,
                },
            ],
        };
        assert_eq!(webhook_event.events.len(), 1);
        assert_eq!(webhook_event.events[0].event_type, "message");
    }
}
