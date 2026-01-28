//! Embedding Providers
//!
//! Text embedding generation for memory storage.

/// Embedding provider trait
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for text
    async fn embed(&self,
        text: &str,
    ) -> Result<Vec<f32>, EmbeddingError>;

    /// Get embedding dimensions
    fn dimensions(&self) -> usize;
}

/// Embedding errors
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Rate limited")]
    RateLimited,
}

/// OpenAI embedding provider
pub struct OpenAiEmbeddingProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiEmbeddingProvider {
    /// Create new provider
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed(&self,
        _text: &str,
    ) -> Result<Vec<f32>, EmbeddingError> {
        // TODO: Implement OpenAI API call
        Ok(vec![0.0; 1536])
    }

    fn dimensions(&self) -> usize {
        1536
    }
}
