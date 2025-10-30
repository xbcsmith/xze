# Phase 7.6: Hybrid Search API Implementation

## Overview

This document describes the implementation of Phase 7.6 from the Search Features Unified Implementation Plan: adding POST endpoint support for advanced search queries while maintaining backward compatibility with the existing GET endpoint.

## Status

**PARTIALLY IMPLEMENTED** - Documentation and planning complete. Code implementation in progress.

## Components Delivered

- `docs/explanations/phase_7_6_hybrid_search_implementation.md` (this document)
- Type definitions planned in `crates/serve/src/search_types.rs`
- Handler implementation planned in `crates/serve/src/handlers.rs`
- Route configuration updates needed in `crates/serve/src/api/v1/mod.rs`

## Implementation Details

### Architecture Decision: Hybrid GET/POST Approach

We chose to implement both GET and POST endpoints for search:

**GET /api/v1/search** (existing, maintained for backward compatibility)
- Simple query string parameters
- Cacheable by HTTP intermediaries
- Bookmarkable URLs
- Best for: simple keyword searches, browser-based queries

**POST /api/v1/search** (new, advanced queries)
- JSON request body with nested structures
- Complex filtering (similarity range, date range, categories, tags, repositories)
- Aggregation requests (faceted search)
- Pagination and result formatting options
- Best for: programmatic API clients, advanced search UIs

### Type Definitions

#### Request Types

**AdvancedSearchRequest**
```rust
pub struct AdvancedSearchRequest {
    pub query: String,                            // Required, non-empty
    pub filters: Option<SearchFilters>,           // Optional filtering
    pub options: Option<SearchOptions>,           // Optional pagination/formatting
    pub aggregations: Option<AggregationRequest>, // Optional faceted results
}
```

**SearchFilters**
```rust
pub struct SearchFilters {
    pub categories: Option<Vec<String>>,      // Filter by Diataxis categories
    pub similarity: Option<SimilarityRange>,  // Min/max similarity scores
    pub date_range: Option<DateRange>,        // Document update timestamps
    pub tags: Option<Vec<String>>,            // Document tags
    pub repositories: Option<Vec<String>>,    // Repository names
}
```

**SearchOptions**
```rust
pub struct SearchOptions {
    pub max_results: Option<usize>,      // 1-1000, default 10
    pub offset: Option<usize>,           // Pagination offset, default 0
    pub include_snippets: Option<bool>,  // Include content snippets, default true
    pub highlight_terms: Option<bool>,   // Highlight query terms, default false
    pub group_by: Option<String>,        // Group results by field
}
```

**AggregationRequest**
```rust
pub struct AggregationRequest {
    pub by_category: bool,         // Count results per category
    pub by_similarity_range: bool, // Count results per similarity range
    pub by_date: bool,             // Count results per time period
}
```

**Helper Types**
```rust
pub struct SimilarityRange {
    pub min: f32,  // 0.0-1.0
    pub max: f32,  // 0.0-1.0
}

pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}
```

#### Response Types

**AdvancedSearchResponse**
```rust
pub struct AdvancedSearchResponse {
    pub query: String,                               // Original query
    pub results: Vec<SearchResult>,                  // Matching documents
    pub total_results: usize,                        // Total before pagination
    pub aggregations: Option<AggregationResponse>,   // Faceted counts
    pub pagination: PaginationInfo,                  // Pagination metadata
}
```

**SearchResult**
```rust
pub struct SearchResult {
    pub id: i64,                          // Chunk ID
    pub title: Option<String>,            // Document title
    pub content: String,                  // Full chunk content
    pub snippet: Option<String>,          // Highlighted snippet (if requested)
    pub category: Option<String>,         // Diataxis category
    pub similarity: f32,                  // Similarity score (0.0-1.0)
    pub repository: Option<String>,       // Repository name
    pub path: String,                     // File path
    pub tags: Vec<String>,                // Document tags
    pub updated_at: Option<DateTime<Utc>>, // Last update timestamp
}
```

