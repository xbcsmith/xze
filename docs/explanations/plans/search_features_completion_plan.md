# Search Features Completion Plan

## Overview

This plan outlines the remaining work to complete the search features for XZe, building on the foundation established in Phase 6 (CLI search) and Phase 7 (API endpoint). The goal is to create a production-ready, performant, and feature-rich search system.

**Critical First Step**: Before proceeding with production deployment, all API endpoints must be migrated to a versioned namespace (`/api/v1/`). Phase 7.5 establishes this foundational infrastructure and must be completed before any other phases begin.

**Total Timeline**: 11-13 weeks, starting with 1-week API versioning foundation (Sprint 0), followed by production readiness and feature enhancements.

## Current State

### Completed (Phase 6 & 7)

- Core semantic search implementation (`search_with_chunks`)
- CLI search command with full functionality
- REST API endpoint (`GET /search`)
- Request/response types and error handling
- Basic documentation and examples
- Unit tests for core components

### Gaps Identified

- No API versioning strategy (all endpoints at root level)
- No integration tests with real database
- No performance optimizations (caching, indexing)
- No pagination for large result sets
- No rate limiting for API
- No metrics/monitoring
- No advanced search features (filters, aggregations)
- No WebSocket support for real-time updates

## Implementation Phases

---

## Phase 7.5: API Versioning & Migration

**Goal**: Establish API versioning infrastructure and migrate all endpoints to `/api/v1/` prefix.

**Estimated Effort**: 1 week

**Priority**: Critical (Foundational)

### 7.5.1: API Version Infrastructure

**Priority**: Critical
**Effort**: 2-3 days

#### Tasks

1. **Create Versioned Router Structure**
   - [ ] Create `crates/serve/src/api/v1/mod.rs` module
   - [ ] Move existing handlers to `crates/serve/src/api/v1/handlers.rs`
   - [ ] Create `create_v1_routes()` function
   - [ ] Update `create_routes()` to nest v1 router under `/api/v1`
   - [ ] Maintain backward compatibility with legacy routes (temporary)

2. **Endpoint Migration**
   - [ ] Migrate `/search` to `/api/v1/search`
   - [ ] Migrate `/health` to `/api/v1/health`
   - [ ] Migrate `/version` to `/api/v1/version`
   - [ ] Migrate `/analyze` to `/api/v1/analyze`
   - [ ] Migrate `/repositories` to `/api/v1/repositories`
   - [ ] Migrate `/repositories/:id` to `/api/v1/repositories/:id`
   - [ ] Migrate `/repositories/:id/analyze` to `/api/v1/repositories/:id/analyze`
   - [ ] Migrate `/documentation` to `/api/v1/documentation`
   - [ ] Migrate `/documentation/:id` to `/api/v1/documentation/:id`

3. **Version Header Support**
   - [ ] Add middleware to extract `Accept-Version` header
   - [ ] Add middleware to set `API-Version` response header
   - [ ] Add version negotiation logic
   - [ ] Add version mismatch error responses

4. **Legacy Route Deprecation**
   - [ ] Add deprecation warnings to legacy endpoints
   - [ ] Add `Deprecation` and `Sunset` HTTP headers
   - [ ] Log usage of deprecated endpoints
   - [ ] Create migration timeline (30-60 days)

#### Deliverables

- `crates/serve/src/api/v1/mod.rs` (150+ lines)
- `crates/serve/src/api/v1/handlers.rs` (400+ lines)
- `crates/serve/src/middleware/version.rs` (100+ lines)
- Updated `crates/serve/src/api.rs` with version routing
- Migration guide for API consumers

#### Acceptance Criteria

- All endpoints accessible via `/api/v1/` prefix
- Legacy endpoints return deprecation headers
- Version negotiation via headers works correctly
- All existing tests updated to use versioned endpoints
- Zero breaking changes for consumers using new paths

#### Example Router Structure

```rust
// crates/serve/src/api.rs
pub fn create_routes() -> Router<AppState> {
    Router::new()
        // API v1 routes (primary)
        .nest("/api/v1", create_v1_routes())
        // Legacy routes (deprecated, to be removed)
        .nest("/", create_legacy_routes_with_deprecation())
        // Root-level health check (special case, not versioned)
        .route("/health", get(health_check))
}

// crates/serve/src/api/v1/mod.rs
pub fn create_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(handlers::handle_search))
        .route("/health", get(handlers::health_check))
        .route("/version", get(handlers::get_version))
        .route("/analyze", post(handlers::analyze_repository))
        .route("/repositories", get(handlers::list_repositories))
        .route("/repositories/:id", get(handlers::get_repository))
        .route("/repositories/:id/analyze", post(handlers::analyze_repository_by_id))
        .route("/documentation", get(handlers::list_documentation))
        .route("/documentation/:id", get(handlers::get_documentation))
        .layer(middleware::from_fn(version_middleware))
}
```

