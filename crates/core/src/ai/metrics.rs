//! Metrics collection for intent classification
//!
//! This module provides Prometheus metrics for monitoring intent classification
//! performance, cache effectiveness, and error rates.
//!
//! # Features
//!
//! This module is only available when the `metrics` feature is enabled.
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "metrics")]
//! # {
//! use xze_core::ai::metrics::ClassifierMetrics;
//! use xze_core::ai::intent_types::DiataxisIntent;
//!
//! let metrics = ClassifierMetrics::new();
//! metrics.record_classification(150, true, &DiataxisIntent::Tutorial);
//! metrics.record_cache_hit();
//! # }
//! ```

#[cfg(feature = "metrics")]
use prometheus::{HistogramVec, IntCounterVec, IntGauge, Registry};

#[cfg(feature = "metrics")]
use once_cell::sync::Lazy;

#[cfg(feature = "metrics")]
use crate::ai::intent_types::DiataxisIntent;

#[cfg(feature = "metrics")]
static GLOBAL_METRICS: Lazy<ClassifierMetrics> = Lazy::new(|| {
    ClassifierMetrics::with_registry(prometheus::default_registry())
        .expect("Failed to register classifier metrics")
});

/// Metrics collector for intent classification
///
/// Tracks classification performance, cache effectiveness, and error rates
/// using Prometheus metrics.
#[cfg(feature = "metrics")]
#[derive(Clone, Debug)]
pub struct ClassifierMetrics {
    /// Histogram of classification latencies in milliseconds
    classification_duration: HistogramVec,

    /// Counter for total classifications performed
    classifications_total: IntCounterVec,

    /// Counter for cache hits and misses
    cache_operations: IntCounterVec,

    /// Counter for classification errors
    classification_errors: IntCounterVec,

    /// Gauge for current cache size
    cache_size: IntGauge,

    /// Counter for intent distribution
    intent_distribution: IntCounterVec,
}

