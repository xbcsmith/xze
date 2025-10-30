# Phase 8.1: Integration Testing - Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing comprehensive integration tests for the XZe search API. This is the first and highest priority task in completing the search features.

## Goals

- Create end-to-end integration tests for search API
- Set up test database infrastructure
- Achieve >80% test coverage for search code
- Establish testing patterns for future features

## Prerequisites

- PostgreSQL 14+ installed
- Ollama service running with `nomic-embed-text` model
- Rust 1.70+ with cargo
- Basic knowledge of sqlx and tokio testing

## Duration

**Estimated Effort**: 3-4 days

## Step-by-Step Implementation

### Day 1: Test Infrastructure Setup

#### 1.1: Create Test Database Utilities

**File**: `xze/tests/common/test_db.rs`

```rust
//! Test database utilities for integration tests

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

/// Test database configuration
pub struct TestDb {
    pub pool: PgPool,
    pub url: String,
}

impl TestDb {
    /// Create a new test database connection
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/xze_test".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        Ok(Self {
            pool,
            url: database_url,
        })
    }

    /// Run migrations on test database
    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Clean all test data
    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("TRUNCATE TABLE semantic_chunks, documents, repositories CASCADE")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Seed test data
    pub async fn seed_test_data(&self) -> Result<(), sqlx::Error> {
        // Insert test repository
        let repo_id = sqlx::query_scalar!(
            r#"
            INSERT INTO repositories (name, url, language, last_analyzed)
            VALUES ('test-repo', 'https://github.com/test/repo', 'rust', NOW())
            RETURNING id
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        // Insert test document
        let doc_id = sqlx::query_scalar!(
            r#"
            INSERT INTO documents (repository_id, file_path, title, category, content_hash)
            VALUES ($1, 'docs/test.md', 'Test Document', 'tutorial', 'abc123')
            RETURNING id
            "#,
            repo_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Insert test chunks with embeddings
        let test_embeddings = vec![0.1f32; 768]; // 768-dimensional embedding
        let embedding_bytes: Vec<u8> = test_embeddings
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        sqlx::query!(
            r#"
            INSERT INTO semantic_chunks (
                document_id, chunk_index, content, embedding,
                sentence_start, sentence_end, avg_similarity
            )
            VALUES ($1, 0, $2, $3, 0, 5, 0.85),
                   ($1, 1, $4, $3, 5, 10, 0.78),
                   ($1, 2, $5, $3, 10, 15, 0.82)
            "#,
            doc_id,
            "This is a test document about installation. It covers basic setup steps.",
            &embedding_bytes,
            "To install XZe, you need Rust 1.70 or later. Run cargo install xze.",
            "The configuration file uses YAML format. See examples in the docs folder."
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Create a test database for a single test
pub async fn setup_test_db() -> TestDb {
    let db = TestDb::new().await.expect("Failed to connect to test database");
    db.run_migrations().await.expect("Failed to run migrations");
    db.cleanup().await.expect("Failed to cleanup test data");
    db.seed_test_data().await.expect("Failed to seed test data");
    db
}

/// Teardown test database
pub async fn teardown_test_db(db: TestDb) {
    db.cleanup().await.expect("Failed to cleanup");
}
```

**Tasks**:
- [ ] Create `tests/common/test_db.rs`
- [ ] Implement `TestDb` struct with connection pooling
- [ ] Add migration runner
- [ ] Add cleanup utilities
- [ ] Add test data seeding

#### 1.2: Update Test Module Structure

**File**: `xze/tests/common/mod.rs`

```rust
//! Common test utilities

pub mod test_db;

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment once
pub fn init_test_env() {
    INIT.call_once(|| {
        // Initialize logging for tests
        let _ = tracing_subscriber::fmt()
            .with_env_filter("xze=debug,sqlx=warn")
            .with_test_writer()
            .try_init();
    });
}
```

**Tasks**:
- [ ] Create/update `tests/common/mod.rs`
- [ ] Add test environment initialization
- [ ] Configure test logging

#### 1.3: Setup Test Database Schema

**File**: `xze/tests/setup_test_db.sh`

