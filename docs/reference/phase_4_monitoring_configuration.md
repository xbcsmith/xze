# Phase 4 Monitoring and Optimization Configuration Reference

## Overview

This document provides a complete reference for all configuration options available in Phase 4 of the LLM-based keyword extraction feature. Phase 4 introduces prompt optimization, performance profiling, monitoring, and quality validation capabilities.

## Environment Variables

### Prompt Template Configuration

#### `KEYWORD_EXTRACTION_PROMPT_VARIANT`

**Type**: String
**Default**: `baseline`
**Options**: `baseline`, `technical`, `domain_aware`, `structured`, `concise`, `detailed`

Selects the prompt template variant to use for keyword extraction.

**Prompt Variants**:

- `baseline` - Original balanced prompt for general documentation
- `technical` - Emphasizes tools, APIs, commands, and technical concepts
- `domain_aware` - Prioritizes domain-specific terminology and product names
- `structured` - Includes explicit examples in the prompt
- `concise` - Minimal prompt optimized for speed
- `detailed` - Comprehensive instructions for complex documents

**Example**:
```bash
export KEYWORD_EXTRACTION_PROMPT_VARIANT=technical
```

**When to Use Each Variant**:
- **baseline**: Default for mixed content types
- **technical**: API documentation, developer guides, SDK references
- **domain_aware**: Product documentation with brand-specific terminology
- **structured**: When LLM needs explicit formatting guidance
- **concise**: High-throughput scenarios where speed is critical
- **detailed**: Complex technical documents with nuanced concepts

---

### Performance Optimization Configuration

#### `KEYWORD_EXTRACTION_SMALL_BATCH_SIZE`

**Type**: Integer
**Default**: `50`
**Range**: 1-1000

Batch size for processing small documents (< 1KB).

**Example**:
```bash
export KEYWORD_EXTRACTION_SMALL_BATCH_SIZE=50
```

#### `KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE`

**Type**: Integer
**Default**: `20`
**Range**: 1-500

Batch size for processing medium documents (1-10KB).

**Example**:
```bash
export KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE=20
```

#### `KEYWORD_EXTRACTION_LARGE_BATCH_SIZE`

**Type**: Integer
**Default**: `5`
**Range**: 1-100

Batch size for processing large documents (> 10KB).

**Example**:
```bash
export KEYWORD_EXTRACTION_LARGE_BATCH_SIZE=5
```

#### `KEYWORD_EXTRACTION_MAX_CONCURRENT`

**Type**: Integer
**Default**: `10`
**Range**: 1-100

Maximum number of concurrent LLM requests.

**Example**:
```bash
export KEYWORD_EXTRACTION_MAX_CONCURRENT=10
```

**Tuning Guidance**:
- Lower values reduce Ollama server load
- Higher values improve throughput for batch operations
- Monitor Ollama CPU/memory usage when tuning

#### `KEYWORD_EXTRACTION_CACHE_WARMING`

**Type**: Boolean
**Default**: `true`
**Options**: `true`, `false`

Enable cache warming on startup.

**Example**:
```bash
export KEYWORD_EXTRACTION_CACHE_WARMING=true
```

#### `KEYWORD_EXTRACTION_CACHE_WARMING_BATCH`

**Type**: Integer
**Default**: `100`
**Range**: 10-10000

Number of documents to pre-load into cache on startup.

**Example**:
```bash
export KEYWORD_EXTRACTION_CACHE_WARMING_BATCH=100
```

---

### Validation Criteria Configuration

#### `VALIDATION_MAX_ERROR_RATE`

**Type**: Float
**Default**: `1.0`
**Range**: 0.0-100.0
**Unit**: Percentage

Maximum acceptable error rate for production validation.

**Example**:
```bash
export VALIDATION_MAX_ERROR_RATE=1.0
```

#### `VALIDATION_MIN_CACHE_HIT_RATE`

**Type**: Float
**Default**: `90.0`
**Range**: 0.0-100.0
**Unit**: Percentage

Minimum required cache hit rate for production validation.

**Example**:
```bash
export VALIDATION_MIN_CACHE_HIT_RATE=90.0
```

