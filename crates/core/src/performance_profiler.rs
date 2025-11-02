//! Performance profiling and optimization for keyword extraction
//!
//! This module provides profiling instrumentation, performance metrics,
//! and optimization helpers for the keyword extraction pipeline.
//!
//! # Examples
//!
//! ```
//! use xze_core::performance_profiler::{PerformanceProfiler, ProfileScope};
//!
//! let mut profiler = PerformanceProfiler::new();
//! let scope = profiler.start_scope("llm_extraction");
//! // ... perform extraction ...
//! profiler.end_scope(scope);
//!
//! let report = profiler.generate_report();
//! println!("Total time: {}ms", report.total_time_ms);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance profile scope identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileScope(usize);

/// Timing information for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    /// Number of times this operation was executed
    pub count: usize,

    /// Total duration across all executions
    pub total_duration_ms: f64,

    /// Minimum duration observed
    pub min_duration_ms: f64,

    /// Maximum duration observed
    pub max_duration_ms: f64,

    /// Last recorded duration
    pub last_duration_ms: f64,
}

impl TimingInfo {
    /// Create new timing info
    fn new() -> Self {
        Self {
            count: 0,
            total_duration_ms: 0.0,
            min_duration_ms: f64::MAX,
            max_duration_ms: 0.0,
            last_duration_ms: 0.0,
        }
    }

    /// Record a new timing measurement
    fn record(&mut self, duration_ms: f64) {
        self.count += 1;
        self.total_duration_ms += duration_ms;
        self.min_duration_ms = self.min_duration_ms.min(duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(duration_ms);
        self.last_duration_ms = duration_ms;
    }

    /// Calculate average duration
    pub fn avg_duration_ms(&self) -> f64 {
        if self.count > 0 {
            self.total_duration_ms / self.count as f64
        } else {
            0.0
        }
    }

    /// Calculate percentile (approximate using min/max/avg)
    pub fn p95_duration_ms(&self) -> f64 {
        // Simple approximation: weighted average closer to max
        let avg = self.avg_duration_ms();
        avg + (self.max_duration_ms - avg) * 0.7
    }

    /// Calculate percentile (approximate)
    pub fn p99_duration_ms(&self) -> f64 {
        // Simple approximation: weighted average very close to max
        let avg = self.avg_duration_ms();
        avg + (self.max_duration_ms - avg) * 0.9
    }
}

/// Active scope tracking
#[derive(Debug)]
struct ActiveScope {
    name: String,
    start: Instant,
}

/// Performance profiler for keyword extraction pipeline
///
/// Tracks timing information for different operations and generates
/// performance reports with detailed statistics.
#[derive(Debug)]
pub struct PerformanceProfiler {
    timings: HashMap<String, TimingInfo>,
    active_scopes: HashMap<usize, ActiveScope>,
    next_scope_id: usize,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        Self {
            timings: HashMap::new(),
            active_scopes: HashMap::new(),
            next_scope_id: 0,
        }
    }

    /// Start a new profiling scope
    ///
    /// # Arguments
    ///
    /// * `name` - Name identifier for this scope
    ///
    /// # Returns
    ///
    /// Returns a ProfileScope handle that must be passed to end_scope
    pub fn start_scope(&mut self, name: &str) -> ProfileScope {
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;

        self.active_scopes.insert(
            scope_id,
            ActiveScope {
                name: name.to_string(),
                start: Instant::now(),
            },
        );

        ProfileScope(scope_id)
    }

    /// End a profiling scope and record its duration
    ///
    /// # Arguments
    ///
    /// * `scope` - The ProfileScope handle returned from start_scope
    pub fn end_scope(&mut self, scope: ProfileScope) {
        if let Some(active) = self.active_scopes.remove(&scope.0) {
            let duration = active.start.elapsed();
            self.record_timing(&active.name, duration);
        }
    }

    /// Record a timing measurement directly
    ///
    /// # Arguments
    ///
    /// * `name` - Operation name
    /// * `duration` - Duration to record
    pub fn record_timing(&mut self, name: &str, duration: Duration) {
        let duration_ms = duration.as_secs_f64() * 1000.0;
        self.timings
            .entry(name.to_string())
            .or_insert_with(TimingInfo::new)
            .record(duration_ms);
    }

    /// Get timing information for a specific operation
    pub fn get_timing(&self, name: &str) -> Option<&TimingInfo> {
        self.timings.get(name)
    }

    /// Get all timing information
    pub fn all_timings(&self) -> &HashMap<String, TimingInfo> {
        &self.timings
    }

    /// Generate a performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let mut total_time_ms = 0.0;
        let mut operation_reports = Vec::new();