```bash
#!/bin/bash
# Setup test database for integration tests

set -e

DB_NAME="xze_test"
DB_USER="${POSTGRES_USER:-postgres}"
DB_HOST="${POSTGRES_HOST:-localhost}"
DB_PORT="${POSTGRES_PORT:-5432}"

echo "Setting up test database: $DB_NAME"

# Drop existing test database if it exists
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -c "DROP DATABASE IF EXISTS $DB_NAME;" postgres

# Create test database
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -c "CREATE DATABASE $DB_NAME;" postgres

# Enable pgvector extension
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -c "CREATE EXTENSION IF NOT EXISTS vector;" "$DB_NAME"

echo "Test database created successfully"
echo "Connection string: postgresql://$DB_USER@$DB_HOST:$DB_PORT/$DB_NAME"
```

**Tasks**:
- [ ] Create test database setup script
- [ ] Add to documentation
- [ ] Test on local environment

---

### Day 2: Core Integration Tests

#### 2.1: Basic Search Tests

**File**: `xze/tests/search_api_tests.rs`

```rust
//! Integration tests for search API

mod common;

use axum_test::TestServer;
use common::test_db::{setup_test_db, teardown_test_db};
use xze_serve::{handlers::AppState, ServerConfig};

#[tokio::test]
async fn test_basic_search_success() {
    common::init_test_env();
    let db = setup_test_db().await;

    // Create test server with real database
    let config = ServerConfig {
        database_url: db.url.clone(),
        ollama_url: "http://localhost:11434".to_string(),
        ..Default::default()
    };

    let state = AppState::from_pool(config.clone(), db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    // Perform search
    let response = server
        .get("/search")
        .add_query_param("q", "installation")
        .await;

    // Assert response
    assert_eq!(response.status_code(), 200);

    let json: serde_json::Value = response.json();
    assert_eq!(json["query"], "installation");
    assert!(json["results"].is_array());
    assert!(json["total_results"].as_u64().unwrap() > 0);

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_search_with_max_results() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "test")
        .add_query_param("max_results", "2")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: serde_json::Value = response.json();
    let results = json["results"].as_array().unwrap();
    assert!(results.len() <= 2);

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_search_with_similarity_threshold() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "test")
        .add_query_param("min_similarity", "0.9")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: serde_json::Value = response.json();
    let results = json["results"].as_array().unwrap();
    
    // All results should have similarity >= 0.9
    for result in results {
        let similarity = result["similarity"].as_f64().unwrap();
        assert!(similarity >= 0.9);
    }

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_search_with_category_filter() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "test")
        .add_query_param("category", "tutorial")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: serde_json::Value = response.json();
    let results = json["results"].as_array().unwrap();
    
    // All results should be tutorials
    for result in results {
        assert_eq!(result["category"], "tutorial");
    }

    teardown_test_db(db).await;
}
```

**Tasks**:
- [ ] Create `tests/search_api_tests.rs`
- [ ] Test basic search flow
- [ ] Test max_results parameter
- [ ] Test min_similarity parameter
- [ ] Test category filtering

#### 2.2: Error Handling Tests

Add to `xze/tests/search_api_tests.rs`:

```rust
#[tokio::test]
async fn test_search_empty_query_error() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "")
        .await;

    assert_eq!(response.status_code(), 400);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().contains("empty"));

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_search_invalid_similarity() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "test")
        .add_query_param("min_similarity", "1.5")
        .await;

    // Should either reject or clamp to valid range
    assert!(response.status_code() == 400 || response.status_code() == 200);

    teardown_test_db(db).await;
}

#[tokio::test]
async fn test_search_no_results() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/search")
        .add_query_param("q", "xyzzyquux12345")
        .add_query_param("min_similarity", "0.99")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: serde_json::Value = response.json();
    assert_eq!(json["total_results"], 0);
    assert_eq!(json["results"].as_array().unwrap().len(), 0);

    teardown_test_db(db).await;
}
```

**Tasks**:
- [ ] Test empty query validation
- [ ] Test invalid parameters
- [ ] Test no results scenario
- [ ] Test database connection errors

---

### Day 3: Performance Tests & Benchmarks

#### 3.1: Performance Benchmarks

**File**: `xze/benches/search_bench.rs`

