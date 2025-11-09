# Search Features Unified Implementation Plan

## Overview

This document provides a comprehensive, phased implementation plan for XZe's search functionality. It combines the API versioning foundation (Phase 7.5 completed), the hybrid GET/POST search approach, OpenAPI documentation, testing strategy, and advanced search features into a single unified roadmap.

**Key Strategy**: Implement a hybrid search API that supports both GET (simple, cacheable searches) and POST (advanced, complex queries) to serve different use cases optimally while maintaining backward compatibility and following REST best practices.

## Current State

### Completed (Phase 7.5)

- API versioning infrastructure with `/api/v1` routing
- Version negotiation via `Accept-Version` header
- Legacy endpoint support with deprecation headers
- Migration middleware and backward compatibility
- Basic integration tests (DB-dependent tests marked `#[ignore]`)
- Initial migration documentation

### Existing Search Functionality

- GET `/api/v1/search` endpoint with query parameters
- Basic search with `q`, `max_results`, `min_similarity` parameters
- Vector similarity search using embeddings
- PostgreSQL with pgvector integration
- Simple category filtering

### Gaps Identified

1. No POST endpoint for complex search queries
2. OpenAPI documentation not yet integrated
3. Limited filter capabilities (only basic category filter)
4. No search result aggregations or faceting
5. No pagination support
6. No caching strategy
7. No rate limiting or security hardening
8. Missing advanced features (multi-field, saved searches, real-time)

## Implementation Philosophy

### Hybrid GET/POST Approach

**GET `/api/v1/search`** - Simple searches (90% of use cases)
- Cacheable via HTTP headers
- Bookmarkable and shareable URLs
- Browser-friendly
- Good for documentation examples
- Query parameters: `q`, `max_results`, `min_similarity`, `category`

**POST `/api/v1/search`** - Advanced searches (10% of use cases)
- No size limitations
- Complex nested filter structures
- Aggregations and grouping
- Better for programmatic API usage
- JSON request body with rich query DSL

Both endpoints share:
- Same response format
- Same internal search logic
- Same authentication/authorization
- Same rate limiting rules

## Implementation Phases

## Phase 7.6: Hybrid Search API Implementation

**Goal**: Add POST endpoint alongside existing GET endpoint to support complex search queries.

**Duration**: 1 week

### 7.6.1: Define Request/Response Types

#### Tasks

1. Create request types for POST endpoint:
   - `AdvancedSearchRequest` - Root request structure
   - `SearchFilters` - Filter options (categories, date ranges, tags, repos)
   - `SearchOptions` - Search behavior options (pagination, highlighting, grouping)
   - `AggregationRequest` - Aggregation configuration
   - `SimilarityRange` - Min/max similarity bounds
   - `DateRange` - Date filtering

2. Create shared response type:
   - `SearchResponse` - Used by both GET and POST
   - `SearchResult` - Individual result item
   - `AggregationResponse` - Aggregation results

3. Add request validation:
   - Required fields validation
   - Range validation (similarity 0.0-1.0, max_results 1-100)
   - Date range validation
   - Filter consistency checks

#### Deliverables

```rust
// crates/serve/src/api/v1/types/search.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct AdvancedSearchRequest {
    #[validate(length(min = 1, max = 1000))]
    pub query: String,

    pub filters: Option<SearchFilters>,
    pub options: Option<SearchOptions>,
    pub aggregations: Option<AggregationRequest>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchFilters {
    pub categories: Option<Vec<String>>,
    pub similarity: Option<SimilarityRange>,
    pub date_range: Option<DateRange>,
    pub tags: Option<Vec<String>>,
    pub repositories: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchOptions {
    #[validate(range(min = 1, max = 100))]
    pub max_results: Option<u32>,

    #[validate(range(min = 0))]
    pub offset: Option<u32>,

    pub include_snippets: Option<bool>,
    pub highlight_terms: Option<bool>,
    pub group_by: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AggregationRequest {
    pub by_category: Option<bool>,
    pub by_similarity_range: Option<bool>,
    pub by_date: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SimilarityRange {
    #[validate(range(min = 0.0, max = 1.0))]
    pub min: Option<f32>,

    #[validate(range(min = 0.0, max = 1.0))]
    pub max: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct DateRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: usize,
    pub aggregations: Option<AggregationResponse>,
    pub pagination: Option<PaginationInfo>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub snippet: Option<String>,
    pub category: String,
    pub similarity: f32,
    pub repository: String,
    pub path: String,
    pub tags: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AggregationResponse {
    pub by_category: Option<Vec<CategoryCount>>,
    pub by_similarity_range: Option<Vec<SimilarityRangeCount>>,
    pub by_date: Option<Vec<DateCount>>,
}

#[derive(Debug, Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SimilarityRangeCount {
    pub range: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct DateCount {
    pub period: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub offset: u32,
    pub limit: u32,
    pub total: usize,
    pub has_more: bool,
}
```

#### Acceptance Criteria

- All types have proper validation rules
- Types are well-documented with doc comments
- Serialization/deserialization works correctly
- Validation catches invalid input
- Types are tested with unit tests

### 7.6.2: Implement POST Search Handler

