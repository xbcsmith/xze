# Search API Design Decision: GET vs POST

## Question

Should the XZe search API use GET with query parameters or POST with JSON payload?

## Decision

**Implement a hybrid approach supporting both GET and POST** to serve different use cases optimally.

## Context

The XZe search API currently uses GET with query parameters:

```http
GET /api/v1/search?q=getting+started&max_results=10&min_similarity=0.8
```

As we expand search capabilities in Phases 10-12 (multi-field search, aggregations, complex filters), the limitations of query strings become problematic.

## Analysis

### Option 1: GET with Query Parameters (Current)

**Pros:**
- HTTP cacheable (CDN, browser, proxy)
- Bookmarkable URLs (share search results)
- Browser-friendly (paste URL, works immediately)
- RESTful (GET for idempotent reads)
- Simple debugging (parameters visible)
- Follows Google/search engine pattern

**Cons:**
- URL length limits (2048 chars practical limit)
- Character encoding complexity
- Cannot express complex nested structures
- No native type safety (everything is strings)
- Parameters visible in logs and browser history

**Best for:**
- Simple keyword searches
- Public search interfaces
- Shareable search results
- Cacheable queries

### Option 2: POST with JSON (Alternative)

**Pros:**
- No size limitations
- Rich nested data structures
- Native JSON type safety
- Complex filter expressions
- Request body not logged
- Better for advanced features

**Cons:**
- Not HTTP cacheable
- Not bookmarkable
- Requires request body construction
- Not RESTful (POST implies mutation)
- More complex client implementation
- Cannot share via URL

**Best for:**
- Advanced search queries
- Complex filter combinations
- Large query expressions
- Programmatic API usage

## Recommended Solution: Hybrid Approach

Implement **both** endpoints to serve different use cases:

### 1. Simple Search: GET /api/v1/search (Current)

For basic keyword searches that fit in URLs:

```http
GET /api/v1/search?q=getting+started&max_results=10&category=tutorial
```

**Use cases:**
- Quick keyword lookups
- Shareable search links
- Browser-based searches
- Documentation "Try it" examples
- Cached results

**Query parameters:**
- `q` (required) - Search query text
- `max_results` (optional) - Result limit (default: 10)
- `min_similarity` (optional) - Similarity threshold (default: 0.7)
- `category` (optional) - Filter by category

### 2. Advanced Search: POST /api/v1/search (New)

For complex searches requiring rich query structure:

```http
POST /api/v1/search
Content-Type: application/json

{
  "query": "getting started with rust",
  "filters": {
    "categories": ["tutorial", "how-to"],
    "similarity": {
      "min": 0.8,
      "max": 1.0
    },
    "date_range": {
      "start": "2024-01-01",
      "end": "2024-12-31"
    },
    "tags": ["rust", "beginner"],
    "repositories": ["repo-123", "repo-456"]
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
    "by_date": "month"
  }
}
```

**Use cases:**
- Multi-field searches (Phase 10.1)
- Complex filter combinations
- Search with aggregations (Phase 10.2)
- Saved searches (Phase 10.4)
- Programmatic API clients

**Request body structure:**
- `query` (required) - Search query text
- `filters` (optional) - Complex filter object
- `options` (optional) - Search options
- `aggregations` (optional) - Aggregation requests

## Implementation Plan

### Phase 7.5.2: Keep GET (Current)

Current GET endpoint remains as-is for simple searches:
- Supports basic keyword search
- Maintains backward compatibility
- Good for most use cases

### Phase 10: Add POST Endpoint

When implementing advanced search features:

1. **Add POST endpoint** alongside GET:
   ```rust
   // GET - simple search (existing)
   .route("/search", get(handle_search))

   // POST - advanced search (new)
   .route("/search", post(handle_search_advanced))
   ```

2. **Define request types**:
   ```rust
   #[derive(Deserialize)]
   pub struct AdvancedSearchRequest {
       pub query: String,
       pub filters: Option<SearchFilters>,
       pub options: Option<SearchOptions>,
       pub aggregations: Option<AggregationRequest>,
   }

   #[derive(Deserialize)]
   pub struct SearchFilters {
       pub categories: Option<Vec<String>>,
       pub similarity: Option<SimilarityRange>,
       pub date_range: Option<DateRange>,
       pub tags: Option<Vec<String>>,
       pub repositories: Option<Vec<String>>,
   }
   ```

