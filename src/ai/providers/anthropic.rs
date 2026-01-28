//! Anthropic Provider Implementation
//!
//! Implements the AiProvider trait for Anthropic's Claude API.

use crate::ai::{
    AiError, AiProvider, BoxFuture, ChatRequest, ChatResponse, ModelInfo, ProviderCapabilities,
    ProviderConfig, Result,
};

/// Anthropic Claude provider
pub struct AnthropicProvider {
    config: ProviderConfig,
    _client: reqwest::Client,
    _headers: reqwest::header::HeaderMap,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AiError::AuthenticationError("API key required".to_string()))?;

        let client = super::build_client(config.timeout_seconds)?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            reqwest::header::HeaderValue::from_str(api_key)
                .map_err(|e| AiError::InvalidRequest(format!("Invalid API key: {}", e)))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static("2023-06-01"),
        );

        Ok(Self {
            config,
            _client: client,
            _headers: headers,
        })
    }

    /// Get the base URL for API requests
    fn base_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string())
    }
}

impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn is_available(&self) -> BoxFuture<'_, Result<bool>> {
        Box::pin(async move {
            match self.list_models().await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        })
    }

    fn chat(&self, _request: ChatRequest) -> BoxFuture<'_, Result<ChatResponse>> {
        Box::pin(async move {
            // TODO: Implement Anthropic API transformation
            Err(AiError::ProviderError(
                "Anthropic provider not yet implemented".to_string(),
            ))
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<ModelInfo>>> {
        Box::pin(async move {
            // Anthropic doesn't have a models endpoint, return known models
            Ok(vec![
                ModelInfo {
                    id: "claude-3-opus-20240229".to_string(),
                    name: "Claude 3 Opus".to_string(),
                    provider: "anthropic".to_string(),
                    context_window: Some(200_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.015, 0.075)),
                },
                ModelInfo {
                    id: "claude-3-sonnet-20240229".to_string(),
                    name: "Claude 3 Sonnet".to_string(),
                    provider: "anthropic".to_string(),
                    context_window: Some(200_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.003, 0.015)),
                },
                ModelInfo {
                    id: "claude-3-haiku-20240307".to_string(),
                    name: "Claude 3 Haiku".to_string(),
                    provider: "anthropic".to_string(),
                    context_window: Some(200_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.00025, 0.00125)),
                },
            ])
        })
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }

    fn default_model(&self) -> &str {
        self.config
            .default_model
            .as_deref()
            .unwrap_or("claude-3-sonnet-20240229")
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_context_length: Some(200_000),
            max_output_tokens: Some(4096),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_json_mode: false,
            supports_system_messages: true,
        }
    }
}
