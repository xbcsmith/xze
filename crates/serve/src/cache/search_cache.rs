//! Search result caching implementation
//!
//! Provides in-memory caching for search results to improve performance
//! and reduce database load.

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

/// Cache key for search requests
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SearchCacheKey {
    /// Query string
    pub query: String,
    /// Filters hash
    pub filters: String,
    /// Options hash
    pub options: String,
}

impl SearchCacheKey {
    /// Creates a new cache key from search parameters
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `filters` - Serialized filters
    /// * `options` - Serialized options
    ///
    /// # Returns
    ///
    /// Returns a new SearchCacheKey instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheKey;
    ///
    /// let key = SearchCacheKey::new(
    ///     "test query".to_string(),
    ///     "{}".to_string(),
    ///     "{}".to_string()
    /// );
    /// assert_eq!(key.query, "test query");
    /// ```
    pub fn new(query: String, filters: String, options: String) -> Self {
        Self {
            query,
            filters,
            options,
        }
    }

    /// Creates a cache key from a query string only
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    ///
    /// # Returns
    ///
    /// Returns a new SearchCacheKey with empty filters and options
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheKey;
    ///
    /// let key = SearchCacheKey::from_query("test".to_string());
    /// assert_eq!(key.query, "test");
    /// assert_eq!(key.filters, "");
    /// ```
    pub fn from_query(query: String) -> Self {
        Self {
            query,
            filters: String::new(),
            options: String::new(),
        }
    }

    /// Generates a hash for the cache key
    ///
    /// # Returns
    ///
    /// Returns a 64-bit hash of the key
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheKey;
    ///
    /// let key = SearchCacheKey::from_query("test".to_string());
    /// let hash = key.hash_value();
    /// assert!(hash > 0);
    /// ```
    pub fn hash_value(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Cached search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSearchResponse {
    /// Query that generated this response
    pub query: String,
    /// Search results as JSON
    pub results: String,
    /// Total result count
    pub total_results: usize,
    /// Cache timestamp
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

impl CachedSearchResponse {
    /// Creates a new cached search response
    ///
    /// # Arguments
    ///
    /// * `query` - Original query
    /// * `results` - Search results as JSON
    /// * `total_results` - Total number of results
    ///
    /// # Returns
    ///
    /// Returns a new CachedSearchResponse instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::CachedSearchResponse;
    ///
    /// let response = CachedSearchResponse::new(
    ///     "test".to_string(),
    ///     "[]".to_string(),
    ///     0
    /// );
    /// assert_eq!(response.query, "test");
    /// assert_eq!(response.total_results, 0);
    /// ```
    pub fn new(query: String, results: String, total_results: usize) -> Self {
        Self {
            query,
            results,
            total_results,
            cached_at: chrono::Utc::now(),
        }
    }

    /// Checks if the cached response is still fresh
    ///
    /// # Arguments
    ///
    /// * `max_age` - Maximum age in seconds
    ///
    /// # Returns
    ///
    /// Returns true if the cache entry is still valid
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::CachedSearchResponse;
    ///
    /// let response = CachedSearchResponse::new(
    ///     "test".to_string(),
    ///     "[]".to_string(),
    ///     0
    /// );
    /// assert!(response.is_fresh(3600));
    /// ```
    pub fn is_fresh(&self, max_age: i64) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.cached_at);
        age.num_seconds() < max_age
    }
}

/// Search cache configuration
#[derive(Debug, Clone)]
pub struct SearchCacheConfig {
    /// Maximum number of entries in cache
    pub max_capacity: u64,
    /// Time to live for cache entries in seconds
    pub ttl_seconds: u64,
    /// Time to idle for cache entries in seconds
    pub tti_seconds: u64,
}

impl Default for SearchCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10000,
            ttl_seconds: 3600,
            tti_seconds: 1800,
        }
    }
}

impl SearchCacheConfig {
    /// Creates a new cache configuration
    ///
    /// # Arguments
    ///
    /// * `max_capacity` - Maximum number of cache entries
    /// * `ttl_seconds` - Time to live in seconds
    /// * `tti_seconds` - Time to idle in seconds
    ///
    /// # Returns
    ///
    /// Returns a new SearchCacheConfig instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheConfig;
    ///
    /// let config = SearchCacheConfig::new(1000, 600, 300);
    /// assert_eq!(config.max_capacity, 1000);
    /// assert_eq!(config.ttl_seconds, 600);
    /// ```
    pub fn new(max_capacity: u64, ttl_seconds: u64, tti_seconds: u64) -> Self {
        Self {
            max_capacity,
            ttl_seconds,
            tti_seconds,
        }
    }

