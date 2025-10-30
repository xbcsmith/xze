//! Search API handlers
//!
//! This module implements the HTTP handlers for the search API, supporting
//! both simple GET requests and advanced POST requests with complex filters
//! and aggregations.
//!
//! # Endpoints
//!
//! - `GET /search?q=query&limit=10` - Simple search
//! - `POST /search/advanced` - Advanced search with JSON body
//!
//! # Examples
//!
//! ```rust,no_run
//! use axum::Router;
//! use xze_serve::search::handlers::search_routes;
//!
//! # async fn example() {
//! let app = Router::new().nest("/api", search_routes());
//! # }
//! ```

use crate::search::types::{
    AdvancedSearchRequest, AggregationResponse, CategoryCount, PaginationInfo, SearchError,
    SearchResponse, SearchResult,
};
use axum::{
    extract::{Json, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Query parameters for GET search endpoint
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchQuery {
    /// Search query string
    pub q: String,

    /// Maximum number of results (default: 20, max: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,

    /// Filter by category
    pub category: Option<String>,

    /// Filter by repository
    pub repository: Option<String>,
}

fn default_limit() -> usize {
    20
}

/// Error response structure
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Optional error details
    pub details: Option<String>,
}

impl IntoResponse for SearchError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SearchError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, msg),
            SearchError::InvalidFilter(msg) => (StatusCode::BAD_REQUEST, msg),
            SearchError::InvalidOptions(msg) => (StatusCode::BAD_REQUEST, msg),
            SearchError::DatabaseError(msg) => {
                error!("Database error during search: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            SearchError::InternalError(msg) => {
                error!("Internal search error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}

/// Creates the search routes
///
/// # Examples
///
/// ```rust,no_run
/// use axum::Router;
/// use xze_serve::search::handlers::search_routes;
///
/// # async fn example() {
/// let app = Router::new().nest("/api", search_routes());
/// # }
/// ```
pub fn search_routes() -> Router {
    Router::new()
        .route("/search", get(handle_search))
        .route("/search/advanced", post(handle_search_advanced))
}

/// GET search handler for simple queries
///
/// Handles simple search requests via query parameters.
///
/// # Query Parameters
///
/// - `q`: Search query string (required)
/// - `limit`: Maximum results to return (default: 20, max: 100)
/// - `offset`: Offset for pagination (default: 0)
/// - `category`: Filter by category (optional)
/// - `repository`: Filter by repository (optional)
///
/// # Errors
///
/// Returns `SearchError::InvalidQuery` if query is empty or invalid
/// Returns `SearchError::InvalidOptions` if limit/offset are invalid
///
/// # Examples
///
/// ```text
/// GET /api/search?q=rust+async&limit=10
/// GET /api/search?q=documentation&category=tutorial
/// ```
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/search",
    tag = "search",
    params(
        ("q" = String, Query, description = "Search query string"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results (default: 20, max: 100)"),
        ("offset" = Option<usize>, Query, description = "Offset for pagination (default: 0)"),
        ("category" = Option<String>, Query, description = "Filter by documentation category"),
        ("repository" = Option<String>, Query, description = "Filter by repository name"),
    ),
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResponse),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
))]
pub async fn handle_search(
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, SearchError> {
    info!(
        "GET search request: query='{}', limit={}, offset={}",
        params.q, params.limit, params.offset
    );

    // Validate query
    if params.q.trim().is_empty() {
        warn!("Empty search query received");
        return Err(SearchError::InvalidQuery(
            "Query cannot be empty".to_string(),
        ));
    }

    // Validate limit
    if params.limit == 0 {
        return Err(SearchError::InvalidOptions(
            "Limit must be greater than 0".to_string(),
        ));
    }
    if params.limit > 100 {
        return Err(SearchError::InvalidOptions(
            "Limit cannot exceed 100".to_string(),
        ));
    }

    // Perform search (mock implementation for now)
    let results = perform_search(&params.q, params.limit, params.offset, &params).await?;

    let total_results = results.len();
    let pagination = PaginationInfo::new(params.offset, params.limit, total_results);

    let response = SearchResponse {
        query: params.q.clone(),
        results,
        total_results,
        aggregations: None,
        pagination,
    };

    info!(
        "GET search completed: {} results for query '{}'",
        total_results, params.q
    );

    Ok(Json(response))
}

/// POST advanced search handler with full filtering and aggregation support
///
/// Handles complex search requests with filters, options, and aggregations.
///
/// # Request Body
///
/// JSON object with:
/// - `query`: Search query string (required)
/// - `filters`: Optional search filters (category, similarity, date range, etc.)
/// - `options`: Optional search options (pagination, snippets, highlighting)
/// - `aggregations`: Optional aggregation requests
///
/// # Errors
///
/// Returns `SearchError::InvalidQuery` if query is invalid
/// Returns `SearchError::InvalidFilter` if filters are invalid
/// Returns `SearchError::InvalidOptions` if options are invalid
///
/// # Examples
///
/// ```text
/// POST /api/search/advanced
/// Content-Type: application/json
///
/// {
///   "query": "rust async programming",
///   "filters": {
///     "categories": ["tutorial", "reference"],
///     "similarity": { "min": 0.7 }
///   },
///   "options": {
///     "max_results": 50,
///     "include_snippets": true,
///     "highlight_terms": true
///   },
///   "aggregations": {
///     "by_category": true
///   }
/// }
/// ```
#[cfg_attr(feature = "openapi", utoipa::path(
    post,
    path = "/search/advanced",
    tag = "search",
    request_body = AdvancedSearchRequest,
    responses(
        (status = 200, description = "Advanced search completed successfully", body = SearchResponse),
        (status = 400, description = "Invalid request body or parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
))]
pub async fn handle_search_advanced(
    Json(request): Json<AdvancedSearchRequest>,
) -> Result<Json<SearchResponse>, SearchError> {
    info!("POST advanced search request: query='{}'", request.query);

    // Validate request
    request.validate()?;

    // Extract options with defaults
    let max_results = request
        .options
        .as_ref()
        .map(|o| o.get_max_results())
        .unwrap_or(20);
    let offset = request
        .options
        .as_ref()
        .map(|o| o.get_offset())
        .unwrap_or(0);
    let include_snippets = request
        .options
        .as_ref()
        .map(|o| o.get_include_snippets())
        .unwrap_or(true);
    let highlight_terms = request
        .options
        .as_ref()
        .map(|o| o.get_highlight_terms())
        .unwrap_or(false);

    info!(
        "Search options: max_results={}, offset={}, snippets={}, highlight={}",
        max_results, offset, include_snippets, highlight_terms
    );

    // Perform advanced search (mock implementation for now)
    let results = perform_advanced_search(&request, max_results, offset).await?;

    // Compute aggregations if requested
    let aggregations = if request.aggregations.is_some() {
        Some(compute_aggregations(&results, &request.aggregations))
    } else {
        None
    };

    let total_results = results.len();
    let pagination = PaginationInfo::new(offset, max_results, total_results);

    let response = SearchResponse {
        query: request.query.clone(),
        results,
        total_results,
        aggregations,
        pagination,
    };

    info!(
        "POST advanced search completed: {} results for query '{}'",
        total_results, request.query
    );

    Ok(Json(response))
}

