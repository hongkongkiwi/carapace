//! WebChat Channel Implementation
//!
//! Provides a web-based chat interface for browser clients.
//! Uses WebSocket for real-time bidirectional communication.

use crate::messages::outbound::MessageContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// WebChat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatConfig {
    /// Enable WebChat channel
    pub enabled: bool,
    /// WebSocket endpoint path
    pub ws_path: String,
    /// HTTP endpoint for REST API
    pub http_path: String,
    /// CORS origins (comma-separated)
    pub cors_origins: String,
    /// Max connections per IP
    pub max_connections_per_ip: usize,
    /// Max total connections
    pub max_total_connections: usize,
    /// Message queue size per connection
    pub queue_size: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl Default for WebChatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ws_path: "/ws/chat".to_string(),
            http_path: "/api/chat".to_string(),
            cors_origins: "*".to_string(),
            max_connections_per_ip: 5,
            max_total_connections: 1000,
            queue_size: 100,
            heartbeat_interval_secs: 30,
            session_timeout_secs: 3600,
        }
    }
}

/// WebChat channel error
#[derive(Debug, thiserror::Error)]
pub enum WebChatError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("message error: {0}")]
    Message(String),
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// WebChat client session
#[derive(Debug)]
pub struct WebChatSession {
    /// Session ID
    pub id: String,
    /// Client IP address
    pub ip: String,
    /// User agent
    pub user_agent: String,
    /// Connected at timestamp
    pub connected_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Outgoing message queue
    pub message_tx: mpsc::Sender<String>,
}

/// WebChat channel state
#[derive(Debug, Default)]
pub struct WebChatState {
    /// Active sessions
    pub sessions: HashMap<String, WebChatSession>,
    /// IP to session count mapping
    pub ip_counts: HashMap<String, usize>,
    /// Total connection count
    pub total_connections: usize,
}

/// WebChat channel struct
#[derive(Debug)]
pub struct WebChatChannel {
    config: WebChatConfig,
    state: Arc<Mutex<WebChatState>>,
    event_tx: mpsc::Sender<MessageContent>,
}