**AggregationResponse**
```rust
pub struct AggregationResponse {
    pub by_category: Option<Vec<CategoryCount>>,
    pub by_similarity_range: Option<Vec<SimilarityRangeCount>>,
    pub by_date: Option<Vec<DateCount>>,
}

pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

pub struct SimilarityRangeCount {
    pub range: String,  // e.g., "0.8-0.9"
    pub count: usize,
}

pub struct DateCount {
    pub period: String,  // e.g., "2024-01", "2024-Q1"
    pub count: usize,
}
```

**PaginationInfo**
```rust
pub struct PaginationInfo {
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub has_more: bool,
}
```

### Validation Rules

The `AdvancedSearchRequest` must implement validation:

```rust
impl AdvancedSearchRequest {
    pub fn validate(&self) -> Result<(), String> {
        // Query must not be empty after trimming
        if self.query.trim().is_empty() {
            return Err("Query string cannot be empty".to_string());
        }

        // Validate options if present
        if let Some(ref options) = self.options {
            if let Some(max) = options.max_results {
                if max == 0 || max > 1000 {
                    return Err("max_results must be between 1 and 1000".to_string());
                }
            }
        }

        // Validate similarity range if present
        if let Some(ref filters) = self.filters {
            if let Some(ref sim) = filters.similarity {
                if sim.min < 0.0 || sim.min > 1.0 {
                    return Err("similarity.min must be between 0.0 and 1.0".to_string());
                }
                if sim.max < 0.0 || sim.max > 1.0 {
                    return Err("similarity.max must be between 0.0 and 1.0".to_string());
                }
                if sim.min > sim.max {
                    return Err("similarity.min cannot be greater than similarity.max".to_string());
                }
            }
        }

        Ok(())
    }
}
```

### Handler Implementation

**POST Search Handler**

```rust
pub async fn handle_search_advanced(
    State(state): State<AppState>,
    Json(request): Json<AdvancedSearchRequest>,
) -> impl IntoResponse {
    // Validate request
    if let Err(e) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(SearchErrorResponse {
                error: e,
                details: None,
            }),
        )
            .into_response();
    }

    // Execute search using shared logic
    match execute_search(&state, &request).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            tracing::error!("Search error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SearchErrorResponse {
                    error: "Search failed".to_string(),
                    details: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
```

**Shared Search Execution Logic**

To avoid code duplication, create a shared `execute_search` function used by both GET and POST handlers:

```rust
async fn execute_search(
    state: &AppState,
    request: &AdvancedSearchRequest,
) -> Result<AdvancedSearchResponse, SearchError> {
    use xze_core::semantic::search::{search_with_chunks, SearchConfig};

    // Build search config from request
    let options = request.options.as_ref();
    let filters = request.filters.as_ref();

    let config = SearchConfig {
        max_results: options.and_then(|o| o.max_results).unwrap_or(10),
        min_similarity: filters
            .and_then(|f| f.similarity.as_ref())
            .map(|s| s.min)
            .unwrap_or(0.0),
        category_filter: filters
            .and_then(|f| f.categories.as_ref())
            .and_then(|cats| cats.first().cloned()),
    };

    // Execute core search
    let results = search_with_chunks(
        &state.database_pool,
        &request.query,
        &state.ollama_url,
        &config,
    )
    .await?;

    // Apply additional filters
    let filtered_results = apply_advanced_filters(results, request);

    // Build response
    let total = filtered_results.len();
    let offset = options.and_then(|o| o.offset).unwrap_or(0);
    let limit = options.and_then(|o| o.max_results).unwrap_or(10);

    let paginated: Vec<_> = filtered_results
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|r| convert_to_search_result(r, options))
        .collect();

    // Build aggregations if requested
    let aggregations = if let Some(ref agg) = request.aggregations {
        Some(build_aggregations(&paginated, agg))
    } else {
        None
    };

    Ok(AdvancedSearchResponse {
        query: request.query.clone(),
        results: paginated,
        total_results: total,
        aggregations,
        pagination: PaginationInfo {
            offset,
            limit,
            total,
            has_more: offset + limit < total,
        },
    })
}
```

**Helper Functions**

