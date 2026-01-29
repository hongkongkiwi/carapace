//! Telegram Channel Implementation
//!
//! Provides messaging support via the Telegram Bot API.
//! Supports text messages, media, and callbacks.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Telegram channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token from @BotFather
    pub bot_token: String,
    /// Webhook URL (optional, uses polling if not set)
    pub webhook_url: Option<String>,
    /// Secret token for webhook verification
    pub secret_token: Option<String>,
    /// Maximum message length
    pub max_message_length: usize,
    /// Enable media handling
    pub enable_media: bool,
    /// Enable callback queries
    pub enable_callbacks: bool,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            webhook_url: None,
            secret_token: None,
            max_message_length: 4096,
            enable_media: true,
            enable_callbacks: true,
        }
    }
}

/// Telegram channel error
#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Telegram channel struct
#[derive(Debug)]
pub struct TelegramChannel {
    config: TelegramConfig,
    client: reqwest::Client,
    // Event sender for incoming messages
    event_tx: mpsc::Sender<MessageContent>,
}

impl TelegramChannel {
    /// Create a new Telegram channel
    pub fn new(config: TelegramConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        Self { config, client, event_tx }
    }

    /// Get the API base URL
    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.config.bot_token, method)
    }

    /// Send a request to the Telegram API
    async fn api_request<T: for<'de> Deserialize<'de>>(&self, method: &str, body: serde_json::Value) -> Result<T, TelegramError> {
        let response = self
            .client
            .post(self.api_url(method))
            .json(&body)
            .send()
            .await
            .map_err(|e| TelegramError::Network(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TelegramError::Parse(e.to_string()))?;

        if json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error_msg = json.get("description").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(TelegramError::Api(error_msg.to_string()));
        }

        serde_json::from_value(json.get("result").cloned().unwrap_or_default())
            .map_err(|e| TelegramError::Parse(e.to_string()))
    }

    /// Send a text message
    pub async fn send_message(&self, chat_id: &str, text: &str) -> Result<String, TelegramError> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "disable_web_page_preview": true,
        });

        self.api_request::<TelegramMessageResponse>("sendMessage", body)
            .await
            .map(|r| r.message_id.to_string())
    }

    /// Send a photo
    pub async fn send_photo(&self, chat_id: &str, photo_url: &str, caption: Option<&str>) -> Result<String, TelegramError> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": photo_url,
        });

        if let Some(caption) = caption {
            body["caption"] = serde_json::json!(caption);
            body["parse_mode"] = serde_json::json!("Markdown");
        }

        self.api_request::<TelegramMessageResponse>("sendPhoto", body)
            .await
            .map(|r| r.message_id.to_string())
    }

    /// Answer callback query
    pub async fn answer_callback(&self, callback_id: &str, text: Option<&str>) -> Result<(), TelegramError> {
        let body = serde_json::json!({
            "callback_query_id": callback_id,
            "text": text.unwrap_or(""),
            "show_alert": false,
        });

        self.api_request::<()>("answerCallbackQuery", body).await
    }

    /// Connect to Telegram
    pub async fn connect(&mut self) -> Result<(), TelegramError> {
        info!("Connecting to Telegram...");

        // Test the bot token
        let body = serde_json::json!({});
        let _: TelegramUserResponse = self.api_request("getMe", body).await?;

        info!("Telegram connected successfully");
        Ok(())
    }

    /// Disconnect from Telegram
    pub async fn disconnect(&mut self) -> Result<(), TelegramError> {
        info!("Disconnecting from Telegram...");
        Ok(())
    }
}

// Telegram API response types

#[derive(Debug, Deserialize)]
struct TelegramUserResponse {
    id: i64,
    is_bot: bool,
    first_name: String,
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessageResponse {
    message_id: i64,
    chat: TelegramChat,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(default)]
    type_: String,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
    callback_query: Option<TelegramCallbackQuery>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    chat: TelegramChat,
    text: Option<String>,
    photo: Option<Vec<TelegramPhoto>>,
    #[serde(default)]
    from: Option<TelegramUser>,
}

#[derive(Debug, Deserialize)]
struct TelegramPhoto {
    file_id: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
    is_bot: bool,
    first_name: String,
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramCallbackQuery {
    id: String,
    from: TelegramUser,
    message: Option<TelegramMessage>,
    data: Option<String>,
}
