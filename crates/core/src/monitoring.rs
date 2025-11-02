//! Monitoring and metrics for keyword extraction
//!
//! This module provides Prometheus-compatible metrics, structured logging,
//! and alerting support for the keyword extraction pipeline.
//!
//! # Examples
//!
//! ```
//! use xze_core::monitoring::{MetricsCollector, AlertLevel};
//!
//! let mut collector = MetricsCollector::new();
//! collector.record_extraction_time(150.0);
//! collector.record_cache_hit();
//!
//! let metrics = collector.export_prometheus();
//! println!("{}", metrics);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    /// Informational message
    Info,
    /// Warning condition detected
    Warning,
    /// Critical condition requiring immediate attention
    Critical,
}

impl AlertLevel {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertLevel::Info => "info",
            AlertLevel::Warning => "warning",
            AlertLevel::Critical => "critical",
        }
    }
}

/// Alert message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert severity level
    pub level: AlertLevel,
    /// Alert title
    pub title: String,
    /// Detailed message
    pub message: String,
    /// Timestamp when alert was generated
    pub timestamp: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Alert {
    /// Create a new alert
    pub fn new(level: AlertLevel, title: String, message: String) -> Self {
        Self {
            level,
            title,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to alert
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Percentile tracker for latency measurements
#[derive(Debug)]
struct PercentileTracker {
    samples: Vec<f64>,
    max_samples: usize,
}

impl PercentileTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    fn record(&mut self, value: f64) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(value);
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    fn p50(&self) -> f64 {
        self.percentile(50.0)
    }

    fn p95(&self) -> f64 {
        self.percentile(95.0)
    }

    fn p99(&self) -> f64 {
        self.percentile(99.0)
    }
}

/// Thread-safe metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    // Counters
    total_extractions: AtomicUsize,
    llm_extractions: AtomicUsize,
    frequency_extractions: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
    errors: AtomicUsize,
    fallbacks: AtomicUsize,

    // Timing (stored as nanoseconds for atomic operations)
    total_time_ns: AtomicU64,

    // Percentile tracking (requires mutex for writes)
    latency_tracker: Arc<parking_lot::Mutex<PercentileTracker>>,

    // Alert thresholds
    error_rate_warning_threshold: f64,
    error_rate_critical_threshold: f64,
    avg_time_warning_threshold_ms: f64,
    cache_hit_rate_warning_threshold: f64,

    // Start time for uptime calculation
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector with default thresholds
    pub fn new() -> Self {
        Self::with_thresholds(5.0, 10.0, 5000.0, 80.0)
    }

    /// Create a new metrics collector with custom alert thresholds
    ///
    /// # Arguments
    ///
    /// * `error_rate_warning` - Error rate percentage for warning alerts
    /// * `error_rate_critical` - Error rate percentage for critical alerts
    /// * `avg_time_warning_ms` - Average extraction time threshold for warnings
    /// * `cache_hit_rate_warning` - Cache hit rate percentage threshold for warnings
    pub fn with_thresholds(
        error_rate_warning: f64,
        error_rate_critical: f64,
        avg_time_warning_ms: f64,
        cache_hit_rate_warning: f64,
    ) -> Self {
        Self {
            total_extractions: AtomicUsize::new(0),
            llm_extractions: AtomicUsize::new(0),
            frequency_extractions: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            fallbacks: AtomicUsize::new(0),
            total_time_ns: AtomicU64::new(0),
            latency_tracker: Arc::new(parking_lot::Mutex::new(PercentileTracker::new(1000))),
            error_rate_warning_threshold: error_rate_warning,
            error_rate_critical_threshold: error_rate_critical,
            avg_time_warning_threshold_ms: avg_time_warning_ms,
            cache_hit_rate_warning_threshold: cache_hit_rate_warning,
            start_time: Instant::now(),
        }
    }

    /// Record a successful extraction
    pub fn record_extraction(&self, method: &str, duration_ms: f64) {
        self.total_extractions.fetch_add(1, Ordering::Relaxed);

        if method == "llm" {
            self.llm_extractions.fetch_add(1, Ordering::Relaxed);
        } else if method == "frequency" {
            self.frequency_extractions.fetch_add(1, Ordering::Relaxed);
        }

        self.record_extraction_time(duration_ms);
    }

    /// Record extraction time
    pub fn record_extraction_time(&self, duration_ms: f64) {
        let duration_ns = (duration_ms * 1_000_000.0) as u64;
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);

        let mut tracker = self.latency_tracker.lock();
        tracker.record(duration_ms);
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record extraction error
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record fallback to frequency extraction
    pub fn record_fallback(&self) {
        self.fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total = self.total_extractions.load(Ordering::Relaxed);
        let llm = self.llm_extractions.load(Ordering::Relaxed);
        let frequency = self.frequency_extractions.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let fallbacks = self.fallbacks.load(Ordering::Relaxed);
        let total_time_ns = self.total_time_ns.load(Ordering::Relaxed);

        let tracker = self.latency_tracker.lock();
        let p50 = tracker.p50();
        let p95 = tracker.p95();
        let p99 = tracker.p99();
        drop(tracker);

        MetricsSnapshot {
            total_extractions: total,
            llm_extractions: llm,
            frequency_extractions: frequency,
            cache_hits,
            cache_misses,
            errors,
            fallbacks,
            total_time_ms: total_time_ns as f64 / 1_000_000.0,
            latency_p50_ms: p50,
            latency_p95_ms: p95,
            latency_p99_ms: p99,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Calculate current error rate
    pub fn error_rate(&self) -> f64 {
        let total = self.total_extractions.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        if total > 0 {
            (errors as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate current cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average extraction time
    pub fn avg_extraction_time_ms(&self) -> f64 {
        let total = self.total_extractions.load(Ordering::Relaxed);
        let total_time_ns = self.total_time_ns.load(Ordering::Relaxed);

        if total > 0 {
            (total_time_ns as f64 / 1_000_000.0) / total as f64
        } else {
            0.0
        }
    }

    /// Check for alert conditions and generate alerts
    pub fn check_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();

        // Check error rate
        let error_rate = self.error_rate();
        if error_rate >= self.error_rate_critical_threshold {
            alerts.push(
                Alert::new(
                    AlertLevel::Critical,
                    "High Error Rate".to_string(),
                    format!(
                        "Error rate is {:.1}% (threshold: {:.1}%)",
                        error_rate, self.error_rate_critical_threshold
                    ),
                )
                .with_metadata("error_rate".to_string(), format!("{:.1}", error_rate))
                .with_metadata(
                    "threshold".to_string(),
                    format!("{:.1}", self.error_rate_critical_threshold),
                ),
            );
        } else if error_rate >= self.error_rate_warning_threshold {
            alerts.push(
                Alert::new(
                    AlertLevel::Warning,
                    "Elevated Error Rate".to_string(),
                    format!(
                        "Error rate is {:.1}% (threshold: {:.1}%)",
                        error_rate, self.error_rate_warning_threshold
                    ),
                )
                .with_metadata("error_rate".to_string(), format!("{:.1}", error_rate))
                .with_metadata(
                    "threshold".to_string(),
                    format!("{:.1}", self.error_rate_warning_threshold),
                ),
            );
        }

        // Check average extraction time
        let avg_time = self.avg_extraction_time_ms();
        if avg_time >= self.avg_time_warning_threshold_ms {
            alerts.push(
                Alert::new(
                    AlertLevel::Warning,
                    "High Average Extraction Time".to_string(),
                    format!(
                        "Average extraction time is {:.2}ms (threshold: {:.2}ms)",
                        avg_time, self.avg_time_warning_threshold_ms
                    ),
                )
                .with_metadata("avg_time_ms".to_string(), format!("{:.2}", avg_time))
                .with_metadata(
                    "threshold_ms".to_string(),
                    format!("{:.2}", self.avg_time_warning_threshold_ms),
                ),
            );
        }

        // Check cache hit rate
        let cache_hit_rate = self.cache_hit_rate();
        let total_cache_ops =
            self.cache_hits.load(Ordering::Relaxed) + self.cache_misses.load(Ordering::Relaxed);
        if total_cache_ops > 100 && cache_hit_rate < self.cache_hit_rate_warning_threshold {
            alerts.push(
                Alert::new(
                    AlertLevel::Warning,
                    "Low Cache Hit Rate".to_string(),
                    format!(
                        "Cache hit rate is {:.1}% (threshold: {:.1}%)",
                        cache_hit_rate, self.cache_hit_rate_warning_threshold
                    ),
                )
                .with_metadata(
                    "cache_hit_rate".to_string(),
                    format!("{:.1}", cache_hit_rate),
                )
                .with_metadata(
                    "threshold".to_string(),
                    format!("{:.1}", self.cache_hit_rate_warning_threshold),
                ),
            );
        }

        alerts
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        // Help and type declarations
        output.push_str("# HELP keyword_extraction_total Total number of keyword extractions\n");
        output.push_str("# TYPE keyword_extraction_total counter\n");
        output.push_str(&format!(
            "keyword_extraction_total {}\n\n",
            snapshot.total_extractions
        ));

        output.push_str("# HELP keyword_extraction_by_method Number of extractions by method\n");
        output.push_str("# TYPE keyword_extraction_by_method counter\n");
        output.push_str(&format!(
            "keyword_extraction_by_method{{method=\"llm\"}} {}\n",
            snapshot.llm_extractions
        ));
        output.push_str(&format!(
            "keyword_extraction_by_method{{method=\"frequency\"}} {}\n\n",
            snapshot.frequency_extractions
        ));

        output.push_str("# HELP keyword_extraction_cache_hits Total cache hits\n");
        output.push_str("# TYPE keyword_extraction_cache_hits counter\n");
        output.push_str(&format!(
            "keyword_extraction_cache_hits {}\n\n",
            snapshot.cache_hits
        ));

        output.push_str("# HELP keyword_extraction_cache_misses Total cache misses\n");
        output.push_str("# TYPE keyword_extraction_cache_misses counter\n");
        output.push_str(&format!(
            "keyword_extraction_cache_misses {}\n\n",
            snapshot.cache_misses
        ));

        output.push_str("# HELP keyword_extraction_errors Total extraction errors\n");
        output.push_str("# TYPE keyword_extraction_errors counter\n");
        output.push_str(&format!(
            "keyword_extraction_errors {}\n\n",
            snapshot.errors
        ));

        output.push_str(
            "# HELP keyword_extraction_fallbacks Total fallbacks to frequency extraction\n",
        );
        output.push_str("# TYPE keyword_extraction_fallbacks counter\n");
        output.push_str(&format!(
            "keyword_extraction_fallbacks {}\n\n",
            snapshot.fallbacks
        ));

        output
            .push_str("# HELP keyword_extraction_latency_seconds Extraction latency percentiles\n");
        output.push_str("# TYPE keyword_extraction_latency_seconds summary\n");
        output.push_str(&format!(
            "keyword_extraction_latency_seconds{{quantile=\"0.5\"}} {}\n",
            snapshot.latency_p50_ms / 1000.0
        ));
        output.push_str(&format!(
            "keyword_extraction_latency_seconds{{quantile=\"0.95\"}} {}\n",
            snapshot.latency_p95_ms / 1000.0
        ));
        output.push_str(&format!(
            "keyword_extraction_latency_seconds{{quantile=\"0.99\"}} {}\n\n",
            snapshot.latency_p99_ms / 1000.0
        ));

        output.push_str(
            "# HELP keyword_extraction_uptime_seconds Time since metrics collector started\n",
        );
        output.push_str("# TYPE keyword_extraction_uptime_seconds gauge\n");
        output.push_str(&format!(
            "keyword_extraction_uptime_seconds {}\n",
            snapshot.uptime_seconds
        ));

        output
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_extractions.store(0, Ordering::Relaxed);
        self.llm_extractions.store(0, Ordering::Relaxed);
        self.frequency_extractions.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.fallbacks.store(0, Ordering::Relaxed);
        self.total_time_ns.store(0, Ordering::Relaxed);

        let mut tracker = self.latency_tracker.lock();
        *tracker = PercentileTracker::new(1000);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_extractions: usize,
    pub llm_extractions: usize,
    pub frequency_extractions: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub errors: usize,
    pub fallbacks: usize,
    pub total_time_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub uptime_seconds: u64,
}

impl MetricsSnapshot {
    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_extractions > 0 {
            (self.errors as f64 / self.total_extractions as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate fallback rate
    pub fn fallback_rate(&self) -> f64 {
        if self.total_extractions > 0 {
            (self.fallbacks as f64 / self.total_extractions as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average extraction time
    pub fn avg_extraction_time_ms(&self) -> f64 {
        if self.total_extractions > 0 {
            self.total_time_ms / self.total_extractions as f64
        } else {
            0.0
        }
    }

    /// Export as JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_extractions": self.total_extractions,
            "methods": {
                "llm": self.llm_extractions,
                "frequency": self.frequency_extractions,
            },
            "cache": {
                "hits": self.cache_hits,
                "misses": self.cache_misses,
                "hit_rate": format!("{:.1}%", self.cache_hit_rate()),
            },
            "errors": {
                "count": self.errors,
                "rate": format!("{:.1}%", self.error_rate()),
            },
            "fallbacks": {
                "count": self.fallbacks,
                "rate": format!("{:.1}%", self.fallback_rate()),
            },
            "latency_ms": {
                "avg": format!("{:.2}", self.avg_extraction_time_ms()),
                "p50": format!("{:.2}", self.latency_p50_ms),
                "p95": format!("{:.2}", self.latency_p95_ms),
                "p99": format!("{:.2}", self.latency_p99_ms),
            },
            "uptime_seconds": self.uptime_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_level_as_str() {
        assert_eq!(AlertLevel::Info.as_str(), "info");
        assert_eq!(AlertLevel::Warning.as_str(), "warning");
        assert_eq!(AlertLevel::Critical.as_str(), "critical");
    }

    #[test]
    fn test_alert_new() {
        let alert = Alert::new(
            AlertLevel::Warning,
            "Test Alert".to_string(),
            "Test message".to_string(),
        );

        assert_eq!(alert.level, AlertLevel::Warning);
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.message, "Test message");
        assert!(alert.metadata.is_empty());
    }

    #[test]
    fn test_alert_with_metadata() {
        let alert = Alert::new(AlertLevel::Info, "Test".to_string(), "Message".to_string())
            .with_metadata("key".to_string(), "value".to_string());

        assert_eq!(alert.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_percentile_tracker_record() {
        let mut tracker = PercentileTracker::new(100);
        tracker.record(100.0);
        tracker.record(200.0);
        tracker.record(300.0);

        assert_eq!(tracker.samples.len(), 3);
    }

    #[test]
    fn test_percentile_tracker_percentiles() {
        let mut tracker = PercentileTracker::new(100);
        for i in 1..=100 {
            tracker.record(i as f64);
        }

        let p50 = tracker.p50();
        let p95 = tracker.p95();
        let p99 = tracker.p99();

        assert!(p50 >= 45.0 && p50 <= 55.0);
        assert!(p95 >= 90.0 && p95 <= 100.0);
        assert!(p99 >= 95.0 && p99 <= 100.0);
    }

    #[test]
    fn test_metrics_collector_new() {
        let collector = MetricsCollector::new();
        let snapshot = collector.snapshot();

        assert_eq!(snapshot.total_extractions, 0);
        assert_eq!(snapshot.errors, 0);
    }

    #[test]
    fn test_metrics_collector_record_extraction() {
        let collector = MetricsCollector::new();
        collector.record_extraction("llm", 100.0);
        collector.record_extraction("frequency", 50.0);

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_extractions, 2);
        assert_eq!(snapshot.llm_extractions, 1);
        assert_eq!(snapshot.frequency_extractions, 1);
    }

    #[test]
    fn test_metrics_collector_record_cache() {
        let collector = MetricsCollector::new();
        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.cache_hits, 2);
        assert_eq!(snapshot.cache_misses, 1);
    }

    #[test]
    fn test_metrics_collector_record_error() {
        let collector = MetricsCollector::new();
        collector.record_error();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.errors, 1);
    }

    #[test]
    fn test_metrics_collector_record_fallback() {
        let collector = MetricsCollector::new();
        collector.record_fallback();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.fallbacks, 1);
    }

    #[test]
    fn test_metrics_collector_error_rate() {
        let collector = MetricsCollector::new();
        collector.record_extraction("llm", 100.0);
        collector.record_extraction("llm", 100.0);
        collector.record_error();

        let error_rate = collector.error_rate();
        assert_eq!(error_rate, 50.0);
    }

    #[test]
    fn test_metrics_collector_cache_hit_rate() {
        let collector = MetricsCollector::new();
        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();

        let hit_rate = collector.cache_hit_rate();
        assert!((hit_rate - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector_avg_extraction_time() {
        let collector = MetricsCollector::new();
        collector.record_extraction("llm", 100.0);
        collector.record_extraction("llm", 200.0);

        let avg_time = collector.avg_extraction_time_ms();
        assert_eq!(avg_time, 150.0);
    }

    #[test]
    fn test_metrics_collector_check_alerts_error_rate() {
        let collector = MetricsCollector::with_thresholds(5.0, 10.0, 5000.0, 80.0);

        // Generate high error rate
        for _ in 0..5 {
            collector.record_extraction("llm", 100.0);
        }
        for _ in 0..1 {
            collector.record_error();
        }

        let alerts = collector.check_alerts();
        assert!(!alerts.is_empty());
        assert!(alerts.iter().any(|a| a.title.contains("Error Rate")));
    }

    #[test]
    fn test_metrics_collector_export_prometheus() {
        let collector = MetricsCollector::new();
        collector.record_extraction("llm", 100.0);

        let prometheus = collector.export_prometheus();
        assert!(prometheus.contains("keyword_extraction_total"));
        assert!(prometheus.contains("keyword_extraction_by_method"));
        assert!(prometheus.contains("keyword_extraction_cache_hits"));
    }

    #[test]
    fn test_metrics_collector_reset() {
        let collector = MetricsCollector::new();
        collector.record_extraction("llm", 100.0);
        collector.reset();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_extractions, 0);
    }

    #[test]
    fn test_metrics_snapshot_cache_hit_rate() {
        let snapshot = MetricsSnapshot {
            total_extractions: 10,
            llm_extractions: 5,
            frequency_extractions: 5,
            cache_hits: 8,
            cache_misses: 2,
            errors: 0,
            fallbacks: 0,
            total_time_ms: 1000.0,
            latency_p50_ms: 100.0,
            latency_p95_ms: 150.0,
            latency_p99_ms: 180.0,
            uptime_seconds: 3600,
        };

        assert_eq!(snapshot.cache_hit_rate(), 80.0);
    }

    #[test]
    fn test_metrics_snapshot_error_rate() {
        let snapshot = MetricsSnapshot {
            total_extractions: 10,
            llm_extractions: 5,
            frequency_extractions: 5,
            cache_hits: 0,
            cache_misses: 0,
            errors: 2,
            fallbacks: 0,
            total_time_ms: 1000.0,
            latency_p50_ms: 100.0,
            latency_p95_ms: 150.0,
            latency_p99_ms: 180.0,
            uptime_seconds: 3600,
        };

        assert_eq!(snapshot.error_rate(), 20.0);
    }

    #[test]
    fn test_metrics_snapshot_to_json() {
        let snapshot = MetricsSnapshot {
            total_extractions: 10,
            llm_extractions: 5,
            frequency_extractions: 5,
            cache_hits: 8,
            cache_misses: 2,
            errors: 1,
            fallbacks: 2,
            total_time_ms: 1000.0,
            latency_p50_ms: 100.0,
            latency_p95_ms: 150.0,
            latency_p99_ms: 180.0,
            uptime_seconds: 3600,
        };

        let json = snapshot.to_json();
        assert_eq!(json["total_extractions"], 10);
        assert!(json["cache"]["hit_rate"].is_string());
        assert!(json["latency_ms"]["p95"].is_string());
    }
}
