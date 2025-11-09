# How to Use Phase 4 Monitoring and Optimization

## Overview

This guide explains how to use the Phase 4 monitoring and optimization features for LLM-based keyword extraction in XZe. You will learn how to configure prompt variants, profile performance, collect metrics, and validate quality.

## Prerequisites

- XZe installed and configured
- Ollama server running
- Phase 4 implementation deployed
- Basic understanding of keyword extraction

## Quick Start

### 1. Enable Metrics Collection

Metrics are enabled by default. To verify:

```bash
export KEYWORD_EXTRACTION_METRICS=true
```

### 2. Choose a Prompt Variant

Select the prompt variant that best matches your content:

```bash
# For general documentation
export KEYWORD_EXTRACTION_PROMPT_VARIANT=baseline

# For technical/API documentation
export KEYWORD_EXTRACTION_PROMPT_VARIANT=technical

# For product-specific documentation
export KEYWORD_EXTRACTION_PROMPT_VARIANT=domain_aware
```

### 3. View Metrics

Export metrics in Prometheus format:

```rust
use xze_core::monitoring::MetricsCollector;

let collector = MetricsCollector::new();
// ... after some operations ...
let prometheus = collector.export_prometheus();
println!("{}", prometheus);
```

Or as JSON:

```rust
let snapshot = collector.snapshot();
println!("{}", serde_json::to_string_pretty(&snapshot.to_json())?);
```

## Prompt Optimization

### Selecting the Right Prompt Variant

#### Baseline Variant

**Use for**: Mixed content types, general documentation

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=baseline
```

**Best for**:
- README files
- General documentation
- Mixed technical and conceptual content

#### Technical Variant

**Use for**: API documentation, developer guides, SDK references

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=technical
```

**Best for**:
- API documentation
- Command-line tool documentation
- Code examples and tutorials
- Technical specifications

#### Domain-Aware Variant

**Use for**: Product documentation with brand-specific terminology

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=domain_aware
```

**Best for**:
- Product manuals
- Brand-specific documentation
- Industry-specific content

#### Structured Variant

**Use for**: When LLM needs explicit formatting guidance

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=structured
```

**Best for**:
- Highly formatted documents
- When LLM struggles with baseline prompt
- Documents with complex structure

#### Concise Variant

**Use for**: High-throughput scenarios where speed is critical

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=concise
```

**Best for**:
- Batch processing large document sets
- Real-time extraction requirements
- When latency is the primary concern

#### Detailed Variant

**Use for**: Complex technical documents with nuanced concepts

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=detailed
```

**Best for**:
- Academic papers
- Research documentation
- Complex technical specifications

### Running A/B Tests

To compare prompt variants, use the A/B testing framework from Phase 3:

```rust
use xze_core::ab_testing::ABTest;
use xze_core::prompt_templates::{PromptTemplate, PromptVariant, PromptPerformanceTracker};

let mut tracker = PromptPerformanceTracker::new();

// Test variant A (baseline)
let template_a = PromptTemplate::new(PromptVariant::Baseline);
let start = std::time::Instant::now();
let keywords_a = extract_with_template(&template_a, content).await?;
let duration_a = start.elapsed().as_secs_f64() * 1000.0;
tracker.record_success(PromptVariant::Baseline, duration_a, keywords_a.total_count());

// Test variant B (technical)
let template_b = PromptTemplate::new(PromptVariant::Technical);
let start = std::time::Instant::now();
let keywords_b = extract_with_template(&template_b, content).await?;
let duration_b = start.elapsed().as_secs_f64() * 1000.0;
tracker.record_success(PromptVariant::Technical, duration_b, keywords_b.total_count());

// Compare results
let stats_a = tracker.get_stats(PromptVariant::Baseline).unwrap();
let stats_b = tracker.get_stats(PromptVariant::Technical).unwrap();

println!("Baseline: avg={:.2}ms, keywords={:.1}",
    stats_a.avg_time_ms(), stats_a.avg_keywords());
println!("Technical: avg={:.2}ms, keywords={:.1}",
    stats_b.avg_time_ms(), stats_b.avg_keywords());
```

## Performance Profiling

### Basic Profiling

Profile individual operations:

```rust
use xze_core::performance_profiler::PerformanceProfiler;

let mut profiler = PerformanceProfiler::new();

// Profile LLM extraction
let scope = profiler.start_scope("llm_extraction");
let keywords = extractor.extract(content).await?;
profiler.end_scope(scope);

// Profile cache lookup
let scope = profiler.start_scope("cache_lookup");
let cached = lookup_cache(content_hash);
profiler.end_scope(scope);

// Generate report
let report = profiler.generate_report();
println!("{}", report.summary());
```

