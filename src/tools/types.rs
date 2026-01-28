//! Tool Types
//!
//! Core types for tool definitions and execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool definition for AI function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
    /// Whether this tool requires approval
    #[serde(default)]
    pub requires_approval: bool,
    /// Tool category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            requires_approval: false,
            category: None,
        }
    }

    /// Set the parameters schema
    pub fn with_parameters(mut self, schema: serde_json::Value) -> Self {
        self.parameters = schema;
        self
    }

    /// Require approval before execution
    pub fn with_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether execution was successful
    pub success: bool,
    /// Result content
    pub content: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Exit code (for shell commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
            exit_code: Some(0),
            execution_time_ms: None,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            content: msg.clone(),
            error: Some(msg),
            exit_code: Some(1),
            execution_time_ms: None,
        }
    }

    /// Set execution time
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = Some(ms);
        self
    }
}
