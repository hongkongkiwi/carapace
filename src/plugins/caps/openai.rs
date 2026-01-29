//! OpenAI Tool Plugin
//!
//! Native implementation of OpenAI API tools for carapace.
//! Supports chat completions, embeddings, and image generation.
//!
//! Security: API key is retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// OpenAI tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// API key (retrieved from credential store, not config)
    #[serde(skip)]
    pub api_key: Option<String>,

    /// Default model for chat completions
    #[serde(default = "default_chat_model")]
    pub chat_model: String,

    /// Default model for embeddings
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Default model for image generation
    #[serde(default = "default_image_model")]
    pub image_model: String,

    /// Maximum tokens for completions
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Default temperature (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Organization ID (optional)
    #[serde(default)]
    pub organization: Option<String>,

    /// Base URL for API (useful for proxies)
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_chat_model() -> String {
    "gpt-4o".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_image_model() -> String {
    "dall-e-3".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_timeout() -> u64 {
    60
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            chat_model: default_chat_model(),
            embedding_model: default_embedding_model(),
            image_model: default_image_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            organization: None,
            base_url: default_base_url(),
            timeout_seconds: default_timeout(),
        }
    }
}

/// OpenAI API client for native use
#[derive(Debug, Clone)]
pub struct OpenAIClient {
    config: OpenAIConfig,
    http_client: reqwest::blocking::Client,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub fn new(config: OpenAIConfig) -> Result<Self, BindingError> {
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
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| BindingError::CallError("OpenAI API key not configured".to_string()))?;

        let mut headers = vec![(
            "Authorization".to_string(),
            format!("Bearer {}", api_key),
        )];

        if let Some(org) = &self.config.organization {
            headers.push(("OpenAI-Organization".to_string(), org.clone()));
        }

        Ok(headers)
    }

    // ============ Chat Completions ============

    /// Create a chat completion
    pub fn chat_completions(
        &self,
        messages: &[ChatMessage],
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatCompletionResponse, BindingError> {
        let model = model.unwrap_or(&self.config.chat_model);
        let temperature = temperature.unwrap_or(self.config.temperature);
        let max_tokens = max_tokens.unwrap_or(self.config.max_tokens);

        let body = json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": false
        });

        let response: serde_json::Value = self.api_request_sync("/chat/completions", &body)?;
        response.try_into().map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    // ============ Embeddings ============

    /// Create embeddings for input text
    pub fn create_embeddings(
        &self,
        input: &[String],
        model: Option<&str>,
    ) -> Result<EmbeddingResponse, BindingError> {
        let model = model.unwrap_or(&self.config.embedding_model);

        let body = json!({
            "model": model,
            "input": input
        });

        let response: serde_json::Value = self.api_request_sync("/embeddings", &body)?;
        response.try_into().map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    // ============ Images ============

    /// Generate images
    pub fn generate_images(
        &self,
        prompt: &str,
        model: Option<&str>,
        size: Option<&str>,
        quality: Option<&str>,
        n: Option<u32>,
    ) -> Result<ImageResponse, BindingError> {
        let model = model.unwrap_or(&self.config.image_model);
        let n = n.unwrap_or(1);

        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "n": n,
        });

        if let Some(size) = size {
            body["size"] = serde_json::Value::String(size.to_string());
        }

        if let Some(quality) = quality {
            body["quality"] = serde_json::Value::String(quality.to_string());
        }

        let response: serde_json::Value = self.api_request_sync("/images/generations", &body)?;
        response.try_into().map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    /// Make a synchronous API request using blocking API
    fn api_request_sync(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value, BindingError> {
        let url = format!("{}{}", self.config.base_url, endpoint);

        let mut request = self.http_client.post(&url);

        for (key, value) in self.auth_headers()? {
            request = request.header(key, value);
        }

        request = request.header("Content-Type", "application/json").json(body);

        let response = request.send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(BindingError::CallError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        response.json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }
}

// ============ Tool Plugin Implementation ============

/// OpenAI tool plugin for carapace
#[derive(Debug)]
pub struct OpenAITool {
    /// OpenAI client
    client: Option<OpenAIClient>,
}

impl OpenAITool {
    /// Create a new OpenAI tool
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Initialize the client with API key
    pub fn initialize(&mut self, config: OpenAIConfig) -> Result<(), BindingError> {
        let client = OpenAIClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for OpenAITool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for OpenAITool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "openai_chat_completion".to_string(),
                description: "Send a chat completion request to OpenAI. Supports GPT-4o, GPT-4, GPT-3.5 Turbo and other models.".to_string(),
                input_schema: ChatCompletionInput::schema().to_string(),
            },
            ToolDefinition {
                name: "openai_create_embeddings".to_string(),
                description: "Create vector embeddings for text using OpenAI's embedding models (text-embedding-3-small, text-embedding-3-large, etc.).".to_string(),
                input_schema: EmbeddingInput::schema().to_string(),
            },
            ToolDefinition {
                name: "openai_generate_image".to_string(),
                description: "Generate images using OpenAI's DALL-E models (DALL-E 3, DALL-E 2).".to_string(),
                input_schema: ImageGenerationInput::schema().to_string(),
            },
        ])
    }

    fn invoke(
        &self,
        name: &str,
        params: &str,
        _ctx: ToolContext,
    ) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("OpenAI tool not initialized".to_string()))?;

        match name {
            "openai_chat_completion" => {
                let input: ChatCompletionInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let messages: Vec<ChatMessage> = input.messages.iter().map(|m| m.clone().into()).collect();

                let response = client.chat_completions(
                    &messages,
                    input.model.as_deref(),
                    input.temperature,
                    input.max_tokens,
                ).map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&response).map_err(|e| BindingError::CallError(e.to_string()))?),
                    error: None,
                })
            }
            "openai_create_embeddings" => {
                let input: EmbeddingInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let response = client.create_embeddings(
                    &input.input,
                    input.model.as_deref(),
                ).map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&response).map_err(|e| BindingError::CallError(e.to_string()))?),
                    error: None,
                })
            }
            "openai_generate_image" => {
                let input: ImageGenerationInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let response = client.generate_images(
                    &input.prompt,
                    input.model.as_deref(),
                    input.size.as_deref(),
                    input.quality.as_deref(),
                    input.n,
                ).map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&response).map_err(|e| BindingError::CallError(e.to_string()))?),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

