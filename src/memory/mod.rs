//! Memory System
//!
//! Vector database and embeddings for agent memory.

pub mod embeddings;
pub mod store;

pub use embeddings::*;
pub use store::*;

use serde::{Deserialize, Serialize};

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: MemoryMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Memory metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub source: String,
    pub session_id: Option<String>,
    pub tags: Vec<String>,
}

/// Memory storage trait
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self,
        content: &str,
        embedding: Vec<f32>,
        metadata: MemoryMetadata,
    ) -> Result<String, MemoryError>;

    async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<Memory>, MemoryError>;
}

/// Memory errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Embedding error: {0}")]
    Embedding(String),
}
