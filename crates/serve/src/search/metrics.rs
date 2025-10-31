//! Search-specific metrics for monitoring and observability
//!
//! Provides Prometheus metrics for tracking search request performance,
//! cache hit rates, and result quality.

#[cfg(feature = "metrics")]
use once_cell::sync::Lazy;
#[cfg(feature = "metrics")]
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec,
};

#[cfg(feature = "metrics")]
/// Global search metrics instance
pub static SEARCH_METRICS: Lazy<SearchMetrics> =
    Lazy::new(|| SearchMetrics::new().expect("Failed to initialize search metrics"));

/// Search metrics collector
#[cfg(feature = "metrics")]
pub struct SearchMetrics {
    /// Total number of search requests by endpoint and status
    pub search_requests_total: IntCounterVec,
    /// Search request duration in seconds
    pub search_duration_seconds: HistogramVec,
    /// Number of results returned
    pub search_results_total: IntCounterVec,
    /// Cache hit/miss counter
    pub cache_operations_total: IntCounterVec,
    /// Current cache size
    pub cache_size: IntGaugeVec,
    /// Cache hit rate
    pub cache_hit_rate: IntGaugeVec,
    /// Search errors by type
    pub search_errors_total: IntCounterVec,
    /// Query complexity (estimated)
    pub query_complexity: HistogramVec,
}

#[cfg(feature = "metrics")]
impl SearchMetrics {
    /// Creates a new search metrics instance
    ///
    /// # Returns
    ///
    /// Returns a Result with SearchMetrics or a prometheus error
    ///
    /// # Errors
    ///
    /// Returns error if metrics registration fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::search::metrics::SearchMetrics;
    ///
    /// let metrics = SearchMetrics::new();
    /// assert!(metrics.is_ok());
    /// ```
    pub fn new() -> Result<Self, prometheus::Error> {
        let search_requests_total = register_int_counter_vec!(
            "xze_search_requests_total",
            "Total number of search requests",
            &["endpoint", "method", "status"]
        )?;

        let search_duration_seconds = register_histogram_vec!(
            "xze_search_duration_seconds",
            "Search request duration in seconds",
            &["endpoint", "method"],
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        )?;

        let search_results_total = register_int_counter_vec!(
            "xze_search_results_total",
            "Total number of search results returned",
            &["endpoint", "category"]
        )?;

        let cache_operations_total = register_int_counter_vec!(
            "xze_search_cache_operations_total",
            "Total cache operations",
            &["operation"]
        )?;

        let cache_size = register_int_gauge_vec!(
            "xze_search_cache_size",
            "Current number of entries in cache",
            &["cache_type"]
        )?;

        let cache_hit_rate = register_int_gauge_vec!(
            "xze_search_cache_hit_rate_percent",
            "Cache hit rate percentage",
            &["cache_type"]
        )?;

        let search_errors_total = register_int_counter_vec!(
            "xze_search_errors_total",
            "Total search errors",
            &["endpoint", "error_type"]
        )?;

        let query_complexity = register_histogram_vec!(
            "xze_search_query_complexity",
            "Estimated query complexity score",
            &["endpoint"],
            vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]
        )?;