// ============ Input/Output Types ============

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatMessageRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "function")]
    Function,
}

/// Chat message for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageInput {
    pub role: ChatMessageRole,
    pub content: String,
    #[serde(default)]
    pub name: Option<String>,
}

impl From<ChatMessageInput> for ChatMessage {
    fn from(input: ChatMessageInput) -> Self {
        ChatMessage {
            role: input.role,
            content: input.content,
            name: input.name,
        }
    }
}

/// Chat message (API format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatMessageRole,
    pub content: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Input for chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionInput {
    #[serde(default)]
    pub messages: Vec<ChatMessageInput>,

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
                                "enum": ["system", "user", "assistant", "tool", "function"]
                            },
                            "content": {
                                "type": "string"
                            },
                            "name": {
                                "type": "string"
                            }
                        },
                        "required": ["role", "content"]
                    },
                    "description": "Messages to send to the model"
                },
                "model": {
                    "type": "string",
                    "description": "Model to use (e.g., gpt-4o, gpt-4, gpt-3.5-turbo)"
                },
                "temperature": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 2,
                    "description": "Temperature for sampling (0-2)"
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

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Input for embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingInput {
    pub input: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl EmbeddingInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Texts to embed"
                },
                "model": {
                    "type": "string",
                    "description": "Embedding model (e.g., text-embedding-3-small, text-embedding-3-large)"
                }
            },
            "required": ["input"]
        })
    }
}

/// Embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

/// Input for image generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationInput {
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub n: Option<u32>,
}

impl ImageGenerationInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Description of the desired image"
                },
                "model": {
                    "type": "string",
                    "description": "Image model (e.g., dall-e-3, dall-e-2)"
                },
                "size": {
                    "type": "string",
                    "description": "Image size (1024x1024, 1792x1024, 1024x1792 for DALL-E 3)"
                },
                "quality": {
                    "type": "string",
                    "description": "Image quality (standard, hd for DALL-E 3)"
                },
                "n": {
                    "type": "integer",
                    "description": "Number of images to generate (1-10 for DALL-E 2)"
                }
            },
            "required": ["prompt"]
        })
    }
}

/// Image generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub created: u64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub b64_json: Option<String>,
    pub url: Option<String>,
    pub revised_prompt: Option<String>,
}

// TryFrom implementations for response parsing

impl TryFrom<serde_json::Value> for ChatCompletionResponse {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for EmbeddingResponse {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for ImageResponse {
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
    fn test_embedding_input_schema() {
        let schema = EmbeddingInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_image_generation_input_schema() {
        let schema = ImageGenerationInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_openai_config_defaults() {
        let config = OpenAIConfig::default();
        assert_eq!(config.chat_model, "gpt-4o");
        assert_eq!(config.embedding_model, "text-embedding-3-small");
        assert_eq!(config.image_model, "dall-e-3");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.temperature, 0.7);
    }
}
