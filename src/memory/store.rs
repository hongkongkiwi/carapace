//! Memory Store
//!
//! SQLite-backed vector storage using sqlite-vec.

use super::{Memory, MemoryMetadata, MemoryError, MemoryStore};

/// SQLite vector store
pub struct SqliteMemoryStore {
    #[allow(dead_code)]
    db_path: String,
}

impl SqliteMemoryStore {
    /// Create a new store
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    /// Initialize the database
    pub async fn init(&self) -> Result<(), MemoryError> {
        // TODO: Initialize sqlite-vec tables
        Ok(())
    }
}

#[async_trait::async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn store(
        &self,
        _content: &str,
        _embedding: Vec<f32>,
        _metadata: MemoryMetadata,
    ) -> Result<String, MemoryError> {
        // TODO: Store in SQLite
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn search(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<Memory>, MemoryError> {
        // TODO: Search using sqlite-vec
        Ok(Vec::new())
    }
}