---

### 7.5.2: OpenAPI Documentation

**Priority**: High
**Effort**: 2-3 days

#### Tasks

1. **OpenAPI Specification**
   - [ ] Add `utoipa` dependency for OpenAPI generation
   - [ ] Add `#[utoipa::path]` annotations to all handlers
   - [ ] Add schema definitions for all request/response types
   - [ ] Generate OpenAPI 3.0 specification
   - [ ] Version specification as `1.0.0`

2. **Swagger UI Integration**
   - [ ] Add `utoipa-swagger-ui` dependency
   - [ ] Mount Swagger UI at `/api/v1/docs`
   - [ ] Configure Swagger UI with XZe branding
   - [ ] Add API authentication to Swagger UI (if auth implemented)

3. **API Documentation**
   - [ ] Create `docs/reference/api_v1_specification.md`
   - [ ] Document all endpoints with examples
   - [ ] Document authentication requirements
   - [ ] Document rate limiting behavior
   - [ ] Document error responses
   - [ ] Document versioning strategy

4. **Code Examples**
   - [ ] Create example curl commands for all endpoints
   - [ ] Create example Rust client code
   - [ ] Create example Python client code
   - [ ] Create Postman collection export

#### Deliverables

- `crates/serve/src/openapi.rs` (200+ lines)
- Swagger UI accessible at `/api/v1/docs`
- `docs/reference/api_v1_specification.md` (500+ lines)
- `examples/api_clients/` directory with examples
- `deployments/postman/xze_api_v1.json` collection

#### Acceptance Criteria

- OpenAPI spec validates against OpenAPI 3.0 schema
- Swagger UI loads and displays all endpoints
- All request/response examples are valid
- Documentation covers all endpoints
- Code examples are tested and working

#### Example OpenAPI Annotation

```rust
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchRequest {
    /// The search query text
    #[schema(example = "authentication implementation")]
    pub query: String,

    /// Maximum number of results to return
    #[schema(example = 10, minimum = 1, maximum = 100)]
    pub limit: Option<usize>,

    /// Minimum similarity threshold (0.0 to 1.0)
    #[schema(example = 0.7, minimum = 0.0, maximum = 1.0)]
    pub threshold: Option<f32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    params(SearchRequest),
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResponse),
        (status = 400, description = "Invalid search parameters", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "search"
)]
pub async fn handle_search(
    Query(params): Query<SearchRequest>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Implementation
}
```

---

### 7.5.3: API Versioning Tests

**Priority**: High
**Effort**: 1-2 days

#### Tasks

1. **Version Routing Tests**
   - [ ] Create `tests/api_versioning_tests.rs`
   - [ ] Test all endpoints accessible via `/api/v1/`
   - [ ] Test legacy endpoints return deprecation headers
   - [ ] Test version negotiation via `Accept-Version` header
   - [ ] Test `API-Version` response header presence
   - [ ] Test 404 for invalid versions (e.g., `/api/v2/search`)

2. **Backward Compatibility Tests**
   - [ ] Test legacy routes still functional
   - [ ] Test identical behavior between legacy and v1 routes
   - [ ] Test gradual migration scenarios
   - [ ] Verify no breaking changes in responses

3. **Documentation Tests**
   - [ ] Verify OpenAPI spec generation
   - [ ] Test Swagger UI accessibility
   - [ ] Validate all examples in documentation
   - [ ] Test code examples execute successfully

#### Deliverables

- `tests/api_versioning_tests.rs` (250+ lines)
- `tests/api_backward_compatibility_tests.rs` (150+ lines)
- Updated integration test suite
- CI pipeline includes versioning tests

#### Acceptance Criteria

- All versioning tests pass
- Backward compatibility verified
- Test coverage >90% for version routing
- CI fails if OpenAPI spec invalid
- All documentation examples tested

---

### 7.5.4: Migration Guide & Communication

**Priority**: Medium
**Effort**: 1 day

#### Tasks

1. **Migration Documentation**
   - [ ] Create `docs/how_to/migrate_to_api_v1.md`
   - [ ] Document breaking changes (none expected)
   - [ ] Document deprecation timeline
   - [ ] Provide before/after examples
   - [ ] List all endpoint mappings

2. **Client Impact Assessment**
   - [ ] Identify all known API consumers
   - [ ] Document migration path for each consumer
   - [ ] Create migration checklist
   - [ ] Estimate migration effort

3. **Deprecation Strategy**
   - [ ] Set sunset date for legacy routes (60 days)
   - [ ] Add deprecation notices to responses
   - [ ] Configure logging for legacy usage tracking
   - [ ] Plan communication timeline

#### Deliverables