        Ok(Self {
            search_requests_total,
            search_duration_seconds,
            search_results_total,
            cache_operations_total,
            cache_size,
            cache_hit_rate,
            search_errors_total,
            query_complexity,
        })
    }

    /// Records a search request
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint name
    /// * `method` - HTTP method
    /// * `status` - Response status code
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_request("/api/v1/search", "GET", "200");
    /// ```
    pub fn record_request(&self, endpoint: &str, method: &str, status: &str) {
        self.search_requests_total
            .with_label_values(&[endpoint, method, status])
            .inc();
    }

    /// Records search duration
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint name
    /// * `method` - HTTP method
    /// * `duration` - Request duration in seconds
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_duration("/api/v1/search", "GET", 0.5);
    /// ```
    pub fn record_duration(&self, endpoint: &str, method: &str, duration: f64) {
        self.search_duration_seconds
            .with_label_values(&[endpoint, method])
            .observe(duration);
    }

    /// Records search results count
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint name
    /// * `category` - Result category
    /// * `count` - Number of results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_results("/api/v1/search", "tutorial", 10);
    /// ```
    pub fn record_results(&self, endpoint: &str, category: &str, count: u64) {
        self.search_results_total
            .with_label_values(&[endpoint, category])
            .inc_by(count);
    }

    /// Records a cache operation
    ///
    /// # Arguments
    ///
    /// * `operation` - Operation type (hit, miss, set, invalidate)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_cache_operation("hit");
    /// SEARCH_METRICS.record_cache_operation("miss");
    /// ```
    pub fn record_cache_operation(&self, operation: &str) {
        self.cache_operations_total
            .with_label_values(&[operation])
            .inc();
    }

    /// Updates cache size metric
    ///
    /// # Arguments
    ///
    /// * `cache_type` - Type of cache (search, embedding, etc.)
    /// * `size` - Current size
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.update_cache_size("search", 1000);
    /// ```
    pub fn update_cache_size(&self, cache_type: &str, size: i64) {
        self.cache_size.with_label_values(&[cache_type]).set(size);
    }

    /// Updates cache hit rate metric
    ///
    /// # Arguments
    ///
    /// * `cache_type` - Type of cache
    /// * `hit_rate` - Hit rate percentage (0-100)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.update_cache_hit_rate("search", 75);
    /// ```
    pub fn update_cache_hit_rate(&self, cache_type: &str, hit_rate: i64) {
        self.cache_hit_rate
            .with_label_values(&[cache_type])
            .set(hit_rate);
    }

    /// Records a search error
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint name
    /// * `error_type` - Type of error
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_error("/api/v1/search", "validation_error");
    /// ```
    pub fn record_error(&self, endpoint: &str, error_type: &str) {
        self.search_errors_total
            .with_label_values(&[endpoint, error_type])
            .inc();
    }

    /// Records query complexity
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint name
    /// * `complexity` - Complexity score
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_serve::search::metrics::SEARCH_METRICS;
    ///
    /// SEARCH_METRICS.record_query_complexity("/api/v1/search", 10.0);
    /// ```
    pub fn record_query_complexity(&self, endpoint: &str, complexity: f64) {
        self.query_complexity
            .with_label_values(&[endpoint])
            .observe(complexity);
    }
}

/// No-op metrics implementation when metrics feature is disabled
#[cfg(not(feature = "metrics"))]
pub struct SearchMetrics;

#[cfg(not(feature = "metrics"))]
impl SearchMetrics {
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn record_request(&self, _endpoint: &str, _method: &str, _status: &str) {}
    pub fn record_duration(&self, _endpoint: &str, _method: &str, _duration: f64) {}
    pub fn record_results(&self, _endpoint: &str, _category: &str, _count: u64) {}
    pub fn record_cache_operation(&self, _operation: &str) {}
    pub fn update_cache_size(&self, _cache_type: &str, _size: i64) {}
    pub fn update_cache_hit_rate(&self, _cache_type: &str, _hit_rate: i64) {}
    pub fn record_error(&self, _endpoint: &str, _error_type: &str) {}
    pub fn record_query_complexity(&self, _endpoint: &str, _complexity: f64) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "metrics")]
    #[test]
    fn test_search_metrics_new() {
        // Test that global metrics is initialized
        let _ = &*SEARCH_METRICS;
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_request() {
        SEARCH_METRICS.record_request("/api/v1/search", "GET", "200");
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_duration() {
        SEARCH_METRICS.record_duration("/api/v1/search", "GET", 0.5);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_results() {
        SEARCH_METRICS.record_results("/api/v1/search", "tutorial", 10);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_cache_operation() {
        SEARCH_METRICS.record_cache_operation("hit");
        SEARCH_METRICS.record_cache_operation("miss");
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_update_cache_size() {
        SEARCH_METRICS.update_cache_size("search", 1000);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_update_cache_hit_rate() {
        SEARCH_METRICS.update_cache_hit_rate("search", 75);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_error() {
        SEARCH_METRICS.record_error("/api/v1/search", "validation_error");
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_query_complexity() {
        SEARCH_METRICS.record_query_complexity("/api/v1/search", 10.0);
    }

    #[cfg(not(feature = "metrics"))]
    #[test]
    fn test_no_op_metrics() {
        let metrics = SearchMetrics::new().unwrap();
        metrics.record_request("/api/v1/search", "GET", "200");
        metrics.record_duration("/api/v1/search", "GET", 0.5);
        metrics.record_results("/api/v1/search", "tutorial", 10);
        metrics.record_cache_operation("hit");
        metrics.update_cache_size("search", 1000);
        metrics.update_cache_hit_rate("search", 75);
        metrics.record_error("/api/v1/search", "validation_error");
        metrics.record_query_complexity("/api/v1/search", 10.0);
    }
}
