//! OpenAPI documentation for search API
//!
//! This module provides OpenAPI 3.0 specification generation for the search endpoints
//! using utoipa. It includes all search-related types, endpoints, and examples.
//!
//! # Features
//!
//! This module is only available when the `openapi` feature is enabled.
//!
//! # Examples
//!
//! ```ignore
//! use xze_serve::search::openapi::SearchApiDoc;
//! use utoipa::OpenApi;
//!
//! let openapi_spec = SearchApiDoc::openapi();
//! let json = openapi_spec.to_json().unwrap();
//! ```

#[cfg(feature = "openapi")]
use utoipa::OpenApi;

#[cfg(feature = "openapi")]
use super::handlers::*;
#[cfg(feature = "openapi")]
use super::types::*;

/// OpenAPI documentation for search endpoints
///
/// Generates the complete OpenAPI 3.0 specification for search functionality.
/// This includes both GET and POST endpoints, all schemas, and examples.
///
/// # Features
///
/// This is only available when the `openapi` feature is enabled.
#[cfg(feature = "openapi")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "XZe Search API",
        version = "1.0.0",
        description = "Hybrid search API supporting both simple GET and advanced POST queries",
    ),
    paths(
        super::handlers::handle_search,
        super::handlers::handle_search_advanced,
    ),
    components(
        schemas(
            SearchQuery,
            AdvancedSearchRequest,
            SearchFilters,
            SearchOptions,
            AggregationRequest,
            SimilarityRange,
            DateRange,
            SearchResponse,
            SearchResult,
            AggregationResponse,
            CategoryCount,
            SimilarityRangeCount,
            DateCount,
            PaginationInfo,
            ErrorResponse,
        )
    ),
    tags(
        (name = "search", description = "Semantic search endpoints for documentation"),
    )
)]
pub struct SearchApiDoc;

#[cfg(test)]
#[cfg(feature = "openapi")]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = SearchApiDoc::openapi();
        assert_eq!(spec.info.title, "XZe Search API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[test]
    fn test_openapi_has_search_paths() {
        let spec = SearchApiDoc::openapi();
        let paths = spec.paths.paths;
        assert!(paths.contains_key("/search"));
        assert!(paths.contains_key("/search/advanced"));
    }

    #[test]
    fn test_openapi_has_schemas() {
        let spec = SearchApiDoc::openapi();
        let schemas = spec.components.as_ref().unwrap().schemas.clone();
        assert!(schemas.contains_key("SearchQuery"));
        assert!(schemas.contains_key("AdvancedSearchRequest"));
        assert!(schemas.contains_key("SearchResponse"));
        assert!(schemas.contains_key("SearchResult"));
    }

    #[test]
    fn test_openapi_json_generation() {
        let spec = SearchApiDoc::openapi();
        let json = spec.to_json();
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("XZe Search API"));
        assert!(json_str.contains("/search"));
    }

    #[test]
    fn test_openapi_has_all_schemas() {
        let spec = SearchApiDoc::openapi();
        let schemas = spec.components.as_ref().unwrap().schemas.clone();
        assert!(schemas.contains_key("SearchFilters"));
        assert!(schemas.contains_key("SearchOptions"));
        assert!(schemas.contains_key("PaginationInfo"));
        assert!(schemas.contains_key("AggregationResponse"));
    }
}