        for (name, timing) in &self.timings {
            total_time_ms += timing.total_duration_ms;
            operation_reports.push(OperationReport {
                name: name.clone(),
                count: timing.count,
                total_ms: timing.total_duration_ms,
                avg_ms: timing.avg_duration_ms(),
                min_ms: timing.min_duration_ms,
                max_ms: timing.max_duration_ms,
                p95_ms: timing.p95_duration_ms(),
                p99_ms: timing.p99_duration_ms(),
            });
        }

        // Sort by total time descending
        operation_reports.sort_by(|a, b| b.total_ms.partial_cmp(&a.total_ms).unwrap());

        PerformanceReport {
            total_time_ms,
            operations: operation_reports,
        }
    }

    /// Reset all timing data
    pub fn reset(&mut self) {
        self.timings.clear();
        self.active_scopes.clear();
    }

    /// Export metrics as JSON
    pub fn to_json(&self) -> serde_json::Value {
        let report = self.generate_report();
        report.to_json()
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Report for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationReport {
    /// Operation name
    pub name: String,

    /// Number of executions
    pub count: usize,

    /// Total time spent in milliseconds
    pub total_ms: f64,

    /// Average time per execution
    pub avg_ms: f64,

    /// Minimum time observed
    pub min_ms: f64,

    /// Maximum time observed
    pub max_ms: f64,

    /// 95th percentile time
    pub p95_ms: f64,

    /// 99th percentile time
    pub p99_ms: f64,
}

/// Complete performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Total time across all operations
    pub total_time_ms: f64,

    /// Reports for individual operations
    pub operations: Vec<OperationReport>,
}

impl PerformanceReport {
    /// Find the slowest operation
    pub fn slowest_operation(&self) -> Option<&OperationReport> {
        self.operations.first()
    }

    /// Get operations sorted by average time
    pub fn by_avg_time(&self) -> Vec<&OperationReport> {
        let mut ops: Vec<_> = self.operations.iter().collect();
        ops.sort_by(|a, b| b.avg_ms.partial_cmp(&a.avg_ms).unwrap());
        ops
    }

    /// Get operations that exceed a time threshold
    pub fn operations_exceeding(&self, threshold_ms: f64) -> Vec<&OperationReport> {
        self.operations
            .iter()
            .filter(|op| op.avg_ms > threshold_ms)
            .collect()
    }

    /// Convert to JSON format
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_time_ms": format!("{:.2}", self.total_time_ms),
            "operations": self.operations.iter().map(|op| {
                serde_json::json!({
                    "name": op.name,
                    "count": op.count,
                    "total_ms": format!("{:.2}", op.total_ms),
                    "avg_ms": format!("{:.2}", op.avg_ms),
                    "min_ms": format!("{:.2}", op.min_ms),
                    "max_ms": format!("{:.2}", op.max_ms),
                    "p95_ms": format!("{:.2}", op.p95_ms),
                    "p99_ms": format!("{:.2}", op.p99_ms),
                })
            }).collect::<Vec<_>>()
        })
    }

    /// Generate a text summary
    pub fn summary(&self) -> String {
        let mut s = "Performance Report\n".to_string();
        s.push_str("==================\n");
        s.push_str(&format!("Total Time: {:.2}ms\n\n", self.total_time_ms));

        for op in &self.operations {
            s.push_str(&format!("{}\n", op.name));
            s.push_str(&format!("  Count: {}\n", op.count));
            s.push_str(&format!("  Avg: {:.2}ms\n", op.avg_ms));
            s.push_str(&format!("  Min: {:.2}ms\n", op.min_ms));
            s.push_str(&format!("  Max: {:.2}ms\n", op.max_ms));
            s.push_str(&format!("  P95: {:.2}ms\n", op.p95_ms));
            s.push_str(&format!("  P99: {:.2}ms\n\n", op.p99_ms));
        }

        s
    }
}

/// Batch processing optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptimizationConfig {
    /// Recommended batch size for small documents (< 1KB)
    pub small_doc_batch_size: usize,

    /// Recommended batch size for medium documents (1-10KB)
    pub medium_doc_batch_size: usize,

    /// Recommended batch size for large documents (> 10KB)
    pub large_doc_batch_size: usize,

    /// Maximum concurrent LLM requests
    pub max_concurrent_requests: usize,

    /// Enable cache warming
    pub enable_cache_warming: bool,

    /// Cache warming batch size
    pub cache_warming_batch_size: usize,
}

