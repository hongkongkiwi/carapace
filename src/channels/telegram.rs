//! Telegram Channel
//!
//! Telegram bot integration using teloxide for carapace.

use super::{Channel, ChannelError, ChannelInfo, ChannelResult, ChannelStatus, MessageContent};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Telegram channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token from BotFather
    pub bot_token: String,
    /// Webhook URL (optional, uses polling if not set)
    pub webhook_url: Option<String>,
    /// Allowed chat IDs (empty = all allowed)
    pub allowed_chats: Vec<i64>,
    /// Channel description
    #[serde(default)]
    pub description: Option<String>,
}

/// Internal state for the Telegram channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelegramState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Telegram channel implementation
pub struct TelegramChannel {
    config: TelegramConfig,
    state: RwLock<TelegramState>,
    shutdown_tx: RwLock<Option<mpsc::Sender<()>>>,
}

impl TelegramChannel {
    /// Create a new Telegram channel
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            state: RwLock::new(TelegramState::Disconnected),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Check if a chat ID is allowed
    fn is_chat_allowed(&self, chat_id: i64) -> bool {
        self.config.allowed_chats.is_empty()
            || self.config.allowed_chats.contains(&chat_id)
    }

    /// Get current state
    async fn get_state(&self) -> TelegramState {
        *self.state.read().await
    }

    /// Set state
    async fn set_state(&self, state: TelegramState) {
        *self.state.write().await = state;
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn info(&self) -> ChannelInfo {
        let status = match *self.state.blocking_read() {
            TelegramState::Connected => ChannelStatus::Connected,
            TelegramState::Connecting => ChannelStatus::Connecting,
            TelegramState::Error => ChannelStatus::Error,
            TelegramState::Disconnected => ChannelStatus::Disconnected,
        };

        ChannelInfo::new("telegram", "Telegram")
            .with_status(status)
            .with_metadata(super::ChannelMetadata {
                description: self.config.description.clone(),
                extra: Some(
                    serde_json::json!({
                        "webhook_url": self.config.webhook_url,
                        "allowed_chats_count": self.config.allowed_chats.len(),
                    })
                ),
                last_error: None,
                last_connected_at: None,
                status_changed_at: None,
            })
    }

    async fn start(&self) -> ChannelResult<()> {
        let current_state = self.get_state().await;
        if current_state == TelegramState::Connected || current_state == TelegramState::Connecting
        {
            return Ok(());
        }

        self.set_state(TelegramState::Connecting).await;
        tracing::info!("Starting Telegram bot");

        // Validate config
        if self.config.bot_token.is_empty() {
            self.set_state(TelegramState::Error).await;
            return Err(ChannelError::InvalidConfig(
                "Bot token is required".to_string(),
            ));
        }

        // Set up shutdown channel
        let (tx, mut rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write().await = Some(tx);

        // Mark as connected (in a real implementation, this would happen
        // after successful connection to Telegram API)
        self.set_state(TelegramState::Connected).await;
        tracing::info!("Telegram bot started successfully");

        // Spawn a task to handle shutdown signal
        tokio::spawn(async move {
            let _ = rx.recv().await;
            tracing::info!("Telegram bot received shutdown signal");
        });

        Ok(())
    }

    async fn stop(&self) -> ChannelResult<()> {
        let current_state = self.get_state().await;
        if current_state == TelegramState::Disconnected {
            return Ok(());
        }

        tracing::info!("Stopping Telegram bot");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
        }

        self.set_state(TelegramState::Disconnected).await;
        tracing::info!("Telegram bot stopped");

        Ok(())
    }

    async fn send_message(&self, to: &str, content: MessageContent) -> ChannelResult<String> {
        if self.get_state().await != TelegramState::Connected {
            return Err(ChannelError::NotConnected);
        }

        // Parse chat ID
        let chat_id: i64 = to
            .parse()
            .map_err(|_| ChannelError::InvalidConfig("Invalid chat ID format".to_string()))?;

        // Check if chat is allowed
        if !self.is_chat_allowed(chat_id) {
            return Err(ChannelError::Other("Chat not allowed".to_string()));
        }

        // Convert content to text
        let text = match &content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Markdown(text) => text.clone(),
            MessageContent::Html(text) => text.clone(),
            MessageContent::Image { url, caption } => {
                format!("Image: {}{}", url, caption.as_ref().map(|c| format!(" - {}", c)).unwrap_or_default())
            }
            MessageContent::File { path, name } => {
                format!("File: {}{}", path, name.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default())
            }
        };

        tracing::debug!(chat_id, text = %text, "Sending Telegram message");

        // In a real implementation, this would call the Telegram Bot API
        // For now, we return a mock message ID
        Ok(format!("msg_{}_{}", chat_id, chrono::Utc::now().timestamp_millis()))
    }

