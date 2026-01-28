//! Bash Tool
//!
//! Execute shell commands with safety controls and approval system.

use crate::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Bash execution tool
pub struct BashTool {
    /// Whether to require approval for all commands
    require_approval: bool,
    /// Command timeout in seconds
    timeout_secs: u64,
    /// Allowed commands (empty = allow all)
    allowed_commands: Vec<String>,
    /// Blocked commands
    blocked_commands: Vec<String>,
}

impl BashTool {
    /// Create new bash tool
    pub fn new() -> Self {
        Self {
            require_approval: true,
            timeout_secs: 30,
            allowed_commands: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".to_string(),
                ":(){ :|:& };:".to_string(),
            ],
        }
    }

    /// Set approval requirement
    pub fn with_approval(mut self, require: bool) -> Self {
        self.require_approval = require;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Check if command requires approval
    fn is_destructive(&self, command: &str) -> bool {
        let destructive_patterns = [
            "rm -rf",
            "dd if=",
            "> /dev/sda",
            "mkfs.",
            "format c:",
            ":(){ :|:& };:",
        ];

        destructive_patterns.iter().any(|p| command.contains(p))
    }

    /// Check if command is allowed
    fn is_allowed(&self, command: &str) -> bool {
        if !self.allowed_commands.is_empty() {
            return self.allowed_commands.iter().any(|c| command.starts_with(c));
        }

        !self.blocked_commands.iter().any(|c| command.contains(c))
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute shell commands. Requires approval for potentially destructive operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            },
            "required": ["command"]
        })
    }

    fn requires_approval(&self) -> bool {
        self.require_approval
    }

    fn category(&self) -> Option<&str> {
        Some("system")
    }

    fn execute(
        &self,
        params: serde_json::Value,
    ) -> crate::tools::BoxFuture<'_, crate::tools::Result<ToolOutput>> {
        Box::pin(async move {
            let command_str = params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("command required".to_string()))?;

            // Security checks
            if !self.is_allowed(command_str) {
                return Err(ToolError::ExecutionError(
                    "Command is not allowed by security policy".to_string()
                ));
            }

            // Build command
            let mut cmd = Command::new("bash");
            cmd.arg("-c").arg(command_str);

            // Set working directory if provided
            if let Some(dir) = params.get("working_dir").and_then(|v| v.as_str()) {
                cmd.current_dir(dir);
            }

            // Configure stdout/stderr
            cmd.stdout(Stdio::piped())
               .stderr(Stdio::piped());

            // Execute with timeout
            let result = timeout(
                Duration::from_secs(self.timeout_secs),
                cmd.output()
            ).await;

            match result {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    let content = if !stdout.is_empty() {
                        stdout.to_string()
                    } else {
                        stderr.to_string()
                    };

                    if output.status.success() {
                        Ok(ToolOutput::success(content))
                    } else {
                        Ok(ToolOutput::error(format!(
                            "Exit code {}: {}",
                            output.status.code().unwrap_or(-1),
                            content
                        )))
                    }
                }
                Ok(Err(e)) => Err(ToolError::IoError(e)),
                Err(_) => Err(ToolError::Timeout(self.timeout_secs)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_creation() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
        assert!(tool.requires_approval());
    }

    #[test]
    fn test_destructive_detection() {
        let tool = BashTool::new();
        assert!(tool.is_destructive("rm -rf /"));
        assert!(tool.is_destructive("dd if=/dev/zero of=/dev/sda"));
        assert!(!tool.is_destructive("ls -la"));
    }

    #[tokio::test]
    async fn test_bash_execution() {
        let tool = BashTool::new();
        let params = json!({"command": "echo hello"});

        let result = tool.execute(params).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.content.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_invalid_command() {
        let tool = BashTool::new();
        let params = json!({"command": "exit 1"});

        let result = tool.execute(params).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.success);
    }
}
