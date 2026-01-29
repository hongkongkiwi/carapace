//! Google Chat Channel Implementation
//!
//! Provides messaging support via Google Chat Web API.
//! Supports text messages, cards, and space management.

use crate::messages::outbound::MessageContent;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Google Chat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    /// Enable Google Chat channel
    pub enabled: bool,
    /// OAuth2 access token
    pub access_token: String,
    /// OAuth2 refresh token
    pub refresh_token: Option<String>,
    /// Client ID for OAuth
    pub client_id: Option<String>,
    /// Client secret for OAuth
    pub client_secret: Option<String>,
    /// Bot name (for mentions)
    pub bot_name: String,
    /// Default space to post to
    pub default_space: String,
    /// Sync interval in seconds
    pub sync_interval_secs: u64,
}

impl Default for GoogleChatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            access_token: String::new(),
            refresh_token: None,
            client_id: None,
            client_secret: None,
            bot_name: String::new(),
            default_space: String::new(),
            sync_interval_secs: 5,
        }
    }
}

/// Google Chat channel error
#[derive(Debug, thiserror::Error)]
pub enum GoogleChatError {
    #[error("network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("space error: {0}")]
    Space(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Google Chat channel struct
#[derive(Debug)]
pub struct GoogleChatChannel {
    config: GoogleChatConfig,
    client: Client,
    event_tx: mpsc::Sender<MessageContent>,
}

impl GoogleChatChannel {
    /// Create a new Google Chat channel
    pub fn new(config: GoogleChatConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        let client = ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            config,
            client,
            event_tx,
        }
    }

    /// Get the API base URL
    fn api_url(&self, path: &str) -> String {
        format!("https://chat.googleapis.com/v1{}", path)
    }

    /// Send a request to the Google Chat API
    async fn api_request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, GoogleChatError> {
        let mut request = self
            .client
            .request(
                method.parse().expect("Invalid HTTP method"),
                self.api_url(path),
            )
            .header("Authorization", format!("Bearer {}", self.config.access_token))
            .header("Content-Type", "application/json; charset=UTF-8");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| GoogleChatError::Network(e.to_string()))?;

        // Google Chat returns 429 on rate limit
        if response.status() == 429 {
            return Err(GoogleChatError::Api("Rate limit exceeded".to_string()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| GoogleChatError::Parse(e.to_string()))?;

        // Check for error
        if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
            return Err(GoogleChatError::Api(error.to_string()));
        }

        serde_json::from_value(json)
            .map_err(|e| GoogleChatError::Parse(e.to_string()))
    }

    /// Send a simple text message
    pub async fn send_message(&self, space: &str, text: &str) -> Result<String, GoogleChatError> {
        let body = serde_json::json!({
            "text": text,
        });

        let response: GoogleChatMessageResponse = self
            .api_request("POST", &format!("/spaces/{}/messages", space), Some(body))
            .await?;

        Ok(response.name)
    }

    /// Send a message to the default space
    pub async fn send_default(&self, text: &str) -> Result<String, GoogleChatError> {
        if self.config.default_space.is_empty() {
            return Err(GoogleChatError::Space("No default space configured".to_string()));
        }
        self.send_message(&self.config.default_space, text).await
    }

    /// Send a message with a card (rich UI)
    pub async fn send_card(
        &self,
        space: &str,
        _title: &str,
        text: &str,
        cards: Vec<GoogleChatCard>,
    ) -> Result<String, GoogleChatError> {
        let body = serde_json::json!({
            "text": text,
            "cards": cards.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
        });

        let response: GoogleChatMessageResponse = self
            .api_request("POST", &format!("/spaces/{}/messages", space), Some(body))
            .await?;

        Ok(response.name)
    }

    /// Create a new space
    pub async fn create_space(&self, name: &str, space_type: &str) -> Result<String, GoogleChatError> {
        let body = serde_json::json!({
            "displayName": name,
            "spaceType": space_type, // "SPACE" or "GROUP_CHAT" or "DIRECT_MESSAGE"
        });

        let response: GoogleChatSpaceResponse = self
            .api_request("POST", "/spaces", Some(body))
            .await?;

        Ok(response.name)
    }

    /// List all spaces the bot is in
    pub async fn list_spaces(&self) -> Result<Vec<GoogleChatSpace>, GoogleChatError> {
        let response: GoogleChatSpacesListResponse = self
            .api_request("GET", "/spaces", None)
            .await?;

        Ok(response.spaces)
    }

    /// Get space details
    pub async fn get_space(&self, space: &str) -> Result<GoogleChatSpace, GoogleChatError> {
        self.api_request("GET", &format!("/spaces/{}", space), None)
            .await
    }

    /// List members in a space
    pub async fn list_members(&self, space: &str) -> Result<Vec<GoogleChatMember>, GoogleChatError> {
        let response: GoogleChatMembersListResponse = self
            .api_request("GET", &format!("/spaces/{}/members", space), None)
            .await?;

        Ok(response.members)
    }

    /// Connect to Google Chat
    pub async fn connect(&mut self) -> Result<(), GoogleChatError> {
        // Verify token by listing spaces
        let _spaces = self.list_spaces().await?;
        info!("Google Chat connected");
        Ok(())
    }

    /// Disconnect from Google Chat
    pub async fn disconnect(&mut self) -> Result<(), GoogleChatError> {
        info!("Google Chat disconnected");
        Ok(())
    }
}

/// Google Chat API response types

#[derive(Debug, Deserialize)]
struct GoogleChatMessageResponse {
    pub name: String,          // Format: spaces/{space}/messages/{message}
    pub thread: Option<String>,
    pub create_time: String,
}

#[derive(Debug, Deserialize)]
struct GoogleChatSpaceResponse {
    pub name: String,          // Format: spaces/{space}
    pub r#type: String,        // "SPACE", "GROUP_CHAT", "DIRECT_MESSAGE"
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct GoogleChatSpacesListResponse {
    pub spaces: Vec<GoogleChatSpace>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleChatSpace {
    pub name: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct GoogleChatMembersListResponse {
    pub members: Vec<GoogleChatMember>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleChatMember {
    pub name: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub state: String,
}

/// Google Chat Card for rich UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatCard {
    pub header: Option<GoogleChatCardHeader>,
    pub sections: Vec<GoogleChatCardSection>,
    pub actions: Vec<GoogleChatCardAction>,
}

impl Default for GoogleChatCard {
    fn default() -> Self {
        Self {
            header: None,
            sections: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl GoogleChatCard {
    /// Create a simple card with header and text
    pub fn simple(title: &str, text: &str) -> Self {
        Self {
            header: Some(GoogleChatCardHeader {
                title: title.to_string(),
                subtitle: None,
                image_url: None,
            }),
            sections: vec![GoogleChatCardSection {
                header: None,
                widgets: vec![GoogleChatWidget::TextParagraph {
                    text: text.to_string(),
                }],
            }],
            actions: Vec::new(),
        }
    }

    /// Convert to JSON for API
    fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({});

        if let Some(header) = &self.header {
            json["header"] = serde_json::json!({
                "title": header.title,
                "subtitle": header.subtitle.clone().unwrap_or_default(),
                "imageUrl": header.image_url.clone().unwrap_or_default(),
            });
        }

        if !self.sections.is_empty() {
            json["sections"] = serde_json::json!(self.sections.iter().map(|s| {
                serde_json::json!({
                    "header": s.header.clone().unwrap_or_default(),
                    "widgets": s.widgets.iter().map(|w| w.to_json()).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>());
        }

        json
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatCardHeader {
    pub title: String,
    pub subtitle: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatCardSection {
    pub header: Option<String>,
    pub widgets: Vec<GoogleChatWidget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatCardAction {
    pub action_label: String,
    pub on_click: GoogleChatOnClick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatOnClick {
    pub action: Option<GoogleChatAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatAction {
    pub function: String,
    pub parameters: Vec<GoogleChatActionParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatActionParameter {
    pub key: String,
    pub value: String,
}

/// Widget types for cards
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GoogleChatWidget {
    #[serde(rename = "textParagraph")]
    TextParagraph { text: String },
    #[serde(rename = "image")]
    Image { image_url: String, on_click: Option<GoogleChatOnClick> },
    #[serde(rename = "keyValue")]
    KeyValue {
        top_label: Option<String>,
        content: String,
        content_multiline: Option<bool>,
        bottom_label: Option<String>,
        button: Option<GoogleChatCardAction>,
    },
    #[serde(rename = "buttons")]
    Buttons { buttons: Vec<GoogleChatCardAction> },
    #[serde(rename = "decoratedText")]
    DecoratedText {
        text: String,
        start_icon: Option<GoogleChatIcon>,
        end_icon: Option<GoogleChatIcon>,
        button: Option<GoogleChatCardAction>,
    },
}

impl GoogleChatWidget {
    fn to_json(&self) -> serde_json::Value {
        match self {
            GoogleChatWidget::TextParagraph { text } => {
                serde_json::json!({ "textParagraph": { "text": text } })
            }
            GoogleChatWidget::Image { image_url, on_click } => {
                serde_json::json!({
                    "image": {
                        "imageUrl": image_url,
                        "onClick": on_click.clone().map(|oc| serde_json::json!({ "action": oc.action }))
                    }
                })
            }
            GoogleChatWidget::KeyValue {
                top_label,
                content,
                content_multiline,
                bottom_label,
                button,
            } => {
                serde_json::json!({
                    "keyValue": {
                        "topLabel": top_label.clone().unwrap_or_default(),
                        "content": content,
                        "contentMultiline": content_multiline.unwrap_or(false),
                        "bottomLabel": bottom_label.clone().unwrap_or_default(),
                        "button": button.clone().map(|b| serde_json::json!({ "text": b.action_label, "onClick": b.on_click }))
                    }
                })
            }
            GoogleChatWidget::Buttons { buttons } => {
                serde_json::json!({
                    "buttons": {
                        "buttons": buttons.iter().map(|b| serde_json::json!({
                            "text": b.action_label,
                            "onClick": b.on_click
                        })).collect::<Vec<_>>()
                    }
                })
            }
            GoogleChatWidget::DecoratedText { text, start_icon, end_icon, button } => {
                serde_json::json!({
                    "decoratedText": {
                        "text": text,
                        "startIcon": start_icon.clone().map(|i| serde_json::json!({ "iconUrl": i.url })),
                        "endIcon": end_icon.clone().map(|i| serde_json::json!({ "iconUrl": i.url })),
                        "button": button.clone().map(|b| serde_json::json!({ "text": b.action_label, "onClick": b.on_click }))
                    }
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatIcon {
    pub url: String,
    pub icon_type: Option<String>,
}

/// Export for module
pub use GoogleChatChannel as Channel;
pub use GoogleChatConfig as Config;
pub use GoogleChatError as Error;
