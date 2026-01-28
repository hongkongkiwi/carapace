//! Tool Executor
//!
//! Handles the execution lifecycle of tools including validation,
//! approval checks, and timeout handling.

use super::{Tool, ToolError, ToolOutput, ToolRegistry, ToolInfo};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Request ID for tracing
    pub request_id: String,
    /// User ID executing the tool
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Additional context
    pub metadata: std::collections::HashMap<String, Value>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            user_id: None,
            session_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Tool executor with validation and lifecycle management
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    default_timeout: Duration,
    approval_callback: Option<Arc<dyn ApprovalCallback>>,
}

/// Callback for approval requests
pub trait ApprovalCallback: Send + Sync {
    /// Request approval for a tool execution
    fn request_approval(
        &self,
        tool_name: String,
        params: Value,
        context: ExecutionContext,
    ) -> super::BoxFuture<'static, bool>;
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            default_timeout: Duration::from_secs(30),
            approval_callback: None,
        }
    }

    /// Set the default timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set the approval callback
    pub fn with_approval_callback(mut self, callback: Arc<dyn ApprovalCallback>) -> Self {
        self.approval_callback = Some(callback);
        self
    }

    /// Execute a tool by name with parameters
    pub async fn execute(
        &self,
        tool_name: &str,
        params: Value,
        context: &ExecutionContext,
    ) -> super::Result<ToolOutput> {
        let start = Instant::now();

        // Get tool from registry
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| ToolError::NotFound(tool_name.to_string()))?;

        debug!(
            request_id = %context.request_id,
            tool = tool_name,
            "Starting tool execution"
        );

        // Validate parameters against schema
        if let Err(e) = self.validate_params(&tool, &params).await {
            return Err(ToolError::InvalidParameters(format!(
                "Parameter validation failed: {}",
                e
            )));
        }

        // Check approval if required
        if tool.requires_approval() {
            if let Some(callback) = &self.approval_callback {
                let approved = callback
                    .request_approval(tool_name.to_string(), params.clone(), context.clone())
                    .await;

                if !approved {
                    warn!(
                        request_id = %context.request_id,
                        tool = tool_name,
                        "Tool execution denied by approval callback"
                    );
                    return Err(ToolError::ApprovalDenied);
                }

                info!(
                    request_id = %context.request_id,
                    tool = tool_name,
                    "Tool execution approved"
                );
            }
        }

        // Execute with timeout
        let result = match timeout(self.default_timeout, tool.execute(params)).await {
            Ok(Ok(output)) => {
                let elapsed = start.elapsed().as_millis() as u64;
                debug!(
                    request_id = %context.request_id,
                    tool = tool_name,
                    elapsed_ms = elapsed,
                    success = output.success,
                    "Tool execution completed"
                );
                Ok(output)
            }
            Ok(Err(e)) => {
                error!(
                    request_id = %context.request_id,
                    tool = tool_name,
                    error = %e,
                    "Tool execution failed"
                );
                Err(e)
            }
            Err(_) => {
                error!(
                    request_id = %context.request_id,
                    tool = tool_name,
                    timeout_secs = self.default_timeout.as_secs(),
                    "Tool execution timed out"
                );
                Err(ToolError::Timeout(self.default_timeout.as_secs()))
            }
        };

        result
    }

    /// Execute multiple tools in sequence
    pub async fn execute_sequence(
        &self,
        calls: Vec<(String, Value)>,
        context: &ExecutionContext,
    ) -> Vec<super::Result<ToolOutput>> {
        let mut results = Vec::with_capacity(calls.len());

        for (tool_name, params) in calls {
            let result = self.execute(&tool_name, params, context).await;
            results.push(result);
        }

        results
    }

    /// Validate parameters against tool schema
    async fn validate_params(
        &self,
        tool: &Arc<dyn Tool>,
        params: &Value,
    ) -> Result<(), String> {
        let schema = tool.parameters_schema();

        // Basic validation - check required fields exist
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if params.get(field_name).is_none() {
                        return Err(format!("Missing required field: {}", field_name));
                    }
                }
            }
        }

        // Check that params is an object
        if !params.is_object() {
            return Err("Parameters must be an object".to_string());
        }

        // TODO: Full JSON Schema validation with a library like jsonschema

        Ok(())
    }

    /// Check if a tool requires approval
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.registry
            .get(tool_name)
            .map(|t| t.requires_approval())
            .unwrap_or(false)
    }

    /// Get tool information
    pub fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.registry.get_tool_info(tool_name)
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        let filter = crate::tools::ToolFilter::default();
        self.registry
            .list_filtered(filter)
            .into_iter()
            .map(|(_, info)| info)
            .collect()
    }
}


/// Simple approval callback that logs and auto-approves non-destructive tools
pub struct LoggingApprovalCallback;

impl ApprovalCallback for LoggingApprovalCallback {
    fn request_approval(
        &self,
        tool_name: String,
        params: Value,
        context: ExecutionContext,
    ) -> super::BoxFuture<'static, bool> {
        Box::pin(async move {
            warn!(
                request_id = %context.request_id,
                tool = %tool_name,
                params = %params,
                "Approval requested for tool execution - auto-approving"
            );
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::builtins::EchoTool;

    #[tokio::test]
    async fn test_executor_basic() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let executor = ToolExecutor::new(Arc::new(registry));
        let context = ExecutionContext::new("test-1");

        let result = executor
            .execute("echo", json!({"message": "hello"}), &context)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "hello");
    }

    #[tokio::test]
    async fn test_executor_not_found() {
        let registry = ToolRegistry::new();
        let executor = ToolExecutor::new(Arc::new(registry));
        let context = ExecutionContext::new("test-1");

        let result = executor
            .execute("nonexistent", json!({}), &context)
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
    }

    use serde_json::json;
    use crate::tools::ToolRegistry;
}