#### Tasks

1. Create POST handler function:
   - `handle_search_advanced()` in `crates/serve/src/api/v1/handlers.rs`
   - Extract and validate `AdvancedSearchRequest` from JSON body
   - Convert request to internal search parameters
   - Call shared search logic
   - Format and return `SearchResponse`

2. Create shared search logic:
   - Extract common search functionality from GET handler
   - Create `execute_search()` function that both handlers use
   - Support all filter types (categories, similarity, dates, tags, repos)
   - Implement aggregation logic
   - Handle pagination

3. Update GET handler:
   - Refactor to use shared `execute_search()` function
   - Maintain backward compatibility
   - Keep simple parameter interface

4. Add error handling:
   - Invalid request body (400 Bad Request)
   - Validation failures (422 Unprocessable Entity)
   - Search service errors (500 Internal Server Error)
   - Rate limit exceeded (429 Too Many Requests)

#### Deliverables

```rust
// crates/serve/src/api/v1/handlers.rs

use axum::{extract::State, Json};
use validator::Validate;
use crate::api::v1::types::search::*;
use crate::error::ApiError;

/// POST /api/v1/search - Advanced search with JSON request
///
/// Supports complex queries with filters, aggregations, and pagination.
///
/// # Request Body
///
/// ```json
/// {
///   "query": "getting started",
///   "filters": {
///     "categories": ["tutorial"],
///     "similarity": {"min": 0.8}
///   },
///   "options": {
///     "max_results": 20,
///     "include_snippets": true
///   }
/// }
/// ```
///
/// # Response
///
/// Returns SearchResponse with results and optional aggregations.
pub async fn handle_search_advanced(
    State(state): State<AppState>,
    Json(request): Json<AdvancedSearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    request.validate()?;

    let search_params = SearchParams {
        query: request.query.clone(),
        filters: request.filters,
        options: request.options.unwrap_or_default(),
        aggregations: request.aggregations,
    };

    let response = execute_search(&state, search_params).await?;

    Ok(Json(response))
}

/// Shared search execution logic used by both GET and POST handlers
async fn execute_search(
    state: &AppState,
    params: SearchParams,
) -> Result<SearchResponse, ApiError> {
    // Generate embedding for query
    let embedding = state.ai_service
        .generate_embedding(&params.query)
        .await?;

    // Build SQL query with filters
    let mut query_builder = QueryBuilder::new();
    query_builder
        .with_embedding(&embedding)
        .with_filters(&params.filters)
        .with_limit(params.options.max_results.unwrap_or(10))
        .with_offset(params.options.offset.unwrap_or(0));

    // Execute search
    let results = state.db_pool
        .search_documents(query_builder.build())
        .await?;

    // Apply aggregations if requested
    let aggregations = if params.aggregations.is_some() {
        Some(compute_aggregations(&results, &params.aggregations.unwrap()))
    } else {
        None
    };

    // Build response
    Ok(SearchResponse {
        query: params.query,
        results: results.into_iter().map(|r| r.into()).collect(),
        total_results: results.len(),
        aggregations,
        pagination: build_pagination_info(&params.options, results.len()),
    })
}
```

#### Acceptance Criteria

- POST handler accepts valid JSON requests
- Request validation works correctly
- Filters are applied properly to search results
- Aggregations are computed accurately
- Pagination works as expected
- Error responses are appropriate and informative
- GET and POST handlers share search logic (no duplication)

### 7.6.3: Update Routing

#### Tasks

1. Add POST route to v1 router
2. Ensure both GET and POST are registered
3. Update route documentation
4. Verify route priority and matching

#### Deliverables

```rust
// crates/serve/src/api/v1/mod.rs

use axum::{routing::{get, post}, Router};

pub fn create_v1_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/version", get(handlers::version))
        .route(
            "/search",
            get(handlers::handle_search)
                .post(handlers::handle_search_advanced)
        )
        .route("/repositories", get(handlers::list_repositories))
        .route("/repositories/:id", get(handlers::get_repository))
        .route("/documentation/:id", get(handlers::get_documentation))
        .with_state(state)
}
```

#### Acceptance Criteria

- GET `/api/v1/search` works with query parameters
- POST `/api/v1/search` works with JSON body
- Routing correctly dispatches to appropriate handler
- Both methods return consistent response format

### 7.6.4: Testing

#### Tasks

1. Add unit tests for types (validation, serialization)
2. Add handler tests (success, failure, edge cases)
3. Add integration tests for both GET and POST
4. Add equivalence tests (GET and POST with same query produce same results)
5. Add performance comparison tests

#### Deliverables

```rust
// crates/serve/tests/search_api_tests.rs

#[tokio::test]
async fn test_post_search_basic_query() {
    let request = AdvancedSearchRequest {
        query: "getting started".to_string(),
        filters: None,
        options: None,
        aggregations: None,
    };

    let response = post_search(request).await.unwrap();

    assert_eq!(response.query, "getting started");
    assert!(!response.results.is_empty());
}