### Analyzing Performance Reports

The performance report shows:

```
Performance Report
==================
Total Time: 1250.50ms

llm_extraction
  Count: 10
  Avg: 120.50ms
  Min: 95.20ms
  Max: 180.30ms
  P95: 175.00ms
  P99: 178.50ms

cache_lookup
  Count: 100
  Avg: 2.50ms
  Min: 1.20ms
  Max: 5.80ms
  P95: 5.00ms
  P99: 5.50ms
```

**Key metrics**:
- **Avg**: Average time per operation
- **P95**: 95% of operations complete faster than this
- **P99**: 99% of operations complete faster than this

### Identifying Bottlenecks

Find operations exceeding threshold:

```rust
let report = profiler.generate_report();

// Operations taking more than 100ms on average
let slow_ops = report.operations_exceeding(100.0);
for op in slow_ops {
    println!("Slow operation: {} - avg {:.2}ms", op.name, op.avg_ms);
}

// Find slowest operation
if let Some(slowest) = report.slowest_operation() {
    println!("Bottleneck: {} - total {:.2}ms", slowest.name, slowest.total_ms);
}
```

### Batch Optimization

Configure batch sizes based on document sizes:

```rust
use xze_core::performance_profiler::BatchOptimizationConfig;

// Load from environment
let config = BatchOptimizationConfig::from_env();

// Get recommended batch size
let doc_size = content.len();
let batch_size = config.recommended_batch_size(doc_size);

println!("Document: {}KB, Batch size: {}", doc_size / 1024, batch_size);
```

Set via environment variables:

```bash
export KEYWORD_EXTRACTION_SMALL_BATCH_SIZE=50   # <1KB
export KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE=20  # 1-10KB
export KEYWORD_EXTRACTION_LARGE_BATCH_SIZE=5    # >10KB
export KEYWORD_EXTRACTION_MAX_CONCURRENT=10
```

## Monitoring and Metrics

### Setting Up Metrics Collection

Create a global metrics collector:

```rust
use xze_core::monitoring::MetricsCollector;
use once_cell::sync::Lazy;

static METRICS: Lazy<MetricsCollector> = Lazy::new(|| {
    MetricsCollector::with_thresholds(
        5.0,    // error_rate_warning
        10.0,   // error_rate_critical
        5000.0, // avg_time_warning_ms
        80.0,   // cache_hit_rate_warning
    )
});
```

### Recording Metrics

Record extraction operations:

```rust
let start = std::time::Instant::now();

match extractor.extract(content).await {
    Ok(keywords) => {
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        METRICS.record_extraction("llm", duration);
        METRICS.record_cache_miss();
    }
    Err(e) => {
        METRICS.record_error();
        METRICS.record_fallback();
    }
}
```

### Viewing Metrics

Get current metrics snapshot:

```rust
let snapshot = METRICS.snapshot();

println!("Total extractions: {}", snapshot.total_extractions);
println!("Cache hit rate: {:.1}%", snapshot.cache_hit_rate());
println!("Error rate: {:.1}%", snapshot.error_rate());
println!("P95 latency: {:.2}ms", snapshot.latency_p95_ms);
println!("P99 latency: {:.2}ms", snapshot.latency_p99_ms);
```

Export as JSON:

```rust
let json = snapshot.to_json();
println!("{}", serde_json::to_string_pretty(&json)?);
```

Output:
```json
{
  "total_extractions": 1000,
  "methods": {
    "llm": 950,
    "frequency": 50
  },
  "cache": {
    "hits": 800,
    "misses": 200,
    "hit_rate": "80.0%"
  },
  "errors": {
    "count": 5,
    "rate": "0.5%"
  },
  "latency_ms": {
    "avg": "145.50",
    "p50": "120.00",
    "p95": "280.00",
    "p99": "450.00"
  },
  "uptime_seconds": 3600
}
```

### Prometheus Integration

Export metrics for Prometheus scraping:

```rust
let prometheus = METRICS.export_prometheus();
println!("{}", prometheus);
```

