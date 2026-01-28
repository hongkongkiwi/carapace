//! Memory Indexer
//!
//! Automatic indexing of conversations and files into memory.

use super::{MemoryMetadata, MemoryStore, MemoryError};
use crate::agent::AgentSession;
use crate::memory::embeddings::EmbeddingProvider;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Memory indexer for automatic content indexing
pub struct MemoryIndexer {
    store: Arc<dyn MemoryStore>,
    embedder: Arc<dyn EmbeddingProvider>,
    config: IndexerConfig,
}

/// Indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Whether auto-indexing is enabled
    pub enabled: bool,
    /// Minimum message length to index
    pub min_message_length: usize,
    /// Index on every N messages
    pub index_interval: usize,
    /// Maximum chunks per session
    pub max_chunks: usize,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_message_length: 20,
            index_interval: 5,
            max_chunks: 100,
        }
    }
}

impl MemoryIndexer {
    /// Create new indexer
    pub fn new(
        store: Arc<dyn MemoryStore>,
        embedder: Arc<dyn EmbeddingProvider>,
        config: IndexerConfig,
    ) -> Self {
        Self {
            store,
            embedder,
            config,
        }
    }

    /// Index a session
    pub async fn index_session(
        &self,
        session: &AgentSession,
    ) -> Result<usize, MemoryError> {
        if !self.config.enabled {
            return Ok(0);
        }

        let messages: Vec<String> = session
            .messages()
            .iter()
            .filter(|m| m.content.as_text().map(|t| t.len()).unwrap_or(0) >= self.config.min_message_length)
            .filter_map(|m| m.content.as_text().map(|s| s.to_string()))
            .collect();

        let mut indexed = 0;
        for content in messages {
            let embedding = self.embedder.embed(&content).await.map_err(|e| {
                MemoryError::Embedding(e.to_string())
            })?;

            let metadata = MemoryMetadata {
                source: "session".to_string(),
                session_id: Some(session.id().to_string()),
                tags: vec!["conversation".to_string()],
            };

            self.store.store(&content, embedding, metadata).await?;
            indexed += 1;
        }

        info!(indexed = indexed, session_id = %session.id(), "Indexed session");
        Ok(indexed)
    }

    /// Search memories
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<super::Memory>, MemoryError> {
        let embedding = self.embedder.embed(query).await.map_err(|e| {
            MemoryError::Embedding(e.to_string())
        })?;

        self.store.search(&embedding, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::SqliteMemoryStore;
    use crate::memory::embeddings::MockEmbeddingProvider;
    use crate::ai::MessageRole;

    #[tokio::test]
    async fn test_indexer() {
        let store = Arc::new(SqliteMemoryStore::new(":memory:"));
        let embedder = Arc::new(MockEmbeddingProvider::new(384));
        let indexer = MemoryIndexer::new(store, embedder, IndexerConfig::default());

        let mut session = AgentSession::new("test");
        session.add_message(MessageRole::User, "Hello, this is a test message that is long enough to index.");

        let indexed = indexer.index_session(&session).await.unwrap();
        assert!(indexed > 0);
    }
}