#[tokio::test]
async fn test_post_search_with_filters() {
    let request = AdvancedSearchRequest {
        query: "configuration".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec!["tutorial".to_string()]),
            similarity: Some(SimilarityRange {
                min: Some(0.8),
                max: None,
            }),
            date_range: None,
            tags: None,
            repositories: None,
        }),
        options: None,
        aggregations: None,
    };

    let response = post_search(request).await.unwrap();

    for result in &response.results {
        assert_eq!(result.category, "tutorial");
        assert!(result.similarity >= 0.8);
    }
}

#[tokio::test]
async fn test_get_post_equivalence() {
    let query = "rust tutorial";

    let get_response = get_search(query, 10, 0.7).await.unwrap();

    let post_request = AdvancedSearchRequest {
        query: query.to_string(),
        filters: Some(SearchFilters {
            similarity: Some(SimilarityRange {
                min: Some(0.7),
                max: None,
            }),
            ..Default::default()
        }),
        options: Some(SearchOptions {
            max_results: Some(10),
            ..Default::default()
        }),
        aggregations: None,
    };
    let post_response = post_search(post_request).await.unwrap();

    assert_eq!(get_response.results.len(), post_response.results.len());
}

#[tokio::test]
async fn test_post_search_validation_errors() {
    let request = AdvancedSearchRequest {
        query: "".to_string(), // Invalid: empty query
        filters: None,
        options: None,
        aggregations: None,
    };

    let result = post_search(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_post_search_with_aggregations() {
    let request = AdvancedSearchRequest {
        query: "documentation".to_string(),
        filters: None,
        options: None,
        aggregations: Some(AggregationRequest {
            by_category: Some(true),
            by_similarity_range: Some(true),
            by_date: None,
        }),
    };

    let response = post_search(request).await.unwrap();

    assert!(response.aggregations.is_some());
    let aggs = response.aggregations.unwrap();
    assert!(aggs.by_category.is_some());
    assert!(aggs.by_similarity_range.is_some());
}
```

#### Acceptance Criteria

- All tests pass
- Test coverage >80%
- Both success and failure cases covered
- Edge cases tested (empty results, max limits, invalid input)
- GET/POST equivalence verified

### Phase 7.6 Success Criteria

- POST `/api/v1/search` endpoint implemented and working
- Request validation works correctly
- Filters, options, and aggregations supported
- GET and POST handlers share search logic
- Both endpoints return consistent response format
- Tests pass with >80% coverage
- Documentation comments complete
- Code formatted, linted, and passes quality gates

## Phase 7.7: OpenAPI Documentation

**Goal**: Add comprehensive OpenAPI (utoipa) documentation for both GET and POST search endpoints.

**Duration**: 1 week

### 7.7.1: Add utoipa Dependencies and Setup

#### Tasks

1. Add `utoipa` and `utoipa-swagger-ui` to dependencies
2. Create OpenAPI schema configuration
3. Set up Swagger UI route at `/api/v1/docs`
4. Configure OpenAPI metadata (title, version, description)

#### Deliverables

```toml
# crates/serve/Cargo.toml

[dependencies]
utoipa = { version = "4.2", features = ["axum_extras", "chrono", "uuid"] }
utoipa-swagger-ui = { version = "6.0", features = ["axum"] }
```

```rust
// crates/serve/src/api/v1/openapi.rs

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use axum::Router;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "XZe API",
        version = "1.0.0",
        description = "AI-powered documentation generator API",
        contact(
            name = "XZe Team",
            url = "https://github.com/xbcsmith/xze"
        ),
        license(
            name = "MIT OR Apache-2.0"
        )
    ),
    paths(
        crate::api::v1::handlers::handle_search,
        crate::api::v1::handlers::handle_search_advanced,
        crate::api::v1::handlers::health,
        crate::api::v1::handlers::version,
        crate::api::v1::handlers::list_repositories,
        crate::api::v1::handlers::get_repository,
        crate::api::v1::handlers::get_documentation,
    ),
    components(
        schemas(
            crate::api::v1::types::search::AdvancedSearchRequest,
            crate::api::v1::types::search::SearchFilters,
            crate::api::v1::types::search::SearchOptions,
            crate::api::v1::types::search::AggregationRequest,
            crate::api::v1::types::search::SearchResponse,
            crate::api::v1::types::search::SearchResult,
            crate::api::v1::types::search::AggregationResponse,
        )
    ),
    tags(
        (name = "search", description = "Search endpoints"),
        (name = "repositories", description = "Repository management"),
        (name = "documentation", description = "Documentation access"),
        (name = "system", description = "System endpoints"),
    )
)]
pub struct ApiDoc;

pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/api/v1/docs")
        .url("/api/v1/openapi.json", ApiDoc::openapi())
}
```

#### Acceptance Criteria

- Dependencies added and compile successfully
- OpenAPI schema generated
- Swagger UI accessible at `/api/v1/docs`
- All endpoints listed in OpenAPI spec

### 7.7.2: Annotate GET Search Endpoint

#### Tasks

1. Add `#[utoipa::path]` annotation to GET handler
2. Document query parameters with descriptions and examples
3. Document response schema
4. Add example requests and responses
5. Document error responses

#### Deliverables