/// Performs a simple search (mock implementation)
///
/// This is a placeholder that will be replaced with actual database queries
/// in future implementations.
async fn perform_search(
    query: &str,
    limit: usize,
    offset: usize,
    params: &SearchQuery,
) -> Result<Vec<SearchResult>, SearchError> {
    // Mock implementation - returns sample data
    info!(
        "Performing search: query='{}', limit={}, offset={}",
        query, limit, offset
    );

    let mut results = Vec::new();

    // Generate mock results based on query
    let result_count = std::cmp::min(limit, 5); // Mock: return up to 5 results

    for i in 0..result_count {
        // Apply category filter if specified
        let category = if let Some(ref cat) = params.category {
            cat.clone()
        } else {
            match i % 4 {
                0 => "tutorial".to_string(),
                1 => "how-to".to_string(),
                2 => "explanation".to_string(),
                _ => "reference".to_string(),
            }
        };

        let repository = params
            .repository
            .clone()
            .unwrap_or_else(|| "xze".to_string());

        results.push(SearchResult {
            id: format!("doc-{}", offset + i),
            title: format!("Document {} about {}", i + 1, query),
            content: format!(
                "This is content for document {} matching query '{}'",
                i + 1,
                query
            ),
            snippet: Some(format!("...content matching '{}'...", query)),
            category: category.clone(),
            similarity: Some(0.85 - (i as f32 * 0.05)),
            repository,
            path: format!("docs/{}/document_{}.md", category, i + 1),
            tags: vec!["rust".to_string(), "documentation".to_string()],
            updated_at: Utc::now(),
        });
    }

    Ok(results)
}