#### `VALIDATION_MAX_AVG_TIME_MS`

**Type**: Float
**Default**: `2000.0`
**Range**: 0.0-60000.0
**Unit**: Milliseconds

Maximum acceptable average extraction time.

**Example**:
```bash
export VALIDATION_MAX_AVG_TIME_MS=2000.0
```

#### `VALIDATION_MIN_QUALITY_SCORE`

**Type**: Float
**Default**: `0.7`
**Range**: 0.0-1.0

Minimum required quality score for production validation.

**Example**:
```bash
export VALIDATION_MIN_QUALITY_SCORE=0.7
```

#### `VALIDATION_MIN_IMPROVEMENT_PCT`

**Type**: Float
**Default**: `15.0`
**Range**: 0.0-100.0
**Unit**: Percentage

Minimum required improvement over baseline for production validation.

**Example**:
```bash
export VALIDATION_MIN_IMPROVEMENT_PCT=15.0
```

---

## Programmatic Configuration

### Prompt Template Configuration

```rust
use xze_core::prompt_templates::{PromptTemplate, PromptVariant};

// Create template with specific variant
let template = PromptTemplate::new(PromptVariant::Technical);

// Build prompt
let prompt = template.build(content, 8, 10);
```

### Performance Profiler Configuration

```rust
use xze_core::performance_profiler::PerformanceProfiler;

let mut profiler = PerformanceProfiler::new();

// Start profiling scope
let scope = profiler.start_scope("llm_extraction");
// ... perform operation ...
profiler.end_scope(scope);

// Generate report
let report = profiler.generate_report();
```

### Batch Optimization Configuration

```rust
use xze_core::performance_profiler::BatchOptimizationConfig;

// Load from environment
let config = BatchOptimizationConfig::from_env();

// Or create with custom values
let config = BatchOptimizationConfig {
    small_doc_batch_size: 50,
    medium_doc_batch_size: 20,
    large_doc_batch_size: 5,
    max_concurrent_requests: 10,
    enable_cache_warming: true,
    cache_warming_batch_size: 100,
};

// Get recommendation
let batch_size = config.recommended_batch_size(5000); // 5KB document
```

### Metrics Collector Configuration

```rust
use xze_core::monitoring::MetricsCollector;

// Create with default thresholds
let collector = MetricsCollector::new();

// Or create with custom thresholds
let collector = MetricsCollector::with_thresholds(
    5.0,    // error_rate_warning (%)
    10.0,   // error_rate_critical (%)
    5000.0, // avg_time_warning (ms)
    80.0,   // cache_hit_rate_warning (%)
);
```

### Quality Validator Configuration

```rust
use xze_core::quality_validator::{QualityValidator, ValidationCriteria};

// Create with default criteria
let validator = QualityValidator::new();

// Or create with custom criteria
let criteria = ValidationCriteria {
    max_error_rate: 1.0,
    min_cache_hit_rate: 90.0,
    max_avg_extraction_time_ms: 2000.0,
    min_quality_score: 0.7,
    min_improvement_percentage: 15.0,
};
let validator = QualityValidator::with_criteria(criteria);

// Or load from environment
let criteria = ValidationCriteria::from_env();
let validator = QualityValidator::with_criteria(criteria);
```

---

## Monitoring Metrics

### Prometheus Metrics

The following metrics are exported in Prometheus format via `MetricsCollector::export_prometheus()`:

#### `keyword_extraction_total`

**Type**: Counter
**Description**: Total number of keyword extractions performed

#### `keyword_extraction_by_method{method="llm|frequency"}`

**Type**: Counter
**Description**: Number of extractions by method (LLM or frequency-based)
**Labels**: `method` (llm, frequency)

#### `keyword_extraction_cache_hits`

**Type**: Counter
**Description**: Total number of cache hits

#### `keyword_extraction_cache_misses`

**Type**: Counter
**Description**: Total number of cache misses

#### `keyword_extraction_errors`

**Type**: Counter
**Description**: Total number of extraction errors

#### `keyword_extraction_fallbacks`