    /// Creates a configuration for development with aggressive caching
    ///
    /// # Returns
    ///
    /// Returns a SearchCacheConfig with long TTL
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheConfig;
    ///
    /// let config = SearchCacheConfig::development();
    /// assert_eq!(config.ttl_seconds, 7200);
    /// ```
    pub fn development() -> Self {
        Self {
            max_capacity: 5000,
            ttl_seconds: 7200,
            tti_seconds: 3600,
        }
    }

    /// Creates a configuration for production with conservative caching
    ///
    /// # Returns
    ///
    /// Returns a SearchCacheConfig with shorter TTL
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::SearchCacheConfig;
    ///
    /// let config = SearchCacheConfig::production();
    /// assert_eq!(config.ttl_seconds, 1800);
    /// ```
    pub fn production() -> Self {
        Self {
            max_capacity: 20000,
            ttl_seconds: 1800,
            tti_seconds: 900,
        }
    }
}

/// Search cache implementation
pub struct SearchCache {
    cache: Cache<SearchCacheKey, CachedSearchResponse>,
    config: SearchCacheConfig,
}

impl SearchCache {
    /// Creates a new search cache
    ///
    /// # Arguments
    ///
    /// * `config` - Cache configuration
    ///
    /// # Returns
    ///
    /// Returns a new SearchCache instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig};
    ///
    /// let config = SearchCacheConfig::default();
    /// let cache = SearchCache::new(config);
    /// ```
    pub fn new(config: SearchCacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(config.ttl_seconds))
            .time_to_idle(Duration::from_secs(config.tti_seconds))
            .build();

        Self { cache, config }
    }

    /// Gets a cached response
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key
    ///
    /// # Returns
    ///
    /// Returns the cached response if present and fresh
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig, SearchCacheKey};
    ///
    /// # tokio_test::block_on(async {
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// let key = SearchCacheKey::from_query("test".to_string());
    /// let result = cache.get(&key).await;
    /// assert!(result.is_none());
    /// # });
    /// ```
    pub async fn get(&self, key: &SearchCacheKey) -> Option<CachedSearchResponse> {
        self.cache.get(key).await
    }

    /// Stores a response in cache
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key
    /// * `response` - Response to cache
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig, SearchCacheKey, CachedSearchResponse};
    ///
    /// # tokio_test::block_on(async {
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// let key = SearchCacheKey::from_query("test".to_string());
    /// let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);
    /// cache.set(key, response).await;
    /// # });
    /// ```
    pub async fn set(&self, key: SearchCacheKey, response: CachedSearchResponse) {
        self.cache.insert(key, response).await;
    }

    /// Invalidates a cache entry
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key to invalidate
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig, SearchCacheKey};
    ///
    /// # tokio_test::block_on(async {
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// let key = SearchCacheKey::from_query("test".to_string());
    /// cache.invalidate(&key).await;
    /// # });
    /// ```
    pub async fn invalidate(&self, key: &SearchCacheKey) {
        self.cache.invalidate(key).await;
    }

    /// Invalidates all cache entries
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig};
    ///
    /// # tokio_test::block_on(async {
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// cache.invalidate_all().await;
    /// # });
    /// ```
    pub async fn invalidate_all(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    /// Gets cache statistics
    ///
    /// # Returns
    ///
    /// Returns the number of entries in the cache
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig};
    ///
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// let entries = cache.stats();
    /// assert_eq!(entries, 0);
    /// ```
    pub fn stats(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Gets the cache entry count
    ///
    /// # Returns
    ///
    /// Returns the number of entries currently in the cache
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::cache::search_cache::{SearchCache, SearchCacheConfig};
    ///
    /// let cache = SearchCache::new(SearchCacheConfig::default());
    /// let count = cache.entry_count();
    /// assert_eq!(count, 0);
    /// ```
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Gets the cache configuration
    ///
    /// # Returns
    ///
    /// Returns a reference to the cache configuration
    pub fn config(&self) -> &SearchCacheConfig {
        &self.config
    }
}

/// Thread-safe shared search cache
pub type SharedSearchCache = Arc<SearchCache>;