```rust
/// GET /api/v1/search - Simple keyword search
///
/// Performs a semantic search using query parameters. Best for simple searches
/// that need to be cacheable, bookmarkable, or shareable.
#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "search",
    params(
        ("q" = String, Query, description = "Search query text", example = "getting started"),
        ("max_results" = Option<u32>, Query, description = "Maximum number of results (1-100)", example = 10),
        ("min_similarity" = Option<f32>, Query, description = "Minimum similarity threshold (0.0-1.0)", example = 0.7),
        ("category" = Option<String>, Query, description = "Filter by category", example = "tutorial"),
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResponse,
            example = json!({
                "query": "getting started",
                "results": [
                    {
                        "id": "doc-123",
                        "title": "Getting Started with XZe",
                        "content": "This guide will help you...",
                        "category": "tutorial",
                        "similarity": 0.95,
                        "repository": "xze",
                        "path": "docs/tutorials/getting_started.md",
                        "tags": ["beginner", "setup"],
                        "updated_at": "2024-01-15T10:00:00Z"
                    }
                ],
                "total_results": 1
            })
        ),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn handle_search(
    Query(params): Query<SearchParams>,
    State(state): State<AppState>,
) -> Result<Json<SearchResponse>, ApiError> {
    // Implementation
}
```

#### Acceptance Criteria

- GET endpoint fully documented in OpenAPI
- All parameters described with types and examples
- Response schema matches actual response
- Error responses documented
- Example requests and responses provided

### 7.7.3: Annotate POST Search Endpoint

#### Tasks

1. Add `#[utoipa::path]` annotation to POST handler
2. Document request body schema with all nested types
3. Document response schema
4. Add comprehensive examples for different use cases
5. Document all error responses

#### Deliverables

```rust
/// POST /api/v1/search - Advanced search with complex filters
///
/// Performs semantic search with rich filtering, aggregations, and pagination.
/// Best for complex queries that cannot fit in URL query parameters.
#[utoipa::path(
    post,
    path = "/api/v1/search",
    tag = "search",
    request_body(
        content = AdvancedSearchRequest,
        description = "Search request with query, filters, options, and aggregations",
        example = json!({
            "query": "getting started with rust",
            "filters": {
                "categories": ["tutorial", "how-to"],
                "similarity": {
                    "min": 0.8,
                    "max": 1.0
                },
                "tags": ["rust", "beginner"]
            },
            "options": {
                "max_results": 20,
                "offset": 0,
                "include_snippets": true,
                "highlight_terms": true
            },
            "aggregations": {
                "by_category": true,
                "by_similarity_range": true
            }
        })
    ),
    responses(
        (status = 200, description = "Search results with aggregations", body = SearchResponse,
            example = json!({
                "query": "getting started with rust",
                "results": [],
                "total_results": 15,
                "aggregations": {
                    "by_category": [
                        {"category": "tutorial", "count": 10},
                        {"category": "how-to", "count": 5}
                    ],
                    "by_similarity_range": [
                        {"range": "0.8-1.0", "count": 12},
                        {"range": "0.6-0.8", "count": 3}
                    ]
                },
                "pagination": {
                    "offset": 0,
                    "limit": 20,
                    "total": 15,
                    "has_more": false
                }
            })
        ),
        (status = 400, description = "Invalid request body"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn handle_search_advanced(
    State(state): State<AppState>,
    Json(request): Json<AdvancedSearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    // Implementation
}
```

#### Acceptance Criteria

- POST endpoint fully documented in OpenAPI
- Request body schema complete with all nested types
- Multiple example requests showing different use cases
- Response schema includes aggregations and pagination
- All error codes documented

### 7.7.4: Add Schema Annotations to Types

#### Tasks

1. Add `#[derive(ToSchema)]` to all request/response types
2. Add field descriptions and examples
3. Add validation annotations in schema
4. Document enums and nested structures

#### Deliverables

```rust
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[schema(example = json!({
    "query": "rust tutorial",
    "filters": {
        "categories": ["tutorial"],
        "similarity": {"min": 0.8}
    }
}))]
pub struct AdvancedSearchRequest {
    #[schema(description = "Search query text", example = "getting started", min_length = 1, max_length = 1000)]
    #[validate(length(min = 1, max = 1000))]
    pub query: String,

    #[schema(description = "Optional filters to narrow search results")]
    pub filters: Option<SearchFilters>,

    #[schema(description = "Search behavior options")]
    pub options: Option<SearchOptions>,

    #[schema(description = "Request aggregations on search results")]
    pub aggregations: Option<AggregationRequest>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct SearchFilters {
    #[schema(description = "Filter by document categories", example = json!(["tutorial", "how-to"]))]
    pub categories: Option<Vec<String>>,

    #[schema(description = "Filter by similarity score range")]
    pub similarity: Option<SimilarityRange>,

    #[schema(description = "Filter by date range")]
    pub date_range: Option<DateRange>,

    #[schema(description = "Filter by tags", example = json!(["rust", "beginner"]))]
    pub tags: Option<Vec<String>>,

    #[schema(description = "Filter by repository IDs")]
    pub repositories: Option<Vec<String>>,
}
```

#### Acceptance Criteria

- All types have `ToSchema` derive
- All fields have descriptions
- Examples provided for complex types
- Validation constraints visible in schema
- Schema generation works without errors

