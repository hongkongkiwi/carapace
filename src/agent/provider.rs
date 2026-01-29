//! LLM provider trait and common types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::agent::AgentError;

/// A streaming event from the LLM.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Incremental text output.
    TextDelta { text: String },

    /// The model wants to call a tool.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// The model finished its turn.
    Stop {
        reason: StopReason,
        usage: TokenUsage,
    },

    /// Unrecoverable error from the provider.
    Error { message: String },
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}

/// Token counts for a single LLM response.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// A request to the LLM.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub system: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
}

/// A message in the LLM conversation.
#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: Vec<ContentBlock>,
}

/// Role of a message in the LLM conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmRole {
    User,
    Assistant,
}

/// A content block within a message.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

/// A tool definition for the LLM.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Trait for LLM providers (Anthropic, OpenAI, etc.).
///
/// Implementations send a completion request and return a channel that
/// yields streaming events until the model stops or errors.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<mpsc::Receiver<StreamEvent>, AgentError>;
}

/// A provider that dispatches to Anthropic, OpenAI, or Ollama based on the
/// model identifier in the request.
///
/// This allows the system to hold a single `Arc<dyn LlmProvider>` while
/// supporting multiple backend providers transparently.
pub struct MultiProvider {
    anthropic: Option<std::sync::Arc<dyn LlmProvider>>,
    openai: Option<std::sync::Arc<dyn LlmProvider>>,
    ollama: Option<std::sync::Arc<dyn LlmProvider>>,
}

impl std::fmt::Debug for MultiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiProvider")
            .field("anthropic", &self.anthropic.is_some())
            .field("openai", &self.openai.is_some())
            .field("ollama", &self.ollama.is_some())
            .finish()
    }
}

impl MultiProvider {
    /// Create a new multi-provider dispatcher.
    ///
    /// At least one provider should be configured; otherwise all requests
    /// will fail.
    pub fn new(
        anthropic: Option<std::sync::Arc<dyn LlmProvider>>,
        openai: Option<std::sync::Arc<dyn LlmProvider>>,
    ) -> Self {
        Self {
            anthropic,
            openai,
            ollama: None,
        }
    }

    /// Set the Ollama provider for local model inference.
    pub fn with_ollama(mut self, ollama: Option<std::sync::Arc<dyn LlmProvider>>) -> Self {
        self.ollama = ollama;
        self
    }

    /// Returns `true` if at least one provider is configured.
    pub fn has_any_provider(&self) -> bool {
        self.anthropic.is_some() || self.openai.is_some() || self.ollama.is_some()
    }

    /// Select the appropriate backend provider for the given model.
    ///
    /// Dispatch order:
    /// 1. Models prefixed with `ollama:` or `ollama/` -> Ollama
    /// 2. Models matching OpenAI patterns (gpt-*, o1-*, etc.) -> OpenAI
    /// 3. Everything else -> Anthropic (default)
    fn select_provider(&self, model: &str) -> Result<&dyn LlmProvider, AgentError> {
        if crate::agent::ollama::is_ollama_model(model) {
            self.ollama.as_deref().ok_or_else(|| {
                AgentError::Provider(format!(
                    "model \"{model}\" requires Ollama provider, but Ollama is not configured"
                ))
            })
        } else if crate::agent::openai::is_openai_model(model) {
            self.openai.as_deref().ok_or_else(|| {
                AgentError::Provider(format!(
                    "model \"{model}\" requires OpenAI provider, but no OPENAI_API_KEY is configured"
                ))
            })
        } else {
            // Default to Anthropic for claude-* and unknown models
            self.anthropic.as_deref().ok_or_else(|| {
                AgentError::Provider(format!(
                    "model \"{model}\" requires Anthropic provider, but no ANTHROPIC_API_KEY is configured"
                ))
            })
        }
    }
}

#[async_trait]
impl LlmProvider for MultiProvider {
    async fn complete(
        &self,
        mut request: CompletionRequest,
    ) -> Result<mpsc::Receiver<StreamEvent>, AgentError> {
        let provider = self.select_provider(&request.model)?;

        // Strip the ollama: or ollama/ prefix before forwarding to the provider,
        // so the Ollama server receives the bare model name (e.g. "llama3").
        if crate::agent::ollama::is_ollama_model(&request.model) {
            request.model = crate::agent::ollama::strip_ollama_prefix(&request.model).to_string();
        }

        provider.complete(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_provider_has_any_provider() {
        let empty = MultiProvider::new(None, None);
        assert!(!empty.has_any_provider());

        // We can't easily create real providers without API keys, but we can
        // test the logic with the struct fields directly.
    }

    #[test]
    fn test_multi_provider_has_any_provider_with_ollama() {
        let provider = MultiProvider::new(None, None);
        assert!(!provider.has_any_provider());

        // With ollama set, has_any_provider should return true
        let ollama = crate::agent::ollama::OllamaProvider::new().unwrap();
        let provider =
            MultiProvider::new(None, None).with_ollama(Some(std::sync::Arc::new(ollama)));
        assert!(provider.has_any_provider());
    }

    #[test]
    fn test_multi_provider_select_anthropic_model() {
        let provider = MultiProvider::new(None, None);
        let err = provider.select_provider("claude-sonnet-4-20250514");
        assert!(err.is_err());
        let msg = match err {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(
            msg.contains("Anthropic"),
            "expected Anthropic in error: {msg}"
        );
    }

    #[test]
    fn test_multi_provider_select_openai_model() {
        let provider = MultiProvider::new(None, None);
        let err = provider.select_provider("gpt-4o");
        assert!(err.is_err());
        let msg = match err {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(msg.contains("OpenAI"), "expected OpenAI in error: {msg}");
    }

    #[test]
    fn test_multi_provider_select_ollama_model_colon() {
        let provider = MultiProvider::new(None, None);
        let err = provider.select_provider("ollama:llama3");
        assert!(err.is_err());
        let msg = match err {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(msg.contains("Ollama"), "expected Ollama in error: {msg}");
    }

    #[test]
    fn test_multi_provider_select_ollama_model_slash() {
        let provider = MultiProvider::new(None, None);
        let err = provider.select_provider("ollama/mistral");
        assert!(err.is_err());
        let msg = match err {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(msg.contains("Ollama"), "expected Ollama in error: {msg}");
    }

    #[test]
    fn test_multi_provider_ollama_dispatch_succeeds_when_configured() {
        let ollama = crate::agent::ollama::OllamaProvider::new().unwrap();
        let provider =
            MultiProvider::new(None, None).with_ollama(Some(std::sync::Arc::new(ollama)));
        // Should succeed (return Ok) when Ollama is configured
        let result = provider.select_provider("ollama:llama3");
        assert!(result.is_ok(), "expected Ok when Ollama is configured");
    }

    #[test]
    fn test_multi_provider_debug_includes_ollama() {
        let provider = MultiProvider::new(None, None);
        let debug = format!("{:?}", provider);
        assert!(
            debug.contains("ollama"),
            "debug output should include ollama: {debug}"
        );
    }
}