```rust
//! Performance benchmarks for search functionality

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xze_core::semantic::search::{search_with_chunks, SearchConfig};
use sqlx::PgPool;

async fn setup_bench_db() -> PgPool {
    let database_url = std::env::var("BENCH_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/xze_bench".to_string());
    
    PgPool::connect(&database_url).await.unwrap()
}

fn search_small_corpus(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let pool = runtime.block_on(setup_bench_db());

    c.bench_function("search_small_corpus_100_chunks", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = SearchConfig {
                max_results: 10,
                min_similarity: 0.0,
                category_filter: None,
            };

            search_with_chunks(
                &pool,
                black_box("test query"),
                "http://localhost:11434",
                &config,
            )
            .await
            .unwrap()
        });
    });
}

fn search_with_filters(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let pool = runtime.block_on(setup_bench_db());

    c.bench_function("search_with_category_filter", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = SearchConfig {
                max_results: 10,
                min_similarity: 0.5,
                category_filter: Some("tutorial".to_string()),
            };

            search_with_chunks(
                &pool,
                black_box("installation guide"),
                "http://localhost:11434",
                &config,
            )
            .await
            .unwrap()
        });
    });
}

criterion_group!(benches, search_small_corpus, search_with_filters);
criterion_main!(benches);
```

**Tasks**:
- [ ] Create `benches/search_bench.rs`
- [ ] Benchmark basic search
- [ ] Benchmark with various corpus sizes
- [ ] Benchmark with filters
- [ ] Document baseline performance

#### 3.2: Load Tests

**File**: `xze/tests/load_tests.rs`

```rust
//! Load tests for search API

mod common;

use common::test_db::setup_test_db;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore] // Run manually with --ignored
async fn test_concurrent_searches() {
    common::init_test_env();
    let db = setup_test_db().await;

    let config = xze_serve::ServerConfig {
        database_url: db.url.clone(),
        ..Default::default()
    };

    let state = xze_serve::handlers::AppState::from_pool(config, db.pool.clone());
    let app = xze_serve::api::create_routes().with_state(state);
    let server = axum_test::TestServer::new(app).unwrap();

    // Spawn 100 concurrent search requests
    let mut handles = vec![];
    
    for i in 0..100 {
        let server = server.clone();
        let handle = tokio::spawn(async move {
            let response = server
                .get("/search")
                .add_query_param("q", &format!("test query {}", i))
                .await;
            
            assert_eq!(response.status_code(), 200);
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }

    println!("Successfully handled 100 concurrent searches");
}
```

**Tasks**:
- [ ] Create load tests
- [ ] Test concurrent requests
- [ ] Test connection pool behavior
- [ ] Document load test results

---

### Day 4: CI/CD Integration & Documentation

#### 4.1: GitHub Actions Workflow

**File**: `.github/workflows/integration-tests.yml`

```yaml
name: Integration Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  integration-tests:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: pgvector/pgvector:pg14
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: xze_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
      
      ollama:
        image: ollama/ollama:latest
        ports:
          - 11434:11434

    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        profile: minimal
        toolchain: stable
        override: true
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Setup test database
      env:
        TEST_DATABASE_URL: postgresql://postgres:postgres@localhost/xze_test
      run: |
        ./tests/setup_test_db.sh
    
    - name: Pull Ollama model
      run: |
        docker exec ${{ job.services.ollama.id }} ollama pull nomic-embed-text
    
    - name: Run integration tests
      env:
        TEST_DATABASE_URL: postgresql://postgres:postgres@localhost/xze_test
      run: |
        cargo test --test search_api_tests -- --nocapture
    
    - name: Run benchmarks
      run: |
        cargo bench --bench search_bench -- --save-baseline main
```

**Tasks**:
- [ ] Create GitHub Actions workflow
- [ ] Configure test database service
- [ ] Configure Ollama service
- [ ] Set up test data
- [ ] Run tests in CI

#### 4.2: Documentation

**File**: `xze/docs/how_to/run_integration_tests.md`

```markdown
# How to Run Integration Tests

## Prerequisites

1. PostgreSQL 14+ with pgvector extension
2. Ollama service with nomic-embed-text model
3. Rust 1.70+

## Local Setup

### 1. Setup Test Database

```bash
# Create test database
./tests/setup_test_db.sh

# Or manually:
createdb xze_test
psql xze_test -c "CREATE EXTENSION vector;"
```

### 2. Configure Environment

```bash
export TEST_DATABASE_URL="postgresql://postgres@localhost/xze_test"
export OLLAMA_URL="http://localhost:11434"
```

### 3. Run Tests

```bash
# Run all integration tests
cargo test --test search_api_tests