Output:
```
# HELP keyword_extraction_total Total number of keyword extractions
# TYPE keyword_extraction_total counter
keyword_extraction_total 1000

# HELP keyword_extraction_by_method Number of extractions by method
# TYPE keyword_extraction_by_method counter
keyword_extraction_by_method{method="llm"} 950
keyword_extraction_by_method{method="frequency"} 50

# HELP keyword_extraction_cache_hits Total cache hits
# TYPE keyword_extraction_cache_hits counter
keyword_extraction_cache_hits 800

# HELP keyword_extraction_latency_seconds Extraction latency percentiles
# TYPE keyword_extraction_latency_seconds summary
keyword_extraction_latency_seconds{quantile="0.5"} 0.120
keyword_extraction_latency_seconds{quantile="0.95"} 0.280
keyword_extraction_latency_seconds{quantile="0.99"} 0.450
```

### Setting Up Alerts

Check for alert conditions:

```rust
use xze_core::monitoring::AlertLevel;

let alerts = METRICS.check_alerts();

for alert in alerts {
    match alert.level {
        AlertLevel::Critical => {
            eprintln!("CRITICAL: {} - {}", alert.title, alert.message);
            // Send to alerting system
        }
        AlertLevel::Warning => {
            println!("WARNING: {} - {}", alert.title, alert.message);
            // Log warning
        }
        AlertLevel::Info => {
            println!("INFO: {} - {}", alert.title, alert.message);
        }
    }
}
```

## Quality Validation

### Calculating Quality Scores

Calculate quality score from metrics:

```rust
use xze_core::quality_validator::QualityValidator;

let validator = QualityValidator::new();

let score = validator.calculate_quality_score(
    10,    // total_keywords extracted
    8,     // useful_keywords (from feedback)
    2,     // missing_keywords (should have been extracted)
    0.92,  // cache_hit_rate (0.0 to 1.0)
    150.0, // avg_extraction_time_ms
);

println!("Quality Score: {:.2} ({})", score.overall, score.grade.as_str());
println!("  Relevance: {:.2}", score.relevance);
println!("  Coverage: {:.2}", score.coverage);
println!("  Precision: {:.2}", score.precision);
println!("  Performance: {:.2}", score.performance);

if score.needs_improvement() {
    println!("Quality needs improvement!");
}
```

### Collecting User Feedback

Collect feedback on keyword quality:

```rust
use xze_core::quality_validator::KeywordFeedback;

let mut feedback = KeywordFeedback::new(
    "doc123".to_string(),
    vec!["rust".to_string(), "cargo".to_string(), "async".to_string()],
);

// Mark useful keywords
feedback.useful_keywords = vec!["rust".to_string(), "cargo".to_string()];

// Mark not useful keywords
feedback.not_useful_keywords = vec!["async".to_string()];

// Add missing keywords
feedback.missing_keywords = vec!["tokio".to_string()];

// Add rating
feedback.rating = Some(4);

// Add comments
feedback.comments = Some("Missing tokio keyword".to_string());

// Add to validator
validator.add_feedback(feedback);
```

### Aggregating Feedback

Get aggregate metrics from feedback:

```rust
let metrics = validator.aggregate_feedback_metrics();

println!("Average Precision: {:.2}", metrics.get("avg_precision").unwrap());
println!("Average Recall: {:.2}", metrics.get("avg_recall").unwrap());
println!("Average F1 Score: {:.2}", metrics.get("avg_f1_score").unwrap());
println!("Average Rating: {:.1}/5", metrics.get("avg_rating").unwrap());
```

### Validating Production Readiness

Validate system against production criteria:

```rust
use xze_core::quality_validator::ValidationCriteria;

// Use default criteria
let validator = QualityValidator::new();

// Or customize
let criteria = ValidationCriteria {
    max_error_rate: 1.0,
    min_cache_hit_rate: 90.0,
    max_avg_extraction_time_ms: 2000.0,
    min_quality_score: 0.7,
    min_improvement_percentage: 15.0,
};
let validator = QualityValidator::with_criteria(criteria);

// Run validation
let snapshot = METRICS.snapshot();
let report = validator.validate_production_readiness(
    snapshot.error_rate(),
    snapshot.cache_hit_rate(),
    snapshot.avg_extraction_time_ms(),
    0.85,  // quality_score from feedback
    20.0,  // improvement_percentage from A/B tests
);

if report.passed {
    println!("System is production ready!");
    println!("{}", report.summary);
} else {
    println!("Production validation failed!");
    for failed in report.failed_validations() {
        println!("  FAILED: {}", failed.metric_name);
        println!("    Expected: {:.2}", failed.expected_value);
        println!("    Actual: {:.2}", failed.actual_value);
        if let Some(msg) = &failed.message {
            println!("    Message: {}", msg);
        }
    }
}
```

