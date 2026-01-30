//! OpenTelemetry-compatible Tracing
//!
//! Basic tracing support with OpenTelemetry-compatible span attributes.
//! This module provides structured logging and span creation for distributed tracing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Tracing error types
#[derive(Debug, Error)]
pub enum TracingError {
    #[error("Initialization error: {0}")]
    InitError(String),

    #[error("Shutdown error: {0}")]
    ShutdownError(String),
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,
    /// Service name
    pub service_name: String,
    /// Log level
    #[serde(default)]
    pub log_level: String,
    /// Export format
    #[serde(default)]
    pub export_format: ExportFormat,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "carapace".to_string(),
            log_level: "info".to_string(),
            export_format: ExportFormat::Json,
        }
    }
}

/// Export format for traces
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type")]
pub enum ExportFormat {
    /// JSON format (structured logging)
    #[default]
    Json,
    /// Plain text format
    Text,
    /// OpenTelemetry format (OTLP - requires additional setup)
    #[serde(skip_serializing)]
    OpenTelemetry,
}

/// Span context for distributed tracing
#[derive(Debug, Clone, Default)]
pub struct SpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub is_sampled: bool,
}

/// Active span
#[derive(Debug)]
pub struct Span {
    name: String,
    start_time: Instant,
    attributes: HashMap<String, String>,
    events: Vec<SpanEvent>,
    context: SpanContext,
}

impl Span {
    /// Create a new span
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_time: Instant::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
            context: SpanContext {
                trace_id: generate_trace_id(),
                span_id: generate_span_id(),
                parent_span_id: None,
                is_sampled: true,
            },
        }
    }

    /// Add an attribute
    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    /// Add multiple attributes
    pub fn set_attributes(&mut self, attrs: &[(&str, &str)]) {
        for (k, v) in attrs {
            self.attributes.insert(k.to_string(), v.to_string());
        }
    }

    /// Record an event
    pub fn add_event(&mut self, name: &str, attributes: Option<HashMap<String, String>>) {
        self.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: Instant::now(),
            attributes,
        });
    }

    /// End the span
    pub fn end(self) -> FinishedSpan {
        FinishedSpan {
            name: self.name,
            duration: self.start_time.elapsed(),
            attributes: self.attributes,
            events: self.events,
            context: self.context,
        }
    }
}

/// Finished span for export
#[derive(Debug, Clone)]
pub struct FinishedSpan {
    pub name: String,
    pub duration: Duration,
    pub attributes: HashMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub context: SpanContext,
}

/// Span event
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: Instant,
    pub attributes: Option<HashMap<String, String>>,
}

/// Tracing exporter trait
pub trait TracingExporter: Send + Sync {
    fn export(&self, span: &FinishedSpan);
    fn shutdown(&self) -> Result<(), TracingError>;
}

/// JSON file exporter
#[derive(Debug)]
pub struct JsonFileExporter {
    path: std::path::PathBuf,
}

impl JsonFileExporter {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
}

impl TracingExporter for JsonFileExporter {
    fn export(&self, span: &FinishedSpan) {
        let output = serde_json::json!({
            "name": span.name,
            "duration_ms": span.duration.as_millis(),
            "attributes": span.attributes,
            "events": span.events.iter().map(|e| serde_json::json!({
                "name": e.name,
                "timestamp_ms": e.timestamp.elapsed().as_millis(),
                "attributes": e.attributes
            })).collect::<Vec<_>>(),
            "trace_id": span.context.trace_id,
            "span_id": span.context.span_id,
        });

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", output.to_string())
            });
    }

    fn shutdown(&self) -> Result<(), TracingError> {
        Ok(())
    }
}

