//! Search API module
//!
//! This module provides the search functionality for the XZe server, including
//! both simple GET-based searches and advanced POST-based searches with filters,
//! aggregations, and pagination.
//!
//! # Architecture
//!
//! The search module is organized into:
//! - `types`: Request/response structures and error types
//! - `handlers`: HTTP handlers for search endpoints
//!
//! # Usage
//!
//! ```rust,no_run
//! use axum::Router;
//! use xze_serve::search::handlers::search_routes;
//!
//! # async fn example() {
//! let app = Router::new().nest("/api", search_routes());
//! # }
//! ```
//!
//! # Endpoints
//!
//! ## GET /search
//!
//! Simple search with query parameters:
//!
//! ```text
//! GET /api/search?q=rust+async&limit=20&offset=0
//! ```
//!
//! ## POST /search/advanced
//!
//! Advanced search with JSON body:
//!
//! ```text
//! POST /api/search/advanced
//! Content-Type: application/json
//!
//! {
//!   "query": "rust async programming",
//!   "filters": {
//!     "categories": ["tutorial", "reference"],
//!     "similarity": { "min": 0.7 }
//!   },
//!   "options": {
//!     "max_results": 50,
//!     "include_snippets": true
//!   }
//! }
//! ```

pub mod handlers;
pub mod metrics;
pub mod types;

#[cfg(feature = "openapi")]
pub mod openapi;

// Re-export commonly used types
pub use handlers::{handle_search, handle_search_advanced, search_routes, SearchQuery};
pub use metrics::SearchMetrics;
pub use types::{
    AdvancedSearchRequest, AggregationRequest, AggregationResponse, CategoryCount, DateCount,
    DateRange, PaginationInfo, SearchError, SearchFilters, SearchOptions, SearchResponse,
    SearchResult, SimilarityRange, SimilarityRangeCount,
};

#[cfg(feature = "openapi")]
pub use openapi::SearchApiDoc;

#[cfg(feature = "metrics")]
pub use metrics::SEARCH_METRICS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that main types are exported
        let _request = AdvancedSearchRequest {
            query: "test".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
    }
}
