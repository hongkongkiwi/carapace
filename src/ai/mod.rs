//! AI Provider Abstractions
//!
//! Provides a unified interface for interacting with various AI/LLM providers
//! including OpenAI, Anthropic, Google Gemini, and others.

pub mod providers;
pub mod types;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

pub use types::*;

/// Errors that can occur when interacting with AI providers
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Rate limit exceeded. Retry after: {retry_after}s")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Timeout")]
    Timeout,
}

/// Result type for AI operations
pub type Result<T> = std::result::Result<T, AiError>;

/// Boxed future type for async trait methods
pub type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Trait for AI provider implementations
///
/// Note: This trait uses boxed futures to maintain object safety.
/// Use the ProviderRegistry to work with providers dynamically.
pub trait AiProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available (has valid credentials)
    fn is_available(&self) -> BoxFuture<'_, Result<bool>>;

    /// Send a chat completion request
    fn chat(&self, request: ChatRequest) -> BoxFuture<'_, Result<ChatResponse>>;

    /// Get available models from this provider
    fn list_models(&self) -> BoxFuture<'_, Result<Vec<ModelInfo>>>;

    /// Check if this provider supports tool calling
    fn supports_tools(&self) -> bool;

    /// Check if this provider supports vision/image input
    fn supports_vision(&self) -> bool;

    /// Get the default model for this provider
    fn default_model(&self) -> &str;

    /// Get provider-specific capabilities
    fn capabilities(&self) -> ProviderCapabilities;
}

/// Capabilities of an AI provider
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    /// Maximum context length in tokens
    pub max_context_length: Option<u32>,
    /// Maximum output tokens
    pub max_output_tokens: Option<u32>,
    /// Supports streaming responses
    pub supports_streaming: bool,
    /// Supports function/tool calling
    pub supports_tools: bool,
    /// Supports vision/image input
    pub supports_vision: bool,
    /// Supports JSON mode
    pub supports_json_mode: bool,
    /// Supports system messages
    pub supports_system_messages: bool,
}

/// Registry of AI providers
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AiProvider>>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    /// Create a new empty provider registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    /// Register a provider
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn AiProvider>) {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn AiProvider>> {
        self.providers.get(name).cloned()
    }

    /// Get the default provider
    pub fn default_provider(&self) -> Option<Arc<dyn AiProvider>> {
        self.default_provider
            .as_ref()
            .and_then(|name| self.providers.get(name).cloned())
    }

    /// Set the default provider
    pub fn set_default(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if !self.providers.contains_key(&name) {
            return Err(AiError::ProviderNotFound(name));
        }
        self.default_provider = Some(name);
        Ok(())
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a provider is registered
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for an AI provider
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProviderConfig {
    /// Provider type (openai, anthropic, etc.)
    pub provider: String,
    /// API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Base URL for API requests (optional, for custom endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Default model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    /// Organization ID (for OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

fn default_timeout() -> u64 {
    60
}

fn default_retries() -> u32 {
    3
}

impl ProviderConfig {
    /// Create a new provider config
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            api_key: None,
            base_url: None,
            default_model: None,
            timeout_seconds: default_timeout(),
            max_retries: default_retries(),
            organization: None,
        }
    }

    /// Set the API key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }
}

/// Builder for chat requests
pub struct ChatRequestBuilder {
    request: ChatRequest,
}

impl ChatRequestBuilder {
    /// Create a new builder with a model
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            request: ChatRequest {
                model: model.into(),
                messages: Vec::new(),
                temperature: None,
                max_tokens: None,
                top_p: None,
                tools: None,
                tool_choice: None,
                stream: false,
                response_format: None,
            },
        }
    }

    /// Add a system message
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(Message {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            name: None,
        });
        self
    }

    /// Add a user message
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(Message {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            name: None,
        });
        self
    }

    /// Add an assistant message
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.request.messages.push(Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            name: None,
        });
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.request.temperature = Some(temp);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.request.max_tokens = Some(tokens);
        self
    }

    /// Enable streaming
    pub fn stream(mut self) -> Self {
        self.request.stream = true;
        self
    }

    /// Build the request
    pub fn build(self) -> ChatRequest {
        self.request
    }
}

/// Parse a model string in the format "provider:model"
pub fn parse_model_string(model: &str) -> (Option<&str>, &str) {
    match model.find(':') {
        Some(idx) => (Some(&model[..idx]), &model[idx + 1..]),
        None => (None, model),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_string() {
        assert_eq!(
            parse_model_string("openai:gpt-4"),
            (Some("openai"), "gpt-4")
        );
        assert_eq!(parse_model_string("gpt-4"), (None, "gpt-4"));
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequestBuilder::new("gpt-4")
            .system("You are a helpful assistant")
            .user("Hello!")
            .temperature(0.7)
            .max_tokens(100)
            .build();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("openai")
            .with_api_key("sk-test")
            .with_default_model("gpt-4");

        assert_eq!(config.provider, "openai");
        assert_eq!(config.api_key, Some("sk-test".to_string()));
    }
}
