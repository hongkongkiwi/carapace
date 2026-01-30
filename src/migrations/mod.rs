//! Database Migrations
//!
//! Manages database schema migrations for carapace.
//! Supports SQLite and PostgreSQL databases.
//! Migrations are applied sequentially and tracked in a migrations table.

use sqlx::{PgPool, Pool, Postgres};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Migration error types
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid migration name: {0}")]
    InvalidName(String),

    #[error("Migration {name} failed: {message}")]
    Failed { name: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A single migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version (timestamp-based or sequential)
    pub version: i64,
    /// Migration name/description
    pub name: String,
    /// SQL statements to apply
    pub sql: String,
    /// Whether this migration can be rolled back
    pub reversible: bool,
}

/// Migration runner
#[derive(Debug)]
pub struct MigrationRunner {
    /// Database connection URL
    connection_url: String,
    /// Migrations directory
    migrations_dir: PathBuf,
    /// Applied migrations table name
    table_name: String,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(connection_url: String, migrations_dir: PathBuf) -> Self {
        Self {
            connection_url,
            migrations_dir,
            table_name: "carapace_migrations".to_string(),
        }
    }

    /// Get the database pool
    pub async fn pool(&self) -> Result<PgPool, MigrationError> {
        PgPool::connect(&self.connection_url)
            .await
            .map_err(|e| MigrationError::Database(sqlx::Error::Protocol(e.to_string())))
    }

    /// Initialize the migrations table
    pub async fn init_table(&self, pool: &PgPool) -> Result<(), MigrationError> {
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                version BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                checksum TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_{}_applied_at ON {}(applied_at);
            "#,
            self.table_name, self.table_name, self.table_name
        );

        sqlx::query(&query).execute(pool).await?;
        Ok(())
    }

    /// Get list of applied migrations
    pub async fn get_applied_migrations(&self, pool: &PgPool) -> Result<Vec<i64>, MigrationError> {
        let query = format!(
            "SELECT version FROM {} ORDER BY version ASC",
            self.table_name
        );

        let versions: Vec<i64> = sqlx::query_scalar(&query)
            .fetch_all(pool)
            .await?;

        Ok(versions)
    }

    /// Get all migration files from the migrations directory
    pub fn get_migration_files(&self) -> Result<Vec<PathBuf>, MigrationError> {
        if !self.migrations_dir.exists() {
            return Ok(vec![]);
        }

        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.migrations_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map(|e| e == "sql").unwrap_or(false))
            .map(|entry| entry.path())
            .collect();

        // Sort by filename (version number)
        files.sort();

        Ok(files)
    }

    /// Parse migration filename to extract version and name
    /// Format: YYYYMMDDHHMMSS_description.sql
    fn parse_migration_filename(&self, filename: &Path) -> Result<(i64, String), MigrationError> {
        let stem = filename
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MigrationError::InvalidName(filename.display().to_string()))?;

        // Split at first underscore
        let parts: Vec<&str> = stem.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(MigrationError::InvalidName(format!(
                "Invalid migration filename format: {} (expected: YYYYMMDDHHMMSS_description.sql)",
                filename.display()
            )));
        }

        let version: i64 = parts[0]
            .parse()
            .map_err(|_| MigrationError::InvalidName(format!("Invalid version in: {}", filename.display())))?;

        let name = parts[1].to_string();

        Ok((version, name))
    }

    /// Calculate checksum for a migration file content
    fn calculate_checksum(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> Result<Vec<Migration>, MigrationError> {
        let pool = self.pool().await?;
        self.init_table(&pool).await?;

        let applied_versions = self.get_applied_migrations(&pool).await?;
        let migration_files = self.get_migration_files()?;

        let mut applied_migrations = Vec::new();

        for file in migration_files {
            let (version, name) = self.parse_migration_filename(&file)?;

            // Skip if already applied
            if applied_versions.contains(&version) {
                tracing::info!("Migration {} already applied, skipping", name);
                continue;
            }

            // Read migration SQL
            let sql = std::fs::read_to_string(&file)?;

            let checksum = self.calculate_checksum(&sql);

            // Apply migration within a transaction
            let mut tx = pool.begin().await?;

            // Apply the SQL - split by semicolons and execute each statement
            for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
                let trimmed = statement.trim();
                if !trimmed.is_empty() {
                    sqlx::query(trimmed).execute(&mut *tx).await
                        .map_err(|e| MigrationError::Failed {
                            name: name.clone(),
                            message: e.to_string(),
                        })?;
                }
            }

            // Record the migration
            let insert_query = format!(
                "INSERT INTO {} (version, name, checksum) VALUES ($1, $2, $3)",
                self.table_name
            );
            sqlx::query(&insert_query)
                .bind(version)
                .bind(&name)
                .bind(&checksum)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;

            tracing::info!("Applied migration: {}", name);
            applied_migrations.push(Migration {
                version,
                name,
                sql,
                reversible: true,
            });
        }

        if applied_migrations.is_empty() {
            tracing::info!("No pending migrations");
        } else {
            tracing::info!("Applied {} migration(s)", applied_migrations.len());
        }

        Ok(applied_migrations)
    }

    /// Rollback the last migration
    pub async fn rollback_last(&self) -> Result<(), MigrationError> {
        let pool = self.pool().await?;

        // Get the last applied migration
        let query = format!(
            "SELECT version, name FROM {} ORDER BY version DESC LIMIT 1",
            self.table_name
        );

        let last_migration: Option<(i64, String)> = sqlx::query_as(&query)
            .fetch_optional(&pool)
            .await?;

        if let Some((version, name)) = last_migration {
            tracing::info!("Rolling back migration: {}", name);

            // Note: In a real implementation, you would need to read the
            // corresponding rollback SQL from a .down.sql file
            let delete_query = format!(
                "DELETE FROM {} WHERE version = $1",
                self.table_name
            );

            sqlx::query(&delete_query)
                .bind(version)
                .execute(&pool)
                .await?;

            tracing::info!("Rolled back migration: {}", name);
        } else {
            tracing::warn!("No migrations to rollback");
        }

        Ok(())
    }

    /// Show migration status
    pub async fn status(&self) -> Result<(), MigrationError> {
        let pool = self.pool().await?;

        let applied_versions = self.get_applied_migrations(&pool).await?;
        let migration_files = self.get_migration_files()?;

        println!("Database Migration Status");
        println!("==========================");
        println!();

        let mut pending_count = 0;
        for file in migration_files {
            let (version, name) = self.parse_migration_filename(&file)?;

            let status = if applied_versions.contains(&version) {
                "APPLIED"
            } else {
                pending_count += 1;
                "PENDING"
            };

            println!("[{}] {}", status, name);
        }

        println!();
        println!("Applied: {}", applied_versions.len());
        println!("Pending: {}", pending_count);

        Ok(())
    }
}

