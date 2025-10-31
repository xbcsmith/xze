//! Query embedding cache for performance optimization
//!
//! This module provides an LRU cache for query embeddings to avoid
//! regenerating embeddings for frequently searched queries. This can
//! reduce search latency by 50-100ms per query.
//!
//! # Examples
//!
//! ```rust
//! use xze_core::search::embedding_cache::EmbeddingCache;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let cache = EmbeddingCache::new(1000);
//!
//! // Cache a query embedding
//! let query = "rust error handling";
//! let embedding = vec![0.1, 0.2, 0.3, 0.4];
//! cache.insert(query, embedding.clone()).await;
//!
//! // Retrieve from cache
//! if let Some(cached) = cache.get(query).await {
//!     println!("Cache hit for query: {}", query);
//! }
//! # Ok(())
//! # }
//! ```

use moka::future::Cache;
use std::sync::Arc;
use tracing::{debug, trace};

/// Query embedding cache with LRU eviction
///
/// Caches query embeddings to avoid regenerating them for frequently
/// searched queries. Uses an LRU eviction policy to maintain a fixed
/// size and TTL-based expiration.
#[derive(Clone)]
pub struct EmbeddingCache {
    cache: Cache<String, Arc<Vec<f32>>>,
}

impl EmbeddingCache {
    /// Create a new embedding cache with specified capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of embeddings to cache
    ///
    /// # Returns
    ///
    /// Returns a new EmbeddingCache instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// let cache = EmbeddingCache::new(1000);
    /// ```
    pub fn new(capacity: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(std::time::Duration::from_secs(3600)) // 1 hour TTL
            .time_to_idle(std::time::Duration::from_secs(1800)) // 30 min idle
            .build();

        debug!(
            "Created embedding cache with capacity {} and 1h TTL",
            capacity
        );

        Self { cache }
    }

    /// Create a new cache with custom TTL and idle timeout
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of embeddings to cache
    /// * `ttl_seconds` - Time-to-live in seconds
    /// * `idle_seconds` - Time-to-idle in seconds
    ///
    /// # Returns
    ///
    /// Returns a new EmbeddingCache instance with custom timings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// // Cache with 2 hour TTL and 1 hour idle timeout
    /// let cache = EmbeddingCache::with_ttl(1000, 7200, 3600);
    /// ```
    pub fn with_ttl(capacity: u64, ttl_seconds: u64, idle_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(std::time::Duration::from_secs(ttl_seconds))
            .time_to_idle(std::time::Duration::from_secs(idle_seconds))
            .build();

        debug!(
            "Created embedding cache with capacity {}, TTL {}s, idle {}s",
            capacity, ttl_seconds, idle_seconds
        );

        Self { cache }
    }