### Phase 7.7 Success Criteria

- OpenAPI spec generated and accessible at `/api/v1/openapi.json`
- Swagger UI accessible at `/api/v1/docs`
- Both GET and POST endpoints fully documented
- All request/response types have schema annotations
- Examples provided for all endpoints
- Interactive API testing works in Swagger UI
- OpenAPI spec validates against OpenAPI 3.0 standard

## Phase 7.8: Comprehensive API Testing

**Goal**: Create comprehensive test suite for both GET and POST search endpoints, including integration tests, performance tests, and migration tests.

**Duration**: 1 week

### 7.8.1: Unit Tests for Types

#### Tasks

1. Test request type validation (all validation rules)
2. Test serialization/deserialization
3. Test default values and optional fields
4. Test edge cases and boundary conditions

#### Deliverables

```rust
// crates/serve/src/api/v1/types/search.rs (test module)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_search_request_validation_success() {
        let request = AdvancedSearchRequest {
            query: "test".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_validation_empty_query() {
        let request = AdvancedSearchRequest {
            query: "".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_similarity_range_validation() {
        let valid = SimilarityRange {
            min: Some(0.7),
            max: Some(0.9),
        };
        assert!(valid.validate().is_ok());

        let invalid = SimilarityRange {
            min: Some(1.5),
            max: Some(2.0),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_search_options_max_results_range() {
        let valid = SearchOptions {
            max_results: Some(50),
            ..Default::default()
        };
        assert!(valid.validate().is_ok());

        let invalid = SearchOptions {
            max_results: Some(500),
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }
}
```

#### Acceptance Criteria

- All validation rules tested
- Edge cases covered (min/max values, empty strings, null)
- Serialization round-trip tests pass
- Test coverage for types module >90%

### 7.8.2: Handler Unit Tests

#### Tasks

1. Test handler request extraction
2. Test handler validation error responses
3. Test handler error handling
4. Test response formatting
5. Mock external dependencies

#### Deliverables

```rust
// crates/serve/src/api/v1/handlers.rs (test module)

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_handle_search_advanced_success() {
        let app = create_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/search")
            .header("content-type", "application/json")
            .body(Body::from(r#"{
                "query": "test query",
                "options": {"max_results": 5}
            }"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_handle_search_advanced_validation_error() {
        let app = create_test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/search")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"query": ""}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
```

#### Acceptance Criteria

- All handler code paths tested
- Error responses validated
- Mocked dependencies work correctly
- Test coverage for handlers >80%

### 7.8.3: Integration Tests

#### Tasks

1. Test GET search with real database
2. Test POST search with real database
3. Test filter combinations
4. Test aggregations
5. Test pagination
6. Test GET/POST equivalence

#### Deliverables

```rust
// crates/serve/tests/search_integration_tests.rs

use sqlx::PgPool;

async fn setup_test_db() -> PgPool {
    // Setup test database with sample data
}

#[tokio::test]
#[ignore] // Run with: cargo test --ignored -- --test-threads=1
async fn test_get_search_integration() {
    let pool = setup_test_db().await;
    let app = create_app_with_db(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=test&max_results=10")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let search_response: SearchResponse = serde_json::from_slice(&body).unwrap();

    assert!(!search_response.results.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_post_search_with_filters_integration() {
    let pool = setup_test_db().await;
    let app = create_app_with_db(pool);

    let request_body = json!({
        "query": "tutorial",
        "filters": {
            "categories": ["tutorial"],
            "similarity": {"min": 0.8}
        },
        "options": {"max_results": 20}
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_get_post_return_same_results() {
    let pool = setup_test_db().await;
    let app = create_app_with_db(pool.clone());

    // GET request
    let get_response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=rust&max_results=10&min_similarity=0.7")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    let get_body = hyper::body::to_bytes(get_response.into_body()).await.unwrap();
    let get_results: SearchResponse = serde_json::from_slice(&get_body).unwrap();

    // POST request with equivalent parameters
    let post_body = json!({
        "query": "rust",
        "filters": {
            "similarity": {"min": 0.7}
        },
        "options": {"max_results": 10}
    });

    let post_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&post_body).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    let post_body = hyper::body::to_bytes(post_response.into_body()).await.unwrap();
    let post_results: SearchResponse = serde_json::from_slice(&post_body).unwrap();

    assert_eq!(get_results.results.len(), post_results.results.len());
    assert_eq!(get_results.total_results, post_results.total_results);
}
```

#### Acceptance Criteria

- Integration tests pass with real database
- All filter combinations tested
- Aggregations produce correct results
- Pagination works correctly
- GET/POST equivalence verified
- Tests are repeatable and isolated

### 7.8.4: Performance Tests

#### Tasks

1. Benchmark GET vs POST response times
2. Test concurrent request handling
3. Test large result set pagination
4. Test complex filter performance
5. Test aggregation performance

#### Deliverables