- `docs/how_to/migrate_to_api_v1.md` (300+ lines)
- `docs/explanations/api_versioning_strategy.md` (200+ lines)
- Migration timeline document
- Communication templates for consumers

#### Acceptance Criteria

- Clear migration path documented
- All consumers notified
- Deprecation timeline published
- Usage tracking in place

---

### Phase 7.5 Success Criteria

- All endpoints accessible via `/api/v1/` prefix
- Legacy endpoints deprecated with clear sunset date
- OpenAPI specification complete and valid
- Swagger UI functional and branded
- Migration guide published
- Zero breaking changes for consumers
- All tests passing with >90% coverage
- Documentation complete and accurate

---

### Phase 7.5 Timeline

**Week 1:**
- Days 1-3: API infrastructure and endpoint migration
- Days 4-5: OpenAPI documentation and Swagger UI
- Days 6-7: Testing and migration guide

---

## Phase 8: Production Readiness

**Goal**: Make search API production-ready with proper testing, monitoring, and security.

**Estimated Effort**: 2-3 weeks

### 8.1: Integration Testing

**Priority**: High
**Effort**: 3-4 days

#### Tasks

1. **Create Test Database Setup**
   - [ ] Add test database configuration to `tests/common/mod.rs`
   - [ ] Create database setup/teardown utilities
   - [ ] Add test data fixtures for search scenarios
   - [ ] Document test database requirements

2. **API Integration Tests**
   - [ ] Create `tests/search_api_tests.rs`
   - [ ] Test basic search flow end-to-end
   - [ ] Test parameter validation
   - [ ] Test error scenarios (empty query, invalid params)
   - [ ] Test category filtering
   - [ ] Test similarity thresholds
   - [ ] Test edge cases (no results, large result sets)

3. **Performance Tests**
   - [ ] Create `benches/search_bench.rs`
   - [ ] Benchmark search with various corpus sizes
   - [ ] Benchmark embedding generation
   - [ ] Benchmark similarity calculations
   - [ ] Document performance baselines

#### Deliverables

- `tests/search_api_tests.rs` (200+ lines)
- `benches/search_bench.rs` (150+ lines)
- `tests/common/test_db.rs` (100+ lines)
- Updated CI configuration to run integration tests

#### Acceptance Criteria

- All integration tests pass with test database
- Performance benchmarks documented
- CI pipeline runs tests automatically
- Test coverage > 80% for search-related code

---

### 8.2: Rate Limiting & Security

**Priority**: High
**Effort**: 2-3 days

#### Tasks

1. **Implement Rate Limiting**
   - [ ] Add `tower-governor` dependency
   - [ ] Create rate limit middleware in `crates/serve/src/middleware.rs`
   - [ ] Configure per-IP rate limits (default: 100 requests/minute)
   - [ ] Add rate limit configuration to `ServerConfig`
   - [ ] Return `429 Too Many Requests` with `Retry-After` header

2. **Input Validation Enhancement**
   - [ ] Add query length limits (max 1000 characters)
   - [ ] Validate category values against enum
   - [ ] Sanitize input for logging (prevent log injection)
   - [ ] Add request size validation

3. **Security Headers**
   - [ ] Add security headers middleware
   - [ ] Set `X-Content-Type-Options: nosniff`
   - [ ] Set `X-Frame-Options: DENY`
   - [ ] Configure CORS properly for production

4. **API Authentication (Optional)**
   - [ ] Design API key authentication scheme
   - [ ] Implement API key validation middleware
   - [ ] Add API key management (generation, revocation)
   - [ ] Document authentication setup

#### Deliverables

- Updated `crates/serve/src/middleware.rs` with rate limiting
- `docs/reference/rate_limiting.md` (100+ lines)
- `docs/how_to/secure_api_deployment.md` (200+ lines)
- Configuration examples for production

#### Acceptance Criteria

- Rate limiting works per-IP with configurable limits
- Proper HTTP headers for security
- Input validation prevents malicious inputs
- Documentation for production deployment

---

### 8.3: Monitoring & Observability

**Priority**: High
**Effort**: 3-4 days

#### Tasks

1. **Metrics Implementation**
   - [ ] Add `prometheus` and `metrics` dependencies
   - [ ] Create metrics middleware in `crates/serve/src/middleware.rs`
   - [ ] Track search request count by category
   - [ ] Track search latency histogram
   - [ ] Track error rate by type
   - [ ] Track result count distribution
   - [ ] Add `/metrics` endpoint for Prometheus

2. **Structured Logging**
   - [ ] Enhance search handler logging
   - [ ] Log query, results count, duration
   - [ ] Add correlation IDs to requests
   - [ ] Configure log levels per environment
   - [ ] Add sampling for high-volume logs

3. **Health Checks**
   - [ ] Enhance `/health` endpoint
   - [ ] Check database connectivity
   - [ ] Check Ollama service availability
   - [ ] Return degraded status when services are down
   - [ ] Add `/ready` endpoint for Kubernetes

