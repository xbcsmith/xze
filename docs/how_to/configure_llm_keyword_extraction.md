# How to Configure LLM Keyword Extraction

This guide walks you through configuring the LLM-based keyword extraction
feature in XZe.

## Prerequisites

Before you begin, ensure you have:

- Ollama installed and running
- An LLM model downloaded (e.g., `llama3.2:3b`)
- XZe installed and configured
- Basic understanding of environment variables

## Quick Start

### Step 1: Install and Start Ollama

```bash
# Install Ollama (macOS/Linux)
curl -fsSL https://ollama.com/install.sh | sh

# Pull the recommended model
ollama pull llama3.2:3b

# Verify Ollama is running
curl http://localhost:11434/api/tags
```

### Step 2: Enable LLM Extraction (Development)

For development, enable LLM extraction for all documents:

```bash
# Enable full rollout
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
export KEYWORD_EXTRACTION_METRICS=true
```

### Step 3: Test the Configuration

Create a test script to verify the setup:

```rust
use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};

#[tokio::main]
async fn main() -> xze_core::Result<()> {
    // Use default configuration
    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config)?;

    // Check if LLM is available
    if extractor.is_llm_available().await {
        println!("LLM is available!");
    } else {
        println!("LLM is not available. Check Ollama service.");
        return Ok(());
    }

    // Extract keywords from sample text
    let content = "This guide explains how to configure Rust projects \
                   using Cargo.toml. You can specify dependencies, \
                   build settings, and package metadata.";

    let keywords = extractor.extract(content).await?;

    println!("Descriptive keywords: {:?}", keywords.descriptive);
    println!("Technical keywords: {:?}", keywords.technical);
    println!("Extraction method: {}", keywords.extraction_method);

    // Check metrics
    let metrics = extractor.get_metrics().await;
    println!("\nMetrics:");
    println!("  Total extractions: {}", metrics.total_extractions);
    println!("  LLM extractions: {}", metrics.llm_extractions);
    println!("  Cache hit rate: {:.1}%", metrics.cache_hit_rate());

    Ok(())
}
```

## Production Configuration

### Stage 1: Prepare Environment

Create a configuration file for production settings:

```bash
# production.env
KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
KEYWORD_EXTRACTION_AB_TEST=false
KEYWORD_EXTRACTION_METRICS=true
```

Load the configuration:

```bash
source production.env
```

### Stage 2: Canary Rollout (10%)

Gradually enable LLM extraction for a small percentage of documents:

```bash
# Enable for 10% of documents
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10
```

Monitor the system for 24-48 hours:

```rust
// Check metrics regularly
let metrics = extractor.get_metrics().await;
let json = extractor.export_metrics().await;
println!("{}", serde_json::to_string_pretty(&json)?);
```

### Stage 3: Limited Rollout (25%)

If canary is successful, increase to 25%:

```bash
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25
```

### Stage 4: A/B Testing (50%)

Run a formal A/B test with 50/50 split:

```bash
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
export KEYWORD_EXTRACTION_AB_TEST=true
```

Use the A/B testing framework:

```rust
use xze_core::ab_testing::ABTest;

let mut test = ABTest::new(50.0);

// Assign documents to groups
let group1 = test.assign_group("docs/guide1.md");
let group2 = test.assign_group("docs/guide2.md");

println!("guide1.md: {}", group1);
println!("guide2.md: {}", group2);

// Save assignments for analysis
test.save_assignments(std::path::Path::new("ab_test_assignments.json"))
    .await?;
```

### Stage 5: Full Rollout (100%)

After successful A/B test, enable for all documents:

```bash
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
export KEYWORD_EXTRACTION_AB_TEST=false
```

## Custom Configuration

### Programmatic Configuration

For more control, configure programmatically:

```rust
use xze_core::keyword_extractor::KeywordExtractorConfig;

let config = KeywordExtractorConfig {
    // Ollama settings
    ollama_base_url: "http://ollama-prod:11434".to_string(),
    model: "llama3.2:7b".to_string(), // Higher quality model

    // Extraction limits
    max_descriptive_keywords: 10,
    max_technical_keywords: 12,

    // Performance
    cache_size: 10000,
    timeout_seconds: 60,
    max_retries: 5,

    // Document processing
    min_document_length: 100,
    max_document_length: 10000,
    temperature: 0.1,

    // Fallback
    enable_fallback: true,

    // Rollout (overridden by environment variables)
    rollout_percentage: 100,
    ab_test_enabled: false,
    metrics_enabled: true,
};

let extractor = KeywordExtractor::new(config)?;
```

### Remote Ollama Server

To use a remote Ollama server:

```rust
let config = KeywordExtractorConfig {
    ollama_base_url: "http://ollama.example.com:11434".to_string(),
    // ... other settings
    ..Default::default()
};
```

### Multiple Models

You can create multiple extractors with different models:

```rust
// Fast model for quick extraction
let fast_config = KeywordExtractorConfig {
    model: "llama3.2:3b".to_string(),
    timeout_seconds: 30,
    ..Default::default()
};
let fast_extractor = KeywordExtractor::new(fast_config)?;

// Quality model for important documents
let quality_config = KeywordExtractorConfig {
    model: "llama3.2:7b".to_string(),
    timeout_seconds: 90,
    max_descriptive_keywords: 15,
    max_technical_keywords: 20,
    ..Default::default()
};
let quality_extractor = KeywordExtractor::new(quality_config)?;
```

