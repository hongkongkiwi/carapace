//! Slack Channel Implementation
//!
//! Provides messaging support via the Slack Web API.
//! Supports text messages, blocks, and interactive components.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Slack channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot token from Slack App (xoxb-...)
    pub bot_token: String,
    /// App level token for socket mode (xapp-...)
    pub app_token: Option<String>,
    /// Signing secret for verification
    pub signing_secret: String,
    /// Use Socket Mode instead of webhooks
    pub socket_mode: bool,
    /// Default channel to post to
    pub default_channel: String,
    /// Maximum message length
    pub max_message_length: usize,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            app_token: None,
            signing_secret: String::new(),
            socket_mode: false,
            default_channel: String::new(),
            max_message_length: 4000,
        }
    }
}

/// Slack channel error
#[derive(Debug, thiserror::Error)]
pub enum SlackError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Slack channel struct
#[derive(Debug)]
pub struct SlackChannel {
    config: SlackConfig,
    client: reqwest::Client,
    event_tx: mpsc::Sender<MessageContent>,
}

impl SlackChannel {
    /// Create a new Slack channel
    pub fn new(config: SlackConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        Self { config, client, event_tx }
    }

    /// Get the API base URL
    fn api_url(&self, method: &str) -> String {
        format!("https://slack.com/api/{}", method)
    }

    /// Send a request to the Slack API
    async fn api_request<T: for<'de> Deserialize<'de>>(&self, method: &str, body: Option<serde_json::Value>) -> Result<T, SlackError> {
        let mut request = self
            .client
            .post(self.api_url(method))
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SlackError::Network(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlackError::Parse(e.to_string()))?;

        if json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error_msg = json.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(SlackError::Api(error_msg.to_string()));
        }

        serde_json::from_value(json.get("result").cloned().unwrap_or_default())
            .map_err(|e| SlackError::Parse(e.to_string()))
    }

    /// Send a text message
    pub async fn send_message(&self, channel: &str, text: &str, blocks: Option<Vec<SlackBlock>>) -> Result<String, SlackError> {
        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        if let Some(blocks) = blocks {
            body["blocks"] = serde_json::json!(blocks.iter().map(|b| b.to_json()).collect::<Vec<_>>());
        }

        self.api_request::<SlackMessageResponse>("chat.postMessage", Some(body))
            .await
            .map(|r| r.ts)
    }

    /// Post to a channel using webhook
    pub async fn send_webhook(&self, channel: &str, text: &str) -> Result<(), SlackError> {
        let body = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        let response = self
            .client
            .post(&self.api_url("chat.postMessage"))
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| SlackError::Network(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlackError::Parse(e.to_string()))?;

        if json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error_msg = json.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(SlackError::Api(error_msg.to_string()));
        }

        Ok(())
    }

    /// Connect to Slack
    pub async fn connect(&mut self) -> Result<(), SlackError> {
        info!("Connecting to Slack...");

        // Test the bot token
        let body = serde_json::json!({});
        let _: SlackAuthResponse = self.api_request("auth.test", Some(body)).await?;

        info!("Slack connected successfully");
        Ok(())
    }

    /// Disconnect from Slack
    pub async fn disconnect(&mut self) -> Result<(), SlackError> {
        info!("Disconnecting from Slack...");
        Ok(())
    }
}

// Slack API response types

#[derive(Debug, Deserialize)]
struct SlackAuthResponse {
    ok: bool,
    user_id: String,
    team_id: String,
    team: String,
}

#[derive(Debug, Deserialize)]
struct SlackMessageResponse {
    ok: bool,
    ts: String,
    channel: String,
    message: SlackMessage,
}

#[derive(Debug, Deserialize)]
struct SlackMessage {
    ts: String,
    text: String,
}

/// Slack Block Kit block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackBlock {
    block_type: String,
    text: Option<SlackText>,
    elements: Vec<SlackBlockElement>,
    accessory: Option<SlackBlockElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SlackText {
    type_: String,
    text: String,
    emoji: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SlackBlockElement {
    type_: String,
    text: Option<SlackText>,
    action_id: Option<String>,
    url: Option<String>,
    value: Option<String>,
    style: Option<String>,
}

impl SlackBlock {
    /// Create a header block
    pub fn header(text: &str) -> Self {
        Self {
            block_type: "header".to_string(),
            text: Some(SlackText {
                type_: "plain_text".to_string(),
                text: text.to_string(),
                emoji: Some(true),
            }),
            elements: vec![],
            accessory: None,
        }
    }

    /// Create a section block
    pub fn section(text: &str) -> Self {
        Self {
            block_type: "section".to_string(),
            text: Some(SlackText {
                type_: "mrkdwn".to_string(),
                text: text.to_string(),
                emoji: None,
            }),
            elements: vec![],
            accessory: None,
        }
    }

    /// Create a button element
    pub fn button(text: &str, action_id: &str, value: &str) -> Self {
        Self {
            block_type: "actions".to_string(),
            text: None,
            elements: vec![SlackBlockElement {
                type_: "button".to_string(),
                text: Some(SlackText {
                    type_: "plain_text".to_string(),
                    text: text.to_string(),
                    emoji: Some(true),
                }),
                action_id: Some(action_id.to_string()),
                url: None,
                value: Some(value.to_string()),
                style: None,
            }],
            accessory: None,
        }
    }

    fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({
            "type": self.block_type,
        });

        if let Some(text) = &self.text {
            json["text"] = serde_json::json!({
                "type": text.type_,
                "text": text.text,
                "emoji": text.emoji.unwrap_or(false),
            });
        }

        if !self.elements.is_empty() {
            json["elements"] = serde_json::json!(self.elements.iter().map(|e| {
                let mut ejson = serde_json::json!({
                    "type": e.type_,
                });
                if let Some(text) = &e.text {
                    ejson["text"] = serde_json::json!({
                        "type": text.type_,
                        "text": text.text,
                        "emoji": text.emoji.unwrap_or(false),
                    });
                }
                if let Some(action_id) = &e.action_id {
                    ejson["action_id"] = serde_json::json!(action_id.as_str());
                }
                if let Some(value) = &e.value {
                    ejson["value"] = serde_json::json!(value.as_str());
                }
                if let Some(style) = &e.style {
                    ejson["style"] = serde_json::json!(style.as_str());
                }
                ejson
            }).collect::<Vec<_>>());
        }

        json
    }
}
