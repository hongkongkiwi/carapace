//! OpenAI Provider Implementation
//!
//! Implements the AiProvider trait for OpenAI's API.

use crate::ai::{
    AiError, AiProvider, BoxFuture, ChatRequest, ChatResponse, ModelInfo, ProviderCapabilities,
    ProviderConfig, Result,
};

/// OpenAI provider
pub struct OpenAiProvider {
    config: ProviderConfig,
    _client: reqwest::Client,
    _headers: reqwest::header::HeaderMap,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| AiError::AuthenticationError("API key required".to_string()))?;

        let client = super::build_client(config.timeout_seconds)?;
        let headers = super::build_headers(api_key, config.organization.as_deref())?;

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
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
    }
}

impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
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
            // TODO: Implement OpenAI API transformation
            Err(AiError::ProviderError(
                "OpenAI provider not yet implemented".to_string(),
            ))
        })
    }

    fn list_models(&self) -> BoxFuture<'_, Result<Vec<ModelInfo>>> {
        Box::pin(async move {
            // Return known OpenAI models
            Ok(vec![
                ModelInfo {
                    id: "gpt-4o".to_string(),
                    name: "GPT-4o".to_string(),
                    provider: "openai".to_string(),
                    context_window: Some(128_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.005, 0.015)),
                },
                ModelInfo {
                    id: "gpt-4o-mini".to_string(),
                    name: "GPT-4o Mini".to_string(),
                    provider: "openai".to_string(),
                    context_window: Some(128_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.00015, 0.0006)),
                },
                ModelInfo {
                    id: "gpt-4-turbo".to_string(),
                    name: "GPT-4 Turbo".to_string(),
                    provider: "openai".to_string(),
                    context_window: Some(128_000),
                    max_output_tokens: Some(4096),
                    supports_vision: true,
                    supports_tools: true,
                    pricing: Some((0.01, 0.03)),
                },
                ModelInfo {
                    id: "gpt-3.5-turbo".to_string(),
                    name: "GPT-3.5 Turbo".to_string(),
                    provider: "openai".to_string(),
                    context_window: Some(16_385),
                    max_output_tokens: Some(4096),
                    supports_vision: false,
                    supports_tools: true,
                    pricing: Some((0.0005, 0.0015)),
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
            .unwrap_or("gpt-4o-mini")
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_context_length: Some(128_000),
            max_output_tokens: Some(4096),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_json_mode: true,
            supports_system_messages: true,
        }
    }
}
