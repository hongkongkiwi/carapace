//! Anthropic Tool Plugin
//!
//! Native implementation of Anthropic API tools for carapace.
//! Supports Claude chat completions and embeddings.
//!
//! Security: API key is retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Anthropic tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// API key (retrieved from credential store, not config)
    #[serde(skip)]
    pub api_key: Option<String>,

    /// Default model for completions
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens for completions
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Default temperature (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Base URL for API (useful for proxies)
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

fn default_base_url() -> String {
    "https://api.anthropic.com/v1".to_string()
}

fn default_timeout() -> u64 {
    60
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            base_url: default_base_url(),
            timeout_seconds: default_timeout(),
        }
    }
}

/// Anthropic API client for native use
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    config: AnthropicConfig,
    http_client: reqwest::blocking::Client,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(config: AnthropicConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| BindingError::CallError(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Set the API key from credential store
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.config.api_key = Some(api_key);
        self
    }

    /// Get the authorization headers
    fn auth_headers(&self) -> Result<Vec<(String, String)>, BindingError> {
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            BindingError::CallError("Anthropic API key not configured".to_string())
        })?;

        Ok(vec![
            ("x-api-key".to_string(), api_key.clone()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ("content-type".to_string(), "application/json".to_string()),
        ])
    }

    /// Make a synchronous API request using blocking API
    fn api_request(
        &self,
        endpoint: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}{}", self.config.base_url, endpoint);

        let mut request = self.http_client.post(&url);

        for (key, value) in self.auth_headers()? {
            request = request.header(key, value);
        }

        let response = request
            .json(body)
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(BindingError::CallError(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        response
            .json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    // ============ Chat Completions ============

    /// Create a chat completion (Claude messages API)
    pub fn chat_completions(
        &self,
        messages: &[ClaudeMessage],
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ClaudeCompletionResponse, BindingError> {
        let model = model.unwrap_or(&self.config.model);
        let temperature = temperature.unwrap_or(self.config.temperature);
        let max_tokens = max_tokens.unwrap_or(self.config.max_tokens);

        let body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        });

        let response: serde_json::Value = self.api_request("/messages", &body)?;
        response
            .try_into()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    /// Create a chat completion (Legacy beta API)
    pub fn complete(
        &self,
        prompt: &str,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ClaudeCompletionResponse, BindingError> {
        let model = model.unwrap_or(&self.config.model);
        let temperature = temperature.unwrap_or(self.config.temperature);
        let max_tokens = max_tokens.unwrap_or(self.config.max_tokens);

        let body = json!({
            "model": model,
            "prompt": prompt,
            "max_tokens_to_sample": max_tokens,
            "temperature": temperature,
        });

        let response: serde_json::Value = self.api_request("/complete", &body)?;
        response
            .try_into()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }
}

// ============ Tool Plugin Implementation ============

/// Anthropic tool plugin for carapace
#[derive(Debug)]
pub struct AnthropicTool {
    /// Anthropic client
    client: Option<AnthropicClient>,
}

impl AnthropicTool {
    /// Create a new Anthropic tool
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Initialize the client with API key
    pub fn initialize(&mut self, config: AnthropicConfig) -> Result<(), BindingError> {
        let client = AnthropicClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for AnthropicTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for AnthropicTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "anthropic_chat".to_string(),
                description: "Send a chat completion request to Anthropic Claude. Supports Claude Sonnet, Claude Haiku, and Claude Opus models.".to_string(),
                input_schema: ChatCompletionInput::schema().to_string(),
            },
            ToolDefinition {
                name: "anthropic_complete".to_string(),
                description: "Complete text using Anthropic Claude (legacy beta API).".to_string(),
                input_schema: CompleteInput::schema().to_string(),
            },
        ])
    }

    fn invoke(
        &self,
        name: &str,
        params: &str,
        _ctx: ToolContext,
    ) -> Result<ToolResult, BindingError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BindingError::CallError("Anthropic tool not initialized".to_string()))?;

        match name {
            "anthropic_chat" => {
                let input: ChatCompletionInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let messages: Vec<ClaudeMessage> =
                    input.messages.iter().map(|m| m.clone().into()).collect();

                let response = client
                    .chat_completions(
                        &messages,
                        input.model.as_deref(),
                        input.temperature,
                        input.max_tokens,
                    )
                    .map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(
                        serde_json::to_string(&response)
                            .map_err(|e| BindingError::CallError(e.to_string()))?,
                    ),
                    error: None,
                })
            }
            "anthropic_complete" => {
                let input: CompleteInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let response = client
                    .complete(
                        &input.prompt,
                        input.model.as_deref(),
                        input.temperature,
                        input.max_tokens,
                    )
                    .map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(
                        serde_json::to_string(&response)
                            .map_err(|e| BindingError::CallError(e.to_string()))?,
                    ),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

// ============ Input/Output Types ============

/// Claude message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ClaudeMessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

/// Claude message for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessageInput {
    pub role: ClaudeMessageRole,
    pub content: String,
}

impl From<ClaudeMessageInput> for ClaudeMessage {
    fn from(input: ClaudeMessageInput) -> Self {
        ClaudeMessage {
            role: input.role,
            content: input.content,
        }
    }
}

/// Claude message (API format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: ClaudeMessageRole,
    pub content: String,
}

/// Input for chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionInput {
    #[serde(default)]
    pub messages: Vec<ClaudeMessageInput>,

    #[serde(default)]
    pub model: Option<String>,

    #[serde(default)]
    pub temperature: Option<f32>,

    #[serde(default = "default_max_tokens_input")]
    pub max_tokens: Option<u32>,
}

fn default_max_tokens_input() -> Option<u32> {
    Some(4096)
}

impl ChatCompletionInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": {
                                "type": "string",
                                "enum": ["user", "assistant", "system"]
                            },
                            "content": {
                                "type": "string"
                            }
                        },
                        "required": ["role", "content"]
                    },
                    "description": "Messages to send to Claude"
                },
                "model": {
                    "type": "string",
                    "description": "Model to use (e.g., claude-sonnet-4-20250514, claude-opus-4-20250514, claude-haiku-4-20250514)"
                },
                "temperature": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "Temperature for sampling (0-1)"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens in response"
                }
            },
            "required": ["messages"]
        })
    }
}

/// Input for legacy completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteInput {
    pub prompt: String,

    #[serde(default)]
    pub model: Option<String>,

    #[serde(default)]
    pub temperature: Option<f32>,

    #[serde(default = "default_max_tokens_input")]
    pub max_tokens: Option<u32>,
}

impl CompleteInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Prompt to complete"
                },
                "model": {
                    "type": "string",
                    "description": "Model to use (e.g., claude-sonnet-4-20250514)"
                },
                "temperature": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "Temperature for sampling (0-1)"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens to generate"
                }
            },
            "required": ["prompt"]
        })
    }
}

/// Claude completion response (messages API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCompletionResponse {
    pub id: String,
    pub type_: String,
    pub role: Option<String>,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<ClaudeUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeContentBlock {
    pub type_: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// TryFrom implementations for response parsing

impl TryFrom<serde_json::Value> for ClaudeCompletionResponse {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_completion_input_schema() {
        let schema = ChatCompletionInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
        assert!(schema["required"].is_array());
    }

    #[test]
    fn test_complete_input_schema() {
        let schema = CompleteInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_anthropic_config_defaults() {
        let config = AnthropicConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.7);
    }
}
