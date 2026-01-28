//! Built-in Tools
//!
//! Default tools available in every Carapace installation.

pub mod bash;

pub use bash::BashTool;

use super::{Tool, ToolOutput};
use crate::tools::ToolError;
use serde_json::json;

/// Echo tool - repeats back the input
pub struct EchoTool;

impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo back the input message. Useful for testing the tool system."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn execute(
        &self,
        params: serde_json::Value,
    ) -> super::BoxFuture<'_, crate::tools::Result<ToolOutput>> {
        Box::pin(async move {
            let message = params
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("message required".to_string()))?;

            Ok(ToolOutput::success(message))
        })
    }
}

/// Time tool - returns current time information
pub struct TimeTool;

impl Tool for TimeTool {
    fn name(&self) -> &str {
        "time"
    }

    fn description(&self) -> &str {
        "Get the current date and time information"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "description": "Optional format (iso, rfc2822, or custom strftime)",
                    "enum": ["iso", "rfc2822", "unix"]
                },
                "timezone": {
                    "type": "string",
                    "description": "Optional timezone (e.g., 'UTC', 'America/New_York')"
                }
            }
        })
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn execute(
        &self,
        params: serde_json::Value,
    ) -> super::BoxFuture<'_, crate::tools::Result<ToolOutput>> {
        Box::pin(async move {
            use chrono::{Local, Utc};

            let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("iso");
            let timezone = params.get("timezone").and_then(|v| v.as_str());

            let result = match format {
                "iso" => {
                    if timezone == Some("UTC") {
                        Utc::now().to_rfc3339()
                    } else {
                        Local::now().to_rfc3339()
                    }
                }
                "rfc2822" => {
                    if timezone == Some("UTC") {
                        Utc::now().to_rfc2822()
                    } else {
                        Local::now().to_rfc2822()
                    }
                }
                "unix" => {
                    if timezone == Some("UTC") {
                        Utc::now().timestamp().to_string()
                    } else {
                        Local::now().timestamp().to_string()
                    }
                }
                _ => {
                    if timezone == Some("UTC") {
                        Utc::now().format(format).to_string()
                    } else {
                        Local::now().format(format).to_string()
                    }
                }
            };

            Ok(ToolOutput::success(result))
        })
    }
}

/// UUID tool - generates UUIDs
pub struct UuidTool;

impl Tool for UuidTool {
    fn name(&self) -> &str {
        "uuid"
    }

    fn description(&self) -> &str {
        "Generate a new UUID (Universally Unique Identifier)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "version": {
                    "type": "string",
                    "description": "UUID version to generate",
                    "enum": ["v4", "v7"],
                    "default": "v4"
                },
                "format": {
                    "type": "string",
                    "description": "Output format",
                    "enum": ["standard", "simple", "urn"],
                    "default": "standard"
                }
            }
        })
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn execute(
        &self,
        params: serde_json::Value,
    ) -> super::BoxFuture<'_, crate::tools::Result<ToolOutput>> {
        Box::pin(async move {
            let version = params.get("version").and_then(|v| v.as_str()).unwrap_or("v4");
            let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("standard");

            let uuid = match version {
                "v4" => uuid::Uuid::new_v4(),
                "v7" => uuid::Uuid::now_v7(),
                _ => uuid::Uuid::new_v4(),
            };

            let result = match format {
                "simple" => uuid.simple().to_string(),
                "urn" => uuid.urn().to_string(),
                _ => uuid.to_string(),
            };

            Ok(ToolOutput::success(result))
        })
    }
}

/// System info tool - returns system information
pub struct SystemInfoTool;

impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "Get system information including OS, architecture, and hostname"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "detailed": {
                    "type": "boolean",
                    "description": "Include detailed system information",
                    "default": false
                }
            }
        })
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn execute(
        &self,
        params: serde_json::Value,
    ) -> super::BoxFuture<'_, crate::tools::Result<ToolOutput>> {
        Box::pin(async move {
            let detailed = params.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

            let info = if detailed {
                json!({
                    "os": std::env::consts::OS,
                    "family": std::env::consts::FAMILY,
                    "arch": std::env::consts::ARCH,
                    "hostname": hostname::get()
                        .ok()
                        .and_then(|h| h.into_string().ok())
                        .unwrap_or_default(),
                    "num_cpus": num_cpus::get(),
                    "pid": std::process::id(),
                })
            } else {
                json!({
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                })
            };

            Ok(ToolOutput::success(info.to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_tool() {
        let tool = EchoTool;
        let params = json!({"message": "hello world"});

        let result = tool.execute(params).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content, "hello world");
    }

    #[tokio::test]
    async fn test_time_tool() {
        let tool = TimeTool;
        let params = json!({"format": "iso"});

        let result = tool.execute(params).await.unwrap();
        assert!(result.success);
        // Should contain a valid ISO timestamp
        assert!(result.content.contains('T'));
    }

    #[tokio::test]
    async fn test_uuid_tool() {
        let tool = UuidTool;
        let params = json!({"version": "v4", "format": "standard"});

        let result = tool.execute(params).await.unwrap();
        assert!(result.success);
        // UUID should be 36 chars with hyphens
        assert_eq!(result.content.len(), 36);
    }

    #[tokio::test]
    async fn test_system_info_tool() {
        let tool = SystemInfoTool;
        let params = json!({"detailed": false});

        let result = tool.execute(params).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("os"));
        assert!(result.content.contains("arch"));
    }
}
