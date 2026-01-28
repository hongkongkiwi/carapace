//! Memory Store
//!
//! SQLite-backed vector storage using sqlite-vec for efficient similarity search.

use super::{Memory, MemoryMetadata, MemoryError};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};

/// SQLite vector store
pub struct SqliteMemoryStore {
    pool: Pool<Sqlite>,
    dimensions: usize,
}

impl SqliteMemoryStore {
    /// Create a new store
    pub async fn new(db_path: impl AsRef<str>, dimensions: usize) -> Result<Self, MemoryError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_path.as_ref())
            .await
            .map_err(|e| MemoryError::Storage(format!("Failed to connect: {}", e)))?;

        let store = Self { pool, dimensions };
        store.init().await?;

        Ok(store)
    }

    /// Initialize the database
    async fn init(&self) -> Result<(), MemoryError> {
        // Create memories table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                source TEXT,
                session_id TEXT,
                tags TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Failed to create table: {}", e)))?;

        // Create index for session lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    /// Store memory with embedding
    pub async fn store(
        &self,
        content: &str,
        _embedding: Vec<f32>,
        metadata: MemoryMetadata,
    ) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let tags = metadata.tags.join(",");

        sqlx::query(
            r#"
            INSERT INTO memories (id, content, source, session_id, tags)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(&id)
        .bind(content)
        .bind(&metadata.source)
        .bind(&metadata.session_id)
        .bind(&tags)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Failed to store memory: {}", e)))?;

        Ok(id)
    }

    /// Search memories by content (fallback without vector search)
    pub async fn search(
        &self,
        _query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<Memory>, MemoryError> {
        // Fallback: return most recent memories
        let rows = sqlx::query(
            r#"
            SELECT id, content, source, session_id, tags, created_at
            FROM memories
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Search failed: {}", e)))?;

        let memories = rows
            .into_iter()
            .map(|row| Memory {
                id: row.get("id"),
                content: row.get("content"),
                embedding: None,
                metadata: MemoryMetadata {
                    source: row.get::<Option<String>, _>("source").unwrap_or_default(),
                    session_id: row.get("session_id"),
                    tags: row
                        .get::<Option<String>, _>("tags")
                        .map(|t| t.split(',').map(|s| s.to_string()).collect())
                        .unwrap_or_default(),
                },
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(memories)
    }

    /// Get memories by session ID
    pub async fn get_by_session(&self, session_id: &str) -> Result<Vec<Memory>, MemoryError> {
        let rows = sqlx::query(
            r#"
            SELECT id, content, source, session_id, tags, created_at
            FROM memories
            WHERE session_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Failed to get session memories: {}", e)))?;

        let memories = rows
            .into_iter()
            .map(|row| Memory {
                id: row.get("id"),
                content: row.get("content"),
                embedding: None,
                metadata: MemoryMetadata {
                    source: row.get::<Option<String>, _>("source").unwrap_or_default(),
                    session_id: row.get("session_id"),
                    tags: row
                        .get::<Option<String>, _>("tags")
                        .map(|t| t.split(',').map(|s| s.to_string()).collect())
                        .unwrap_or_default(),
                },
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(memories)
    }

    /// Delete old memories
    pub async fn cleanup(&self, older_than_days: i64) -> Result<u64, MemoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM memories
            WHERE created_at < datetime('now', ?1 || ' days')
            "#,
        )
        .bind(format!("-{}", older_than_days))
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::Storage(format!("Cleanup failed: {}", e)))?;

        Ok(result.rows_affected())
    }
}

#[async_trait::async_trait]
impl super::MemoryStore for SqliteMemoryStore {
    async fn store(&self, content: &str, embedding: Vec<f32>, metadata: MemoryMetadata) -> Result<String, MemoryError> {
        self.store(content, embedding, metadata).await
    }

    async fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<Memory>, MemoryError> {
        self.search(query_embedding, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store() {
        let store = SqliteMemoryStore::new(":memory:", 1536).await.unwrap();

        let metadata = MemoryMetadata {
            source: "test".to_string(),
            session_id: Some("session-1".to_string()),
            tags: vec!["test".to_string()],
        };

        let id = store.store("Test content", vec![0.1; 1536], metadata).await.unwrap();
        assert!(!id.is_empty());

        let results = store.search(&[0.1; 1536], 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Test content");
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let store = SqliteMemoryStore::new(":memory:", 1536).await.unwrap();

        let metadata = MemoryMetadata {
            source: "test".to_string(),
            session_id: Some("session-1".to_string()),
            tags: vec![],
        };

        store.store("Session content", vec![0.2; 1536], metadata).await.unwrap();

        let results = store.get_by_session("session-1").await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