4. **Dashboards**
   - [ ] Create Grafana dashboard JSON
   - [ ] Create example Prometheus alerting rules
   - [ ] Document monitoring setup

#### Deliverables

- `crates/serve/src/metrics.rs` (200+ lines)
- Updated `/health` and new `/ready` endpoints
- `deployments/grafana/search-dashboard.json`
- `deployments/prometheus/alerts.yml`
- `docs/how_to/monitor_search_api.md` (300+ lines)

#### Acceptance Criteria

- Prometheus metrics exposed at `/metrics`
- Health checks include dependency status
- Grafana dashboard displays key metrics
- Alerting rules for common issues
- Documentation for monitoring setup

---

## Phase 9: Performance Optimization

**Goal**: Improve search performance and scalability.

**Estimated Effort**: 2-3 weeks

### 9.1: Query Embedding Cache

**Priority**: High
**Effort**: 3-4 days

#### Tasks

1. **Cache Implementation**
   - [ ] Add `moka` or `cached` dependency for in-memory cache
   - [ ] Create `EmbeddingCache` struct in `crates/core/src/semantic/cache.rs`
   - [ ] Implement LRU eviction policy
   - [ ] Configure TTL (default: 1 hour)
   - [ ] Configure max cache size (default: 10,000 entries)

2. **Cache Integration**
   - [ ] Modify `search_with_chunks` to check cache first
   - [ ] Generate cache key from query string
   - [ ] Store embeddings in cache after generation
   - [ ] Add cache hit/miss metrics

3. **Cache Management**
   - [ ] Add cache statistics endpoint (`/api/v1/cache/stats`)
   - [ ] Add cache clear endpoint (`/api/v1/cache/clear`)
   - [ ] Add cache warming on server start (optional)

#### Deliverables

- `crates/core/src/semantic/cache.rs` (250+ lines)
- Cache configuration in `ServerConfig`
- `docs/explanations/embedding_cache.md` (200+ lines)
- Performance comparison benchmarks

#### Acceptance Criteria

- Cache reduces query embedding time by >90% on hits
- Cache hit rate >60% in typical usage
- Configurable cache size and TTL
- Metrics show cache effectiveness

#### Performance Impact

- **Before**: 100-500ms per query for embedding generation
- **After**: <10ms for cached queries (90%+ improvement)
- **Memory**: ~50MB for 10,000 cached embeddings

---

### 9.2: Database Query Optimization

**Priority**: High
**Effort**: 3-4 days

#### Tasks

1. **Index Analysis**
   - [ ] Review current database schema
   - [ ] Analyze query execution plans
   - [ ] Identify missing indexes
   - [ ] Create composite indexes for common queries

2. **Vector Index Optimization**
   - [ ] Add pgvector IVFFlat index
   - [ ] Configure optimal index parameters
   - [ ] Test approximate vs exact search tradeoffs
   - [ ] Document index maintenance

3. **Query Optimization**
   - [ ] Use query builder for dynamic filters
   - [ ] Implement early termination for large result sets
   - [ ] Add EXPLAIN ANALYZE to slow query logs
   - [ ] Optimize category filtering with index

4. **Connection Pool Tuning**
   - [ ] Configure optimal pool size
   - [ ] Add connection pool metrics
   - [ ] Implement graceful degradation on pool exhaustion

#### Deliverables

- `migrations/add_vector_indexes.sql`
- `docs/how_to/optimize_search_database.md` (250+ lines)
- Performance benchmarks before/after
- Database tuning guide

#### Acceptance Criteria

- Search queries use indexes effectively
- Query time reduced by >50% for large corpora
- Database CPU usage reduced
- Documentation for index maintenance

#### Performance Impact

- **Before**: 2-5s for 10,000+ chunks
- **After**: 500ms-1s for 10,000+ chunks (60-80% improvement)
- **Trade-off**: Slight accuracy decrease with approximate search (configurable)

---

### 9.3: Pagination Implementation

**Priority**: Medium
**Effort**: 2-3 days

#### Tasks

1. **Cursor-Based Pagination**
   - [ ] Design cursor encoding scheme
   - [ ] Add `cursor` and `limit` query parameters
   - [ ] Modify search query to support pagination
   - [ ] Return `next_cursor` in response

2. **API Updates**
   - [ ] Update `SearchQueryParams` with pagination fields
   - [ ] Update `SearchResponse` with pagination metadata
   - [ ] Add `has_more` field to indicate more results
   - [ ] Document pagination in API reference

3. **Client Support**
   - [ ] Update example clients (Python, JavaScript)
   - [ ] Add pagination helpers
   - [ ] Document pagination patterns

#### Deliverables