```rust
fn apply_advanced_filters(
    results: Vec<ChunkSearchResult>,
    request: &AdvancedSearchRequest,
) -> Vec<ChunkSearchResult> {
    let mut filtered = results;

    // Apply similarity range filter
    if let Some(ref filters) = request.filters {
        if let Some(ref sim_range) = filters.similarity {
            filtered.retain(|r| r.similarity >= sim_range.min && r.similarity <= sim_range.max);
        }

        // Apply date range filter
        // Note: requires adding updated_at to ChunkSearchResult or fetching from DB

        // Apply tags filter (OR logic)
        // Note: requires adding tags to ChunkSearchResult or fetching from DB

        // Apply repositories filter (OR logic)
        // Note: requires repository info in database schema
    }

    filtered
}

fn convert_to_search_result(
    chunk: ChunkSearchResult,
    options: Option<&SearchOptions>,
) -> SearchResult {
    let include_snippets = options
        .and_then(|o| o.include_snippets)
        .unwrap_or(true);

    let snippet = if include_snippets {
        Some(generate_snippet(&chunk.content, 200))
    } else {
        None
    };

    SearchResult {
        id: chunk.id,
        title: chunk.title,
        content: chunk.content,
        snippet,
        category: chunk.category,
        similarity: chunk.similarity,
        repository: None, // TODO: Add to database schema
        path: chunk.source_file,
        tags: vec![],     // TODO: Add to database schema
        updated_at: None, // TODO: Add to database schema
    }
}

fn generate_snippet(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}

fn build_aggregations(
    results: &[SearchResult],
    request: &AggregationRequest,
) -> AggregationResponse {
    use std::collections::HashMap;

    let mut response = AggregationResponse {
        by_category: None,
        by_similarity_range: None,
        by_date: None,
    };

    if request.by_category {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for result in results {
            if let Some(ref cat) = result.category {
                *counts.entry(cat.clone()).or_insert(0) += 1;
            }
        }
        response.by_category = Some(
            counts
                .into_iter()
                .map(|(category, count)| CategoryCount { category, count })
                .collect(),
        );
    }

    if request.by_similarity_range {
        // Group by 0.1 ranges: 0.0-0.1, 0.1-0.2, ..., 0.9-1.0
        let mut ranges: HashMap<String, usize> = HashMap::new();
        for result in results {
            let bucket = (result.similarity * 10.0).floor() as i32;
            let min = bucket as f32 / 10.0;
            let max = (bucket + 1) as f32 / 10.0;
            let range_key = format!("{:.1}-{:.1}", min, max);
            *ranges.entry(range_key).or_insert(0) += 1;
        }
        response.by_similarity_range = Some(
            ranges
                .into_iter()
                .map(|(range, count)| SimilarityRangeCount { range, count })
                .collect(),
        );
    }

    // TODO: Implement by_date aggregation when updated_at is available

    response
}
```

### Routing Configuration

Update `crates/serve/src/api/v1/mod.rs`:

```rust
pub fn create_v1_routes(state: AppState) -> Router {
    Router::new()
        // ... existing routes ...
        .route("/search", get(crate::handlers::handle_search))
        .route("/search", post(crate::handlers::handle_search_advanced))  // NEW
        // ... rest of routes ...
        .with_state(state)
}
```

### Module Exports

Update `crates/serve/src/lib.rs`:

```rust
pub mod search_types;  // NEW

pub use search_types::{
    AdvancedSearchRequest,
    AdvancedSearchResponse,
    SearchFilters,
    SearchOptions,
    SearchResult,
    // ... other types ...
};
```

## Testing

### Unit Tests for Types

