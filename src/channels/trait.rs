//! Channel Trait
//!
//! Defines the interface for all messaging channel implementations.

use super::{ChannelInfo, ChannelStatus};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Result type for channel operations
pub type ChannelResult<T> = Result<T, ChannelError>;

/// Errors that can occur in channel operations
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Message send failed: {0}")]
    SendFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Channel not connected")]
    NotConnected,

    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),

    #[error("Channel error: {0}")]
    Other(String),
}

/// Message content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    /// Plain text message
    Text(String),
    /// Message with markdown formatting
    Markdown(String),
    /// Message with HTML formatting
    Html(String),
    /// Image with optional caption
    Image { url: String, caption: Option<String> },
    /// File attachment
    File { path: String, name: Option<String> },
}

/// Incoming message from a channel
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    /// Unique message ID (channel-specific)
    pub id: String,
    /// Channel type and ID
    pub channel: String,
    /// Chat/room ID
    pub chat_id: String,
    /// Sender ID
    pub sender_id: String,
    /// Sender name
    pub sender_name: Option<String>,
    /// Message content
    pub content: MessageContent,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Reply to message ID
    pub reply_to: Option<String>,
    /// Additional metadata
    pub metadata: Value,
}

/// Message handler for incoming messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle_message(&self, message: IncomingMessage);

    /// Handle an error
    async fn handle_error(&self, channel: &str, error: ChannelError);
}

/// Core channel trait
#[async_trait]
pub trait Channel: Send + Sync {
    /// Get channel info
    fn info(&self) -> ChannelInfo;

    /// Start the channel (connect and begin processing)
    async fn start(&self) -> ChannelResult<()>;

    /// Stop the channel (disconnect gracefully)
    async fn stop(&self) -> ChannelResult<()>;

    /// Send a message
    async fn send_message(
        &self,
        to: &str,
        content: MessageContent,
    ) -> ChannelResult<String>;

    /// Get current status
    fn status(&self) -> ChannelStatus;

    /// Update configuration
    async fn configure(&self, config: Value) -> ChannelResult<()>;
}

/// Type-erased channel for storage
pub type DynChannel = Arc<dyn Channel>;

/// Channel manager for handling multiple channels
pub struct ChannelManager {
    channels: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DynChannel>>>,
    handler: Arc<dyn MessageHandler>,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new(handler: Arc<dyn MessageHandler>) -> Self {
        Self {
            channels: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            handler,
        }
    }

    /// Register a channel
    pub async fn register(&self, channel: DynChannel) {
        let info = channel.info();
        let mut channels = self.channels.write().await;
        channels.insert(info.id.clone(), channel);
    }

    /// Get a channel by ID
    pub async fn get(&self, channel_id: &str) -> Option<DynChannel> {
        let channels = self.channels.read().await;
        channels.get(channel_id).cloned()
    }

    /// Start all registered channels
    pub async fn start_all(&self) {
        let channels = self.channels.read().await;
        for (id, channel) in channels.iter() {
            if let Err(e) = channel.start().await {
                tracing::error!(channel = %id, error = %e, "Failed to start channel");
            }
        }
    }

    /// Stop all registered channels
    pub async fn stop_all(&self) {
        let channels = self.channels.read().await;
        for (id, channel) in channels.iter() {
            if let Err(e) = channel.stop().await {
                tracing::error!(channel = %id, error = %e, "Failed to stop channel");
            }
        }
    }

    /// List all channels
    pub async fn list(&self) -> Vec<ChannelInfo> {
        let channels = self.channels.read().await;
        channels.values().map(|c| c.info()).collect()
    }
}
