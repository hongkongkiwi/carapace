//! Metrics collection and reporting
//!
//! Provides basic metrics for observability including request counts,
//! latency tracking, and component health status.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Counter metric type
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    /// Create a new counter
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter by 1
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by a specific amount
    pub fn add(&self, amount: u64) {
        self.value.fetch_add(amount, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset the counter to 0
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// Gauge metric type (stores a single value)
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    /// Create a new gauge
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the gauge value
    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram for tracking latency distributions
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<(u64, AtomicU64)>, // (upper_bound, count)
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    /// Create a new histogram with default buckets (in milliseconds)
    pub fn new() -> Self {
        let bucket_bounds = vec![5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];
        Self::with_buckets(&bucket_bounds)
    }

    /// Create a histogram with custom bucket bounds (in milliseconds)
    pub fn with_buckets(bounds: &[u64]) -> Self {
        let buckets = bounds
            .iter()
            .map(|&b| (b, AtomicU64::new(0)))
            .collect();

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Record a value in the histogram (value in milliseconds)
    pub fn observe(&self, value: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        for (bound, counter) in &self.buckets {
            if value <= *bound {
                counter.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    /// Record a duration
    pub fn observe_duration(&self, duration: Duration) {
        let millis = duration.as_millis() as u64;
        self.observe(millis);
    }

    /// Get the total count
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get the sum of all values
    pub fn sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    /// Calculate the average (mean)
    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() as f64 / count as f64
        }
    }

    /// Get bucket counts
    pub fn buckets(&self) -> Vec<(u64, u64)> {
        self.buckets
            .iter()
            .map(|(bound, count)| (*bound, count.load(Ordering::Relaxed)))
            .collect()
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Component health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functioning
    Degraded,
    /// Component is unhealthy
    Unhealthy,
    /// Component status is unknown
    Unknown,
}

impl HealthStatus {
    /// Get the status as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Unknown => "unknown",
        }
    }
}

/// Health check information for a component
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: Instant,
}

/// Metrics registry for collecting application metrics
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, Arc<Counter>>>,
    gauges: RwLock<HashMap<String, Arc<Gauge>>>,
    histograms: RwLock<HashMap<String, Arc<Histogram>>>,
    component_health: RwLock<HashMap<String, ComponentHealth>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a counter
    pub fn counter(&self, name: &str) -> Arc<Counter> {
        let mut counters = self.counters.write().unwrap();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Counter::new()))
            .clone()
    }

    /// Get or create a gauge
    pub fn gauge(&self, name: &str) -> Arc<Gauge> {
        let mut gauges = self.gauges.write().unwrap();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Gauge::new()))
            .clone()
    }

    /// Get or create a histogram
    pub fn histogram(&self, name: &str) -> Arc<Histogram> {
        let mut histograms = self.histograms.write().unwrap();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new()))
            .clone()
    }

    /// Update component health status
    pub fn set_component_health(&self, name: &str, status: HealthStatus, message: Option<String>) {
        let mut health = self.component_health.write().unwrap();
        health.insert(
            name.to_string(),
            ComponentHealth {
                name: name.to_string(),
                status,
                message,
                last_check: Instant::now(),
            },
        );
    }

    /// Get all component health statuses
    pub fn component_health(&self) -> Vec<ComponentHealth> {
        let health = self.component_health.read().unwrap();
        health.values().cloned().collect()
    }

    /// Get a specific component's health
    pub fn get_component_health(&self, name: &str) -> Option<ComponentHealth> {
        let health = self.component_health.read().unwrap();
        health.get(name).cloned()
    }

    /// Get overall health status (worst of all components)
    pub fn overall_health(&self) -> HealthStatus {
        let health = self.component_health.read().unwrap();
        if health.is_empty() {
            return HealthStatus::Unknown;
        }

        let mut overall = HealthStatus::Healthy;
        for component in health.values() {
            match component.status {
                HealthStatus::Unhealthy => return HealthStatus::Unhealthy,
                HealthStatus::Degraded => overall = HealthStatus::Degraded,
                _ => {}
            }
        }
        overall
    }

    /// Get a snapshot of all counter values
    pub fn counter_values(&self) -> HashMap<String, u64> {
        let counters = self.counters.read().unwrap();
        counters
            .iter()
            .map(|(name, counter)| (name.clone(), counter.get()))
            .collect()
    }

    /// Get a snapshot of all gauge values
    pub fn gauge_values(&self) -> HashMap<String, u64> {
        let gauges = self.gauges.read().unwrap();
        gauges
            .iter()
            .map(|(name, gauge)| (name.clone(), gauge.get()))
            .collect()
    }

    /// Get a snapshot of all histogram statistics
    pub fn histogram_stats(&self) -> HashMap<String, (u64, u64, f64)> {
        let histograms = self.histograms.read().unwrap();
        histograms
            .iter()
            .map(|(name, hist)| {
                (name.clone(), (hist.count(), hist.sum(), hist.mean()))
            })
            .collect()
    }
}