impl Default for BatchOptimizationConfig {
    fn default() -> Self {
        Self {
            small_doc_batch_size: 50,
            medium_doc_batch_size: 20,
            large_doc_batch_size: 5,
            max_concurrent_requests: 10,
            enable_cache_warming: true,
            cache_warming_batch_size: 100,
        }
    }
}

impl BatchOptimizationConfig {
    /// Get recommended batch size based on document size
    pub fn recommended_batch_size(&self, doc_size_bytes: usize) -> usize {
        if doc_size_bytes < 1024 {
            self.small_doc_batch_size
        } else if doc_size_bytes < 10240 {
            self.medium_doc_batch_size
        } else {
            self.large_doc_batch_size
        }
    }

    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            small_doc_batch_size: std::env::var("KEYWORD_EXTRACTION_SMALL_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            medium_doc_batch_size: std::env::var("KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),
            large_doc_batch_size: std::env::var("KEYWORD_EXTRACTION_LARGE_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            max_concurrent_requests: std::env::var("KEYWORD_EXTRACTION_MAX_CONCURRENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            enable_cache_warming: std::env::var("KEYWORD_EXTRACTION_CACHE_WARMING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            cache_warming_batch_size: std::env::var("KEYWORD_EXTRACTION_CACHE_WARMING_BATCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timing_info_new() {
        let timing = TimingInfo::new();
        assert_eq!(timing.count, 0);
        assert_eq!(timing.total_duration_ms, 0.0);
    }

    #[test]
    fn test_timing_info_record() {
        let mut timing = TimingInfo::new();
        timing.record(100.0);
        timing.record(200.0);

        assert_eq!(timing.count, 2);
        assert_eq!(timing.total_duration_ms, 300.0);
        assert_eq!(timing.min_duration_ms, 100.0);
        assert_eq!(timing.max_duration_ms, 200.0);
        assert_eq!(timing.last_duration_ms, 200.0);
    }

    #[test]
    fn test_timing_info_avg_duration() {
        let mut timing = TimingInfo::new();
        timing.record(100.0);
        timing.record(200.0);
        assert_eq!(timing.avg_duration_ms(), 150.0);
    }

    #[test]
    fn test_timing_info_percentiles() {
        let mut timing = TimingInfo::new();
        timing.record(100.0);
        timing.record(200.0);

        let p95 = timing.p95_duration_ms();
        let p99 = timing.p99_duration_ms();

        assert!(p95 > timing.avg_duration_ms());
        assert!(p99 > p95);
        assert!(p99 <= timing.max_duration_ms);
    }

    #[test]
    fn test_profiler_new() {
        let profiler = PerformanceProfiler::new();
        assert_eq!(profiler.timings.len(), 0);
        assert_eq!(profiler.active_scopes.len(), 0);
    }

    #[test]
    fn test_profiler_scope() {
        let mut profiler = PerformanceProfiler::new();
        let scope = profiler.start_scope("test_operation");

        thread::sleep(Duration::from_millis(10));

        profiler.end_scope(scope);

        let timing = profiler.get_timing("test_operation").unwrap();
        assert_eq!(timing.count, 1);
        assert!(timing.total_duration_ms >= 10.0);
    }

    #[test]
    fn test_profiler_multiple_scopes() {
        let mut profiler = PerformanceProfiler::new();

        for _ in 0..3 {
            let scope = profiler.start_scope("repeated_op");
            thread::sleep(Duration::from_millis(5));
            profiler.end_scope(scope);
        }

        let timing = profiler.get_timing("repeated_op").unwrap();
        assert_eq!(timing.count, 3);
    }

    #[test]
    fn test_profiler_record_timing() {
        let mut profiler = PerformanceProfiler::new();
        profiler.record_timing("manual_op", Duration::from_millis(100));

        let timing = profiler.get_timing("manual_op").unwrap();
        assert_eq!(timing.count, 1);
        assert!(timing.total_duration_ms >= 99.0 && timing.total_duration_ms <= 101.0);
    }

    #[test]
    fn test_profiler_generate_report() {
        let mut profiler = PerformanceProfiler::new();
        profiler.record_timing("op1", Duration::from_millis(100));
        profiler.record_timing("op2", Duration::from_millis(50));

        let report = profiler.generate_report();
        assert_eq!(report.operations.len(), 2);
        assert!(report.total_time_ms >= 150.0);

        // Should be sorted by total time
        assert_eq!(report.operations[0].name, "op1");
    }

    #[test]
    fn test_profiler_reset() {
        let mut profiler = PerformanceProfiler::new();
        profiler.record_timing("op1", Duration::from_millis(100));
        profiler.reset();

        assert_eq!(profiler.timings.len(), 0);
        assert_eq!(profiler.active_scopes.len(), 0);
    }

    #[test]
    fn test_performance_report_slowest_operation() {
        let report = PerformanceReport {
            total_time_ms: 150.0,
            operations: vec![
                OperationReport {
                    name: "fast_op".to_string(),
                    count: 1,
                    total_ms: 50.0,
                    avg_ms: 50.0,
                    min_ms: 50.0,
                    max_ms: 50.0,
                    p95_ms: 50.0,
                    p99_ms: 50.0,
                },
                OperationReport {
                    name: "slow_op".to_string(),
                    count: 1,
                    total_ms: 100.0,
                    avg_ms: 100.0,
                    min_ms: 100.0,
                    max_ms: 100.0,
                    p95_ms: 100.0,
                    p99_ms: 100.0,
                },
            ],
        };

        let slowest = report.slowest_operation().unwrap();
        assert_eq!(slowest.name, "fast_op"); // First in sorted list
    }

    #[test]
    fn test_performance_report_by_avg_time() {
        let report = PerformanceReport {
            total_time_ms: 150.0,
            operations: vec![
                OperationReport {
                    name: "op1".to_string(),
                    count: 1,
                    total_ms: 50.0,
                    avg_ms: 50.0,
                    min_ms: 50.0,
                    max_ms: 50.0,
                    p95_ms: 50.0,
                    p99_ms: 50.0,
                },
                OperationReport {
                    name: "op2".to_string(),
                    count: 1,
                    total_ms: 100.0,
                    avg_ms: 100.0,
                    min_ms: 100.0,
                    max_ms: 100.0,
                    p95_ms: 100.0,
                    p99_ms: 100.0,
                },
            ],
        };

        let by_avg = report.by_avg_time();
        assert_eq!(by_avg[0].name, "op2");
        assert_eq!(by_avg[1].name, "op1");
    }

    #[test]
    fn test_performance_report_operations_exceeding() {
        let report = PerformanceReport {
            total_time_ms: 150.0,
            operations: vec![
                OperationReport {
                    name: "fast".to_string(),
                    count: 1,
                    total_ms: 50.0,
                    avg_ms: 50.0,
                    min_ms: 50.0,
                    max_ms: 50.0,
                    p95_ms: 50.0,
                    p99_ms: 50.0,
                },
                OperationReport {
                    name: "slow".to_string(),
                    count: 1,
                    total_ms: 100.0,
                    avg_ms: 100.0,
                    min_ms: 100.0,
                    max_ms: 100.0,
                    p95_ms: 100.0,
                    p99_ms: 100.0,
                },
            ],
        };

        let exceeding = report.operations_exceeding(75.0);
        assert_eq!(exceeding.len(), 1);
        assert_eq!(exceeding[0].name, "slow");
    }

    #[test]
    fn test_performance_report_to_json() {
        let report = PerformanceReport {
            total_time_ms: 150.0,
            operations: vec![OperationReport {
                name: "test_op".to_string(),
                count: 1,
                total_ms: 150.0,
                avg_ms: 150.0,
                min_ms: 150.0,
                max_ms: 150.0,
                p95_ms: 150.0,
                p99_ms: 150.0,
            }],
        };

        let json = report.to_json();
        assert!(json["total_time_ms"].is_string());
        assert!(json["operations"].is_array());
    }

    #[test]
    fn test_performance_report_summary() {
        let report = PerformanceReport {
            total_time_ms: 150.0,
            operations: vec![OperationReport {
                name: "test_op".to_string(),
                count: 5,
                total_ms: 150.0,
                avg_ms: 30.0,
                min_ms: 25.0,
                max_ms: 35.0,
                p95_ms: 34.0,
                p99_ms: 34.5,
            }],
        };

        let summary = report.summary();
        assert!(summary.contains("Performance Report"));
        assert!(summary.contains("test_op"));
        assert!(summary.contains("Count: 5"));
    }

    #[test]
    fn test_batch_optimization_config_default() {
        let config = BatchOptimizationConfig::default();
        assert_eq!(config.small_doc_batch_size, 50);
        assert_eq!(config.medium_doc_batch_size, 20);
        assert_eq!(config.large_doc_batch_size, 5);
        assert_eq!(config.max_concurrent_requests, 10);
        assert!(config.enable_cache_warming);
    }

    #[test]
    fn test_batch_optimization_config_recommended_batch_size() {
        let config = BatchOptimizationConfig::default();

        assert_eq!(config.recommended_batch_size(500), 50); // Small
        assert_eq!(config.recommended_batch_size(5000), 20); // Medium
        assert_eq!(config.recommended_batch_size(15000), 5); // Large
    }
}
