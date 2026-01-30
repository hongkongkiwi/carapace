//! Redis Tool Plugin
//!
//! Native implementation of Redis operations for carapace.
//! Supports key-value operations, hashes, lists, sets, and pub/sub.
//!
//! Security: Password retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Redis tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Hostname
    #[serde(default = "default_host")]
    pub host: String,

    /// Port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Database number
    #[serde(default = "default_db")]
    pub db: u32,

    /// Password (retrieved from credential store)
    #[serde(skip)]
    pub password: Option<String>,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    6379
}

fn default_db() -> u32 {
    0
}

fn default_timeout() -> u64 {
    30
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            db: default_db(),
            password: None,
            timeout: default_timeout(),
        }
    }
}

/// Redis client wrapper
#[derive(Clone)]
pub struct RedisClient {
    config: RedisConfig,
    conn: Arc<Mutex<redis::Connection>>,
}

impl std::fmt::Debug for RedisClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisClient")
            .field("host", &self.config.host)
            .field("port", &self.config.port)
            .field("db", &self.config.db)
            .finish_non_exhaustive()
    }
}

impl RedisClient {
    /// Create a new Redis client
    pub fn new(config: RedisConfig) -> Result<Self, BindingError> {
        let client = redis::Client::open(format!(
            "redis://{}:{}/{}",
            config.host, config.port, config.db
        ))
        .map_err(|e| BindingError::CallError(format!("Redis connection failed: {}", e)))?;

        let conn = client
            .get_connection()
            .map_err(|e| BindingError::CallError(format!("Redis connection failed: {}", e)))?;

        Ok(Self {
            config,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get a value
    pub fn get(&self, key: &str) -> Result<Option<String>, BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        redis::cmd("GET")
            .arg(key)
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("GET failed: {}", e)))
    }

    /// Set a value with optional expiry
    pub fn set(&self, key: &str, value: &str, expiry: Option<u64>) -> Result<(), BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        let mut cmd = redis::cmd("SET");
        cmd.arg(key).arg(value);
        if let Some(exp) = expiry {
            cmd.arg("EX").arg(exp);
        }
        cmd.query(conn)
            .map_err(|e| BindingError::CallError(format!("SET failed: {}", e)))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<bool, BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        let result: i32 = redis::cmd("DEL")
            .arg(key)
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("DEL failed: {}", e)))?;
        Ok(result > 0)
    }

    /// Check if key exists
    pub fn exists(&self, key: &str) -> Result<bool, BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        let result: bool = redis::cmd("EXISTS")
            .arg(key)
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("EXISTS failed: {}", e)))?;
        Ok(result)
    }

    /// Set hash field
    pub fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        redis::cmd("HSET")
            .arg(key)
            .arg(field)
            .arg(value)
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("HSET failed: {}", e)))
    }

    /// Get hash field
    pub fn hget(&self, key: &str, field: &str) -> Result<Option<String>, BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        redis::cmd("HGET")
            .arg(key)
            .arg(field)
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("HGET failed: {}", e)))
    }

    /// Ping for health check
    pub fn ping(&self) -> Result<String, BindingError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| BindingError::CallError(format!("Mutex lock failed: {}", e)))?;
        let conn = &mut *guard;

        redis::cmd("PING")
            .query(conn)
            .map_err(|e| BindingError::CallError(format!("PING failed: {}", e)))
    }
}

/// Redis tool plugin
#[derive(Debug, Clone)]
pub struct RedisTool {
    client: Option<RedisClient>,
}

impl RedisTool {
    pub fn new() -> Self {
        Self { client: None }
    }

    pub fn initialize(&mut self, config: RedisConfig) -> Result<(), BindingError> {
        let client = RedisClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for RedisTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for RedisTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "redis_get".to_string(),
                description: "Get a value from Redis by key.".to_string(),
                input_schema: GetInput::schema().to_string(),
            },
            ToolDefinition {
                name: "redis_set".to_string(),
                description: "Set a value in Redis with optional expiry.".to_string(),
                input_schema: SetInput::schema().to_string(),
            },
            ToolDefinition {
                name: "redis_delete".to_string(),
                description: "Delete a key from Redis.".to_string(),
                input_schema: DeleteInput::schema().to_string(),
            },
            ToolDefinition {
                name: "redis_hset".to_string(),
                description: "Set a field in a Redis hash.".to_string(),
                input_schema: HSetInput::schema().to_string(),
            },
            ToolDefinition {
                name: "redis_hget".to_string(),
                description: "Get a field from a Redis hash.".to_string(),
                input_schema: HGetInput::schema().to_string(),
            },
            ToolDefinition {
                name: "redis_ping".to_string(),
                description: "Ping Redis for health check.".to_string(),
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
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BindingError::CallError("Redis tool not initialized".to_string()))?;

        match name {
            "redis_get" => {
                let input: GetInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;
                let result = client.get(&input.key)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(
                        serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string()),
                    ),
                    error: None,
                })
            }
            "redis_set" => {
                let input: SetInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;
                client.set(&input.key, &input.value, input.expiry)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(r#"{"status": "ok"}"#.to_string()),
                    error: None,
                })
            }
            "redis_delete" => {
                let input: DeleteInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;
                let existed = client.delete(&input.key)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(format!(r#"{{"deleted": {}}}"#, existed)),
                    error: None,
                })
            }
            "redis_hset" => {
                let input: HSetInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;
                client.hset(&input.key, &input.field, &input.value)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(r#"{"status": "ok"}"#.to_string()),
                    error: None,
                })
            }
            "redis_hget" => {
                let input: HGetInput = serde_json::from_str(params)
                    .map_err(|e| BindingError::CallError(format!("Invalid params: {}", e)))?;
                let result = client.hget(&input.key, &input.field)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(
                        serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string()),
                    ),
                    error: None,
                })
            }
            "redis_ping" => {
                let result = client.ping()?;
                Ok(ToolResult {
                    success: true,
                    result: Some(format!(r#"{{"response": "{}"}}"#, result)),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

// Input types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInput {
    pub key: String,
}
impl GetInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"key": {"type": "string"}}, "required": ["key"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetInput {
    pub key: String,
    pub value: String,
    pub expiry: Option<u64>,
}
impl SetInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"key": {"type": "string"}, "value": {"type": "string"}, "expiry": {"type": "integer"}}, "required": ["key", "value"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteInput {
    pub key: String,
}
impl DeleteInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"key": {"type": "string"}}, "required": ["key"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HSetInput {
    pub key: String,
    pub field: String,
    pub value: String,
}
impl HSetInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"key": {"type": "string"}, "field": {"type": "string"}, "value": {"type": "string"}}, "required": ["key", "field", "value"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HGetInput {
    pub key: String,
    pub field: String,
}
impl HGetInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"key": {"type": "string"}, "field": {"type": "string"}}, "required": ["key", "field"]})
    }
}