```rust
// crates/serve/benches/search_benchmarks.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_get_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = create_test_app();

    c.bench_function("GET simple search", |b| {
        b.to_async(&rt).iter(|| async {
            let response = app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/search?q=test")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap();
            black_box(response)
        })
    });
}

fn benchmark_post_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = create_test_app();

    c.bench_function("POST simple search", |b| {
        b.to_async(&rt).iter(|| async {
            let body = json!({"query": "test"});
            let response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/search")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&body).unwrap()))
                        .unwrap()
                )
                .await
                .unwrap();
            black_box(response)
        })
    });
}

fn benchmark_post_with_filters(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = create_test_app();

    c.bench_function("POST complex search", |b| {
        b.to_async(&rt).iter(|| async {
            let body = json!({
                "query": "test",
                "filters": {
                    "categories": ["tutorial"],
                    "similarity": {"min": 0.8}
                },
                "aggregations": {
                    "by_category": true,
                    "by_similarity_range": true
                }
            });
            let response = app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/search")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&body).unwrap()))
                        .unwrap()
                )
                .await
                .unwrap();
            black_box(response)
        })
    });
}

criterion_group!(
    benches,
    benchmark_get_search,
    benchmark_post_search,
    benchmark_post_with_filters
);
criterion_main!(benches);
```

#### Acceptance Criteria

- Benchmarks run successfully
- Performance metrics collected
- GET and POST performance compared
- Complex queries don't degrade significantly
- Concurrent requests handled efficiently

### Phase 7.8 Success Criteria

- Unit tests pass with >80% coverage
- Integration tests pass with real database
- Performance benchmarks established
- All test documentation complete
- CI/CD pipeline runs all tests
- Test failures are informative and actionable

## Phase 8: Production Readiness

**Goal**: Harden the search API for production use with security, rate limiting, monitoring, and operational tooling.

**Duration**: 2 weeks

### 8.1: Rate Limiting & Security

#### Tasks

1. Implement token bucket rate limiting per client IP
2. Add API key authentication support
3. Implement request size limits
4. Add query complexity scoring and limits
5. Add CORS configuration
6. Implement security headers (CSP, HSTS, etc.)

#### Deliverables

- Rate limiting middleware with Redis backend
- API key authentication middleware
- Request validation and size limits
- Security headers middleware
- CORS configuration
- Rate limit documentation

#### Acceptance Criteria

- Rate limiting works per IP and per API key
- Authenticated requests have higher limits
- Large requests are rejected appropriately
- Security headers present in all responses
- CORS configured for allowed origins

### 8.2: Caching Strategy

#### Tasks

1. Implement HTTP caching for GET requests
2. Add Cache-Control headers based on query
3. Implement Redis cache for POST requests (hash-based)
4. Add cache warming for popular queries
5. Implement cache invalidation on document updates
6. Add cache hit/miss metrics

#### Deliverables

```rust
// HTTP caching for GET
pub async fn handle_search_with_cache(
    Query(params): Query<SearchParams>,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    let cache_key = generate_cache_key(&params);

    if let Some(cached) = state.cache.get(&cache_key).await? {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("X-Cache", "HIT")
            .header("Cache-Control", "public, max-age=300")
            .body(cached)
            .unwrap());
    }

    let results = execute_search(&state, params).await?;
    state.cache.set(&cache_key, &results, 300).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("X-Cache", "MISS")
        .header("Cache-Control", "public, max-age=300")
        .json(results)
        .unwrap())
}
```

#### Acceptance Criteria

- GET requests cached with appropriate TTL
- POST requests cached based on body hash
- Cache hit rate >60% for common queries
- Cache invalidation works on updates
- Cache metrics available

### 8.3: Monitoring & Observability

#### Tasks

1. Add structured logging for all search requests
2. Implement Prometheus metrics
3. Add request tracing with OpenTelemetry
4. Create Grafana dashboards
5. Set up alerts for errors and performance
6. Add health check for search functionality

#### Deliverables

```rust
// Metrics
lazy_static! {
    static ref SEARCH_REQUESTS: Counter = register_counter!(
        "xze_search_requests_total",
        "Total number of search requests"
    ).unwrap();

    static ref SEARCH_DURATION: Histogram = register_histogram!(
        "xze_search_duration_seconds",
        "Search request duration"
    ).unwrap();

    static ref SEARCH_RESULTS: Histogram = register_histogram!(
        "xze_search_results_count",
        "Number of search results returned"
    ).unwrap();
}

pub async fn handle_search_with_metrics(
    params: SearchParams,
    state: AppState,
) -> Result<SearchResponse, ApiError> {
    SEARCH_REQUESTS.inc();
    let timer = SEARCH_DURATION.start_timer();

    let results = execute_search(&state, params).await?;

    SEARCH_RESULTS.observe(results.total_results as f64);
    timer.observe_duration();

    Ok(results)
}
```

#### Acceptance Criteria

- All requests logged with structured format
- Key metrics exported to Prometheus
- Distributed tracing works end-to-end
- Dashboards show search performance
- Alerts fire on error rate or latency spikes

## Phase 9: Performance Optimization

**Goal**: Optimize search performance through caching, query optimization, and efficient pagination.

**Duration**: 2 weeks

### 9.1: Query Embedding Cache

#### Tasks

1. Implement LRU cache for query embeddings
2. Cache embedding generation results
3. Add cache warming for common queries
4. Implement cache metrics