## Docker Configuration

### Docker Compose

```yaml
version: "3.8"

services:
  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
      interval: 30s
      timeout: 10s
      retries: 3

  xze:
    build: .
    depends_on:
      ollama:
        condition: service_healthy
    environment:
      - KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
      - KEYWORD_EXTRACTION_METRICS=true
    volumes:
      - ./data:/app/data

volumes:
  ollama_data:
```

### Pull Model in Docker

```bash
docker-compose exec ollama ollama pull llama3.2:3b
```

## Kubernetes Configuration

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: xze-keyword-config
  namespace: xze
data:
  KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE: "50"
  KEYWORD_EXTRACTION_AB_TEST: "true"
  KEYWORD_EXTRACTION_METRICS: "true"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xze
  namespace: xze
spec:
  replicas: 3
  selector:
    matchLabels:
      app: xze
  template:
    metadata:
      labels:
        app: xze
    spec:
      containers:
        - name: xze
          image: xze:latest
          envFrom:
            - configMapRef:
                name: xze-keyword-config
          env:
            - name: OLLAMA_URL
              value: "http://ollama-service:11434"
```

## Monitoring Configuration

### Export Metrics

Create a metrics endpoint to monitor extraction performance:

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use std::sync::Arc;

async fn metrics_handler(
    extractor: web::Data<Arc<KeywordExtractor>>,
) -> HttpResponse {
    let json = extractor.export_metrics().await;
    HttpResponse::Ok().json(json)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = KeywordExtractorConfig::default();
    let extractor = Arc::new(KeywordExtractor::new(config).unwrap());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(extractor.clone()))
            .route("/metrics/keywords", web::get().to(metrics_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Prometheus Integration (Future)

Metrics can be exported to Prometheus:

```rust
// Future implementation example
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref EXTRACTION_COUNT: Counter =
        register_counter!("keyword_extraction_total", "Total extractions").unwrap();

    static ref EXTRACTION_DURATION: Histogram =
        register_histogram!("keyword_extraction_duration_seconds",
                           "Extraction duration").unwrap();
}
```

## Configuration Best Practices

### Development

1. **Use full rollout**: Set `ROLLOUT_PERCENTAGE=100`
2. **Enable metrics**: Always set `METRICS=true`
3. **Enable fallback**: Set `enable_fallback=true`
4. **Use smaller model**: `llama3.2:3b` for faster iteration
5. **Lower cache**: 100-500 entries sufficient

### Staging

1. **Test rollout mechanism**: Try 0%, 50%, 100%
2. **Verify metrics**: Ensure metrics are collected correctly
3. **Test fallback**: Temporarily stop Ollama to verify fallback
4. **Load test**: Use batch extraction to test performance

### Production

1. **Start with 0%**: Disable initially, verify base system
2. **Gradual rollout**: 0% → 10% → 25% → 50% → 100%
3. **Monitor closely**: Watch metrics at each stage
4. **Enable fallback**: Always enable for reliability
5. **Large cache**: 5000-10000 entries for performance
6. **Longer timeout**: 60-90 seconds for reliability

## Common Configuration Patterns

### High Performance

```rust
let config = KeywordExtractorConfig {
    model: "llama3.2:3b".to_string(),
    cache_size: 10000,
    timeout_seconds: 30,
    max_document_length: 5000,
    max_descriptive_keywords: 6,
    max_technical_keywords: 8,
    ..Default::default()
};
```

### High Quality

```rust
let config = KeywordExtractorConfig {
    model: "llama3.2:7b".to_string(),
    timeout_seconds: 90,
    max_descriptive_keywords: 15,
    max_technical_keywords: 20,
    temperature: 0.05, // More deterministic
    ..Default::default()
};
```

### High Reliability

```rust
let config = KeywordExtractorConfig {
    enable_fallback: true,
    max_retries: 5,
    timeout_seconds: 60,
    cache_size: 10000,
    ..Default::default()
};
```

## Troubleshooting

### Cannot Connect to Ollama

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama if not running
ollama serve

# Check firewall rules if remote
telnet ollama-server 11434
```

### Model Not Found

```bash
# List installed models
ollama list

# Pull the model
ollama pull llama3.2:3b

# Verify model is available
ollama run llama3.2:3b "test"
```

### Slow Extraction

1. Use smaller model: `llama3.2:3b` instead of `7b`
2. Reduce document length: `max_document_length: 5000`
3. Increase cache: `cache_size: 10000`
4. Use GPU acceleration for Ollama if available

### High Memory Usage

1. Reduce cache size: `cache_size: 1000`
2. Limit concurrent extractions
3. Use smaller model
4. Monitor Ollama memory usage separately

## Next Steps

- Read [Troubleshooting Guide](troubleshoot_keyword_extraction.md)
- Review [Configuration Reference](../reference/keyword_extraction_configuration.md)
- Learn about [Rollback Procedures](rollback_llm_keyword_extraction.md)
- Understand [Architecture](../explanation/keyword_extraction_architecture.md)

## See Also

- [Keyword Extraction Configuration Reference](../reference/keyword_extraction_configuration.md)
- [Troubleshooting Keyword Extraction](troubleshoot_keyword_extraction.md)
- [Rollback Procedures](rollback_llm_keyword_extraction.md)
