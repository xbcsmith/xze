//! Caching module for XZe serve crate
//!
//! Provides caching implementations for search results and other data
//! to improve performance and reduce database load.

pub mod search_cache;

pub use search_cache::{
    create_shared_cache, CachedSearchResponse, SearchCache, SearchCacheConfig, SearchCacheKey,
    SharedSearchCache,
};
