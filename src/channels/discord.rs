//! Discord Channel Implementation
//!
//! Provides messaging support via the Discord Bot API.
//! Supports text messages, embeds, and interactions.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Discord channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token from Discord Developer Portal
    pub bot_token: String,
    /// Application ID
    pub application_id: String,
    /// Guild ID for commands
    pub guild_id: Option<String>,
    /// Enable slash commands
    pub enable_commands: bool,
    /// Enable message components (buttons, select menus)
    pub enable_components: bool,
    /// Maximum message length
    pub max_message_length: usize,
    /// Intent configuration
    pub intents: DiscordIntents,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            application_id: String::new(),
            guild_id: None,
            enable_commands: true,
            enable_components: true,
            max_message_length: 2000,
            intents: DiscordIntents::default(),
        }
    }
}

/// Discord Gateway Intents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordIntents {
    pub guilds: bool,
    pub guild_messages: bool,
    pub direct_messages: bool,
    pub message_content: bool,
    pub guild_members: bool,
    pub presence: bool,
    // Additional intents
    pub guild_bans: bool,
    pub guild_emojis: bool,
    pub guild_integrations: bool,
    pub guild_webhooks: bool,
    pub guild_message_reactions: bool,
    pub guild_message_typing: bool,
    pub direct_message_reactions: bool,
    pub direct_message_typing: bool,
    pub guild_scheduled_events: bool,
}

impl Default for DiscordIntents {
    fn default() -> Self {
        Self {
            guilds: true,
            guild_messages: true,
            direct_messages: true,
            message_content: true,
            guild_members: false,
            presence: false,
            guild_bans: false,
            guild_emojis: false,
            guild_integrations: false,
            guild_webhooks: false,
            guild_message_reactions: false,
            guild_message_typing: false,
            direct_message_reactions: false,
            direct_message_typing: false,
            guild_scheduled_events: false,
        }
    }
}

/// Discord channel error
#[derive(Debug, thiserror::Error)]
pub enum DiscordError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Authentication error: {0}")]
    Authentication(String),
}

fn intents_to_bits(intents: &DiscordIntents) -> u32 {
    let mut bits = 0u32;
    if intents.guilds { bits |= 1 << 0; }
    if intents.guild_members { bits |= 1 << 1; }
    if intents.guild_bans { bits |= 1 << 2; }
    if intents.guild_emojis { bits |= 1 << 3; }
    if intents.guild_integrations { bits |= 1 << 4; }
    if intents.guild_webhooks { bits |= 1 << 5; }
    if intents.guild_messages { bits |= 1 << 9; }
    if intents.guild_message_reactions { bits |= 1 << 10; }
    if intents.guild_message_typing { bits |= 1 << 11; }
    if intents.direct_messages { bits |= 1 << 12; }
    if intents.direct_message_reactions { bits |= 1 << 13; }
    if intents.direct_message_typing { bits |= 1 << 14; }
    if intents.message_content { bits |= 1 << 15; }
    if intents.guild_scheduled_events { bits |= 1 << 16; }
    bits
}

/// Discord channel struct
#[derive(Debug)]
pub struct DiscordChannel {
    config: DiscordConfig,
    client: reqwest::Client,
    event_tx: mpsc::Sender<MessageContent>,
    http_url: String,
    ws_url: String,
}

impl DiscordChannel {
    /// Create a new Discord channel
    pub fn new(config: DiscordConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        let http_url = format!("https://discord.com/api/v10");
        let ws_url = format!("wss://gateway.discord.gg");

        Self { config, client, event_tx, http_url, ws_url }
    }

    /// Send a request to the Discord API
    async fn api_request<T: for<'de> Deserialize<'de>>(&self, method: reqwest::Method, endpoint: &str, body: Option<serde_json::Value>) -> Result<T, DiscordError> {
        let mut request = self
            .client
            .request(method, format!("{}/{}", self.http_url, endpoint))
            .bearer_auth(&self.config.bot_token);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| DiscordError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscordError::Api(error_text));
        }

        response
            .json()
            .await
            .map_err(|e| DiscordError::Parse(e.to_string()))
    }

    /// Send a text message
    pub async fn send_message(&self, channel_id: &str, content: &str, embeds: Option<Vec<DiscordEmbed>>) -> Result<String, DiscordError> {
        let mut body = serde_json::json!({
            "content": content,
        });

        if let Some(embeds) = embeds {
            body["embeds"] = serde_json::json!(embeds);
        }

        self.api_request::<DiscordMessageResponse>(reqwest::Method::POST, &format!("channels/{}/messages", channel_id), Some(body))
            .await
            .map(|r| r.id)
    }

    /// Create a message with components (buttons, select menus)
    pub async fn send_with_components(&self, channel_id: &str, content: &str, components: Vec<DiscordComponent>) -> Result<String, DiscordError> {
        let body = serde_json::json!({
            "content": content,
            "components": components.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
        });

        self.api_request::<DiscordMessageResponse>(reqwest::Method::POST, &format!("channels/{}/messages", channel_id), Some(body))
            .await
            .map(|r| r.id)
    }

    /// Connect to Discord
    pub async fn connect(&mut self) -> Result<(), DiscordError> {
        info!("Connecting to Discord...");

        // Verify the bot token
        let _: DiscordUserResponse = self.api_request(reqwest::Method::GET, "users/@me", None).await?;

        info!("Discord connected successfully");
        Ok(())
    }

    /// Disconnect from Discord
    pub async fn disconnect(&mut self) -> Result<(), DiscordError> {
        info!("Disconnecting from Discord...");
        Ok(())
    }
}

// Discord API response types
#[derive(Debug, Deserialize)]
struct DiscordUserResponse {
    id: String,
    username: String,
    discriminator: String,
    bot: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DiscordMessageResponse {
    id: String,
    channel_id: String,
    content: String,
    embeds: Vec<DiscordEmbed>,
    author: DiscordUser,
}

#[derive(Debug, Deserialize, Clone)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
    avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbed {
    title: Option<String>,
    description: Option<String>,
    fields: Vec<DiscordEmbedField>,
    color: Option<u32>,
    footer: Option<DiscordEmbedFooter>,
    thumbnail: Option<DiscordImage>,
    image: Option<DiscordImage>,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedField {
    name: String,
    value: String,
    inline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbedFooter {
    text: String,
    icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordImage {
    url: String,
}

#[derive(Debug, Clone)]
pub struct DiscordComponent {
    component_type: u8,
    custom_id: String,
    style: Option<u8>,
    label: Option<String>,
    emoji: Option<DiscordEmoji>,
    options: Vec<DiscordSelectOption>,
    placeholder: Option<String>,
}

impl DiscordComponent {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.component_type,
            "custom_id": self.custom_id,
            "style": self.style,
            "label": self.label,
            "emoji": self.emoji,
            "options": self.options.iter().map(|o| o.to_json()).collect::<Vec<_>>(),
            "placeholder": self.placeholder,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmoji {
    id: Option<String>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordSelectOption {
    label: String,
    value: String,
    description: Option<String>,
    emoji: Option<DiscordEmoji>,
    default: Option<bool>,
}

impl DiscordSelectOption {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "label": self.label,
            "value": self.value,
            "description": self.description,
            "emoji": self.emoji,
            "default": self.default,
        })
    }
}