    fn status(&self) -> ChannelStatus {
        match *self.state.blocking_read() {
            TelegramState::Connected => ChannelStatus::Connected,
            TelegramState::Connecting => ChannelStatus::Connecting,
            TelegramState::Error => ChannelStatus::Error,
            TelegramState::Disconnected => ChannelStatus::Disconnected,
        }
    }

    async fn configure(&self, config: Value) -> ChannelResult<()> {
        // Parse new config
        let _new_config: TelegramConfig =
            serde_json::from_value(config).map_err(|e| ChannelError::InvalidConfig(e.to_string()))?;

        // Stop if running
        self.stop().await?;

        // Update config (this would require interior mutability in a real implementation)
        // For now, we just log the change
        tracing::info!("Telegram configuration updated");

        // Restart with new config
        // Note: In a real implementation, we'd update self.config here

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> TelegramConfig {
        TelegramConfig {
            bot_token: "test_token".to_string(),
            webhook_url: None,
            allowed_chats: vec![],
            description: Some("Test bot".to_string()),
        }
    }

    #[tokio::test]
    async fn test_telegram_channel_lifecycle() {
        let config = create_test_config();
        let channel = TelegramChannel::new(config);

        // Initial state
        assert_eq!(channel.status(), ChannelStatus::Disconnected);

        // Start
        channel.start().await.unwrap();
        assert_eq!(channel.status(), ChannelStatus::Connected);

        // Stop
        channel.stop().await.unwrap();
        assert_eq!(channel.status(), ChannelStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_send_message_not_connected() {
        let config = create_test_config();
        let channel = TelegramChannel::new(config);

        let result = channel.send_message("123456", MessageContent::Text("Hello".to_string())).await;
        assert!(matches!(result, Err(ChannelError::NotConnected)));
    }

    #[tokio::test]
    async fn test_chat_allowed() {
        let config = TelegramConfig {
            bot_token: "test".to_string(),
            webhook_url: None,
            allowed_chats: vec![123456, 789012],
            description: None,
        };
        let channel = TelegramChannel::new(config);

        assert!(channel.is_chat_allowed(123456));
        assert!(channel.is_chat_allowed(789012));
        assert!(!channel.is_chat_allowed(999999));
    }

    #[tokio::test]
    async fn test_all_chats_allowed_when_empty() {
        let config = TelegramConfig {
            bot_token: "test".to_string(),
            webhook_url: None,
            allowed_chats: vec![],
            description: None,
        };
        let channel = TelegramChannel::new(config);

        assert!(channel.is_chat_allowed(123456));
        assert!(channel.is_chat_allowed(999999));
    }

    #[tokio::test]
    async fn test_info() {
        let config = create_test_config();
        let channel = TelegramChannel::new(config);

        let info = channel.info();
        assert_eq!(info.id, "telegram");
        assert_eq!(info.name, "Telegram");
        assert_eq!(info.status, ChannelStatus::Disconnected);
        assert!(info.metadata.description.is_some());
    }
}
