//! Search functionality for XZe
//!
//! This module provides search-related functionality including:
//! - Query embedding caching for performance optimization
//! - Database query optimization
//! - Pagination support
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

pub mod embedding_cache;
pub mod expansion;
pub mod hybrid;
pub mod pipeline;
pub mod rerank;

pub use embedding_cache::EmbeddingCache;
pub use expansion::{ContextExpander, ExpandedDocument, ExpansionError};
pub use hybrid::{HybridSearcher, SearchQuery, SearchResult};
pub use pipeline::{PipelineError, SearchPipeline, SearchPipelineConfig};
pub use rerank::{RerankError, RerankResult, Reranker};