## Complete Integration Example

Here is a complete example integrating all Phase 4 features:

```rust
use xze_core::{
    KeywordExtractor, KeywordExtractorConfig,
    PromptTemplate, PromptVariant, PromptPerformanceTracker,
    PerformanceProfiler, MetricsCollector,
    QualityValidator, KeywordFeedback,
};
use once_cell::sync::Lazy;

static METRICS: Lazy<MetricsCollector> = Lazy::new(MetricsCollector::new);
static PROFILER: Lazy<parking_lot::Mutex<PerformanceProfiler>> =
    Lazy::new(|| parking_lot::Mutex::new(PerformanceProfiler::new()));

async fn extract_keywords_with_monitoring(content: &str) -> Result<()> {
    // Choose prompt variant
    let template = PromptTemplate::from_env();

    // Start profiling
    let mut profiler = PROFILER.lock();
    let scope = profiler.start_scope("extraction_pipeline");

    // Extract keywords
    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config)?;

    let start = std::time::Instant::now();
    let result = extractor.extract(content).await;
    let duration = start.elapsed().as_secs_f64() * 1000.0;

    // Record metrics
    match result {
        Ok(keywords) => {
            METRICS.record_extraction("llm", duration);
            if keywords.is_cached() {
                METRICS.record_cache_hit();
            } else {
                METRICS.record_cache_miss();
            }
            println!("Extracted {} keywords", keywords.total_count());
        }
        Err(e) => {
            METRICS.record_error();
            eprintln!("Extraction failed: {}", e);
        }
    }

    profiler.end_scope(scope);

    // Check alerts
    for alert in METRICS.check_alerts() {
        println!("{:?}: {} - {}", alert.level, alert.title, alert.message);
    }

    // Print performance summary every 100 operations
    if METRICS.snapshot().total_extractions % 100 == 0 {
        let report = profiler.generate_report();
        println!("\nPerformance Report:\n{}", report.summary());

        let snapshot = METRICS.snapshot();
        println!("\nMetrics Snapshot:");
        println!("{}", serde_json::to_string_pretty(&snapshot.to_json())?);
    }

    Ok(())
}

async fn validate_system_health() -> Result<()> {
    let validator = QualityValidator::new();
    let snapshot = METRICS.snapshot();

    let report = validator.validate_production_readiness(
        snapshot.error_rate(),
        snapshot.cache_hit_rate(),
        snapshot.avg_extraction_time_ms(),
        0.85,  // Replace with actual quality score
        20.0,  // Replace with actual improvement
    );

    println!("{}", serde_json::to_string_pretty(&report.to_json())?);

    Ok(())
}
```

## Troubleshooting

### High Error Rate

If error rate exceeds threshold:

1. Check Ollama server status
2. Verify prompt variant is appropriate for content
3. Enable fallback to frequency extraction
4. Check network connectivity

```bash
# Check Ollama
curl http://localhost:11434/api/tags

# Enable fallback
export KEYWORD_EXTRACTION_FALLBACK=true
```

### Low Cache Hit Rate

If cache hit rate is below 80%:

1. Increase cache size
2. Enable cache warming
3. Monitor document change frequency

```rust
let mut config = KeywordExtractorConfig::default();
config.cache_size = 2000;  // Increase from 1000
```

### High Latency

If P95/P99 latency is high:

1. Reduce batch sizes
2. Reduce concurrent requests
3. Try concise prompt variant
4. Profile to find bottlenecks

```bash
export KEYWORD_EXTRACTION_LARGE_BATCH_SIZE=3
export KEYWORD_EXTRACTION_MAX_CONCURRENT=5
export KEYWORD_EXTRACTION_PROMPT_VARIANT=concise
```

## Best Practices

1. **Monitor continuously**: Check metrics daily
2. **Profile regularly**: Run profiling weekly to identify bottlenecks
3. **Collect feedback**: Gather user feedback on keyword quality
4. **Validate before deploy**: Run production validation before each deployment
5. **Tune iteratively**: Adjust configuration based on production data
6. **Alert on anomalies**: Set up alerts for critical thresholds
7. **Review trends**: Track quality and performance trends over time

## References

- Configuration Reference: `docs/reference/phase_4_monitoring_configuration.md`
- Implementation Details: `docs/explanation/phase_4_optimization_monitoring_implementation.md`
- API Documentation: Run `cargo doc --open`

## Support

For issues or questions, refer to:
- Project documentation: `docs/`
- GitHub issues: https://github.com/xbcsmith/xze/issues