3. **Unified response format**:
   Both GET and POST return the same response structure:
   ```json
   {
     "query": "...",
     "results": [...],
     "total_results": 42,
     "aggregations": {...}
   }
   ```

### Phase 10.4: Enable Complex Features

Use POST endpoint for:
- **Saved searches** - Store complex JSON queries
- **Search history** - Complex queries need POST
- **Search templates** - Parameterized JSON queries
- **Boolean operators** - Complex AND/OR/NOT logic

## Industry Examples

### GET Approach
- Google: `google.com/search?q=keyword`
- GitHub: `github.com/search?q=repo+language:rust`
- Stack Overflow: `stackoverflow.com/search?q=rust`

### POST Approach
- Elasticsearch: POST `/_search` with JSON DSL
- Algolia: POST `/indexes/*/queries` with JSON
- Azure Search: POST `/indexes/*/docs/search` with JSON

### Hybrid Approach
- Elasticsearch: Supports both GET with `source` param and POST
- AWS CloudSearch: GET for simple, POST for structured queries
- Many modern APIs: GET for simple, POST for complex

## Migration Path

1. **Now (Phase 7.5)**: GET endpoint works, well-documented
2. **Phase 10**: Add POST endpoint for advanced features
3. **Documentation**: Clearly explain when to use each
4. **Client libraries**: Support both methods appropriately

## Security Considerations

### GET Considerations
- Query params logged in web server logs
- Visible in browser history
- Cached by intermediaries
- Solution: Don't put sensitive data in query params

### POST Considerations
- Request body not logged by default
- Not visible in browser history
- Not cached automatically
- Better for sensitive searches

For XZe (documentation search), this is less critical since searches are not typically sensitive.

## Caching Strategy

### GET Endpoint
- Enable HTTP caching headers
- CDN/proxy caching possible
- Browser caching works automatically
- Cache-Control: `public, max-age=300`

### POST Endpoint
- No automatic caching
- Implement application-level cache if needed
- Cache based on request body hash
- Use Redis/Memcached for query result cache

## Performance Implications

### GET Performance
- Better: HTTP cache hits avoid backend
- Worse: No caching for unique queries
- Network: Smaller requests (URL only)

### POST Performance
- Better: Can implement smart backend caching
- Worse: Every request hits backend
- Network: Larger requests (JSON body)

For XZe search:
- GET: ~90% of searches (simple keywords)
- POST: ~10% of searches (advanced queries)

## Testing Requirements

Both endpoints must be tested:

```rust
#[tokio::test]
async fn test_simple_search_via_get() {
    // Test GET /api/v1/search?q=test
}

#[tokio::test]
async fn test_advanced_search_via_post() {
    // Test POST /api/v1/search with JSON body
}

#[tokio::test]
async fn test_get_and_post_produce_same_results() {
    // Simple query via GET should match POST equivalent
}
```

## Documentation Updates

### API Reference
Document both methods:
- Simple searches: Use GET
- Advanced searches: Use POST
- Provide examples for both

### Migration Guide
When POST is added:
- Existing GET queries continue working
- No breaking changes
- New POST endpoint is additive

## Recommendation Summary

**Implement hybrid approach:**

1. **Keep GET** (Phase 7.5-9):
   - Simple keyword search
   - Basic filters (category, max_results, min_similarity)
   - Cacheable, bookmarkable, shareable

2. **Add POST** (Phase 10+):
   - Complex multi-field queries
   - Nested filter structures
   - Aggregations and grouping
   - Saved searches and templates

3. **Benefits:**
   - Best of both worlds
   - No breaking changes
   - Use right tool for each job
   - Industry standard approach

4. **Complexity:**
   - Minimal - two handlers, shared logic
   - Well-supported pattern
   - Clear separation of concerns

## Decision

**Approved**: Implement hybrid GET/POST approach starting in Phase 10.

**Rationale**:
- GET serves 90% of use cases optimally (simple searches)
- POST enables 10% advanced features without compromise
- No breaking changes to existing API
- Follows industry best practices
- Supports future feature expansion

## References

- RFC 7231: HTTP GET method semantics
- Elasticsearch Search API design
- REST API best practices (Microsoft Azure API Guidelines)
- Google JSON Style Guide
- Current implementation: `crates/serve/src/handlers.rs`
- Plan: `docs/explanations/plans/search_features_completion_plan.md`
