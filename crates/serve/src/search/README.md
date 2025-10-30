# Search Module

This module provides the hybrid search API for XZe, supporting both simple GET-based searches and advanced POST-based searches with filtering, pagination, and aggregations.

## Architecture

The search module is organized into three main components:

- **types.rs**: Request/response structures, validation logic, and error types
- **handlers.rs**: HTTP handlers for GET and POST endpoints
- **mod.rs**: Module interface and public exports

## API Endpoints

### GET /api/search

Simple search with query parameters for basic use cases.

**Query Parameters:**
- `q` (required): Search query string
- `limit` (optional): Maximum results (default: 20, max: 100)
- `offset` (optional): Pagination offset (default: 0)
- `category` (optional): Filter by category
- `repository` (optional): Filter by repository

**Example:**
```bash
curl "http://localhost:3000/api/search?q=rust+documentation&limit=10"
```

### POST /api/search/advanced

Advanced search with JSON body for complex queries.

**Request Body:**
```json
{
  "query": "rust async programming",
  "filters": {
    "categories": ["tutorial", "reference"],
    "similarity": {
      "min": 0.7,
      "max": 1.0
    },
    "date_range": {
      "start": "2024-01-01T00:00:00Z"
    },
    "tags": ["rust"],
    "repositories": ["xze"]
  },
  "options": {
    "max_results": 50,
    "offset": 0,
    "include_snippets": true,
    "highlight_terms": true,
    "group_by": "category"
  },
  "aggregations": {
    "by_category": true,
    "by_similarity_range": true,
    "by_date": true
  }
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/search/advanced \
  -H "Content-Type: application/json" \
  -d '{"query": "rust", "options": {"max_results": 20}}'
```

## Response Format

Both endpoints return the same response structure:

```json
{
  "query": "rust",
  "results": [
    {
      "id": "doc-1",
      "title": "Getting Started with Rust",
      "content": "...",
      "snippet": "...content snippet...",
      "category": "tutorial",
      "similarity": 0.85,
      "repository": "xze",
      "path": "docs/tutorial/getting_started.md",
      "tags": ["rust", "tutorial"],
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total_results": 25,
  "aggregations": {
    "by_category": [
      {"category": "tutorial", "count": 10},
      {"category": "reference", "count": 15}
    ],
    "by_similarity_range": [
      {"range": "0.8-1.0", "count": 20},
      {"range": "0.6-0.8", "count": 5}
    ]
  },
  "pagination": {
    "offset": 0,
    "limit": 20,
    "total": 25,
    "has_more": true
  }
}
```

## Error Handling

All validation errors return 400 Bad Request with error details:

```json
{
  "error": "Invalid query: Query cannot be empty",
  "details": null
}
```

Internal errors return 500 Internal Server Error without exposing details.

## Usage in Code

### Using the Router

```rust
use axum::Router;
use xze_serve::search::handlers::search_routes;

let app = Router::new().nest("/api", search_routes());
```

### Creating Search Requests

```rust
use xze_serve::search::{
    AdvancedSearchRequest, 
    SearchFilters, 
    SearchOptions
};

let request = AdvancedSearchRequest {
    query: "rust documentation".to_string(),
    filters: Some(SearchFilters {
        categories: Some(vec!["tutorial".to_string()]),
        ..Default::default()
    }),
    options: Some(SearchOptions {
        max_results: Some(50),
        ..Default::default()
    }),
    aggregations: None,
};

// Validate before use
request.validate()?;
```

## Validation Rules

### Query Validation
- Must not be empty or whitespace-only

### Filter Validation
- Categories must be valid Diataxis types: tutorial, how-to, explanation, reference
- Similarity range must be 0.0 to 1.0, min <= max
- Date ranges must have start before end
- Lists cannot be empty

### Options Validation
- max_results: 1 to 100 (default: 20)
- offset: >= 0 (default: 0)

## Current Implementation

The handlers currently use mock implementations that return sample data. This allows:
- Immediate testing and development
- API contract validation
- Integration with frontend before database is ready

Future phases will replace mock implementations with:
- Database queries via sqlx
- Vector similarity search
- Query result caching
- Real aggregation computation

## Testing

Run all search tests:
```bash
cargo test --all-features -p xze-serve -- search
```

Run integration tests:
```bash
cargo test --all-features -p xze-serve --test search_integration_tests
```

## Documentation

For detailed implementation documentation, see:
- `docs/explanations/phase_7_6_hybrid_search_api_implementation.md`
- `docs/explanations/search_features_unified_implementation_plan.md`

## Next Steps

- Phase 7.7: OpenAPI documentation with utoipa annotations
- Phase 7.8: Comprehensive testing (unit, integration, performance)
- Phase 8: Production readiness (rate limiting, caching, metrics)
- Phase 9: Performance optimization (query cache, database tuning)
- Phase 10: Advanced features (multi-field search, boolean operators)