**Type**: Counter
**Description**: Total number of fallbacks to frequency extraction

#### `keyword_extraction_latency_seconds{quantile="0.5|0.95|0.99"}`

**Type**: Summary
**Description**: Extraction latency percentiles in seconds
**Labels**: `quantile` (0.5, 0.95, 0.99)

#### `keyword_extraction_uptime_seconds`

**Type**: Gauge
**Description**: Time since metrics collector started in seconds

---

### Alert Thresholds

Alerts are automatically generated when metrics exceed configured thresholds:

| Alert | Level | Default Threshold | Configuration |
|-------|-------|-------------------|---------------|
| High Error Rate | Critical | 10% | `error_rate_critical_threshold` |
| Elevated Error Rate | Warning | 5% | `error_rate_warning_threshold` |
| High Average Extraction Time | Warning | 5000ms | `avg_time_warning_threshold_ms` |
| Low Cache Hit Rate | Warning | 80% | `cache_hit_rate_warning_threshold` |

**Example Alert**:
```json
{
  "level": "warning",
  "title": "Elevated Error Rate",
  "message": "Error rate is 6.5% (threshold: 5.0%)",
  "timestamp": "2024-11-02T15:30:00Z",
  "metadata": {
    "error_rate": "6.5",
    "threshold": "5.0"
  }
}
```

---

## Quality Scoring

### Quality Score Components

Quality scores are calculated using a weighted average of multiple factors:

| Component | Weight | Description |
|-----------|--------|-------------|
| Relevance | 40% | F1 score of precision and recall |
| Coverage | 20% | Proportion of expected keywords extracted |
| Precision | 20% | Proportion of extracted keywords that are useful |
| Performance | 20% | Based on cache hit rate and extraction time |

### Quality Grades

| Grade | Score Range | Description |
|-------|-------------|-------------|
| Excellent | 0.9 - 1.0 | Outstanding quality, exceeds expectations |
| Good | 0.7 - 0.9 | High quality, meets production standards |
| Fair | 0.5 - 0.7 | Acceptable quality, may need tuning |
| Poor | 0.0 - 0.5 | Unacceptable quality, requires improvement |

---

## Performance Targets

### Latency Targets

| Percentile | Target | Description |
|------------|--------|-------------|
| P50 | < 1000ms | Median extraction time |
| P95 | < 2000ms | 95th percentile extraction time |
| P99 | < 5000ms | 99th percentile extraction time |

### Reliability Targets

| Metric | Target | Description |
|--------|--------|-------------|
| Error Rate | < 1% | Extraction failures |
| Cache Hit Rate | > 90% | Cache effectiveness |
| Fallback Rate | < 5% | LLM unavailability |
| Uptime | > 99.9% | Service availability |

---

## Configuration Examples

### Development Environment

```bash
# Prompt optimization - try different variants
export KEYWORD_EXTRACTION_PROMPT_VARIANT=baseline

# Performance - prioritize speed
export KEYWORD_EXTRACTION_SMALL_BATCH_SIZE=100
export KEYWORD_EXTRACTION_MAX_CONCURRENT=5

# Validation - relaxed criteria
export VALIDATION_MAX_ERROR_RATE=5.0
export VALIDATION_MIN_CACHE_HIT_RATE=70.0
export VALIDATION_MAX_AVG_TIME_MS=5000.0
```

### Staging Environment

```bash
# Prompt optimization - test technical variant
export KEYWORD_EXTRACTION_PROMPT_VARIANT=technical

# Performance - balanced
export KEYWORD_EXTRACTION_SMALL_BATCH_SIZE=50
export KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE=20
export KEYWORD_EXTRACTION_LARGE_BATCH_SIZE=5
export KEYWORD_EXTRACTION_MAX_CONCURRENT=10
export KEYWORD_EXTRACTION_CACHE_WARMING=true

# Validation - production-like criteria
export VALIDATION_MAX_ERROR_RATE=2.0
export VALIDATION_MIN_CACHE_HIT_RATE=85.0
export VALIDATION_MAX_AVG_TIME_MS=3000.0
export VALIDATION_MIN_QUALITY_SCORE=0.65
export VALIDATION_MIN_IMPROVEMENT_PCT=10.0
```

