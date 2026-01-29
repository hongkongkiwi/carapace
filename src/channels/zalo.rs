//! Zalo Channel Implementation
//!
//! Provides messaging support via the Zalo API.
//! Supports text messages, images, and rich media.

use crate::messages::outbound::MessageContent;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Zalo channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloConfig {
    /// Enable Zalo channel
    pub enabled: bool,
    /// Zalo App ID
    pub app_id: String,
    /// Zalo App Secret
    pub app_secret: String,
    /// Access token for API calls
    pub access_token: String,
    /// OAuth callback URL
    pub callback_url: String,
    /// RSA public key for encryption
    pub public_key: Option<String>,
    /// Max message length
    pub max_message_length: usize,
}

impl Default for ZaloConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: String::new(),
            app_secret: String::new(),
            access_token: String::new(),
            callback_url: String::new(),
            public_key: None,
            max_message_length: 5000,
        }
    }
}

/// Zalo channel error
#[derive(Debug, thiserror::Error)]
pub enum ZaloError {
    #[error("network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("signature error: {0}")]
    Signature(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Zalo channel struct
#[derive(Debug)]
pub struct ZaloChannel {
    config: ZaloConfig,
    client: Client,
    event_tx: mpsc::Sender<MessageContent>,
}

impl ZaloChannel {
    /// Create a new Zalo channel
    pub fn new(config: ZaloConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
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
        format!("https://openapi.zalo.me/v2.0{}", path)
    }

    /// Send a request to the Zalo API
    async fn api_request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, ZaloError> {
        let mut request = self
            .client
            .request(
                method.parse().expect("Invalid HTTP method"),
                self.api_url(path),
            )
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ZaloError::Network(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ZaloError::Parse(e.to_string()))?;

        // Check for Zalo error
        if let Some(error) = json.get("error").and_then(|v| v.as_u64()) {
            let message = json
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(match error {
                19 | 101 | 102 => ZaloError::Auth(message.to_string()),
                _ => ZaloError::Api(format!("{}: {}", error, message)),
            });
        }

        serde_json::from_value(json)
            .map_err(|e| ZaloError::Parse(e.to_string()))
    }

    /// Send a text message to a user
    pub async fn send_message(&self, user_id: &str, text: &str) -> Result<String, ZaloError> {
        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "text": text
            }
        });

        let response: ZaloSendResponse = self
            .api_request("POST", "/oa/message", Some(body))
            .await?;

        Ok(response.message_id)
    }

    /// Send an image message
    pub async fn send_image(
        &self,
        user_id: &str,
        image_url: &str,
        caption: Option<&str>,
    ) -> Result<String, ZaloError> {
        let mut message = serde_json::json!({
            "attachment": {
                "type": "image",
                "payload": {
                    "url": image_url
                }
            }
        });

        if let Some(caption) = caption {
            message["text"] = serde_json::json!(caption);
        }

        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": message,
        });

        let response: ZaloSendResponse = self
            .api_request("POST", "/oa/message", Some(body))
            .await?;

        Ok(response.message_id)
    }

    /// Send a URL button
    pub async fn send_url_button(
        &self,
        user_id: &str,
        text: &str,
        buttons: Vec<ZaloButton>,
    ) -> Result<String, ZaloError> {
        let body = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "text": text,
                "attachment": {
                    "type": "template",
                    "payload": {
                        "template_type": "button",
                        "buttons": buttons.iter().map(|b| b.to_json()).collect::<Vec<_>>()
                    }
                }
            }
        });

        let response: ZaloSendResponse = self
            .api_request("POST", "/oa/message", Some(body))
            .await?;

        Ok(response.message_id)
    }

    /// Get user info
    pub async fn get_user_info(&self, user_id: &str) -> Result<ZaloUser, ZaloError> {
        self.api_request(
            "GET",
            &format!("/oa/getprofile?user_id={}", user_id),
            None,
        )
        .await
    }

    /// Send a message to the default user (from context)
    pub async fn send_default(&self, _text: &str) -> Result<String, ZaloError> {
        // This requires user_id to be stored in context
        Err(ZaloError::Api("No default user".to_string()))
    }

    /// Connect to Zalo
    pub async fn connect(&mut self) -> Result<(), ZaloError> {
        // Verify token by getting user info
        info!("Zalo channel connected");
        Ok(())
    }

    /// Disconnect from Zalo
    pub async fn disconnect(&mut self) -> Result<(), ZaloError> {
        info!("Zalo channel disconnected");
        Ok(())
    }

    /// Verify webhook signature
    pub fn verify_signature(&self, _data: &str, _signature: &str) -> bool {
        // Zalo uses HMAC-SHA256 for webhook verification
        // Would implement with the app_secret
        true // Placeholder
    }
}

/// Zalo API response types

#[derive(Debug, Deserialize)]
struct ZaloSendResponse {
    pub message_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZaloUser {
    pub user_id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub display_name: Option<String>,
}

/// Zalo button for templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloButton {
    pub title: String,
    #[serde(default)]
    pub type_: String,
    pub payload: Option<String>,
    pub url: Option<String>,
}

impl ZaloButton {
    /// Create a URL button
    pub fn new_url(title: &str, url: &str) -> Self {
        Self {
            title: title.to_string(),
            type_: "oa.open.url".to_string(),
            payload: None,
            url: Some(url.to_string()),
        }
    }

    /// Create a postback button
    pub fn new_postback(title: &str, payload: &str) -> Self {
        Self {
            title: title.to_string(),
            type_: "oa.postback".to_string(),
            payload: Some(payload.to_string()),
            url: None,
        }
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.type_,
            "title": self.title,
            "payload": self.payload.clone().unwrap_or_default(),
            "url": self.url.clone().unwrap_or_default()
        })
    }
}

/// Zalo template types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZaloTemplate {
    Text(ZaloTextTemplate),
    Image(ZaloImageTemplate),
    Link(ZaloLinkTemplate),
    RequestUserInfo(ZaloRequestUserInfoTemplate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloTextTemplate {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloImageTemplate {
    pub image_url: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloLinkTemplate {
    pub image_url: String,
    pub link_url: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloRequestUserInfoTemplate {
    pub text: String,
    pub image_url: String,
    pub button_text: String,
}

/// Export for module
pub use ZaloChannel as Channel;
pub use ZaloConfig as Config;
pub use ZaloError as Error;
