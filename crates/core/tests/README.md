# XZe Core Integration Tests

## Overview

This directory contains integration tests for XZe core functionality. Integration tests validate complete workflows and interactions between components, requiring external dependencies like databases.

## Test Files

### Git Integration Tests

- `git_integration_tests.rs` - Git repository operations
- `git_credentials_tests.rs` - Git authentication and credentials
- `pr_management_tests.rs` - Pull request management workflows

### Knowledge Base Integration Tests

- `kb_incremental_loading_tests.rs` - Incremental loading workflows

## Running Tests

### Unit Tests

Unit tests are embedded in source files and run without external dependencies:

```bash
# Run all unit tests for KB module
cargo test -p xze-core --lib kb

# Run all unit tests for core
cargo test -p xze-core --lib
```

### Integration Tests

Integration tests are marked with `#[ignore]` when they require external dependencies. This allows unit tests to run in CI/CD environments without database or Git infrastructure.

#### Knowledge Base Integration Tests

**Requirements**:
- PostgreSQL database
- Database schema with `documents` and `document_chunks` tables

**Setup**:

```bash
# Create test database
createdb xze_test

# Set database URL (optional, uses default if not set)
export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost:5432/xze_test"

# Run migrations (if needed)
sqlx migrate run --database-url $TEST_DATABASE_URL
```

**Running**:

```bash
# Run all KB integration tests
cargo test --test kb_incremental_loading_tests -- --ignored

# Run specific test
cargo test --test kb_incremental_loading_tests test_resume_after_full_load -- --ignored

# Run with output visible
cargo test --test kb_incremental_loading_tests -- --ignored --nocapture

# Run tests serially (prevents interference)
cargo test --test kb_incremental_loading_tests -- --ignored --test-threads=1
```

**Tests Included**:
- `test_resume_after_full_load` - Verify resume functionality
- `test_update_modified_files` - Verify update detection and processing
- `test_cleanup_deleted_files` - Verify cleanup of deleted files
- `test_dry_run_mode` - Verify dry-run preview mode
- `test_force_full_reload` - Verify force reload functionality
- `test_mixed_scenario_add_update_delete` - Verify complex mixed operations

#### Git Integration Tests

**Requirements**:
- Git installed
- File system access for temporary repositories

**Running**:

```bash
# Run Git integration tests
cargo test --test git_integration_tests

# Run PR management tests
cargo test --test pr_management_tests
```

## Test Database Configuration

### Default Connection

If `TEST_DATABASE_URL` is not set, tests use:

```
postgresql://postgres:postgres@localhost:5432/xze_test
```

### Custom Connection

Override with environment variable:

```bash
export TEST_DATABASE_URL="postgresql://user:pass@host:port/dbname"
```

### Database Schema

Tests expect the following tables:

**documents**:
```sql
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    file_path TEXT UNIQUE NOT NULL,
    file_hash TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

**document_chunks**:
```sql
CREATE TABLE document_chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding VECTOR(1536),
    metadata JSONB,
    UNIQUE(document_id, chunk_index)
);
```

## Continuous Integration

### Running in CI/CD

**Without Database** (unit tests only):
```bash
cargo test -p xze-core --lib
```

**With Database** (integration tests):
```bash
# Setup database
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:15

# Wait for database to be ready
until pg_isready -h localhost; do sleep 1; done

# Run migrations
sqlx migrate run

# Run all tests including integration tests
cargo test --test kb_incremental_loading_tests -- --ignored
```

### GitHub Actions Example

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: xze_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test -p xze-core --lib

      - name: Run integration tests
        run: cargo test --test kb_incremental_loading_tests -- --ignored
        env:
          TEST_DATABASE_URL: postgresql://postgres:postgres@localhost:5432/xze_test
```

## Troubleshooting

### Database Connection Errors

**Error**: "connection refused" or "database does not exist"

**Solution**:
1. Verify PostgreSQL is running: `pg_isready`
2. Check database exists: `psql -l | grep xze_test`
3. Create if missing: `createdb xze_test`
4. Verify connection string: `echo $TEST_DATABASE_URL`

### Test Failures Due to Dirty Data

**Error**: Tests fail with unexpected data in database

**Solution**:
Tests automatically clean data before running via `setup_test_db()`. If issues persist:

```sql
-- Manual cleanup
TRUNCATE TABLE documents, document_chunks CASCADE;
```

### Compilation Errors

**Error**: "no test target named..."

**Solution**:
Ensure you specify the package:
```bash
cargo test -p xze-core --test kb_incremental_loading_tests -- --ignored
```

### Concurrent Test Interference

**Error**: Tests pass individually but fail when run together

**Solution**:
Run tests serially:
```bash
cargo test --test kb_incremental_loading_tests -- --ignored --test-threads=1
```

## Test Coverage

### Current Coverage

Run tests with coverage reporting:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --test-threads=1 --out Html --output-dir coverage
```

### Expected Coverage

- Unit tests: >80% line coverage
- Integration tests: All critical workflows covered
- Error paths: All error types validated

## Contributing

When adding new integration tests:

1. Mark with `#[ignore]` if requires external dependencies
2. Document requirements in this README
3. Implement cleanup in test setup to prevent interference
4. Use helper functions from `common/mod.rs` where applicable
5. Follow naming convention: `test_{feature}_{scenario}_{expected}`

## References

- User guide: `docs/how_to/incremental_loading_guide.md`
- Architecture: `docs/explanation/incremental_loading_architecture.md`
- Implementation: `docs/explanation/phase7_testing_documentation_implementation.md`