**File: `crates/serve/src/search_types.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_search_request_validation_success() {
        let request = AdvancedSearchRequest {
            query: "test query".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_advanced_search_request_validation_empty_query() {
        let request = AdvancedSearchRequest {
            query: "   ".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };
        assert!(request.validate().is_err());
        assert_eq!(
            request.validate().unwrap_err(),
            "Query string cannot be empty"
        );
    }

    #[test]
    fn test_similarity_range_validation() {
        let valid = SimilarityRange { min: 0.5, max: 0.9 };
        assert!(valid.validate().is_ok());

        let invalid_min = SimilarityRange { min: -0.1, max: 0.9 };
        assert!(invalid_min.validate().is_err());

        let invalid_max = SimilarityRange { min: 0.5, max: 1.1 };
        assert!(invalid_max.validate().is_err());

        let min_greater_than_max = SimilarityRange { min: 0.9, max: 0.5 };
        assert!(min_greater_than_max.validate().is_err());
    }

    #[test]
    fn test_search_options_max_results_range() {
        let valid = SearchOptions {
            max_results: Some(100),
            ..Default::default()
        };
        assert!(valid.validate().is_ok());

        let zero = SearchOptions {
            max_results: Some(0),
            ..Default::default()
        };
        assert!(zero.validate().is_err());

        let too_large = SearchOptions {
            max_results: Some(1001),
            ..Default::default()
        };
        assert!(too_large.validate().is_err());
    }
}
```

### Handler Unit Tests

**File: `crates/serve/src/handlers.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_search_advanced_validation_error() {
        // Mock state setup (requires test helpers)
        let state = create_test_state();

        let request = AdvancedSearchRequest {
            query: "".to_string(),
            filters: None,
            options: None,
            aggregations: None,
        };

        let response = handle_search_advanced(
            State(state),
            Json(request),
        ).await;

        // Assert response is BAD_REQUEST
        // Note: Full assertion requires integration test framework
    }
}
```

### Integration Tests

**File: `crates/serve/tests/search_integration.rs`**

```rust
use xze_serve::search_types::*;

#[tokio::test]
#[ignore] // Requires database setup
async fn test_post_search_basic_query() {
    let client = setup_test_client().await;

    let request = AdvancedSearchRequest {
        query: "authentication".to_string(),
        filters: None,
        options: None,
        aggregations: None,
    };

    let response = client
        .post("/api/v1/search")
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: AdvancedSearchResponse = response.json().await.unwrap();
    assert_eq!(body.query, "authentication");
    assert!(!body.results.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_post_search_with_filters() {
    let client = setup_test_client().await;

    let request = AdvancedSearchRequest {
        query: "configuration".to_string(),
        filters: Some(SearchFilters {
            categories: Some(vec!["how_to".to_string()]),
            similarity: Some(SimilarityRange { min: 0.7, max: 1.0 }),
            date_range: None,
            tags: None,
            repositories: None,
        }),
        options: Some(SearchOptions {
            max_results: Some(5),
            offset: Some(0),
            include_snippets: Some(true),
            highlight_terms: Some(false),
            group_by: None,
        }),
        aggregations: None,
    };

    let response = client
        .post("/api/v1/search")
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: AdvancedSearchResponse = response.json().await.unwrap();
    assert!(body.results.len() <= 5);
    for result in &body.results {
        assert!(result.similarity >= 0.7);
        assert_eq!(result.category, Some("how_to".to_string()));
    }
}

#[tokio::test]
#[ignore]
async fn test_get_post_equivalence() {
    let client = setup_test_client().await;

    // GET request
    let get_response = client
        .get("/api/v1/search?q=testing&max_results=10&min_similarity=0.5")
        .send()
        .await
        .unwrap();

    let get_body: SearchResponse = get_response.json().await.unwrap();

    // Equivalent POST request
    let post_request = AdvancedSearchRequest {
        query: "testing".to_string(),
        filters: Some(SearchFilters {
            similarity: Some(SimilarityRange { min: 0.5, max: 1.0 }),
            ..Default::default()
        }),
        options: Some(SearchOptions {
            max_results: Some(10),
            ..Default::default()
        }),
        aggregations: None,
    };

    let post_response = client
        .post("/api/v1/search")
        .json(&post_request)
        .send()
        .await
        .unwrap();

    let post_body: AdvancedSearchResponse = post_response.json().await.unwrap();

    // Results should be equivalent
    assert_eq!(get_body.total_results, post_body.total_results);
    assert_eq!(get_body.results.len(), post_body.results.len());
}
```

