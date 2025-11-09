# Keyword Extraction Configuration Reference

This document provides a comprehensive reference for all configuration options
available for the LLM-based keyword extraction system.

## Configuration Structure

Keyword extraction is configured through the `KeywordExtractorConfig` structure
in the core library. Configuration can be set programmatically or via
environment variables.

## Configuration Options

### Core Settings

#### `ollama_base_url`

- **Type**: String
- **Default**: `"http://localhost:11434"`
- **Environment Variable**: None (programmatic only)
- **Description**: Base URL for the Ollama API server

**Example**:

```rust
config.ollama_base_url = "http://ollama.example.com:11434".to_string();
```

#### `model`

- **Type**: String
- **Default**: `"llama3.2:3b"`
- **Environment Variable**: None (programmatic only)
- **Description**: LLM model to use for keyword extraction

**Supported Models**:

- `llama3.2:3b` - Recommended for performance
- `llama3.2:7b` - Better quality, slower
- `mistral:7b` - Alternative model

**Example**:

```rust
config.model = "llama3.2:7b".to_string();
```

### Extraction Limits

#### `max_descriptive_keywords`

- **Type**: usize
- **Default**: `8`
- **Range**: 1-50
- **Description**: Maximum number of descriptive keywords to extract per document

**Example**:

```rust
config.max_descriptive_keywords = 10;
```

#### `max_technical_keywords`

- **Type**: usize
- **Default**: `10`
- **Range**: 1-50
- **Description**: Maximum number of technical keywords to extract per document

**Example**:

```rust
config.max_technical_keywords = 15;
```

### Caching

#### `cache_size`

- **Type**: usize
- **Default**: `1000`
- **Range**: 1-100000
- **Description**: Maximum number of documents to cache keyword results for

**Example**:

```rust
config.cache_size = 5000;
```

### Performance

#### `timeout_seconds`

- **Type**: u64
- **Default**: `30`
- **Range**: 5-300
- **Description**: Timeout for LLM API requests in seconds

**Example**:

```rust
config.timeout_seconds = 60;
```

#### `max_retries`

- **Type**: usize
- **Default**: `3`
- **Range**: 0-10
- **Description**: Maximum number of retry attempts for failed LLM requests

**Example**:

```rust
config.max_retries = 5;
```

### Fallback

#### `enable_fallback`

- **Type**: bool
- **Default**: `true`
- **Description**: Enable fallback to frequency-based extraction on LLM failure

**Example**:

```rust
config.enable_fallback = true;
```

### Document Processing

#### `min_document_length`

- **Type**: usize
- **Default**: `50`
- **Range**: 10-10000
- **Description**: Minimum document length (characters) for LLM extraction

Documents shorter than this will use frequency-based extraction automatically.

**Example**:

```rust
config.min_document_length = 100;
```

#### `max_document_length`

- **Type**: usize
- **Default**: `8000`
- **Range**: 1000-50000
- **Description**: Maximum document length (characters) to send to LLM

Documents longer than this will be truncated.

**Example**:

```rust
config.max_document_length = 10000;
```

#### `temperature`

- **Type**: f32
- **Default**: `0.1`
- **Range**: 0.0-1.0
- **Description**: LLM temperature for generation (lower = more deterministic)

**Example**:

```rust
config.temperature = 0.2;
```

### Production Rollout Settings

#### `rollout_percentage`

- **Type**: u8
- **Default**: `0` (reads from environment variable)
- **Range**: 0-100
- **Environment Variable**: `KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE`
- **Description**: Percentage of documents to use LLM extraction for (0-100)

**Example**:

```bash
# Enable for 50% of documents
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
```

**Rollout Strategy**:

- `0` - LLM disabled, all documents use frequency extraction
- `10` - Canary rollout (10% of documents)
- `25` - Limited rollout
- `50` - A/B testing (50/50 split)
- `100` - Full rollout (all documents use LLM)

#### `ab_test_enabled`

- **Type**: bool
- **Default**: `false` (reads from environment variable)
- **Environment Variable**: `KEYWORD_EXTRACTION_AB_TEST`
- **Description**: Enable A/B testing mode with explicit group tracking

**Example**:

```bash
# Enable A/B testing
export KEYWORD_EXTRACTION_AB_TEST=true
```

#### `metrics_enabled`

- **Type**: bool
- **Default**: `true` (reads from environment variable)
- **Environment Variable**: `KEYWORD_EXTRACTION_METRICS`
- **Description**: Enable metrics collection for monitoring

**Example**:

```bash
# Disable metrics
export KEYWORD_EXTRACTION_METRICS=false
```

## Complete Configuration Examples

### Development Configuration

```rust
use xze_core::keyword_extractor::KeywordExtractorConfig;

let config = KeywordExtractorConfig {
    ollama_base_url: "http://localhost:11434".to_string(),
    model: "llama3.2:3b".to_string(),
    max_descriptive_keywords: 8,
    max_technical_keywords: 10,
    cache_size: 100,
    timeout_seconds: 30,
    max_retries: 3,
    enable_fallback: true,
    min_document_length: 50,
    max_document_length: 8000,
    temperature: 0.1,
    rollout_percentage: 100, // Full LLM for dev
    ab_test_enabled: false,
    metrics_enabled: true,
};
```

### Production Configuration (Canary)

```rust
let config = KeywordExtractorConfig {
    ollama_base_url: "http://ollama-prod:11434".to_string(),
    model: "llama3.2:3b".to_string(),
    max_descriptive_keywords: 8,
    max_technical_keywords: 10,
    cache_size: 10000,
    timeout_seconds: 30,
    max_retries: 5,
    enable_fallback: true,
    min_document_length: 50,
    max_document_length: 8000,
    temperature: 0.1,
    rollout_percentage: 10, // Canary: 10%
    ab_test_enabled: false,
    metrics_enabled: true,
};
```

