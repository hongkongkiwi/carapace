//! Memory Coordinator
//!
//! Coordinates memory operations including storage, search, and auto-indexing.

use super::{embeddings::EmbeddingProvider, store::SqliteMemoryStore, Memory, MemoryMetadata};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Memory coordinator manages the memory system
pub struct MemoryCoordinator {
    store: Arc<SqliteMemoryStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    config: MemoryConfig,
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Auto-index session transcripts
    pub auto_index_sessions: bool,
    /// Maximum memories to return in search
    pub max_search_results: usize,
    /// Default source name
    pub default_source: String,
    /// Enable memory search tool
    pub enable_search_tool: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            auto_index_sessions: true,
            max_search_results: 5,
            default_source: "memory".to_string(),
            enable_search_tool: true,
        }
    }
}

impl MemoryCoordinator {
    /// Create a new memory coordinator
    pub async fn new(
        store: SqliteMemoryStore,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        config: MemoryConfig,
    ) -> Result<Self, super::MemoryError> {
        Ok(Self {
            store: Arc::new(store),
            embedding_provider,
            config,
        })
    }

    /// Store a memory entry
    pub async fn store(
        &self,
        content: &str,
        metadata: MemoryMetadata,
    ) -> Result<String, super::MemoryError> {
        debug!(content = %content, "Storing memory");

        // Generate embedding
        let embedding = self
            .embedding_provider
            .embed(content)
            .await
            .map_err(|e| super::MemoryError::Embedding(e.to_string()))?;

        // Store in database
        let id = self.store.store(content, embedding, metadata.clone()).await?;

        info!(id = %id, source = %metadata.source, "Memory stored");

        Ok(id)
    }

    /// Search memories by query
    pub async fn search(&self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>, super::MemoryError> {
        let limit = limit.unwrap_or(self.config.max_search_results);

        debug!(query = %query, limit, "Searching memories");

        // Generate query embedding
        let query_embedding = self
            .embedding_provider
            .embed(query)
            .await
            .map_err(|e| super::MemoryError::Embedding(e.to_string()))?;

        // Search in store
        let memories = self.store.search(&query_embedding, limit).await?;

        // Convert to search results with relevance scores
        let results: Vec<SearchResult> = memories
            .into_iter()
            .enumerate()
            .map(|(idx, memory)| {
                // Simple relevance score based on position
                let relevance = 1.0 - (idx as f32 / limit as f32);
                SearchResult {
                    memory,
                    relevance,
                }
            })
            .collect();

        info!(query = %query, count = results.len(), "Memory search completed");

        Ok(results)
    }

    /// Get memories by session ID
    pub async fn get_session_memories(&self,
        session_id: &str,
    ) -> Result<Vec<Memory>, super::MemoryError> {
        self.store.get_by_session(session_id).await
    }

    /// Index a conversation message
    pub async fn index_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<Option<String>, super::MemoryError> {
        if !self.config.auto_index_sessions {
            return Ok(None);
        }

        // Skip empty messages
        if content.trim().is_empty() {
            return Ok(None);
        }

        // Skip system messages
        if role == "system" {
            return Ok(None);
        }

        let metadata = MemoryMetadata {
            source: format!("session:{}", session_id),
            session_id: Some(session_id.to_string()),
            tags: vec![role.to_string(), "auto-indexed".to_string()],
        };

        let id = self.store(content, metadata).await?;

        Ok(Some(id))
    }

    /// Cleanup old memories
    pub async fn cleanup(&self, older_than_days: i64) -> Result<u64, super::MemoryError> {
        let deleted = self.store.cleanup(older_than_days).await?;
        info!(deleted, "Cleaned up old memories");
        Ok(deleted)
    }
}

/// Memory search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub memory: Memory,
    pub relevance: f32,
}

/// Global memory coordinator instance
static GLOBAL_COORDINATOR: std::sync::OnceLock<Arc<RwLock<Option<MemoryCoordinator>>>> =
    std::sync::OnceLock::new();

/// Initialize global memory coordinator
pub async fn init_global(
    coordinator: MemoryCoordinator) {
    let _ = GLOBAL_COORDINATOR.set(Arc::new(RwLock::new(Some(coordinator))));
}

/// Get global memory coordinator
pub fn global() -> Option<Arc<RwLock<Option<MemoryCoordinator>>>> {
    GLOBAL_COORDINATOR.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::embeddings::MockEmbeddingProvider;

    #[tokio::test]
    async fn test_memory_coordinator() {
        let store = SqliteMemoryStore::new(":memory:", 1536).await.unwrap();
        let provider = Arc::new(MockEmbeddingProvider::new(1536));
        let coordinator = MemoryCoordinator::new(
            store,
            provider,
            MemoryConfig::default(),
        )
        .await
        .unwrap();

        // Store a memory
        let metadata = MemoryMetadata {
            source: "test".to_string(),
            session_id: Some("session-1".to_string()),
            tags: vec![],
        };

        let id = coordinator.store("Test memory content", metadata).await.unwrap();
        assert!(!id.is_empty());

        // Search for it
        let results = coordinator.search("test", Some(5)).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.content, "Test memory content");
    }

    #[tokio::test]
    async fn test_index_message() {
        let store = SqliteMemoryStore::new(":memory:", 1536).await.unwrap();
        let provider = Arc::new(MockEmbeddingProvider::new(1536));
        let coordinator = MemoryCoordinator::new(
            store,
            provider,
            MemoryConfig::default(),
        )
        .await
        .unwrap();

        // Index a user message
        let id = coordinator
            .index_message("session-1", "user", "Hello, world!")
            .await
            .unwrap();
        assert!(id.is_some());

        // System messages should be skipped
        let id = coordinator
            .index_message("session-1", "system", "You are an assistant")
            .await
            .unwrap();
        assert!(id.is_none());
    }
}