# Run specific test
cargo test --test search_api_tests test_basic_search_success

# Run with output
cargo test --test search_api_tests -- --nocapture

# Run load tests
cargo test --test load_tests --ignored -- --nocapture
```

## CI/CD

Integration tests run automatically on:
- Push to main/develop branches
- Pull requests

See `.github/workflows/integration-tests.yml`

## Troubleshooting

### Test Database Connection Fails

```bash
# Check PostgreSQL is running
pg_isready

# Check connection string
psql $TEST_DATABASE_URL -c "SELECT 1;"
```

### Ollama Service Unavailable

```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Pull model if missing
ollama pull nomic-embed-text
```

### Tests Timeout

- Increase timeout in test code
- Check database has test data
- Check Ollama model is loaded
```

**Tasks**:
- [ ] Document test setup
- [ ] Document how to run tests
- [ ] Document troubleshooting
- [ ] Add to main README

---

## Validation Checklist

### Infrastructure
- [ ] Test database setup script works
- [ ] Test data seeding is reliable
- [ ] Cleanup between tests works
- [ ] Test utilities are reusable

### Test Coverage
- [ ] Basic search functionality tested
- [ ] All query parameters tested
- [ ] Category filtering tested
- [ ] Error scenarios tested
- [ ] Edge cases covered
- [ ] Coverage >80% for search code

### Performance
- [ ] Benchmarks run successfully
- [ ] Baseline performance documented
- [ ] Load tests pass
- [ ] No memory leaks detected

### CI/CD
- [ ] Tests run in GitHub Actions
- [ ] Test database provisioned correctly
- [ ] Ollama service configured
- [ ] Tests pass consistently

### Documentation
- [ ] Test setup documented
- [ ] How to run tests documented
- [ ] Troubleshooting guide complete
- [ ] Examples provided

## Success Metrics

- ✅ All integration tests passing
- ✅ Test coverage >80%
- ✅ CI pipeline green
- ✅ Performance baselines established
- ✅ Documentation complete

## Next Steps

After completing Phase 8.1:
1. Review test results with team
2. Address any failing tests
3. Move to Phase 8.2: Rate Limiting & Security
4. Continue with Phase 8.3: Monitoring

## References

- [Testing in Rust](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [sqlx Testing](https://github.com/launchbadge/sqlx/tree/main/examples)
- [axum Testing](https://docs.rs/axum-test/latest/axum_test/)
- [Phase 8 Overview](./search_features_completion_plan.md#phase-8-production-readiness)
```

---

## Common Pitfalls & Solutions

### Issue: Test Database Connection Pool Exhaustion
**Solution**: Use smaller pool size in tests (3-5 connections), close connections properly

### Issue: Tests Flaky Due to Timing
**Solution**: Use proper async/await, add retry logic, increase timeouts

### Issue: Ollama Service Slow/Unavailable in CI
**Solution**: Cache Ollama model, use mock embeddings for unit tests, separate integration/unit tests

### Issue: Test Data Cleanup Incomplete
**Solution**: Use transactions that rollback, truncate tables in teardown, use unique test namespaces

### Issue: Tests Interfere With Each Other
**Solution**: Use separate test databases, proper cleanup, avoid shared mutable state

## Timeline

- **Day 1 Morning**: Test database setup
- **Day 1 Afternoon**: Test utilities and common code
- **Day 2 Morning**: Basic search tests
- **Day 2 Afternoon**: Error handling tests
- **Day 3 Morning**: Performance benchmarks
- **Day 3 Afternoon**: Load tests
- **Day 4 Morning**: CI/CD integration
- **Day 4 Afternoon**: Documentation and review

## Resources

- Test database: 1 PostgreSQL instance (can be shared)
- Ollama: Local instance or shared service
- CI runners: GitHub Actions free tier sufficient

## Review & Sign-off

Before moving to Phase 8.2:

- [ ] Code review by senior engineer
- [ ] All tests passing locally and in CI
- [ ] Documentation reviewed
- [ ] Team demo of test coverage
- [ ] Product owner approval

---

**Version**: 1.0  
**Author**: XZe Development Team  
**Last Updated**: December 2024