impl WebChatChannel {
    /// Create a new WebChat channel
    pub fn new(config: WebChatConfig, event_tx: mpsc::Sender<MessageContent>) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(WebChatState::default())),
            event_tx,
        }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        session_id: &str,
        ip: &str,
        user_agent: &str,
    ) -> Result<mpsc::Receiver<String>, WebChatError> {
        let (tx, rx) = mpsc::channel(self.config.queue_size);

        let mut state = self.state.lock().await;

        // Check limits - read-only checks first
        {
            let ip_count = state.ip_counts.get(ip).copied().unwrap_or(0);
            if ip_count >= self.config.max_connections_per_ip {
                return Err(WebChatError::Connection("Too many connections from IP".to_string()));
            }
            if state.total_connections >= self.config.max_total_connections {
                return Err(WebChatError::Connection("Maximum connections reached".to_string()));
            }
        }

        // Create session
        let session = WebChatSession {
            id: session_id.to_string(),
            ip: ip.to_string(),
            user_agent: user_agent.to_string(),
            connected_at: now_secs(),
            last_activity: now_secs(),
            message_tx: tx,
        };

        // Now do the mutable operations
        *state.ip_counts.entry(ip.to_string()).or_insert(0) += 1;
        state.sessions.insert(session_id.to_string(), session);
        state.total_connections += 1;

        info!("WebChat session created: {} from {}", session_id, ip);
        Ok(rx)
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) {
        let mut state = self.state.lock().await;

        if let Some(session) = state.sessions.remove(session_id) {
            let ip_count = state.ip_counts.entry(session.ip.clone()).or_insert(0);
            if *ip_count > 0 {
                *ip_count -= 1;
            }
            if ip_count == &0 {
                state.ip_counts.remove(&session.ip);
            }
            state.total_connections -= 1;
            info!("WebChat session removed: {}", session_id);
        }
    }

    /// Send a message to a specific session
    pub async fn send_to_session(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<(), WebChatError> {
        let state = self.state.lock().await;

        if let Some(session) = state.sessions.get(session_id) {
            let msg = message.to_string();
            let tx = session.message_tx.clone();
            drop(state); // Release the lock before trying to send
            if let Err(e) = tx.try_send(msg) {
                return Err(WebChatError::Message(e.to_string()));
            }
            Ok(())
        } else {
            Err(WebChatError::Session("Session not found".to_string()))
        }
    }

    /// Broadcast a message to all sessions
    pub async fn broadcast(&self, message: &str) {
        let state = self.state.lock().await;
        let msg = message.to_string();

        for session in state.sessions.values() {
            let _ = session.message_tx.try_send(msg.clone());
        }
    }

    /// Send to default/recipient sessions matching a filter
    pub async fn send_to_matching(
        &self,
        filter: impl Fn(&WebChatSession) -> bool,
        message: &str,
    ) {
        let state = self.state.lock().await;
        let msg = message.to_string();

        for session in state.sessions.values() {
            if filter(session) {
                let _ = session.message_tx.try_send(msg.clone());
            }
        }
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.state.lock().await.sessions.len()
    }

    /// Get all active session IDs
    pub async fn session_ids(&self) -> Vec<String> {
        self.state.lock().await.sessions.keys().cloned().collect()
    }

    /// Update session activity
    pub async fn update_activity(&self, session_id: &str) {
        let mut state = self.state.lock().await;
        if let Some(session) = state.sessions.get_mut(session_id) {
            session.last_activity = now_secs();
        }
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let mut state = self.state.lock().await;
        let timeout = self.config.session_timeout_secs as u64;
        let now = now_secs();

        let expired: Vec<String> = state
            .sessions
            .values()
            .filter(|s| now - s.last_activity > timeout)
            .map(|s| s.id.clone())
            .collect();

        for session_id in &expired {
            if let Some(session) = state.sessions.remove(session_id) {
                let ip_count = state.ip_counts.entry(session.ip.clone()).or_insert(0);
                if *ip_count > 0 {
                    *ip_count -= 1;
                }
                state.total_connections -= 1;
            }
        }

        info!("Cleaned up {} expired WebChat sessions", expired.len());
        expired.len()
    }

    /// Connect to WebChat
    pub async fn connect(&mut self) -> Result<(), WebChatError> {
        info!("WebChat channel connected");
        Ok(())
    }

    /// Disconnect from WebChat
    pub async fn disconnect(&mut self) -> Result<(), WebChatError> {
        // Close all sessions
        let mut state = self.state.lock().await;
        state.sessions.clear();
        state.ip_counts.clear();
        state.total_connections = 0;
        info!("WebChat channel disconnected");
        Ok(())
    }
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebChatMessage {
    /// Client sending a message
    #[serde(rename = "message")]
    Message {
        content: String,
        reply_to: Option<String>,
    },
    /// Client joining a room/channel
    #[serde(rename = "join")]
    Join { room: String },
    /// Client leaving a room/channel
    #[serde(rename = "leave")]
    Leave { room: String },
    /// Client requesting typing status
    #[serde(rename = "typing")]
    ClientTyping { room: Option<String> },
    /// Client sending a heartbeat
    #[serde(rename = "ping")]
    Ping,
    /// Server confirming connection
    #[serde(rename = "connected")]
    Connected { session_id: String },
    /// Server sending a message
    #[serde(rename = "server_message")]
    ServerMessage {
        id: String,
        content: String,
        sender: String,
        room: Option<String>,
        timestamp: u64,
    },
    /// Server acknowledging receipt
    #[serde(rename = "ack")]
    Ack { message_id: String },
    /// Server broadcasting user joined
    #[serde(rename = "user_joined")]
    UserJoined { room: String, user_id: String },
    /// Server broadcasting user left
    #[serde(rename = "user_left")]
    UserLeft { room: String, user_id: String },
    /// Server indicating user is typing
    #[serde(rename = "user_typing")]
    UserTyping { room: String, user_id: String, is_typing: bool },
    /// Server heartbeat response
    #[serde(rename = "pong")]
    Pong,
    /// Error message
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

/// Convert WebChatMessage to MessageContent
impl TryFrom<WebChatMessage> for MessageContent {
    type Error = String;

    fn try_from(msg: WebChatMessage) -> Result<Self, Self::Error> {
        match msg {
            WebChatMessage::Message { content, reply_to: _ } => Ok(MessageContent::Text { text: content }),
            _ => Err("Cannot convert non-message type".to_string()),
        }
    }
}

/// Helper to generate unique session ID
pub fn generate_session_id() -> String {
    format!("ws_{}", uuid::Uuid::new_v4().simple())
}

/// Helper to get current time in seconds
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Export for module
pub use WebChatChannel as Channel;
pub use WebChatConfig as Config;
pub use WebChatError as Error;
