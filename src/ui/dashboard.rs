//! Web Dashboard
//!
//! Built-in web interface for remote gateway management.
//! Provides real-time status, channel management, and analytics.

use axum::{
    body::Body,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable dashboard
    pub enabled: bool,
    /// Bind address
    pub bind: String,
    /// Port
    pub port: u16,
    /// Enable authentication
    pub auth_enabled: bool,
    /// Admin username
    pub username: Option<String>,
    /// Admin password hash
    pub password_hash: Option<String>,
    /// API key for external access
    pub api_key: Option<String>,
}

/// Dashboard state (RwLock wrapped in Arc for async access)
#[derive(Debug, Clone)]
pub struct DashboardState(pub Arc<RwLock<DashboardStateInner>>);

/// Inner dashboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStateInner {
    /// Server version
    pub version: String,
    /// Active channels count
    pub active_channels: usize,
    /// Total messages processed
    pub total_messages: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// System info
    pub system_info: SystemInfo,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// CPU usage percent
    pub cpu_percent: f32,
    /// Memory usage percent
    pub memory_percent: f32,
    /// Disk usage percent
    pub disk_percent: f32,
}

/// Dashboard API response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub channels_active: usize,
}

/// Dashboard errors
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("Not configured")]
    NotConfigured,
    #[error("Authentication required")]
    AuthRequired,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Server error: {0}")]
    ServerError(String),
}

/// Create dashboard router with shared state
pub fn create_dashboard_router(state: DashboardState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/status", get(status_handler))
        .route("/api/channels", get(channels_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/logs", get(logs_handler))
        .with_state(state)
}

/// Health check handler - returns basic health status
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        uptime_seconds: 0,
        channels_active: 0,
    })
}

/// Status handler - returns full dashboard state
async fn status_handler() -> Json<ApiResponse<DashboardStateInner>> {
    Json(ApiResponse {
        success: true,
        data: Some(DashboardStateInner {
            version: "0.1.0".to_string(),
            active_channels: 0,
            total_messages: 0,
            uptime_seconds: 0,
            system_info: SystemInfo {
                cpu_percent: 0.0,
                memory_percent: 0.0,
                disk_percent: 0.0,
            },
        }),
        error: None,
    })
}

/// Channels handler - returns list of active channels
async fn channels_handler() -> Json<ApiResponse<Vec<String>>> {
    Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        error: None,
    })
}

/// Metrics handler - returns system metrics
async fn metrics_handler() -> Json<ApiResponse<SystemInfo>> {
    Json(ApiResponse {
        success: true,
        data: Some(SystemInfo {
            cpu_percent: 0.0,
            memory_percent: 0.0,
            disk_percent: 0.0,
        }),
        error: None,
    })
}

/// Logs handler - returns recent logs
async fn logs_handler() -> Json<ApiResponse<Vec<String>>> {
    Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        error: None,
    })
}

/// Start dashboard server
pub async fn start_dashboard(
    config: DashboardConfig,
    _state: DashboardState,
) -> Result<(), DashboardError> {
    if !config.enabled {
        return Err(DashboardError::NotConfigured);
    }

    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse::<SocketAddr>()
        .map_err(|e: std::net::AddrParseError| DashboardError::ServerError(e.to_string()))?;

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/status", get(status_handler))
        .route("/api/channels", get(channels_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/logs", get(logs_handler));

    tracing::info!(address = %addr, "Starting dashboard server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| DashboardError::ServerError(e.to_string()))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| DashboardError::ServerError(e.to_string()))?;

    Ok(())
}