/// Global metrics registry singleton
use std::sync::OnceLock;
static GLOBAL_REGISTRY: OnceLock<Arc<MetricsRegistry>> = OnceLock::new();

/// Get the global metrics registry
pub fn global_registry() -> Arc<MetricsRegistry> {
    GLOBAL_REGISTRY
        .get_or_init(|| Arc::new(MetricsRegistry::new()))
        .clone()
}

/// Initialize the global registry (optional, called automatically)
pub fn init_global_registry() {
    let _ = global_registry();
}

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
    histogram: Option<Arc<Histogram>>,
}

impl Timer {
    /// Start a new timer
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            histogram: None,
        }
    }

    /// Start a timer that will record to a histogram
    pub fn with_histogram(histogram: Arc<Histogram>) -> Self {
        Self {
            start: Instant::now(),
            histogram: Some(histogram),
        }
    }

    /// Stop the timer and return the elapsed duration
    pub fn stop(&self) -> Duration {
        let elapsed = self.start.elapsed();
        if let Some(ref histogram) = self.histogram {
            histogram.observe_duration(elapsed);
        }
        elapsed
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.add(5);
        assert_eq!(counter.get(), 6);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        assert_eq!(gauge.get(), 0);

        gauge.set(42);
        assert_eq!(gauge.get(), 42);

        gauge.set(100);
        assert_eq!(gauge.get(), 100);
    }

    #[test]
    fn test_histogram() {
        let hist = Histogram::new();

        hist.observe(10);
        hist.observe(20);
        hist.observe(30);

        assert_eq!(hist.count(), 3);
        assert_eq!(hist.sum(), 60);
        assert_eq!(hist.mean(), 20.0);

        let buckets = hist.buckets();
        assert!(!buckets.is_empty());
    }

    #[test]
    fn test_histogram_duration() {
        let hist = Histogram::new();

        hist.observe_duration(Duration::from_millis(50));
        hist.observe_duration(Duration::from_millis(100));

        assert_eq!(hist.count(), 2);
        assert_eq!(hist.sum(), 150);
    }

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        // Test counters
        let counter = registry.counter("requests");
        counter.inc();
        assert_eq!(counter.get(), 1);

        // Test gauges
        let gauge = registry.gauge("memory");
        gauge.set(1024);
        assert_eq!(gauge.get(), 1024);

        // Test histograms
        let histogram = registry.histogram("latency");
        histogram.observe(50);
        assert_eq!(histogram.count(), 1);

        // Test health
        registry.set_component_health("database", HealthStatus::Healthy, None);
        let health = registry.get_component_health("database");
        assert!(health.is_some());
        assert_eq!(health.unwrap().status, HealthStatus::Healthy);

        assert_eq!(registry.overall_health(), HealthStatus::Healthy);
    }

    #[test]
    fn test_overall_health_worst_case() {
        let registry = MetricsRegistry::new();

        registry.set_component_health("db", HealthStatus::Healthy, None);
        registry.set_component_health("cache", HealthStatus::Degraded, None);
        registry.set_component_health("queue", HealthStatus::Healthy, None);

        // Overall should be degraded (worst of healthy/degraded)
        assert_eq!(registry.overall_health(), HealthStatus::Degraded);
    }

    #[test]
    fn test_overall_health_unhealthy() {
        let registry = MetricsRegistry::new();

        registry.set_component_health("db", HealthStatus::Healthy, None);
        registry.set_component_health("cache", HealthStatus::Unhealthy, None);

        // Overall should be unhealthy (worst case)
        assert_eq!(registry.overall_health(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::new();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_timer_with_histogram() {
        let registry = MetricsRegistry::new();
        let hist = registry.histogram("test_latency");

        let timer = Timer::with_histogram(hist.clone());
        std::thread::sleep(Duration::from_millis(5));
        timer.stop();

        assert_eq!(hist.count(), 1);
        assert!(hist.sum() >= 5);
    }

    #[test]
    fn test_global_registry() {
        let registry1 = global_registry();
        let registry2 = global_registry();

        // Should be the same instance
        assert!(Arc::ptr_eq(&registry1, &registry2));
    }
}