- Updated `SearchQueryParams` and `SearchResponse`
- `docs/reference/search_pagination.md` (150+ lines)
- Updated client examples
- Integration tests for pagination

#### Acceptance Criteria

- Can paginate through large result sets
- Cursor is opaque and tamper-resistant
- Consistent results across pages
- Documentation with examples

#### API Changes

```
GET /search?q=query&limit=20
→ Returns first 20 results + next_cursor

GET /search?q=query&limit=20&cursor=<token>
→ Returns next 20 results + next_cursor
```

---

## Phase 10: Advanced Search Features

**Goal**: Add sophisticated search capabilities.

**Estimated Effort**: 3-4 weeks

### 10.1: Multi-Field Search

**Priority**: Medium
**Effort**: 4-5 days

#### Tasks

1. **Schema Updates**
   - [ ] Add indexed fields: title, category, tags
   - [ ] Create full-text search indexes
   - [ ] Design hybrid search (semantic + keyword)

2. **Search Implementation**
   - [ ] Implement keyword search
   - [ ] Combine semantic and keyword scores
   - [ ] Add boosting by field (title > content)
   - [ ] Add recency boosting (optional)

3. **Query DSL**
   - [ ] Design simple query language
   - [ ] Support field-specific queries (`title:install`)
   - [ ] Support boolean operators (`AND`, `OR`, `NOT`)
   - [ ] Support phrase queries (`"exact phrase"`)

#### Deliverables

- `crates/core/src/semantic/hybrid_search.rs` (300+ lines)
- `docs/reference/advanced_search_syntax.md` (200+ lines)
- Migration for text indexes
- Updated API with search mode parameter

#### Example Queries

```
q=installation title:setup
q="quick start" category:tutorial
q=API -deprecated
```

---

### 10.2: Search Aggregations

**Priority**: Medium
**Effort**: 3-4 days

#### Tasks

1. **Aggregation Implementation**
   - [ ] Count by category
   - [ ] Count by source file/directory
   - [ ] Similarity score distribution
   - [ ] Date histogram (if timestamps available)

2. **API Endpoint**
   - [ ] Add `GET /search/aggregations` endpoint
   - [ ] Support filters before aggregation
   - [ ] Return facet counts
   - [ ] Support multi-level aggregations

3. **UI Integration**
   - [ ] Design faceted search interface
   - [ ] Add filter chips
   - [ ] Show result counts per facet

#### Deliverables

- `SearchAggregations` response type
- `/search/aggregations` endpoint
- `docs/reference/search_aggregations.md` (150+ lines)
- Example faceted search UI (HTML/JS)

#### Example Response

```json
{
  "aggregations": {
    "by_category": {
      "tutorial": 45,
      "how_to": 78,
      "reference": 123,
      "explanation": 34
    },
    "by_similarity": {
      "0.8-1.0": 12,
      "0.6-0.8": 45,
      "0.4-0.6": 67,
      "0.0-0.4": 23
    }
  }
}
```

---

### 10.3: Search Suggestions & Autocomplete

**Priority**: Low
**Effort**: 3-4 days

#### Tasks

1. **Query Analysis**
   - [ ] Extract common queries from logs
   - [ ] Build query frequency index
   - [ ] Create query variations (typos, synonyms)

2. **Suggestion Engine**
   - [ ] Implement prefix-based suggestions
   - [ ] Implement semantic-based suggestions
   - [ ] Rank suggestions by popularity
   - [ ] Return top 10 suggestions

3. **API Endpoint**
   - [ ] Add `GET /search/suggestions` endpoint
   - [ ] Accept partial query parameter
   - [ ] Return ranked suggestions quickly (<50ms)

#### Deliverables

- `GET /search/suggestions` endpoint
- Suggestion cache/index
- `docs/reference/search_suggestions.md` (100+ lines)
- Example autocomplete UI

---

### 10.4: Saved Searches & Search History

**Priority**: Low
**Effort**: 4-5 days

#### Tasks

1. **Database Schema**
   - [ ] Create `saved_searches` table
   - [ ] Create `search_history` table
   - [ ] Add user association (if auth implemented)

2. **API Endpoints**
   - [ ] `POST /search/save` - Save a search
   - [ ] `GET /search/saved` - List saved searches
   - [ ] `DELETE /search/saved/:id` - Delete saved search
   - [ ] `GET /search/history` - Recent searches

3. **Search Alerts (Optional)**
   - [ ] Email when new results match saved search
   - [ ] Webhook notifications
   - [ ] Periodic search execution

#### Deliverables

- Saved search CRUD endpoints
- Migration for new tables
- `docs/reference/saved_searches.md` (150+ lines)

---

## Phase 11: Real-Time Search

**Goal**: Add WebSocket support for real-time search updates.

**Estimated Effort**: 2 weeks

### 11.1: WebSocket Infrastructure

