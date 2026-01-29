//! Matrix Channel Implementation
//!
//! Provides messaging support via the Matrix protocol.
//! Supports text messages, room management, and user presence.

use crate::messages::outbound::MessageContent;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use url::Url;

/// Matrix channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Enable Matrix channel
    pub enabled: bool,
    /// Homeserver URL (e.g., "https://matrix.org")
    pub homeserver: String,
    /// User ID (e.g., "@user:matrix.org")
    pub user_id: String,
    /// Access token for authentication
    pub access_token: String,
    /// Default room to send messages to
    pub default_room: String,
    /// Device ID for E2E encryption
    pub device_id: Option<String>,
    /// Sync timeout in milliseconds
    pub sync_timeout_ms: u64,
    /// Maximum message length
    pub max_message_length: usize,
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            homeserver: String::new(),
            user_id: String::new(),
            access_token: String::new(),
            default_room: String::new(),
            device_id: None,
            sync_timeout_ms: 30000,
            max_message_length: 65536,
        }
    }
}

/// Matrix channel error
#[derive(Debug, thiserror::Error)]
pub enum MatrixError {
    #[error("network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("room error: {0}")]
    Room(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Matrix channel struct
#[derive(Debug)]
pub struct MatrixChannel {
    config: MatrixConfig,
    client: Client,
    event_tx: mpsc::Sender<MessageContent>,
}

impl MatrixChannel {
    /// Create a new Matrix channel
    pub fn new(config: MatrixConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
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
        format!("{}/_matrix/client/v3{}", self.config.homeserver, path)
    }

    /// Sync API endpoint
    fn sync_url(&self, since: Option<&str>, timeout: u64) -> String {
        let mut url = format!("{}/sync?timeout={}", self.api_url(""), timeout);
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        url
    }

    /// Send a request to the Matrix API
    async fn api_request<T>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, MatrixError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let mut request = self
            .client
            .request(
                method.parse().expect("Invalid HTTP method"),
                self.api_url(path),
            )
            .header("Authorization", format!("Bearer {}", self.config.access_token))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| MatrixError::Network(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MatrixError::Parse(e.to_string()))?;

        if let Some(err_code) = json.get("errcode").and_then(|v| v.as_str()) {
            let msg = json.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(match err_code {
                "M_FORBIDDEN" => MatrixError::Auth(msg.to_string()),
                "M_UNKNOWN" | "M_NOT_FOUND" => MatrixError::Room(msg.to_string()),
                _ => MatrixError::Api(format!("{}: {}", err_code, msg)),
            });
        }

        serde_json::from_value(json)
            .map_err(|e| MatrixError::Parse(e.to_string()))
    }

    /// Send a text message to a room
    pub async fn send_message(
        &self,
        room_id: &str,
        text: &str,
    ) -> Result<String, MatrixError> {
        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": text,
        });

        let response: MatrixSendResponse = self
            .api_request("POST", &format!("/rooms/{}/send/m.room.message", room_id), Some(body))
            .await?;

        Ok(response.event_id)
    }

    /// Send a message to the default room
    pub async fn send_default(&self, text: &str) -> Result<String, MatrixError> {
        if self.config.default_room.is_empty() {
            return Err(MatrixError::Room("No default room configured".to_string()));
        }
        self.send_message(&self.config.default_room, text).await
    }

    /// Send a formatted message (HTML)
    pub async fn send_html(
        &self,
        room_id: &str,
        text: &str,
        html: &str,
    ) -> Result<String, MatrixError> {
        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": text,
            "format": "org.matrix.custom.html",
            "formatted_body": html,
        });

        let response: MatrixSendResponse = self
            .api_request("POST", &format!("/rooms/{}/send/m.room.message", room_id), Some(body))
            .await?;

        Ok(response.event_id)
    }

    /// Join a room
    pub async fn join_room(&self, room_id: &str) -> Result<(), MatrixError> {
        let body = serde_json::json!({});
        self.api_request::<serde_json::Value>(
            "POST",
            &format!("/rooms/{}/join", room_id),
            Some(body),
        )
        .await?;
        Ok(())
    }

    /// Leave a room
    pub async fn leave_room(&self, room_id: &str) -> Result<(), MatrixError> {
        let body = serde_json::json!({});
        self.api_request::<serde_json::Value>(
            "POST",
            &format!("/rooms/{}/leave", room_id),
            Some(body),
        )
        .await?;
        Ok(())
    }

    /// Create a new room
    pub async fn create_room(
        &self,
        name: &str,
        is_private: bool,
    ) -> Result<String, MatrixError> {
        let body = serde_json::json!({
            "name": name,
            "visibility": if is_private { "private" } else { "public" },
        });

        let response: MatrixRoomResponse = self
            .api_request("POST", "/createRoom", Some(body))
            .await?;

        Ok(response.room_id)
    }

    /// Get user info
    pub async fn get_user_info(&self) -> Result<MatrixUser, MatrixError> {
        self.api_request("GET", "/account/whoami", None)
            .await
    }

    /// Connect to Matrix
    pub async fn connect(&mut self) -> Result<(), MatrixError> {
        // Verify credentials by fetching user info
        let _user = self.get_user_info().await?;
        info!("Matrix connected as {}", self.config.user_id);
        Ok(())
    }

    /// Disconnect from Matrix
    pub async fn disconnect(&mut self) -> Result<(), MatrixError> {
        info!("Matrix disconnected");
        Ok(())
    }
}

/// Matrix API response types
#[derive(Debug, Deserialize)]
struct MatrixSendResponse {
    event_id: String,
}

#[derive(Debug, Deserialize)]
struct MatrixRoomResponse {
    room_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum MatrixRoomType {
    #[serde(rename = "m.room")]
    Room,
    #[serde(rename = "m.room.direct")]
    Direct,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatrixUser {
    user_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

/// Sync response from Matrix
#[derive(Debug, Deserialize)]
pub struct MatrixSyncResponse {
    pub next_batch: String,
    pub rooms: MatrixRoomsSection,
    pub presence: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MatrixRoomsSection {
    pub join: std::collections::HashMap<String, MatrixRoomData>,
    pub invite: std::collections::HashMap<String, serde_json::Value>,
    pub leave: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixRoomData {
    pub room_id: String,
    pub summary: Option<MatrixRoomSummary>,
    pub state: MatrixStateResponse,
    pub timeline: MatrixTimeline,
}

#[derive(Debug, Deserialize)]
pub struct MatrixRoomSummary {
    #[serde(default)]
    pub m_heroes: Vec<String>,
    #[serde(default)]
    pub m_member_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct MatrixStateResponse {
    pub events: Vec<MatrixEvent>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixTimeline {
    pub events: Vec<MatrixEvent>,
    #[serde(default)]
    pub limited: bool,
    pub prev_batch: Option<String>,
}

/// Matrix event from sync
#[derive(Debug, Clone, Deserialize)]
pub struct MatrixEvent {
    #[serde(default)]
    pub event_id: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub sender: String,
    #[serde(default)]
    pub content: serde_json::Value,
    #[serde(default)]
    pub origin_server_ts: u64,
    #[serde(default)]
    pub unsigned: serde_json::Value,
}

/// Convert Matrix event to MessageContent
impl TryFrom<MatrixEvent> for MessageContent {
    type Error = String;

    fn try_from(event: MatrixEvent) -> Result<Self, Self::Error> {
        let text = event
            .content
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(MessageContent::Text { text })
    }
}

/// Export for module
pub use MatrixChannel as Channel;
pub use MatrixConfig as Config;
pub use MatrixError as Error;
