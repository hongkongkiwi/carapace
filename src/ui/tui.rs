//! Terminal User Interface (TUI)
//!
//! Interactive terminal interface for managing the gateway.
//! Uses ratatui for rendering.

use std::time::Duration;

/// TUI state
#[derive(Debug, Clone)]
pub struct TuiState {
    /// Current screen
    pub current_screen: TuiScreen,
    /// Connection status
    pub is_connected: bool,
    /// Active channels
    pub active_channels: Vec<String>,
    /// Pending messages
    pub pending_messages: u32,
    /// System status
    pub system_status: SystemStatus,
}

/// System status
#[derive(Debug, Clone)]
pub struct SystemStatus {
    /// CPU usage percent
    pub cpu_percent: f32,
    /// Memory usage percent
    pub memory_percent: f32,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// TUI screens
#[derive(Debug, Clone, PartialEq)]
pub enum TuiScreen {
    /// Dashboard overview
    Dashboard,
    /// Channel list
    Channels,
    /// Message log
    Messages,
    /// Settings
    Settings,
    /// Help
    Help,
}

/// TUI configuration
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Refresh interval
    pub refresh_interval: Duration,
    /// Enable mouse support
    pub mouse_support: bool,
    /// Theme
    pub theme: TuiTheme,
}

/// TUI themes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TuiTheme {
    /// Dark theme
    Dark,
    /// Light theme
    Light,
    /// Monochrome
    Monochrome,
}

/// TUI errors
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("Not a terminal")]
    NotTerminal,
    #[error("Terminal too small")]
    TerminalTooSmall,
    #[error("I/O error: {0}")]
    IoError(String),
}

/// Initialize the TUI
pub async fn initialize_tui(config: TuiConfig) -> Result<TuiHandle, TuiError> {
    tracing::info!("Initializing TUI");
    // TODO: Initialize ratatui terminal
    Ok(TuiHandle::new(config))
}

/// TUI handle for the running interface
pub struct TuiHandle {
    config: TuiConfig,
    state: TuiState,
}

impl TuiHandle {
    /// Create new TUI handle
    fn new(config: TuiConfig) -> Self {
        Self {
            config,
            state: TuiState {
                current_screen: TuiScreen::Dashboard,
                is_connected: false,
                active_channels: vec![],
                pending_messages: 0,
                system_status: SystemStatus {
                    cpu_percent: 0.0,
                    memory_percent: 0.0,
                    uptime_seconds: 0,
                },
            },
        }
    }

    /// Run the main TUI loop
    pub async fn run(&mut self) -> Result<(), TuiError> {
        tracing::info!("Starting TUI main loop");
        // TODO: Implement main loop with ratatui
        Ok(())
    }

    /// Render current screen
    pub fn render(&self) {
        // TODO: Implement rendering
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: &str) {
        // TODO: Handle key navigation
        match key {
            "q" | "ctrl-c" => {
                self.state.current_screen = TuiScreen::Dashboard;
            }
            "1" => self.state.current_screen = TuiScreen::Dashboard,
            "2" => self.state.current_screen = TuiScreen::Channels,
            "3" => self.state.current_screen = TuiScreen::Messages,
            "4" => self.state.current_screen = TuiScreen::Settings,
            "?" | "h" => self.state.current_screen = TuiScreen::Help,
            _ => {}
        }
    }

    /// Update state
    pub fn update_state(&mut self, state: TuiState) {
        self.state = state;
    }

    /// Shutdown TUI
    pub async fn shutdown(self) {
        tracing::info!("Shutting down TUI");
        // TODO: Restore terminal
    }
}

/// Start the TUI from command line
pub async fn start_tui() -> Result<(), TuiError> {
    let config = TuiConfig {
        refresh_interval: Duration::from_millis(250),
        mouse_support: true,
        theme: TuiTheme::Dark,
    };

    let mut tui = initialize_tui(config).await?;
    tui.run().await?;
    tui.shutdown().await;
    Ok(())
}