**Priority**: Medium
**Effort**: 4-5 days

#### Tasks

1. **WebSocket Setup**
   - [ ] Add `axum-tungstenite` dependency
   - [ ] Create WebSocket handler
   - [ ] Implement connection management
   - [ ] Add heartbeat/ping-pong

2. **Message Protocol**
   - [ ] Design JSON message format
   - [ ] Define message types (search, result, error)
   - [ ] Add message validation
   - [ ] Document protocol

3. **Connection Management**
   - [ ] Track active connections
   - [ ] Implement connection limits
   - [ ] Add graceful disconnection
   - [ ] Add connection metrics

#### Deliverables

- `crates/serve/src/websocket.rs` (300+ lines)
- WebSocket endpoint at `/ws/search`
- `docs/reference/websocket_protocol.md` (200+ lines)
- Example WebSocket client (JavaScript)

---

### 11.2: Streaming Search Results

**Priority**: Medium
**Effort**: 3-4 days

#### Tasks

1. **Streaming Implementation**
   - [ ] Stream results as they're found
   - [ ] Send partial results incrementally
   - [ ] Add progress updates
   - [ ] Handle cancellation

2. **Result Prioritization**
   - [ ] Send highest similarity results first
   - [ ] Buffer and sort before streaming
   - [ ] Add early termination option

3. **Client-Side Handling**
   - [ ] Example streaming client
   - [ ] Progressive rendering
   - [ ] Handle connection interruption

#### Deliverables

- Streaming search over WebSocket
- Updated WebSocket protocol
- Example streaming UI
- Performance comparison

---

### 11.3: Live Document Updates

**Priority**: Low
**Effort**: 4-5 days

#### Tasks

1. **Change Detection**
   - [ ] Integrate with file watcher
   - [ ] Detect document additions/changes
   - [ ] Detect document deletions

2. **Update Notification**
   - [ ] Notify connected clients of changes
   - [ ] Send updated search results
   - [ ] Handle re-indexing events

3. **Subscription Management**
   - [ ] Subscribe to specific searches
   - [ ] Unsubscribe from searches
   - [ ] Manage subscription limits

#### Deliverables

- Live update notifications
- Subscription management
- Example live dashboard
- `docs/reference/live_updates.md` (150+ lines)

---

## Phase 12: Search Analytics

**Goal**: Provide insights into search usage and effectiveness.

**Estimated Effort**: 1-2 weeks

### 12.1: Usage Analytics

**Priority**: Low
**Effort**: 3-4 days

#### Tasks

1. **Analytics Data Collection**
   - [ ] Log search queries (anonymized)
   - [ ] Track result clicks/selections
   - [ ] Track query refinements
   - [ ] Track no-result queries

2. **Analytics Database**
   - [ ] Create analytics tables
   - [ ] Design data retention policy
   - [ ] Implement data aggregation

3. **Analytics API**
   - [ ] `GET /analytics/popular-queries`
   - [ ] `GET /analytics/zero-result-queries`
   - [ ] `GET /analytics/search-trends`
   - [ ] Add date range filtering

#### Deliverables

- Analytics collection middleware
- Analytics API endpoints
- `docs/reference/search_analytics.md` (150+ lines)
- Privacy considerations documentation

---

### 12.2: Search Quality Metrics

**Priority**: Low
**Effort**: 3-4 days

#### Tasks

1. **Quality Metrics**
   - [ ] Mean Reciprocal Rank (MRR)
   - [ ] Click-through rate
   - [ ] Time to first result
   - [ ] Query abandonment rate

2. **A/B Testing Support**
   - [ ] Implement experiment framework
   - [ ] Track variant performance
   - [ ] Statistical significance testing

3. **Search Relevance Feedback**
   - [ ] Add "Was this helpful?" button
   - [ ] Collect relevance feedback
   - [ ] Use feedback for ranking improvement

#### Deliverables

- Quality metrics dashboard
- A/B testing framework
- Feedback collection API
- `docs/explanations/search_quality.md` (200+ lines)

---

## Implementation Priority Matrix