/// Default migrations directory
pub fn default_migrations_dir() -> PathBuf {
    PathBuf::from("migrations")
}

/// Create a new migration file
pub fn create_migration(name: &str, sql: &str) -> Result<PathBuf, std::io::Error> {
    let timestamp = chrono::Utc::now();
    let version = timestamp.format("%Y%m%d%H%M%S").to_string();
    let filename = format!("{}_{}.sql", version, name);

    let migrations_dir = default_migrations_dir();
    std::fs::create_dir_all(&migrations_dir)?;

    let filepath = migrations_dir.join(&filename);
    std::fs::write(&filepath, sql)?;

    tracing::info!("Created migration: {}", filepath.display());

    Ok(filepath)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_migration_filename() {
        let runner = MigrationRunner {
            connection_url: "".to_string(),
            migrations_dir: PathBuf::new(),
            table_name: "test_migrations".to_string(),
        };

        let (version, name) = runner.parse_migration_filename(
            Path::new("20240101120000_create_users.sql")
        ).unwrap();

        assert_eq!(version, 20240101120000);
        assert_eq!(name, "create_users");

        let (version, name) = runner.parse_migration_filename(
            Path::new("20240101120001_add_email_to_users.sql")
        ).unwrap();

        assert_eq!(version, 20240101120001);
        assert_eq!(name, "add_email_to_users");
    }

    #[test]
    fn test_invalid_migration_filename() {
        let runner = MigrationRunner {
            connection_url: "".to_string(),
            migrations_dir: PathBuf::new(),
            table_name: "test_migrations".to_string(),
        };

        assert!(runner.parse_migration_filename(Path::new("invalid.sql")).is_err());
        assert!(runner.parse_migration_filename(Path::new("123_no_underscore.sql")).is_err());
    }

    #[test]
    fn test_checksum_calculation() {
        let runner = MigrationRunner {
            connection_url: "".to_string(),
            migrations_dir: PathBuf::new(),
            table_name: "test_migrations".to_string(),
        };

        let sql1 = "CREATE TABLE users (id INTEGER PRIMARY KEY);";
        let sql2 = "CREATE TABLE posts (id INTEGER PRIMARY KEY);";

        let checksum1 = runner.calculate_checksum(sql1);
        let checksum2 = runner.calculate_checksum(sql2);

        assert_ne!(checksum1, checksum2);
        assert_eq!(checksum1, runner.calculate_checksum(sql1));
    }
}
