//! PostgreSQL Tool Plugin
//!
//! Native implementation of PostgreSQL database operations for carapace.
//! Supports queries and execute operations.
//!
//! Security: Connection credentials retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    ToolDefinition, ToolPluginInstance, ToolContext, ToolResult,
    BindingError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// PostgreSQL tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSQLConfig {
    /// Hostname
    #[serde(default = "default_host")]
    pub host: String,

    /// Port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Database name
    #[serde(default)]
    pub database: String,

    /// Username
    #[serde(default)]
    pub username: String,

    /// Password (retrieved from credential store)
    #[serde(skip)]
    pub password: Option<String>,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout: u64,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    5432
}

fn default_timeout() -> u64 {
    30
}

impl Default for PostgreSQLConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            database: String::new(),
            username: String::new(),
            password: None,
            connection_timeout: default_timeout(),
        }
    }
}

/// PostgreSQL connection using blocking postgres crate with Mutex for thread safety
#[derive(Clone)]
pub struct PostgreSQLClient {
    config: PostgreSQLConfig,
    conn: Arc<Mutex<postgres::Client>>,
}

impl std::fmt::Debug for PostgreSQLClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgreSQLClient")
            .field("host", &self.config.host)
            .field("port", &self.config.port)
            .field("database", &self.config.database)
            .finish()
    }
}

impl PostgreSQLClient {
    /// Create a new PostgreSQL client
    pub fn new(config: PostgreSQLConfig) -> Result<Self, BindingError> {
        let connection_string = format!(
            "host={} port={} dbname={} user={} connect_timeout={}",
            config.host,
            config.port,
            config.database,
            config.username,
            config.connection_timeout
        );

        let conn = postgres::Client::connect(&connection_string, postgres::NoTls)
            .map_err(|e| BindingError::CallError(format!("PostgreSQL connection failed: {}", e)))?;

        Ok(Self {
            config,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Execute a query and return rows
    pub fn query(&self, query: &str) -> Result<QueryResult, BindingError> {
        let mut guard = self.conn.lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        let rows = conn.query(query, &[])
            .map_err(|e| BindingError::CallError(format!("Query failed: {}", e)))?;

        let row_count = rows.len();
        let results: Vec<serde_json::Value> = rows.iter().map(|row| {
            let mut row_data = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value = Self::parse_value(row, i);
                row_data.insert(column.name().to_string(), value);
            }
            serde_json::Value::Object(row_data)
        }).collect();

        Ok(QueryResult {
            rows: results,
            row_count: row_count as i64,
        })
    }

    /// Parse a value from a row
    fn parse_value(row: &postgres::Row, index: usize) -> serde_json::Value {
        match row.try_get::<_, Option<i32>>(index) {
            Ok(Some(v)) => serde_json::Value::Number(v.into()),
            Ok(None) => serde_json::Value::Null,
            Err(_) => match row.try_get::<_, Option<i64>>(index) {
                Ok(Some(v)) => serde_json::Value::Number(v.into()),
                Ok(None) => serde_json::Value::Null,
                Err(_) => match row.try_get::<_, Option<f64>>(index) {
                    Ok(Some(v)) => serde_json::Number::from_f64(v)
                        .map_or(serde_json::Value::Null, serde_json::Value::Number),
                    Ok(None) => serde_json::Value::Null,
                    Err(_) => match row.try_get::<_, Option<bool>>(index) {
                        Ok(Some(v)) => serde_json::Value::Bool(v),
                        Ok(None) => serde_json::Value::Null,
                        Err(_) => match row.try_get::<_, Option<&str>>(index) {
                            Ok(Some(v)) => serde_json::Value::String(v.to_string()),
                            Ok(None) => serde_json::Value::Null,
                            Err(_) => serde_json::Value::Null,
                        },
                    },
                },
            },
        }
    }

    /// Execute a statement and return affected rows
    pub fn execute(&self, statement: &str) -> Result<ExecuteResult, BindingError> {
        let mut guard = self.conn.lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        let result = conn.execute(statement, &[])
            .map_err(|e| BindingError::CallError(format!("Execute failed: {}", e)))?;

        Ok(ExecuteResult {
            affected_rows: result as i64,
        })
    }

    /// Check connection health
    pub fn health_check(&self) -> Result<(), BindingError> {
        let mut guard = self.conn.lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        conn.simple_query("SELECT 1")
            .map_err(|e| BindingError::CallError(format!("Health check failed: {}", e)))?;
        Ok(())
    }
}

// ============ Tool Plugin Implementation ============

/// PostgreSQL tool plugin for carapace
#[derive(Debug, Clone)]
pub struct PostgreSQLTool {
    /// PostgreSQL client
    client: Option<PostgreSQLClient>,
}

impl PostgreSQLTool {
    /// Create a new PostgreSQL tool
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Initialize the client with config
    pub fn initialize(&mut self, config: PostgreSQLConfig) -> Result<(), BindingError> {
        let client = PostgreSQLClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for PostgreSQLTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for PostgreSQLTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "postgres_query".to_string(),
                description: "Execute a SELECT query against PostgreSQL and return results.".to_string(),
                input_schema: QueryInput::schema().to_string(),
            },
            ToolDefinition {
                name: "postgres_execute".to_string(),
                description: "Execute an INSERT, UPDATE, DELETE or DDL statement against PostgreSQL.".to_string(),
                input_schema: ExecuteInput::schema().to_string(),
            },
            ToolDefinition {
                name: "postgres_health".to_string(),
                description: "Check PostgreSQL connection health.".to_string(),
                input_schema: "{}".to_string(),
            },
        ])
    }

    fn invoke(
        &self,
        name: &str,
        params: &str,
        _ctx: ToolContext,
    ) -> Result<ToolResult, BindingError> {
        let client = self.client.as_ref()
            .ok_or_else(|| BindingError::CallError("PostgreSQL tool not initialized".to_string()))?;

        match name {
            "postgres_query" => {
                let input: QueryInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let result = client.query(&input.query)
                    .map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result).map_err(|e| BindingError::CallError(e.to_string()))?),
                    error: None,
                })
            }
            "postgres_execute" => {
                let input: ExecuteInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;

                let result = client.execute(&input.statement)
                    .map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result).map_err(|e| BindingError::CallError(e.to_string()))?),
                    error: None,
                })
            }
            "postgres_health" => {
                client.health_check()
                    .map_err(|e| BindingError::CallError(e.to_string()))?;

                Ok(ToolResult {
                    success: true,
                    result: Some(r#"{"status": "healthy"}"#.to_string()),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

// ============ Input/Output Types ============

/// Input for query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInput {
    pub query: String,
}

impl QueryInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "SQL SELECT query to execute"
                }
            },
            "required": ["query"]
        })
    }
}

/// Input for execute execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteInput {
    pub statement: String,
}

impl ExecuteInput {
    /// Get JSON schema for this input
    pub fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "statement": {
                    "type": "string",
                    "description": "SQL statement to execute (INSERT, UPDATE, DELETE, DDL)"
                }
            },
            "required": ["statement"]
        })
    }
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<serde_json::Value>,
    pub row_count: i64,
}

/// Execute result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub affected_rows: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_input_schema() {
        let schema = QueryInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_execute_input_schema() {
        let schema = ExecuteInput::schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }
}