## Database Schema Extensions Required

To fully support the advanced search features, the database schema needs extensions:

```sql
-- Add tags support
ALTER TABLE semantic_chunks ADD COLUMN IF NOT EXISTS tags TEXT[];

-- Add repository information
ALTER TABLE semantic_chunks ADD COLUMN IF NOT EXISTS repository VARCHAR(255);

-- Add updated_at timestamp if not present
ALTER TABLE semantic_chunks ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_semantic_chunks_tags ON semantic_chunks USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_semantic_chunks_repository ON semantic_chunks(repository);
CREATE INDEX IF NOT EXISTS idx_semantic_chunks_updated_at ON semantic_chunks(updated_at DESC);
```

## Known Limitations and Future Work

### Current Limitations

1. **Tags and Repository Filtering**: Not fully implemented pending database schema updates
2. **Date Range Filtering**: Requires `updated_at` field in database
3. **Snippet Highlighting**: Basic truncation only; no term highlighting yet
4. **Group By**: Not implemented; requires result transformation logic
5. **Aggregations**: Basic implementation; date aggregation pending

### Future Enhancements

1. **Advanced Highlighting**: Use query terms to highlight matches in snippets
2. **Multi-Field Search**: Search across title, content, and tags with different weights
3. **Boolean Operators**: Support AND, OR, NOT in query syntax
4. **Fuzzy Matching**: Tolerate typos and spelling variations
5. **Query Suggestions**: Suggest alternative queries or autocomplete
6. **Result Caching**: Cache POST results (requires cache key generation from request)
7. **Streaming Results**: Support Server-Sent Events or WebSockets for real-time results

## Validation Checklist

### Code Quality

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes with >80% coverage
- [ ] No `unwrap()` or `expect()` without justification
- [ ] All public items have doc comments with examples

### Documentation

- [x] Implementation documentation created in `docs/explanations/`
- [x] Filename uses lowercase_with_underscores.md
- [x] No emojis in documentation
- [x] All code blocks specify language
- [x] Documentation includes overview, details, testing, and examples

### Files and Structure

- [x] Documentation uses `.md` extension
- [ ] Type definitions file created: `crates/serve/src/search_types.rs`
- [ ] Handler functions added to `crates/serve/src/handlers.rs`
- [ ] Routing updated in `crates/serve/src/api/v1/mod.rs`
- [ ] Module exports in `crates/serve/src/lib.rs`

## Next Steps

1. **Implement search_types.rs**:
   - Create the file with all request/response types
   - Add validation methods
   - Add unit tests (target >90% coverage)

2. **Update handlers.rs**:
   - Add `handle_search_advanced` handler
   - Add `execute_search` shared logic
   - Add helper functions for filtering and aggregation
   - Add handler unit tests

3. **Update routing**:
   - Register POST `/api/v1/search` route
   - Ensure both GET and POST are accessible

4. **Add integration tests**:
   - Create `tests/search_integration.rs`
   - Add GET/POST equivalence tests
   - Add filter validation tests
   - Add aggregation tests

5. **Database migrations**:
   - Add tags column and index
   - Add repository column and index
   - Add updated_at column and index
   - Update seed data

6. **OpenAPI documentation** (Phase 7.7):
   - Add utoipa annotations to types
   - Add annotations to handlers
   - Register schemas in OpenAPI spec
   - Test Swagger UI

## References

- **Architecture**: `docs/explanations/search_features_unified_implementation_plan.md`
- **Core Search**: `crates/core/src/semantic/search.rs`
- **Existing Handler**: `crates/serve/src/handlers.rs` (GET search implementation)
- **Phase 7.5**: API versioning and `/api/v1` routes foundation

## Conclusion

Phase 7.6 introduces advanced search capabilities via a POST endpoint while maintaining full backward compatibility with the existing GET endpoint. The implementation uses shared logic to avoid duplication and provides a foundation for future enhancements like real-time search, saved searches, and advanced analytics.

The hybrid approach (supporting both GET and POST) balances simplicity for common use cases with power for advanced queries, aligning with REST best practices and user needs.