| Phase | Priority | Effort | Impact | Dependencies |
|-------|----------|--------|--------|--------------|
| 7.5.1 API Version Infrastructure | **Critical** | 2-3 days | Critical | None |
| 7.5.2 OpenAPI Documentation | **Critical** | 2-3 days | High | 7.5.1 |
| 7.5.3 API Versioning Tests | **Critical** | 1-2 days | High | 7.5.1, 7.5.2 |
| 7.5.4 Migration Guide | High | 1 day | Medium | 7.5.1 |
| 8.1 Integration Testing | **High** | 3-4 days | High | 7.5.1 |
| 8.2 Rate Limiting | **High** | 2-3 days | High | 7.5.1 |
| 8.3 Monitoring | **High** | 3-4 days | High | 7.5.1 |
| 9.1 Query Cache | **High** | 3-4 days | High | 7.5.1, 8.1 |
| 9.2 DB Optimization | **High** | 3-4 days | High | 8.1 |
| 9.3 Pagination | Medium | 2-3 days | Medium | 8.1 |
| 10.1 Multi-Field Search | Medium | 4-5 days | High | 9.2 |
| 10.2 Aggregations | Medium | 3-4 days | Medium | 9.2 |
| 10.3 Suggestions | Low | 3-4 days | Medium | 9.1 |
| 10.4 Saved Searches | Low | 4-5 days | Low | 8.2 |
| 11.1 WebSocket Setup | Medium | 4-5 days | High | 8.2 |
| 11.2 Streaming Search | Medium | 3-4 days | Medium | 11.1 |
| 11.3 Live Updates | Low | 4-5 days | Low | 11.1 |
| 12.1 Usage Analytics | Low | 3-4 days | Medium | 8.3 |
| 12.2 Quality Metrics | Low | 3-4 days | Low | 12.1 |

## Recommended Implementation Order

### Sprint 0 (1 week): API Versioning Foundation
1. Phase 7.5.1: API Version Infrastructure
2. Phase 7.5.2: OpenAPI Documentation
3. Phase 7.5.3: API Versioning Tests
4. Phase 7.5.4: Migration Guide & Communication

**Goal**: Establish versioned API foundation before production deployment

**Critical**: This must be completed before Sprint 1. All subsequent work assumes `/api/v1/` prefix.

### Sprint 1 (2 weeks): Production Readiness
1. Phase 8.1: Integration Testing
2. Phase 8.2: Rate Limiting & Security
3. Phase 8.3: Monitoring & Observability

**Goal**: Deploy search API to production safely

### Sprint 2 (2 weeks): Performance
1. Phase 9.1: Query Embedding Cache
2. Phase 9.2: Database Optimization
3. Phase 9.3: Pagination

**Goal**: Handle production load efficiently

### Sprint 3 (2 weeks): Advanced Features (Part 1)
1. Phase 10.1: Multi-Field Search
2. Phase 10.2: Aggregations

**Goal**: Enhance search capabilities

### Sprint 4 (2 weeks): Real-Time Features
1. Phase 11.1: WebSocket Infrastructure
2. Phase 11.2: Streaming Search Results

**Goal**: Enable real-time search experiences

### Sprint 5 (1 week): Advanced Features (Part 2)
1. Phase 10.3: Search Suggestions
2. Phase 10.4: Saved Searches

**Goal**: Improve user experience

### Sprint 6 (1 week): Analytics
1. Phase 12.1: Usage Analytics
2. Phase 12.2: Search Quality Metrics

**Goal**: Measure and improve search effectiveness

## Success Criteria

### Phase 7.5 (API Versioning & Migration)
- [ ] All endpoints accessible via `/api/v1/` prefix
- [ ] OpenAPI specification complete and validated
- [ ] Swagger UI functional at `/api/v1/docs`
- [ ] Legacy endpoints deprecated with sunset headers
- [ ] Migration guide published
- [ ] Zero breaking changes for consumers
- [ ] All tests passing with >90% coverage on version routing
- [ ] Backward compatibility verified

### Phase 8 (Production Readiness)
- [ ] Search API runs in production environment
- [ ] All integration tests passing
- [ ] Monitoring dashboards operational
- [ ] Rate limiting prevents abuse
- [ ] Security audit passed

### Phase 9 (Performance)
- [ ] 90%+ cache hit rate achieved
- [ ] Search latency < 500ms for p95
- [ ] Supports 10,000+ document corpus
- [ ] Database queries optimized with indexes

### Phase 10 (Advanced Features)
- [ ] Multi-field search improves relevance by 20%+
- [ ] Aggregations enable faceted search
- [ ] Suggestions improve query formulation

### Phase 11 (Real-Time)
- [ ] WebSocket connections stable
- [ ] Streaming results improve perceived performance
- [ ] Live updates work reliably

### Phase 12 (Analytics)
- [ ] Track search usage trends
- [ ] Measure search quality metrics
- [ ] Identify areas for improvement

## Risk Management

### Technical Risks

1. **API Version Migration Disruption**
   - **Risk**: Breaking changes during migration to `/api/v1/` cause service disruption
   - **Mitigation**: Maintain backward compatibility with legacy routes, comprehensive testing
   - **Contingency**: Roll back to legacy routes if issues detected, extend deprecation timeline

2. **Client Migration Lag**
   - **Risk**: Clients continue using deprecated endpoints past sunset date
   - **Mitigation**: Clear communication, extended deprecation period (60+ days), usage monitoring
   - **Contingency**: Extend sunset date, provide automated migration tools