#### Performance Impact

- Embedding generation time: ~100ms per query
- With cache hit: ~1ms
- Expected cache hit rate: 40-50%
- Average latency reduction: 40-50ms

### 9.2: Database Query Optimization

#### Tasks

1. Add database indexes for common filter fields
2. Optimize vector similarity queries
3. Implement query result caching
4. Add query plan analysis
5. Optimize aggregation queries

#### Performance Impact

- Current p95 latency: ~200ms
- Target p95 latency: <100ms
- Expected improvement: 50% reduction

### 9.3: Pagination Implementation

#### Tasks

1. Implement cursor-based pagination for POST endpoint
2. Add pagination metadata to responses
3. Implement efficient offset pagination for GET
4. Add pagination documentation
5. Test pagination edge cases

#### API Changes

```json
{
  "query": "test",
  "results": [...],
  "pagination": {
    "cursor": "eyJpZCI6MTIzLCJzY29yZSI6MC45fQ==",
    "has_more": true,
    "total": 150
  }
}
```

## Phase 10: Advanced Search Features

**Goal**: Implement advanced search capabilities that leverage the POST endpoint.

**Duration**: 2 weeks

### 10.1: Multi-Field Search

#### Tasks

1. Support searching across multiple fields with different weights
2. Implement field boosting
3. Add field-specific filters
4. Test multi-field ranking

#### Example Query

```json
{
  "query": {
    "multi_match": {
      "query": "rust tutorial",
      "fields": {
        "title": 3.0,
        "content": 1.0,
        "tags": 2.0
      }
    }
  }
}
```

### 10.2: Boolean Search Operators

#### Tasks

1. Implement AND/OR/NOT operators
2. Add phrase search support
3. Implement wildcard search
4. Add fuzzy matching

#### Example Query

```json
{
  "query": {
    "bool": {
      "must": [{"match": "rust"}],
      "should": [{"match": "tutorial"}],
      "must_not": [{"match": "advanced"}]
    }
  }
}
```

### 10.3: Saved Searches

#### Tasks

1. Add saved search storage (database table)
2. Implement save/load/delete endpoints
3. Add search history tracking
4. Implement search templates with parameters

#### New Endpoints

```rust
POST /api/v1/searches/saved
GET /api/v1/searches/saved
GET /api/v1/searches/saved/:id
DELETE /api/v1/searches/saved/:id
GET /api/v1/searches/history
```

## Phase 11: Real-Time Search

**Goal**: Add WebSocket support for streaming search results and live updates.

**Duration**: 2 weeks

### 11.1: WebSocket Infrastructure

#### Tasks

1. Add WebSocket support to axum server
2. Implement connection management
3. Add authentication for WebSocket connections
4. Implement heartbeat and reconnection

### 11.2: Streaming Search Results

#### Tasks

1. Implement streaming search that sends results as they're found
2. Add progress updates during long searches
3. Implement client-side result streaming
4. Test with large result sets

### 11.3: Live Document Updates

#### Tasks

1. Implement document change notifications
2. Send search result updates when documents change
3. Add subscription management
4. Test real-time update delivery

## Phase 12: Search Analytics

**Goal**: Add analytics and insights about search usage and quality.

**Duration**: 1 week

### 12.1: Usage Analytics

#### Tasks

1. Track popular search queries
2. Track zero-result searches
3. Track average results per query
4. Implement analytics dashboard endpoints

### 12.2: Search Quality Metrics

#### Tasks

1. Track click-through rates on search results
2. Track search session duration
3. Implement A/B testing framework
4. Add search relevance scoring

## Implementation Timeline

### Sprint 0 (Week 1): Hybrid Search Foundation
- Phase 7.6: POST endpoint implementation
- Types, handlers, routing, basic tests
- Deliverable: Working POST endpoint

### Sprint 1 (Week 2): Documentation
- Phase 7.7: OpenAPI integration
- Swagger UI setup
- Complete API documentation
- Deliverable: Interactive API docs

### Sprint 2 (Week 3): Testing
- Phase 7.8: Comprehensive test suite
- Integration tests, performance tests
- Test automation in CI
- Deliverable: Full test coverage

### Sprint 3 (Weeks 4-5): Production Hardening
- Phase 8.1: Rate limiting and security
- Phase 8.2: Caching strategy
- Phase 8.3: Monitoring and observability
- Deliverable: Production-ready API

### Sprint 4 (Weeks 6-7): Performance
- Phase 9.1: Embedding cache
- Phase 9.2: Query optimization
- Phase 9.3: Pagination
- Deliverable: Optimized search performance

### Sprint 5 (Weeks 8-9): Advanced Features
- Phase 10.1: Multi-field search
- Phase 10.2: Boolean operators
- Phase 10.3: Saved searches
- Deliverable: Advanced search capabilities

### Sprint 6 (Weeks 10-11): Real-Time
- Phase 11.1: WebSocket infrastructure
- Phase 11.2: Streaming results
- Phase 11.3: Live updates
- Deliverable: Real-time search features

### Sprint 7 (Week 12): Analytics
- Phase 12.1: Usage analytics
- Phase 12.2: Quality metrics
- Deliverable: Search insights and analytics

