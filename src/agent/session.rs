//! Agent Session
//!
//! Session management for agent conversations with persistence support.

use crate::ai::{Message, MessageRole, MessageContent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A session with an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique session ID
    id: String,
    /// Session name/title (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Agent configuration name
    pub agent_name: String,
    /// Conversation messages
    messages: Vec<Message>,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Maximum number of messages to retain
    #[serde(skip)]
    max_messages: usize,
}

impl AgentSession {
    /// Create a new session
    pub fn new(agent_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            agent_name: agent_name.into(),
            messages: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: None,
            max_messages: 1000,
        }
    }

    /// Create a session with a specific ID
    pub fn with_id(id: impl Into<String>, agent_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            agent_name: agent_name.into(),
            messages: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: None,
            max_messages: 1000,
        }
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Set the session name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set max messages
    pub fn with_max_messages(mut self, max: usize) -> Self {
        self.max_messages = max;
        self
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: MessageRole, content: impl Into<String>) {
        self.messages.push(Message {
            role,
            content: MessageContent::Text(content.into()),
            name: None,
        });
        self.updated_at = Some(Utc::now());
        self.trim_if_needed();
    }

    /// Add a tool result message
    pub fn add_tool_result(&mut self, tool_name: &str, output: &str) {
        let content = format!("Tool '{}' result: {}", tool_name, output);
        self.add_message(MessageRole::Tool, content);
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the last message
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get turn count (user-assistant pairs)
    pub fn turn_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::User))
            .count()
    }

    /// Set metadata value
    pub fn set_metadata(
        &mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) {
        self.metadata.insert(key.into(), value.into());
        self.updated_at = Some(Utc::now());
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = Some(Utc::now());
    }

    /// Check if session is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get a summary of the session
    pub fn summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            agent_name: self.agent_name.clone(),
            message_count: self.message_count(),
            turn_count: self.turn_count(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Trim messages if exceeding max
    fn trim_if_needed(&mut self) {
        if self.messages.len() > self.max_messages {
            // Keep system message if present, then trim oldest
            let has_system = self
                .messages
                .first()
                .map(|m| matches!(m.role, MessageRole::System))
                .unwrap_or(false);

            let keep_count = if has_system {
                self.max_messages - 1
            } else {
                self.max_messages
            };

            let start_idx = self.messages.len() - keep_count;
            let new_messages: Vec<_> = if has_system {
                let mut msgs = vec![self.messages[0].clone()];
                msgs.extend(self.messages[start_idx..].iter().cloned());
                msgs
            } else {
                self.messages[start_idx..].to_vec()
            };

            self.messages = new_messages;
        }
    }
}

/// Summary of a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub agent_name: String,
    pub message_count: usize,
    pub turn_count: usize,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_basic() {
        let mut session = AgentSession::new("test-agent");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi there!");

        assert_eq!(session.message_count(), 2);
        assert_eq!(session.turn_count(), 1);
        assert!(!session.is_empty());
    }

    #[test]
    fn test_session_json_roundtrip() {
        let mut session = AgentSession::new("test-agent").with_name("Test Session");
        session.add_message(MessageRole::User, "Hello");

        let json = session.to_json().unwrap();
        let loaded = AgentSession::from_json(&json).unwrap();

        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.name, session.name);
        assert_eq!(loaded.message_count(), session.message_count());
    }
}