/// Performs an advanced search with filters (mock implementation)
///
/// This is a placeholder that will be replaced with actual database queries
/// and vector similarity search in future implementations.
async fn perform_advanced_search(
    request: &AdvancedSearchRequest,
    max_results: usize,
    offset: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    info!(
        "Performing advanced search: query='{}', max_results={}, offset={}",
        request.query, max_results, offset
    );

    let mut results = Vec::new();

    // Generate mock results
    let result_count = std::cmp::min(max_results, 10);

    // Apply category filter if present
    let categories = if let Some(ref filters) = request.filters {
        filters.categories.clone().unwrap_or_else(|| {
            vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "explanation".to_string(),
                "reference".to_string(),
            ]
        })
    } else {
        vec![
            "tutorial".to_string(),
            "how-to".to_string(),
            "explanation".to_string(),
            "reference".to_string(),
        ]
    };

    for i in 0..result_count {
        let category = categories[i % categories.len()].clone();

        // Apply similarity filter if present
        let mut similarity = 0.9 - (i as f32 * 0.05);
        if let Some(ref filters) = request.filters {
            if let Some(ref sim_range) = filters.similarity {
                if let Some(min) = sim_range.min {
                    if similarity < min {
                        continue; // Skip results below minimum similarity
                    }
                }
                if let Some(max) = sim_range.max {
                    if similarity > max {
                        similarity = max; // Cap at maximum similarity
                    }
                }
            }
        }

        let repository = if let Some(ref filters) = request.filters {
            filters
                .repositories
                .as_ref()
                .and_then(|repos| repos.first())
                .cloned()
                .unwrap_or_else(|| "xze".to_string())
        } else {
            "xze".to_string()
        };

        let snippet = if let Some(ref options) = request.options {
            if options.get_include_snippets() {
                Some(format!("...content matching '{}'...", request.query))
            } else {
                None
            }
        } else {
            None
        };

        results.push(SearchResult {
            id: format!("doc-advanced-{}", offset + i),
            title: format!("Advanced Document {} about {}", i + 1, request.query),
            content: format!(
                "This is advanced content for document {} matching query '{}'",
                i + 1,
                request.query
            ),
            snippet,
            category: category.clone(),
            similarity: Some(similarity),
            repository,
            path: format!("docs/{}/advanced_{}.md", category, i + 1),
            tags: vec!["rust".to_string(), "advanced".to_string()],
            updated_at: Utc::now(),
        });
    }

    Ok(results)
}

/// Computes aggregations from search results (mock implementation)
fn compute_aggregations(
    results: &[SearchResult],
    aggregation_request: &Option<crate::search::types::AggregationRequest>,
) -> AggregationResponse {
    let mut response = AggregationResponse::default();

    if let Some(ref agg_req) = aggregation_request {
        // Aggregate by category
        if agg_req.by_category.unwrap_or(false) {
            let mut category_counts: HashMap<String, usize> = HashMap::new();
            for result in results {
                *category_counts.entry(result.category.clone()).or_insert(0) += 1;
            }

            response.by_category = Some(
                category_counts
                    .into_iter()
                    .map(|(category, count)| CategoryCount { category, count })
                    .collect(),
            );
        }

        // Aggregate by similarity range
        if agg_req.by_similarity_range.unwrap_or(false) {
            // Mock implementation: group into ranges
            response.by_similarity_range = Some(vec![
                crate::search::types::SimilarityRangeCount {
                    range: "0.8-1.0".to_string(),
                    count: results
                        .iter()
                        .filter(|r| r.similarity.unwrap_or(0.0) >= 0.8)
                        .count(),
                },
                crate::search::types::SimilarityRangeCount {
                    range: "0.6-0.8".to_string(),
                    count: results
                        .iter()
                        .filter(|r| {
                            let sim = r.similarity.unwrap_or(0.0);
                            (0.6..0.8).contains(&sim)
                        })
                        .count(),
                },
            ]);
        }

        // Aggregate by date
        if agg_req.by_date.unwrap_or(false) {
            // Mock implementation
            response.by_date = Some(vec![crate::search::types::DateCount {
                period: "2024-01".to_string(),
                count: results.len(),
            }]);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::types::{AggregationRequest, SearchFilters, SearchOptions, SimilarityRange};

    #[tokio::test]
    async fn test_handle_search_advanced_success() {
        let request = AdvancedSearchRequest {
            query: "rust async".to_string(),
            filters: Some(SearchFilters {
                categories: Some(vec!["tutorial".to_string()]),
                similarity: None,
                date_range: None,
                tags: None,
                repositories: None,
            }),
            options: Some(SearchOptions {
                max_results: Some(10),
                offset: Some(0),
                include_snippets: Some(true),
                highlight_terms: Some(false),
                group_by: None,
            }),
            aggregations: None,
        };

        let result = handle_search_advanced(Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.query, "rust async");
        assert!(!response.results.is_empty());
    }

    #[tokio::test]
    async fn test_handle_search_advanced_validation_error() {
        let request = AdvancedSearchRequest {
            query: "   ".to_string(), // Empty query
            filters: None,
            options: None,
            aggregations: None,
        };

        let result = handle_search_advanced(Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_search_with_filters() {
        let request = AdvancedSearchRequest {
            query: "test".to_string(),
            filters: Some(SearchFilters {
                categories: Some(vec!["tutorial".to_string(), "reference".to_string()]),
                similarity: Some(SimilarityRange {
                    min: Some(0.7),
                    max: Some(1.0),
                }),
                date_range: None,
                tags: None,
                repositories: None,
            }),
            options: None,
            aggregations: None,
        };

        let result = handle_search_advanced(Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_search_with_aggregations() {
        let request = AdvancedSearchRequest {
            query: "test".to_string(),
            filters: None,
            options: None,
            aggregations: Some(AggregationRequest {
                by_category: Some(true),
                by_similarity_range: Some(true),
                by_date: Some(false),
            }),
        };

        let result = handle_search_advanced(Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.aggregations.is_some());
        let aggs = response.aggregations.unwrap();
        assert!(aggs.by_category.is_some());
        assert!(aggs.by_similarity_range.is_some());
    }

    #[test]
    fn test_search_query_defaults() {
        let json = r#"{"q": "test"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
    }
}