## Success Criteria

### Phase 7.6 (Hybrid Search)
- POST endpoint functional and tested
- GET and POST produce consistent results
- Request validation works correctly
- Shared search logic implemented
- Code quality gates pass

### Phase 7.7 (OpenAPI)
- Swagger UI accessible and functional
- All endpoints documented
- Interactive testing works
- Examples are accurate and helpful

### Phase 7.8 (Testing)
- Test coverage >80%
- All integration tests pass
- Performance benchmarks established
- CI/CD runs all tests automatically

### Phase 8 (Production)
- Rate limiting prevents abuse
- Caching reduces latency by 40%+
- Monitoring dashboards operational
- Security headers configured
- Authentication working

### Phase 9 (Performance)
- p95 latency <100ms
- Embedding cache hit rate >40%
- Efficient pagination implemented
- Database queries optimized

### Phase 10 (Advanced Features)
- Multi-field search working
- Boolean operators supported
- Saved searches functional
- Complex queries perform well

### Phase 11 (Real-Time)
- WebSocket connections stable
- Streaming results work smoothly
- Live updates delivered reliably
- Reconnection handling works

### Phase 12 (Analytics)
- Usage metrics collected
- Quality metrics tracked
- Analytics dashboard functional
- Insights actionable

## Risk Management

### Technical Risks

**Risk**: POST endpoint complexity increases maintenance burden
- Mitigation: Share logic between GET and POST handlers
- Mitigation: Comprehensive test coverage
- Mitigation: Clear documentation of differences

**Risk**: Performance degradation with complex filters
- Mitigation: Database query optimization
- Mitigation: Implement query timeouts
- Mitigation: Add query complexity limits
- Mitigation: Performance testing in CI

**Risk**: Cache invalidation complexity
- Mitigation: Simple TTL-based caching initially
- Mitigation: Document cache behavior clearly
- Mitigation: Add cache metrics and monitoring

**Risk**: WebSocket connection management complexity
- Mitigation: Use battle-tested WebSocket library
- Mitigation: Implement proper error handling
- Mitigation: Test reconnection scenarios thoroughly

### Timeline Risks

**Risk**: Integration tests require database setup
- Mitigation: Use Docker Compose for test database
- Mitigation: Document test database setup clearly
- Mitigation: Consider using testcontainers-rs

**Risk**: OpenAPI integration may require significant refactoring
- Mitigation: Add annotations incrementally
- Mitigation: Start with simple endpoints
- Mitigation: Budget extra time for annotation work

**Risk**: Feature creep in advanced search
- Mitigation: Stick to defined scope for each phase
- Mitigation: Defer nice-to-have features to future phases
- Mitigation: Regular scope reviews

## Documentation Requirements

### API Documentation
- OpenAPI specification (auto-generated)
- Swagger UI with examples
- Migration guide from GET to POST
- Best practices guide (when to use GET vs POST)

### Implementation Documentation
- Architecture decision records for key choices
- Performance optimization guide
- Caching strategy documentation
- Monitoring and alerting runbook

### User Documentation
- Search API tutorial
- Advanced search examples
- Filter reference
- Aggregation guide

### Developer Documentation
- Contributing guide for search features
- Test writing guide
- Debugging guide
- Performance profiling guide

## Validation Checklist

Before moving to next phase, verify:

### Code Quality
- `cargo fmt --all` passes
- `cargo check --all-targets --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- `cargo test --all-features` passes with >80% coverage

### Documentation
- Implementation summary in `docs/explanation/`
- API reference updated
- Examples tested and working
- No emojis in documentation
- Lowercase filenames with underscores

### Testing
- Unit tests for all new code
- Integration tests for API endpoints
- Performance benchmarks run
- Edge cases covered

### Git
- Branch name follows `pr-feat-issue` format
- Commits follow conventional commits format
- Commit messages include JIRA issue
- No merge conflicts

## Conclusion

This unified implementation plan provides a clear roadmap for evolving XZe's search API from the current basic GET endpoint to a comprehensive, production-ready search system with both simple (GET) and advanced (POST) search capabilities.

The hybrid approach balances simplicity for common use cases with power for complex queries, following industry best practices and REST principles. The phased approach ensures each feature is properly implemented, tested, documented, and hardened before moving to the next phase.

Key differentiators:
- Hybrid GET/POST approach serves different use cases optimally
- OpenAPI documentation ensures API is discoverable and testable
- Comprehensive testing at all levels (unit, integration, performance)
- Production hardening with rate limiting, caching, monitoring
- Advanced features build on solid foundation
- Clear migration path with no breaking changes

## Next Steps

1. Review and approve this plan
2. Set up project tracking (JIRA issues for each phase)
3. Begin Phase 7.6 implementation (POST endpoint)
4. Schedule weekly reviews to track progress
5. Adjust timeline based on actual implementation speed

**Estimated Total Duration**: 12 weeks (3 months)

**Team Size**: 2-3 developers

**Key Milestones**:
- Week 3: Basic hybrid API working with tests
- Week 5: Production-ready with monitoring
- Week 7: Optimized performance
- Week 9: Advanced search features
- Week 12: Full feature set with analytics
