//! Dashboard
//!
//! Web dashboard for monitoring and control.

use serde::{Deserialize, Serialize};

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable dashboard
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Bind address
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Port
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_bind() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Dashboard server
pub struct Dashboard {
    config: DashboardConfig,
}

impl Dashboard {
    /// Create new dashboard
    pub fn new(config: DashboardConfig) -> Self {
        Self { config }
    }

    /// Start dashboard
    pub async fn start(&self) -> Result<(), DashboardError> {
        if !self.config.enabled {
            return Ok(());
        }
        tracing::info!("Starting dashboard on {}:{}", self.config.bind, self.config.port);
        Ok(())
    }

    /// Stop dashboard
    pub async fn stop(&self) -> Result<(), DashboardError> {
        tracing::info!("Stopping dashboard");
        Ok(())
    }
}

/// Dashboard errors
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("Bind error: {0}")]
    Bind(String),
    #[error("Server error: {0}")]
    Server(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config() {
        let config = DashboardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.port, 8080);
    }
}
