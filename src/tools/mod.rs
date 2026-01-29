//! Tool Calling Framework
//!
//! Provides a registry and execution engine for AI-callable tools.
//! Tools can be built-in (Rust code) or external (shell commands, scripts).

pub mod builtins;
pub mod executor;
pub mod registry;
pub mod schema;
pub mod types;

pub use executor::*;
pub use registry::*;
pub use schema::*;
pub use types::*;

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur when working with tools
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionError(String),

    #[error("Invalid tool parameters: {0}")]
    InvalidParameters(String),

    #[error("Tool timed out after {0}s")]
    Timeout(u64),

    #[error("Tool approval denied")]
    ApprovalDenied,

    #[error("Tool is disabled: {0}")]
    Disabled(String),

    #[error("Schema validation error: {0}")]
    SchemaError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for tool operations
pub type Result<T> = std::result::Result<T, ToolError>;

/// A tool that can be called by AI agents
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the JSON schema for parameters
    fn parameters_schema(&self) -> serde_json::Value;

    /// Check if this tool requires approval before execution
    fn requires_approval(&self) -> bool;

    /// Get the tool category
    fn category(&self) -> Option<&str> {
        None
    }

    /// Execute the tool with the given parameters
    fn execute(&self, params: serde_json::Value) -> BoxFuture<'_, Result<ToolOutput>>;
}

/// Boxed future for tool execution
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Output from a tool execution
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// The result content
    pub content: String,
    /// Whether the execution was successful
    pub success: bool,
    /// Optional error message if failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolOutput {
    /// Create a successful output
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an error output
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            content: msg.clone(),
            success: false,
            error: Some(msg),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Initialize a tool registry with built-in tools
pub fn create_registry_with_builtins() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Register built-in tools
    registry.register(Arc::new(builtins::EchoTool));
    registry.register(Arc::new(builtins::TimeTool));
    registry.register(Arc::new(builtins::UuidTool));
    registry.register(Arc::new(builtins::SystemInfoTool));
    registry.register(Arc::new(builtins::BrowserTool));
    registry.register(Arc::new(builtins::ScreenTool));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_output() {
        let output = ToolOutput::success("Hello");
        assert!(output.success);
        assert_eq!(output.content, "Hello");
        assert!(output.error.is_none());

        let output = ToolOutput::error("Failed");
        assert!(!output.success);
        assert!(output.error.is_some());
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        registry.register(Arc::new(builtins::EchoTool));
        assert!(registry.has("echo"));

        registry.disable("echo");
        assert!(!registry.has("echo"));

        registry.enable("echo");
        assert!(registry.has("echo"));
    }
}
