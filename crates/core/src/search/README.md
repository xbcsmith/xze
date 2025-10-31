# Search Module

This module provides search-related functionality for XZe, including query embedding caching for performance optimization.

## Components

### Embedding Cache

The `EmbeddingCache` provides an LRU cache for query embeddings to avoid regenerating them for frequently searched queries.

**Features**:
- LRU eviction with configurable capacity
- TTL (time-to-live) and TTI (time-to-idle) support
- Thread-safe Arc-based storage
- Get-or-compute pattern for automatic population
- Async operations compatible with Tokio

**Performance Impact**:
- 50-100ms reduction per cached query
- Expected hit rate: 60-80% for typical workloads
- Memory usage: ~4KB per cached embedding (1000-dim float32)

## Usage

### Basic Usage

```rust
use xze_core::search::EmbeddingCache;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = EmbeddingCache::new(1000);

    // Cache a query embedding
    let query = "rust error handling";
    let embedding = vec![0.1, 0.2, 0.3, 0.4];
    cache.insert(query, embedding.clone()).await;

    // Retrieve from cache
    if let Some(cached) = cache.get(query).await {
        println!("Cache hit! Dimension: {}", cached.len());
    } else {
        println!("Cache miss");
    }

    Ok(())
}
```

### Get-or-Compute Pattern

```rust
use xze_core::search::EmbeddingCache;

async fn search_with_cache(
    cache: &EmbeddingCache,
    query: &str,
) -> Result<Vec<f32>, std::io::Error> {
    // Automatically compute and cache if not present
    let embedding = cache.get_or_compute(query, |q| async move {
        // Call AI service to generate embedding
        generate_embedding_from_ai(&q).await
    }).await?;

    Ok(embedding.as_ref().clone())
}
```

### Custom Configuration

```rust
use xze_core::search::EmbeddingCache;

// Cache with 2 hour TTL and 1 hour idle timeout
let cache = EmbeddingCache::with_ttl(
    1000,  // capacity
    7200,  // TTL in seconds
    3600,  // idle timeout in seconds
);
```

### Application Integration

```rust
use xze_core::search::EmbeddingCache;
use std::sync::Arc;

// In application state
pub struct AppState {
    pub embedding_cache: Arc<EmbeddingCache>,
}

// In search handler
pub async fn handle_search(
    state: Arc<AppState>,
    query: String,
) -> Result<SearchResponse> {
    // Get or compute embedding with cache
    let embedding = state.embedding_cache
        .get_or_compute(&query, |q| async move {
            generate_embedding(&q).await
        })
        .await?;

    // Use embedding for similarity search
    let results = search_by_embedding(&embedding).await?;

    Ok(SearchResponse { results })
}
```

## Configuration

### Environment Variables

```bash
EMBEDDING_CACHE_SIZE=1000      # Number of entries
EMBEDDING_CACHE_TTL=3600       # TTL in seconds (1 hour)
EMBEDDING_CACHE_IDLE=1800      # Idle timeout in seconds (30 min)
```

### Default Values

- Capacity: 1000 entries
- TTL: 3600 seconds (1 hour)
- TTI: 1800 seconds (30 minutes)

## Performance Metrics

### Cache Effectiveness

| Metric | Value |
|--------|-------|
| Expected hit rate | 60-80% |
| Time saved per hit | 50-100ms |
| Memory per embedding | ~4KB (1000-dim) |
| Total memory (1000 entries) | ~4MB |

### Monitoring

Track these metrics in production:

- **Hit rate**: `cache_hits / (cache_hits + cache_misses)`
- **Entry count**: Current number of cached embeddings
- **Memory usage**: Estimated at 4KB per entry

## Implementation Details

### Architecture

```rust
pub struct EmbeddingCache {
    cache: Cache<String, Arc<Vec<f32>>>,
}
```

The cache uses Moka's async LRU cache with:
- String keys (query text)
- Arc-wrapped Vec<f32> values (embeddings)
- Automatic eviction based on LRU policy
- TTL and TTI-based expiration

### Thread Safety

The cache is thread-safe and can be shared across multiple async tasks:

```rust
let cache = Arc::new(EmbeddingCache::new(1000));
let cache_clone = Arc::clone(&cache);

tokio::spawn(async move {
    cache_clone.insert("query", vec![0.1, 0.2]).await;
});
```

## Testing

Run tests for the embedding cache:

```bash
cargo test --lib -p xze-core embedding_cache
```

All 12 tests should pass:
- Basic insert and get operations
- Cache miss behavior
- Invalidation and clearing
- Entry count tracking
- Get-or-compute pattern
- Error propagation
- Custom TTL configuration

## See Also

- **Phase 9 Implementation**: `docs/explanations/phase_9_performance_optimization_implementation.md`
- **Phase 9 Summary**: `docs/explanations/phase_9_summary.md`
- **Implementation Plan**: `docs/explanations/search_features_unified_implementation_plan.md`

## Future Enhancements

- Distributed caching with Redis
- Cache warming on startup
- Adaptive TTL based on query frequency
- Cache statistics and metrics export
