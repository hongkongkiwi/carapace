//! TUI (Terminal User Interface)
//!
//! Ratatui-based terminal interface.

use serde::{Deserialize, Serialize};

/// TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Enable TUI
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Frame rate
    #[serde(default = "default_fps")]
    pub fps: u8,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fps: 30,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_fps() -> u8 {
    30
}

/// TUI application
pub struct TuiApp {
    config: TuiConfig,
}

impl TuiApp {
    /// Create new TUI
    pub fn new(config: TuiConfig) -> Self {
        Self { config }
    }

    /// Run TUI
    pub async fn run(&self) -> Result<(), TuiError> {
        if !self.config.enabled {
            return Ok(());
        }
        tracing::info!("Starting TUI");
        Ok(())
    }
}

/// TUI errors
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Event error: {0}")]
    Event(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_config() {
        let config = TuiConfig::default();
        assert!(config.enabled);
        assert_eq!(config.fps, 30);
    }
}