#[cfg(feature = "metrics")]
impl ClassifierMetrics {
    /// Returns a reference to the global metrics instance
    ///
    /// This ensures metrics are registered only once per process, avoiding
    /// Prometheus registry collisions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::global();
    /// # }
    /// ```
    pub fn global() -> &'static ClassifierMetrics {
        &GLOBAL_METRICS
    }

    /// Creates a new metrics collector (alias for global())
    ///
    /// Returns a clone of the global metrics instance. This method exists
    /// for backward compatibility.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// # }
    /// ```
    pub fn new() -> Self {
        GLOBAL_METRICS.clone()
    }

    /// Creates a new metrics collector with custom registry
    ///
    /// # Arguments
    ///
    /// * `registry` - Prometheus registry to use for metrics
    ///
    /// # Returns
    ///
    /// Returns `Ok(ClassifierMetrics)` on success or error if registration fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    /// use prometheus::Registry;
    ///
    /// let registry = Registry::new();
    /// let metrics = ClassifierMetrics::with_registry(&registry).unwrap();
    /// # }
    /// ```
    pub fn with_registry(registry: &Registry) -> Result<Self, prometheus::Error> {
        let classification_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "xze_classifier_duration_milliseconds",
                "Duration of intent classification requests in milliseconds",
            )
            .buckets(vec![
                10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
            ]),
            &["model", "cached"],
        )?;
        registry.register(Box::new(classification_duration.clone()))?;

        let classifications_total = IntCounterVec::new(
            prometheus::Opts::new(
                "xze_classifications_total",
                "Total number of intent classifications performed",
            ),
            &["model", "primary_intent", "multi_intent"],
        )?;
        registry.register(Box::new(classifications_total.clone()))?;

        let cache_operations = IntCounterVec::new(
            prometheus::Opts::new(
                "xze_classifier_cache_operations_total",
                "Total number of cache operations",
            ),
            &["operation"],
        )?;
        registry.register(Box::new(cache_operations.clone()))?;

        let classification_errors = IntCounterVec::new(
            prometheus::Opts::new(
                "xze_classifier_errors_total",
                "Total number of classification errors",
            ),
            &["error_type"],
        )?;
        registry.register(Box::new(classification_errors.clone()))?;

        let cache_size = IntGauge::new(
            "xze_classifier_cache_size",
            "Current number of entries in classification cache",
        )?;
        registry.register(Box::new(cache_size.clone()))?;

        let intent_distribution = IntCounterVec::new(
            prometheus::Opts::new(
                "xze_classifier_intent_distribution_total",
                "Distribution of classified intents",
            ),
            &["intent"],
        )?;
        registry.register(Box::new(intent_distribution.clone()))?;

        Ok(Self {
            classification_duration,
            classifications_total,
            cache_operations,
            classification_errors,
            cache_size,
            intent_distribution,
        })
    }

    /// Records a classification operation
    ///
    /// # Arguments
    ///
    /// * `duration_ms` - Duration of the classification in milliseconds
    /// * `cached` - Whether the result came from cache
    /// * `primary_intent` - The primary intent that was classified
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    /// use xze_core::ai::intent_types::DiataxisIntent;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_classification(150, false, &DiataxisIntent::HowTo);
    /// # }
    /// ```
    pub fn record_classification(
        &self,
        duration_ms: u64,
        cached: bool,
        primary_intent: &DiataxisIntent,
    ) {
        let model = "default"; // Could be passed as parameter
        let cached_label = if cached { "true" } else { "false" };

        self.classification_duration
            .with_label_values(&[model, cached_label])
            .observe(duration_ms as f64);

        self.classifications_total
            .with_label_values(&[model, &primary_intent.to_string(), "false"])
            .inc();

        self.intent_distribution
            .with_label_values(&[&primary_intent.to_string()])
            .inc();
    }

    /// Records a multi-intent classification operation
    ///
    /// # Arguments
    ///
    /// * `duration_ms` - Duration of the classification in milliseconds
    /// * `cached` - Whether the result came from cache
    /// * `primary_intent` - The primary intent that was classified
    /// * `secondary_count` - Number of secondary intents detected
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    /// use xze_core::ai::intent_types::DiataxisIntent;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_multi_intent_classification(200, false, &DiataxisIntent::Explanation, 2);
    /// # }
    /// ```
    pub fn record_multi_intent_classification(
        &self,
        duration_ms: u64,
        cached: bool,
        primary_intent: &DiataxisIntent,
        secondary_count: usize,
    ) {
        let model = "default";
        let cached_label = if cached { "true" } else { "false" };

        self.classification_duration
            .with_label_values(&[model, cached_label])
            .observe(duration_ms as f64);

        self.classifications_total
            .with_label_values(&[model, &primary_intent.to_string(), "true"])
            .inc();

        self.intent_distribution
            .with_label_values(&[&primary_intent.to_string()])
            .inc();

        // Record secondary intents
        for _ in 0..secondary_count {
            self.intent_distribution
                .with_label_values(&["secondary"])
                .inc();
        }
    }

    /// Records a cache hit
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_cache_hit();
    /// # }
    /// ```
    pub fn record_cache_hit(&self) {
        self.cache_operations.with_label_values(&["hit"]).inc();
    }

    /// Records a cache miss
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_cache_miss();
    /// # }
    /// ```
    pub fn record_cache_miss(&self) {
        self.cache_operations.with_label_values(&["miss"]).inc();
    }

    /// Records a cache clear operation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_cache_clear();
    /// # }
    /// ```
    pub fn record_cache_clear(&self) {
        self.cache_operations.with_label_values(&["clear"]).inc();
    }

    /// Updates the cache size gauge
    ///
    /// # Arguments
    ///
    /// * `size` - Current number of entries in the cache
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.set_cache_size(42);
    /// # }
    /// ```
    pub fn set_cache_size(&self, size: u64) {
        self.cache_size.set(size as i64);
    }

    /// Records a classification error
    ///
    /// # Arguments
    ///
    /// * `error_type` - Type of error that occurred
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// use xze_core::ai::metrics::ClassifierMetrics;
    ///
    /// let metrics = ClassifierMetrics::new();
    /// metrics.record_error("parse_error");
    /// # }
    /// ```
    pub fn record_error(&self, error_type: &str) {
        self.classification_errors
            .with_label_values(&[error_type])
            .inc();
    }
}