### Production Configuration (A/B Test)

```rust
let config = KeywordExtractorConfig {
    ollama_base_url: "http://ollama-prod:11434".to_string(),
    model: "llama3.2:3b".to_string(),
    max_descriptive_keywords: 8,
    max_technical_keywords: 10,
    cache_size: 10000,
    timeout_seconds: 30,
    max_retries: 5,
    enable_fallback: true,
    min_document_length: 50,
    max_document_length: 8000,
    temperature: 0.1,
    rollout_percentage: 50, // A/B test: 50/50
    ab_test_enabled: true,
    metrics_enabled: true,
};
```

## Environment Variables

### Quick Reference

| Variable                                | Type | Default | Description                |
| --------------------------------------- | ---- | ------- | -------------------------- |
| `KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE` | u8   | `0`     | Rollout percentage (0-100) |
| `KEYWORD_EXTRACTION_AB_TEST`            | bool | `false` | Enable A/B testing         |
| `KEYWORD_EXTRACTION_METRICS`            | bool | `true`  | Enable metrics collection  |

### Setting Environment Variables

**Linux/macOS**:

```bash
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
export KEYWORD_EXTRACTION_AB_TEST=true
export KEYWORD_EXTRACTION_METRICS=true
```

**Windows (PowerShell)**:

```powershell
$env:KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE = "50"
$env:KEYWORD_EXTRACTION_AB_TEST = "true"
$env:KEYWORD_EXTRACTION_METRICS = "true"
```

**Docker Compose**:

```yaml
services:
  xze:
    environment:
      - KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
      - KEYWORD_EXTRACTION_AB_TEST=true
      - KEYWORD_EXTRACTION_METRICS=true
```

**Kubernetes**:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: xze-config
data:
  KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE: "50"
  KEYWORD_EXTRACTION_AB_TEST: "true"
  KEYWORD_EXTRACTION_METRICS: "true"
```

## Metrics

When `metrics_enabled` is true, the following metrics are collected:

### Metric Fields

- `total_extractions` - Total number of keyword extractions performed
- `llm_extractions` - Number using LLM method
- `frequency_extractions` - Number using frequency method
- `cache_hits` - Number of cache hits
- `cache_misses` - Number of cache misses
- `fallback_count` - Number of fallbacks from LLM to frequency
- `error_count` - Number of errors encountered
- `total_time_ms` - Total extraction time in milliseconds

### Accessing Metrics

```rust
let extractor = KeywordExtractor::new(config)?;

// Perform extractions...

// Get metrics
let metrics = extractor.get_metrics().await;
println!("Total extractions: {}", metrics.total_extractions);
println!("Cache hit rate: {:.1}%", metrics.cache_hit_rate());
println!("Avg time: {:.2}ms", metrics.avg_extraction_time_ms());

// Export as JSON
let json = extractor.export_metrics().await;
println!("{}", serde_json::to_string_pretty(&json)?);

// Reset metrics
extractor.reset_metrics().await;
```

## Configuration Validation

The configuration is validated when creating a `KeywordExtractor` instance:

```rust
use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};

let config = KeywordExtractorConfig {
    cache_size: 0, // Invalid!
    ..Default::default()
};

let result = KeywordExtractor::new(config);
assert!(result.is_err()); // Fails validation
```

### Validation Rules

- `cache_size` must be greater than zero
- `rollout_percentage` is clamped to 0-100
- `temperature` should be between 0.0 and 1.0
- `timeout_seconds` should be at least 5
- `max_retries` should be reasonable (0-10)

## Best Practices

### Development

1. Use `rollout_percentage: 100` for consistent behavior
2. Set `cache_size` low (100-500) to save memory
3. Enable `metrics_enabled: true` for debugging
4. Use shorter timeouts for faster feedback

### Production

1. Start with `rollout_percentage: 0` and increase gradually
2. Set `cache_size` high (5000-10000) for performance
3. Always enable `metrics_enabled: true`
4. Set reasonable timeouts (30-60 seconds)
5. Enable `enable_fallback: true` for reliability
6. Use `ab_test_enabled: true` during evaluation

### Performance Tuning

1. **For speed**: Use smaller model (`llama3.2:3b`), lower cache, shorter timeout
2. **For quality**: Use larger model (`llama3.2:7b`), higher max keywords
3. **For reliability**: Enable fallback, increase retries, extend timeout
4. **For scale**: Increase cache size, optimize batch processing

## Troubleshooting

### LLM Always Fails

Check:

- Ollama service is running: `curl http://localhost:11434/api/tags`
- Model is installed: `ollama list`
- Network connectivity to Ollama server
- Timeout is sufficient for your model

### Low Cache Hit Rate

Increase:

- `cache_size` - Allow more documents to be cached
- Document content stability - Ensure content hashes match

### Slow Extraction

Optimize:

- Use smaller model (`llama3.2:3b` instead of `7b`)
- Reduce `max_document_length` to process less text
- Reduce `max_descriptive_keywords` and `max_technical_keywords`
- Increase `cache_size` to cache more results

### High Error Rate

Enable:

- `enable_fallback: true` - Fallback to frequency extraction
- Increase `max_retries` - Retry failed requests
- Increase `timeout_seconds` - Allow more time for LLM

## See Also

- [How to Configure LLM Keyword Extraction](../how_to/configure_llm_keyword_extraction.md)
- [Troubleshooting Guide](../how_to/troubleshoot_keyword_extraction.md)
- [Keyword Extraction Architecture](../explanation/keyword_extraction_architecture.md)