    /// Insert a query embedding into the cache
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `embedding` - The embedding vector
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() {
    /// let cache = EmbeddingCache::new(1000);
    /// let query = "rust error handling";
    /// let embedding = vec![0.1, 0.2, 0.3, 0.4];
    /// cache.insert(query, embedding).await;
    /// # }
    /// ```
    pub async fn insert(&self, query: impl Into<String>, embedding: Vec<f32>) {
        let query_str = query.into();
        trace!(
            "Caching embedding for query '{}' (dimension: {})",
            query_str,
            embedding.len()
        );
        self.cache.insert(query_str, Arc::new(embedding)).await;
    }

    /// Retrieve a cached embedding for a query
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    ///
    /// # Returns
    ///
    /// Returns Some(embedding) if found in cache, None otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() {
    /// let cache = EmbeddingCache::new(1000);
    /// let query = "rust error handling";
    ///
    /// if let Some(embedding) = cache.get(query).await {
    ///     println!("Cache hit! Embedding dimension: {}", embedding.len());
    /// } else {
    ///     println!("Cache miss");
    /// }
    /// # }
    /// ```
    pub async fn get(&self, query: &str) -> Option<Arc<Vec<f32>>> {
        let result = self.cache.get(query).await;
        if result.is_some() {
            trace!("Cache HIT for query '{}'", query);
        } else {
            trace!("Cache MISS for query '{}'", query);
        }
        result
    }

    /// Invalidate a specific query from the cache
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string to invalidate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() {
    /// let cache = EmbeddingCache::new(1000);
    /// cache.invalidate("rust error handling").await;
    /// # }
    /// ```
    pub async fn invalidate(&self, query: &str) {
        debug!("Invalidating cache entry for query '{}'", query);
        self.cache.invalidate(query).await;
    }

    /// Clear all entries from the cache
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() {
    /// let cache = EmbeddingCache::new(1000);
    /// cache.clear().await;
    /// # }
    /// ```
    pub async fn clear(&self) {
        debug!("Clearing entire embedding cache");
        self.cache.invalidate_all();
        // Run pending tasks to ensure invalidation completes
        self.cache.run_pending_tasks().await;
    }

    /// Get the current number of entries in the cache
    ///
    /// # Returns
    ///
    /// Returns the number of cached embeddings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() {
    /// let cache = EmbeddingCache::new(1000);
    /// println!("Cache size: {}", cache.entry_count());
    /// # }
    /// ```
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Get or compute an embedding
    ///
    /// Retrieves from cache if available, otherwise computes using the
    /// provided function and caches the result.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `compute_fn` - Async function to compute the embedding if not cached
    ///
    /// # Returns
    ///
    /// Returns the embedding vector (from cache or newly computed)
    ///
    /// # Errors
    ///
    /// Returns any error from the compute function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xze_core::search::embedding_cache::EmbeddingCache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cache = EmbeddingCache::new(1000);
    /// let query = "rust error handling";
    ///
    /// let embedding = cache.get_or_compute(query, |q| async move {
    ///     // Simulate embedding generation
    ///     Ok::<Vec<f32>, std::io::Error>(vec![0.1, 0.2, 0.3])
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_compute<F, Fut, E>(
        &self,
        query: &str,
        compute_fn: F,
    ) -> Result<Arc<Vec<f32>>, E>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<f32>, E>>,
    {
        // Try cache first
        if let Some(cached) = self.get(query).await {
            return Ok(cached);
        }

        // Cache miss - compute embedding
        trace!("Computing embedding for query '{}'", query);
        let embedding = compute_fn(query.to_string()).await?;
        let arc_embedding = Arc::new(embedding);

        // Cache the result
        self.cache
            .insert(query.to_string(), arc_embedding.clone())
            .await;

        Ok(arc_embedding)
    }
}

impl Default for EmbeddingCache {
    /// Create a default cache with capacity of 1000 entries
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_cache_insert_and_get() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        cache.insert(query, embedding.clone()).await;

        let cached = cache.get(query).await;
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_embedding_cache_miss() {
        let cache = EmbeddingCache::new(100);

        let result = cache.get("nonexistent query").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_embedding_cache_invalidate() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";
        let embedding = vec![0.1, 0.2, 0.3];

        cache.insert(query, embedding).await;
        assert!(cache.get(query).await.is_some());

        cache.invalidate(query).await;
        assert!(cache.get(query).await.is_none());
    }

    #[tokio::test]
    async fn test_embedding_cache_clear() {
        let cache = EmbeddingCache::new(100);

        cache.insert("query1", vec![0.1, 0.2]).await;
        cache.insert("query2", vec![0.3, 0.4]).await;
        cache.insert("query3", vec![0.5, 0.6]).await;
        cache.cache.run_pending_tasks().await;

        assert_eq!(cache.entry_count(), 3);

        cache.clear().await;
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn test_embedding_cache_entry_count() {
        let cache = EmbeddingCache::new(100);

        assert_eq!(cache.entry_count(), 0);

        cache.insert("query1", vec![0.1]).await;
        cache.cache.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);

        cache.insert("query2", vec![0.2]).await;
        cache.cache.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 2);
    }

    #[tokio::test]
    async fn test_embedding_cache_with_ttl() {
        let cache = EmbeddingCache::with_ttl(100, 3600, 1800);
        let query = "test query";
        let embedding = vec![0.1, 0.2, 0.3];

        cache.insert(query, embedding.clone()).await;

        let cached = cache.get(query).await;
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_embedding_cache_get_or_compute_hit() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";
        let embedding = vec![0.1, 0.2, 0.3];

        // Pre-populate cache
        cache.insert(query, embedding.clone()).await;

        // Should return cached value without calling compute_fn
        let result: Result<Arc<Vec<f32>>, std::io::Error> = cache
            .get_or_compute(query, |_| async {
                panic!("Should not be called");
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_embedding_cache_get_or_compute_miss() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";
        let expected = vec![0.1, 0.2, 0.3];

        let expected_clone = expected.clone();
        let result = cache
            .get_or_compute(query, |q| async move {
                assert_eq!(q, query);
                Ok::<Vec<f32>, std::io::Error>(expected_clone.clone())
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), expected);

        // Verify it was cached
        let cached = cache.get(query).await;
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), expected);
    }

    #[tokio::test]
    async fn test_embedding_cache_get_or_compute_error() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";

        let result = cache
            .get_or_compute(query, |_| async {
                Err::<Vec<f32>, std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "test error",
                ))
            })
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "test error");
    }

    #[tokio::test]
    async fn test_embedding_cache_default() {
        let cache = EmbeddingCache::default();
        let query = "test query";
        let embedding = vec![0.1, 0.2, 0.3];

        cache.insert(query, embedding.clone()).await;

        let cached = cache.get(query).await;
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_embedding_cache_multiple_queries() {
        let cache = EmbeddingCache::new(100);

        let queries = vec![
            ("rust error handling", vec![0.1, 0.2, 0.3]),
            ("python async await", vec![0.4, 0.5, 0.6]),
            ("javascript promises", vec![0.7, 0.8, 0.9]),
        ];

        for (query, embedding) in &queries {
            cache.insert(*query, embedding.clone()).await;
        }

        for (query, expected_embedding) in &queries {
            let cached = cache.get(*query).await;
            assert!(cached.is_some());
            assert_eq!(*cached.unwrap(), *expected_embedding);
        }
    }

    #[tokio::test]
    async fn test_embedding_cache_overwrite() {
        let cache = EmbeddingCache::new(100);
        let query = "test query";
        let embedding1 = vec![0.1, 0.2, 0.3];
        let embedding2 = vec![0.4, 0.5, 0.6];

        cache.insert(query, embedding1).await;
        cache.insert(query, embedding2.clone()).await;

        let cached = cache.get(query).await;
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), embedding2);
    }
}