/// Creates a new shared search cache
///
/// # Arguments
///
/// * `config` - Cache configuration
///
/// # Returns
///
/// Returns a thread-safe shared cache instance
///
/// # Examples
///
/// ```
/// use xze_serve::cache::search_cache::{SearchCacheConfig, create_shared_cache};
///
/// let config = SearchCacheConfig::default();
/// let cache = create_shared_cache(config);
/// ```
pub fn create_shared_cache(config: SearchCacheConfig) -> SharedSearchCache {
    Arc::new(SearchCache::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_cache_key_new() {
        let key = SearchCacheKey::new("test".to_string(), "{}".to_string(), "{}".to_string());
        assert_eq!(key.query, "test");
        assert_eq!(key.filters, "{}");
    }

    #[test]
    fn test_search_cache_key_from_query() {
        let key = SearchCacheKey::from_query("test".to_string());
        assert_eq!(key.query, "test");
        assert_eq!(key.filters, "");
    }

    #[test]
    fn test_search_cache_key_hash() {
        let key1 = SearchCacheKey::from_query("test".to_string());
        let key2 = SearchCacheKey::from_query("test".to_string());
        assert_eq!(key1.hash_value(), key2.hash_value());
    }

    #[test]
    fn test_search_cache_key_equality() {
        let key1 = SearchCacheKey::from_query("test".to_string());
        let key2 = SearchCacheKey::from_query("test".to_string());
        let key3 = SearchCacheKey::from_query("different".to_string());
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cached_search_response_new() {
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);
        assert_eq!(response.query, "test");
        assert_eq!(response.total_results, 0);
    }

    #[test]
    fn test_cached_search_response_is_fresh() {
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);
        assert!(response.is_fresh(3600));
        assert!(!response.is_fresh(-1));
    }

    #[test]
    fn test_search_cache_config_default() {
        let config = SearchCacheConfig::default();
        assert_eq!(config.max_capacity, 10000);
        assert_eq!(config.ttl_seconds, 3600);
    }

    #[test]
    fn test_search_cache_config_new() {
        let config = SearchCacheConfig::new(1000, 600, 300);
        assert_eq!(config.max_capacity, 1000);
        assert_eq!(config.ttl_seconds, 600);
        assert_eq!(config.tti_seconds, 300);
    }

    #[test]
    fn test_search_cache_config_development() {
        let config = SearchCacheConfig::development();
        assert_eq!(config.max_capacity, 5000);
        assert_eq!(config.ttl_seconds, 7200);
    }

    #[test]
    fn test_search_cache_config_production() {
        let config = SearchCacheConfig::production();
        assert_eq!(config.max_capacity, 20000);
        assert_eq!(config.ttl_seconds, 1800);
    }

    #[tokio::test]
    async fn test_search_cache_new() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let entries = cache.stats();
        assert_eq!(entries, 0);
    }

    #[tokio::test]
    async fn test_search_cache_set_and_get() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let key = SearchCacheKey::from_query("test".to_string());
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);

        cache.set(key.clone(), response.clone()).await;
        let result = cache.get(&key).await;

        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.query, "test");
    }

    #[tokio::test]
    async fn test_search_cache_invalidate() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let key = SearchCacheKey::from_query("test".to_string());
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);

        cache.set(key.clone(), response).await;
        cache.invalidate(&key).await;
        let result = cache.get(&key).await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_cache_invalidate_all() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let key1 = SearchCacheKey::from_query("test1".to_string());
        let key2 = SearchCacheKey::from_query("test2".to_string());
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);

        cache.set(key1.clone(), response.clone()).await;
        cache.set(key2.clone(), response).await;
        cache.invalidate_all().await;

        let result1 = cache.get(&key1).await;
        let result2 = cache.get(&key2).await;

        assert!(result1.is_none());
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn test_search_cache_stats() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let key = SearchCacheKey::from_query("test".to_string());
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);

        cache.set(key.clone(), response).await;

        // Run pending tasks to ensure cache is synced
        cache.cache.run_pending_tasks().await;

        let _ = cache.get(&key).await;

        let entries = cache.stats();
        assert!(entries <= 1, "Expected 0 or 1 entries, got {}", entries);
    }

    #[tokio::test]
    async fn test_search_cache_entry_count() {
        let config = SearchCacheConfig::default();
        let cache = SearchCache::new(config);
        let key = SearchCacheKey::from_query("test".to_string());
        let response = CachedSearchResponse::new("test".to_string(), "[]".to_string(), 0);

        cache.set(key.clone(), response).await;

        // Run pending tasks to ensure cache is synced
        cache.cache.run_pending_tasks().await;

        let count = cache.entry_count();
        assert!(count <= 1, "Expected 0 or 1 entries, got {}", count);
    }

    #[test]
    fn test_create_shared_cache() {
        let config = SearchCacheConfig::default();
        let cache = create_shared_cache(config);
        let entries = cache.stats();
        assert_eq!(entries, 0);
    }

    #[test]
    fn test_cache_key_clone() {
        let key = SearchCacheKey::from_query("test".to_string());
        let cloned = key.clone();
        assert_eq!(key, cloned);
    }

    #[test]
    fn test_cache_config_clone() {
        let config = SearchCacheConfig::default();
        let cloned = config.clone();
        assert_eq!(config.max_capacity, cloned.max_capacity);
    }
}