3. **Performance Degradation with Scale**
   - **Risk**: Search becomes slow with large corpora
   - **Mitigation**: Implement caching and DB optimization early (Phase 9)
   - **Contingency**: Use approximate vector search, limit max results

4. **Database Connection Pool Exhaustion**
   - **Risk**: High concurrent search load exhausts connections
   - **Mitigation**: Proper pool sizing, connection limits, monitoring
   - **Contingency**: Implement request queuing, return 503 gracefully

5. **Ollama Service Availability**
   - **Risk**: Ollama downtime blocks all searches
   - **Mitigation**: Health checks, graceful degradation, retry logic
   - **Contingency**: Cache embeddings aggressively, use fallback service

### Timeline Risks

1. **Scope Creep**
   - **Risk**: Feature requests extend timeline
   - **Mitigation**: Stick to priority matrix, defer low-priority items
   - **Contingency**: Ship MVP faster, iterate with feedback

2. **Integration Complexity**
   - **Risk**: Integration with existing systems takes longer
   - **Mitigation**: Start with integration tests early (Phase 8.1)
   - **Contingency**: Simplify integrations, use adapters

## Resource Requirements

### Development
- 1 backend engineer (primary) - All phases
- 1 technical writer (Phase 7.5 for API documentation)
- 1 DevOps engineer (Phase 7.5 for CI/CD updates, Phase 8.3, 9.2)
- 0.5 frontend engineer (Phase 10, 11 for examples)

**Phase 7.5 Specific**:
- Backend engineer: 5 days (API infrastructure, versioning, testing)
- Technical writer: 2 days (API documentation, migration guide)
- DevOps engineer: 1 day (CI/CD pipeline updates)

### Infrastructure
- PostgreSQL with pgvector extension
- Ollama service (or OpenAI API)
- Redis (optional, for distributed caching)
- Prometheus + Grafana (monitoring)

### Testing
- Test database instance
- Load testing tools (k6, locust)
- CI/CD pipeline with test environment

## Documentation Requirements

Each phase must deliver:
- [ ] API reference updates
- [ ] How-to guides for new features
- [ ] Explanation documents for complex features
- [ ] Updated examples and tutorials
- [ ] Migration guides (if breaking changes)

**Phase 7.5 Specific Requirements**:
- [ ] `docs/reference/api_v1_specification.md` - Complete OpenAPI documentation
- [ ] `docs/how_to/migrate_to_api_v1.md` - Migration guide for API consumers
- [ ] `docs/explanations/api_versioning_strategy.md` - Versioning approach and rationale
- [ ] OpenAPI 3.0 specification file (generated via utoipa)
- [ ] Swagger UI accessible at `/api/v1/docs`
- [ ] Code examples for all endpoints in multiple languages
- [ ] Postman collection export for testing
- [ ] Endpoint mapping table (legacy to v1 paths)

## Validation & Quality Gates

### Before Moving to Next Phase

1. **Code Review**: All changes reviewed and approved
2. **Tests Passing**: Unit + integration tests at >80% coverage
3. **Documentation Complete**: All required docs written
4. **Performance Validated**: Benchmarks show improvement
5. **Security Review**: No high-severity vulnerabilities
6. **Stakeholder Approval**: Product owner signs off

## Conclusion

This plan provides a comprehensive roadmap to complete the search features for XZe. By following the phased approach and priority matrix, we can deliver a production-ready, performant, and feature-rich search system over the course of 11-13 weeks.

**Critical Foundation**: Phase 7.5 (API Versioning & Migration) is a mandatory first step that establishes the `/api/v1/` namespace for all endpoints. This provides:
- Clean versioning strategy for future API evolution
- Professional API documentation via OpenAPI/Swagger
- Backward compatibility during migration
- Clear deprecation path for legacy endpoints

The focus on API versioning first (Phase 7.5), followed by production readiness (Phase 8), ensures we can ship safely with a solid foundation. Subsequent phases add valuable features based on user feedback and usage patterns.

## Next Steps

1. **Review this plan** with stakeholders
2. **Prioritize Phase 7.5** (API Versioning) as critical foundation
3. **Allocate resources** for Sprint 0 (1 week)
4. **Set up project tracking** (Jira, GitHub Projects, etc.)
5. **Begin Phase 7.5.1** (API Version Infrastructure)

**Critical Path**: Phase 7.5 must be completed before any production deployment. All subsequent phases assume the `/api/v1/` prefix is in place.

**Immediate Actions**:
- [ ] Create `crates/serve/src/api/v1/` module structure
- [ ] Begin endpoint migration planning
- [ ] Set up OpenAPI/utoipa dependencies
- [ ] Draft migration timeline for API consumers
- [ ] Communicate API versioning plan to stakeholders

---

**Document Version**: 1.0
**Last Updated**: December 2024
**Status**: Draft - Pending Review