### Production Environment

```bash
# Prompt optimization - proven variant
export KEYWORD_EXTRACTION_PROMPT_VARIANT=technical

# Performance - optimized settings
export KEYWORD_EXTRACTION_SMALL_BATCH_SIZE=50
export KEYWORD_EXTRACTION_MEDIUM_BATCH_SIZE=20
export KEYWORD_EXTRACTION_LARGE_BATCH_SIZE=5
export KEYWORD_EXTRACTION_MAX_CONCURRENT=10
export KEYWORD_EXTRACTION_CACHE_WARMING=true
export KEYWORD_EXTRACTION_CACHE_WARMING_BATCH=100

# Validation - strict production criteria
export VALIDATION_MAX_ERROR_RATE=1.0
export VALIDATION_MIN_CACHE_HIT_RATE=90.0
export VALIDATION_MAX_AVG_TIME_MS=2000.0
export VALIDATION_MIN_QUALITY_SCORE=0.7
export VALIDATION_MIN_IMPROVEMENT_PCT=15.0
```

---

## Troubleshooting

### High Error Rate

**Symptoms**: `keyword_extraction_errors` counter increasing rapidly

**Possible Causes**:
1. Ollama server unavailable or overloaded
2. Invalid prompt format
3. Network connectivity issues

**Solutions**:
1. Check Ollama server health
2. Verify prompt template configuration
3. Enable fallback to frequency extraction
4. Check network connectivity

### Low Cache Hit Rate

**Symptoms**: `keyword_extraction_cache_misses` >> `keyword_extraction_cache_hits`

**Possible Causes**:
1. Documents changing frequently
2. Cache size too small
3. Cache warming disabled

**Solutions**:
1. Increase cache size in `KeywordExtractorConfig`
2. Enable cache warming
3. Monitor document change frequency

### High Latency

**Symptoms**: P95/P99 latency exceeding targets

**Possible Causes**:
1. Large documents taking too long
2. Ollama server overloaded
3. Network latency
4. Too many concurrent requests

**Solutions**:
1. Reduce batch sizes for large documents
2. Reduce max concurrent requests
3. Scale Ollama server horizontally
4. Optimize document truncation settings

### Quality Score Degradation

**Symptoms**: Quality score trending downward over time

**Possible Causes**:
1. Prompt variant not suitable for content type
2. LLM model needs updating
3. Content type changed
4. Feedback indicates poor keyword relevance

**Solutions**:
1. Try different prompt variant
2. Update Ollama model version
3. Analyze user feedback patterns
4. Run A/B test with multiple variants

---

## Best Practices

### Prompt Variant Selection

1. Start with `baseline` variant
2. Run A/B test comparing 2-3 variants
3. Monitor quality scores and user feedback
4. Select variant with highest quality score
5. Re-evaluate quarterly

### Performance Tuning

1. Profile operations using `PerformanceProfiler`
2. Identify bottlenecks from performance report
3. Tune batch sizes based on document distribution
4. Monitor P95/P99 latencies
5. Iterate and re-measure

### Monitoring and Alerting

1. Export Prometheus metrics at `/metrics` endpoint
2. Configure Prometheus scraping (interval: 15s)
3. Create Grafana dashboard for visualization
4. Set up alerting rules in Prometheus/Alertmanager
5. Review alerts daily and tune thresholds

### Quality Validation

1. Collect user feedback continuously
2. Calculate quality scores weekly
3. Run production readiness validation before deploys
4. Track quality trends over time
5. Investigate sudden quality drops immediately

---

## References

- Phase 4 Implementation: `docs/explanation/phase_4_optimization_monitoring_implementation.md`
- Configuration Guide: `docs/how_to/configure_llm_keyword_extraction.md`
- Architecture Overview: `docs/explanation/implement_llm_keyword_extraction.md`
- API Documentation: Generated via `cargo doc --open`

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-11-02 | Initial Phase 4 configuration documentation |

---

**Last Updated**: 2024-11-02
**Status**: Production Ready