/// Initialize tracing with the given configuration
pub fn init_tracing(config: &TracingConfig) -> Result<(), TracingError> {
    if !config.enabled {
        return Ok(());
    }

    let env_filter = EnvFilter::new(&config.log_level);

    match config.export_format {
        ExportFormat::Json => {
            let subscriber = Registry::default()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json());
            subscriber.init();
        }
        ExportFormat::Text => {
            let subscriber = Registry::default()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer());
            subscriber.init();
        }
        ExportFormat::OpenTelemetry => {
            // OpenTelemetry format requires additional setup
            // For now, fall back to JSON
            let subscriber = Registry::default()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json());
            subscriber.init();
            warn!("OpenTelemetry export requires opentelemetry-otlp crate. Using JSON format.");
        }
    }

    info!("Tracing initialized for service: {}", config.service_name);
    Ok(())
}

/// Create a new span
pub fn span(name: &str) -> Span {
    Span::new(name)
}

/// Execute a traced operation
#[macro_export]
macro_rules! traced {
    ($name:expr, $($arg:tt)*) => {
        let span = $crate::tracing::span($name);
        let _enter = span.enter();
        tracing::info!($($arg)*);
    };
}

/// Message processing span attributes
pub fn message_attrs(
    channel: &str,
    message_id: &str,
    direction: &str,
) -> Vec<(String, String)> {
    vec![
        ("messaging.system".to_string(), "carapace".to_string()),
        ("messaging.destination".to_string(), channel.to_string()),
        ("messaging.message_id".to_string(), message_id.to_string()),
        ("messaging.message.direction".to_string(), direction.to_string()),
    ]
}

/// Plugin execution span attributes
pub fn plugin_attrs(
    plugin_id: &str,
    tool_name: &str,
) -> Vec<(String, String)> {
    vec![
        ("plugin.id".to_string(), plugin_id.to_string()),
        ("plugin.tool".to_string(), tool_name.to_string()),
    ]
}

/// HTTP server span attributes
pub fn http_server_attrs(
    method: &str,
    path: &str,
    status_code: u16,
) -> Vec<(String, String)> {
    vec![
        ("http.method".to_string(), method.to_string()),
        ("http.url".to_string(), path.to_string()),
        ("http.status_code".to_string(), status_code.to_string()),
        ("http.scheme".to_string(), "https".to_string()),
    ]
}

/// Channel operation span attributes
pub fn channel_attrs(
    channel: &str,
    operation: &str,
) -> Vec<(String, String)> {
    vec![
        ("channel.type".to_string(), channel.to_string()),
        ("channel.operation".to_string(), operation.to_string()),
    ]
}

/// Generate a random trace ID
fn generate_trace_id() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(bytes)
}

/// Generate a random span ID
fn generate_span_id() -> String {
    let bytes: [u8; 8] = rand::random();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.service_name, "carapace");
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new("test_span");
        assert_eq!(span.name, "test_span");
        assert!(!span.context.trace_id.is_empty());
        assert!(!span.context.span_id.is_empty());
    }

    #[test]
    fn test_span_attributes() {
        let mut span = Span::new("test_span");
        span.set_attribute("key", "value");
        span.set_attributes(&[("key2", "value2")]);

        assert_eq!(span.attributes.get("key"), Some(&"value".to_string()));
        assert_eq!(span.attributes.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_span_events() {
        let mut span = Span::new("test_span");
        span.add_event("test_event", None);
        span.add_event(
            "event_with_attrs",
            Some(vec![("attr".to_string(), "value".to_string())].into_iter().collect()),
        );

        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "test_event");
        assert!(span.events[1].attributes.is_some());
    }

    #[test]
    fn test_span_end() {
        let span = Span::new("test_span");
        std::thread::sleep(Duration::from_millis(10));
        let finished = span.end();

        assert_eq!(finished.name, "test_span");
        assert!(finished.duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_message_attrs() {
        let attrs = message_attrs("telegram", "msg123", "inbound");
        assert_eq!(attrs.len(), 4);

        let system = &attrs[0].0;
        assert_eq!(system.as_str(), "messaging.system");
    }

    #[test]
    fn test_channel_attrs() {
        let attrs = channel_attrs("discord", "send_message");
        assert_eq!(attrs.len(), 2);
    }
}
