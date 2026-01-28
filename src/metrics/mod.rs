//! Metrics collection module
//!
//! Provides basic metrics collection for monitoring the gateway.
//! Includes request counts, latency tracking, and health status.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    /// Request counters by endpoint
    request_counts: RwLock<HashMap<String, AtomicU64>>,
    /// Error counters by endpoint
    error_counts: RwLock<HashMap<String, AtomicU64>>,
    /// Total requests
    total_requests: AtomicU64,
    /// Total errors
    total_errors: AtomicU64,
    /// Start time
    start_time: Instant,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                request_counts: RwLock::new(HashMap::new()),
                error_counts: RwLock::new(HashMap::new()),
                total_requests: AtomicU64::new(0),
                total_errors: AtomicU64::new(0),
                start_time: Instant::now(),
            }),
        }
    }

    /// Record a request
    pub fn record_request(&self, endpoint: &str) {
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);

        let mut counts = self.inner.request_counts.write().unwrap();
        counts
            .entry(endpoint.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self, endpoint: &str) {
        self.inner.total_errors.fetch_add(1, Ordering::Relaxed);

        let mut counts = self.inner.error_counts.write().unwrap();
        counts
            .entry(endpoint.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get total requests
    pub fn total_requests(&self) -> u64 {
        self.inner.total_requests.load(Ordering::Relaxed)
    }

    /// Get total errors
    pub fn total_errors(&self) -> u64 {
        self.inner.total_errors.load(Ordering::Relaxed)
    }

    /// Get request count for an endpoint
    pub fn request_count(&self, endpoint: &str) -> u64 {
        let counts = self.inner.request_counts.read().unwrap();
        counts
            .get(endpoint)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.inner.start_time.elapsed().as_secs()
    }

    /// Get all metrics as a snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let request_counts = self
            .inner
            .request_counts
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        let error_counts = self
            .inner
            .error_counts
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        MetricsSnapshot {
            total_requests: self.total_requests(),
            total_errors: self.total_errors(),
            request_counts,
            error_counts,
            uptime_seconds: self.uptime_seconds(),
        }
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub request_counts: HashMap<String, u64>,
    pub error_counts: HashMap<String, u64>,
    pub uptime_seconds: u64,
}

/// Global metrics instance
static GLOBAL_METRICS: std::sync::OnceLock<MetricsCollector> = std::sync::OnceLock::new();

/// Initialize global metrics
pub fn init_global_metrics() -> MetricsCollector {
    let metrics = MetricsCollector::new();
    let _ = GLOBAL_METRICS.set(metrics.clone());
    metrics
}

/// Get global metrics
pub fn global_metrics() -> Option<MetricsCollector> {
    GLOBAL_METRICS.get().cloned()
}

/// Request timer for latency tracking
pub struct RequestTimer {
    start: Instant,
    endpoint: String,
}

impl RequestTimer {
    /// Start timing a request
    pub fn start(endpoint: &str) -> Self {
        Self {
            start: Instant::now(),
            endpoint: endpoint.to_string(),
        }
    }

    /// Record the request and return latency
    pub fn record(self) -> std::time::Duration {
        let latency = self.start.elapsed();
        if let Some(metrics) = global_metrics() {
            metrics.record_request(&self.endpoint);
        }
        latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let metrics = MetricsCollector::new();

        metrics.record_request("/api/test");
        metrics.record_request("/api/test");
        metrics.record_error("/api/test");

        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.total_errors(), 1);
        assert_eq!(metrics.request_count("/api/test"), 2);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = MetricsCollector::new();
        metrics.record_request("/api/test");

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 1);
        // Note: uptime_seconds is u64 so it's always >= 0
    }
}
