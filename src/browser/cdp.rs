//! CDP (Chrome DevTools Protocol)
//!
//! CDP client for browser automation.

use serde::{Deserialize, Serialize};

/// CDP message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpMessage {
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// CDP client
pub struct CdpClient;

impl CdpClient {
    /// Create new client
    pub fn new() -> Self {
        Self
    }
}

impl Default for CdpClient {
    fn default() -> Self {
        Self::new()
    }
}