#[cfg(feature = "metrics")]
impl Default for ClassifierMetrics {
    fn default() -> Self {
        GLOBAL_METRICS.clone()
    }
}

/// Stub metrics collector when metrics feature is disabled
///
/// This no-op implementation allows the code to compile and run
/// without the metrics feature enabled.
#[cfg(not(feature = "metrics"))]
#[derive(Clone, Default, Debug)]
pub struct ClassifierMetrics;

#[cfg(not(feature = "metrics"))]
impl ClassifierMetrics {
    /// Creates a new no-op metrics collector
    pub fn new() -> Self {
        Self
    }

    /// No-op: records a classification operation
    pub fn record_classification(
        &self,
        _duration_ms: u64,
        _cached: bool,
        _primary_intent: &crate::ai::intent_types::DiataxisIntent,
    ) {
    }

    /// No-op: records a multi-intent classification operation
    pub fn record_multi_intent_classification(
        &self,
        _duration_ms: u64,
        _cached: bool,
        _primary_intent: &crate::ai::intent_types::DiataxisIntent,
        _secondary_count: usize,
    ) {
    }

    /// No-op: records a cache hit
    pub fn record_cache_hit(&self) {}

    /// No-op: records a cache miss
    pub fn record_cache_miss(&self) {}

    /// No-op: records a cache clear operation
    pub fn record_cache_clear(&self) {}

    /// No-op: updates the cache size gauge
    pub fn set_cache_size(&self, _size: u64) {}

    /// No-op: records a classification error
    pub fn record_error(&self, _error_type: &str) {}
}

#[cfg(all(test, feature = "metrics"))]
mod tests {
    use super::*;
    use prometheus::Registry;

    #[test]
    fn test_metrics_creation() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry);
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_record_classification() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry).unwrap();

        metrics.record_classification(100, false, &DiataxisIntent::Tutorial);
        metrics.record_classification(50, true, &DiataxisIntent::HowTo);

        // Verify metrics were recorded (basic smoke test)
        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_record_multi_intent_classification() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry).unwrap();

        metrics.record_multi_intent_classification(200, false, &DiataxisIntent::Explanation, 2);

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_cache_operations() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry).unwrap();

        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_cache_clear();
        metrics.set_cache_size(100);

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_record_error() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry).unwrap();

        metrics.record_error("parse_error");
        metrics.record_error("service_unavailable");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_intent_distribution() {
        let registry = Registry::new();
        let metrics = ClassifierMetrics::with_registry(&registry).unwrap();

        metrics.record_classification(100, false, &DiataxisIntent::Tutorial);
        metrics.record_classification(100, false, &DiataxisIntent::HowTo);
        metrics.record_classification(100, false, &DiataxisIntent::Reference);

        let metric_families = registry.gather();
        let intent_metric = metric_families
            .iter()
            .find(|mf| mf.get_name() == "xze_classifier_intent_distribution_total");
        assert!(intent_metric.is_some());
    }
}

#[cfg(all(test, not(feature = "metrics")))]
mod tests_no_metrics {
    use super::*;

    #[test]
    fn test_stub_metrics_no_panic() {
        let metrics = ClassifierMetrics::new();

        // All operations should be no-ops and not panic
        metrics.record_classification(
            100,
            false,
            &crate::ai::intent_types::DiataxisIntent::Tutorial,
        );
        metrics.record_multi_intent_classification(
            200,
            false,
            &crate::ai::intent_types::DiataxisIntent::HowTo,
            2,
        );
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_cache_clear();
        metrics.set_cache_size(100);
        metrics.record_error("test_error");
    }
}
