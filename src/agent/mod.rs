//! Agent System
//!
//! Provides agent orchestration with multi-turn conversations,
//! tool calling, and session management.

pub mod context;
pub mod session;

pub use context::*;
pub use session::*;

use crate::ai::{AiProvider, ChatRequest, ChatResponse, Message, MessageRole};
use crate::tools::ToolExecutor;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Model to use
    pub model: String,
    /// Temperature
    pub temperature: f32,
    /// Max tokens
    pub max_tokens: Option<u32>,
    /// Enable tools
    pub tools_enabled: bool,
}

impl AgentConfig {
    /// Create new config
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: None,
            model: model.into(),
            temperature: 0.7,
            max_tokens: None,
            tools_enabled: true,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            system_prompt: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            max_tokens: None,
            tools_enabled: true,
        }
    }
}

/// Agent for handling conversations
pub struct Agent {
    config: AgentConfig,
    #[allow(dead_code)]
    provider: Arc<dyn AiProvider>,
}

impl Agent {
    /// Create a new agent
    pub fn new(config: AgentConfig, provider: Arc<dyn AiProvider>) -> Self {
        Self { config, provider }
    }

    /// Get configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

/// Agent errors
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("AI error: {0}")]
    AiError(String),
    #[error("Tool error: {0}")]
    ToolError(#[from] crate::tools::ToolError),
    #[error("Session error: {0}")]
    SessionError(String),
}

/// Response from an agent turn
#[derive(Debug, Clone)]
pub struct AgentResponse {
    /// Content of the response
    pub content: String,
    /// Token usage
    pub usage: Option<crate::ai::Usage>,
}

/// Result of a tool call
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    /// Name of the tool
    pub tool_name: String,
    /// Whether the call was successful
    pub success: bool,
    /// Output from the tool
    pub output: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = AgentConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert!(config.tools_enabled);
    }
}
