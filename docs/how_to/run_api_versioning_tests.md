# How to Run API Versioning Tests

## Overview

This guide explains how to run the comprehensive API versioning tests for XZe, including contract tests, breaking change detection, and migration path validation.

## Prerequisites

### Required

- Rust toolchain (stable)
- Cargo installed
- Git repository cloned

### Optional (for runtime tests)

- PostgreSQL database (version 12+)
- DATABASE_URL environment variable set
- Ollama service running (for search tests)

## Test Categories

XZe has three categories of API versioning tests:

1. **Static Tests**: No database required, validate code structure and OpenAPI spec
2. **Runtime Tests**: Require database, validate actual API behavior
3. **Integration Tests**: Require full stack (database + Ollama)

## Quick Start

### Run All Static Tests

```bash
# Run all tests that don't require a database
cargo test --all-features
```

Expected output:
```
test result: ok. 56 passed; 0 failed; 39 ignored
```

### Run Specific Test Suites

```bash
# Contract tests (static only)
cargo test --package xze-serve --test contract_tests --features openapi

# Versioning tests (static only)
cargo test --package xze-serve --test api_versioning_tests --all-features

# Backward compatibility tests (static only)
cargo test --package xze-serve --test api_backward_compatibility_tests --all-features
```

## Running Runtime Tests

### Step 1: Set Up Test Database

```bash
# Create test database
createdb xze_test

# Or use Docker
docker run --name xze-test-db \
  -e POSTGRES_PASSWORD=test \
  -e POSTGRES_DB=xze_test \
  -p 5432:5432 \
  -d postgres:15

# Set DATABASE_URL
export DATABASE_URL="postgresql://postgres:test@localhost/xze_test"
```

### Step 2: Run Migrations

```bash
# Run database migrations
sqlx database create
sqlx migrate run
```

### Step 3: Run All Tests (Including Ignored)

```bash
# Run contract tests with database
cargo test --package xze-serve --test contract_tests --features openapi -- --ignored

# Run versioning tests with database
cargo test --package xze-serve --test api_versioning_tests --all-features -- --ignored

# Run backward compatibility tests with database
cargo test --package xze-serve --test api_backward_compatibility_tests --all-features -- --ignored
```

### Step 4: Run All Tests Together

```bash
# Run everything (static + runtime)
DATABASE_URL="postgresql://postgres:test@localhost/xze_test" \
  cargo test --all-features -- --ignored --test-threads=1
```

Note: `--test-threads=1` ensures tests run sequentially to avoid database conflicts.

## Running Specific Tests

### Run Single Test

```bash
# Run specific test by name
cargo test --package xze-serve --test contract_tests --features openapi \
  test_openapi_spec_has_all_v1_endpoints

# Run specific test with database
DATABASE_URL="postgresql://localhost/xze_test" \
  cargo test --package xze-serve --test api_versioning_tests --all-features \
  test_v1_health_response_structure_stable -- --ignored
```

### Run Tests by Pattern

```bash
# Run all breaking change detection tests
cargo test --package xze-serve --test api_versioning_tests --all-features \
  test_v1.*structure_stable -- --ignored

# Run all migration path tests
cargo test --package xze-serve --test api_versioning_tests --all-features \
  test_migration_path -- --ignored

# Run all version negotiation tests
cargo test --package xze-serve --test api_versioning_tests --all-features \
  test_version_negotiation -- --ignored
```

## Running Tests with Output

### Show Test Output

```bash
# Show println! and dbg! output from tests
cargo test --all-features -- --nocapture

# Show output for specific test
cargo test test_name -- --nocapture --ignored
```

### Show Backtraces on Failure

```bash
# Show full backtrace on panic
RUST_BACKTRACE=1 cargo test --all-features -- --ignored

# Show full backtrace with colors
RUST_BACKTRACE=full cargo test --all-features -- --ignored
```

### Enable Debug Logging

```bash
# Show debug logs during tests
RUST_LOG=debug cargo test --all-features -- --nocapture --ignored

# Show trace logs for specific module
RUST_LOG=xze_serve=trace cargo test --all-features -- --nocapture --ignored
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: API Versioning Tests

on: [push, pull_request]

jobs:
  static-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Run static tests
        run: cargo test --all-features

  runtime-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
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
        uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Run database migrations
        env:
          DATABASE_URL: postgresql://postgres:test@localhost:5432/xze_test
        run: |
          cargo install sqlx-cli --no-default-features --features postgres
          sqlx migrate run
      
      - name: Run contract tests
        env:
          DATABASE_URL: postgresql://postgres:test@localhost:5432/xze_test
        run: |
          cargo test --package xze-serve --test contract_tests \
            --features openapi -- --ignored --test-threads=1
      
      - name: Run versioning tests
        env:
          DATABASE_URL: postgresql://postgres:test@localhost:5432/xze_test
        run: |
          cargo test --package xze-serve --test api_versioning_tests \
            --all-features -- --ignored --test-threads=1
```

## Troubleshooting

### Database Connection Errors

**Problem**: Tests fail with "connection refused" or "database does not exist"

**Solution**:
```bash
# Check PostgreSQL is running
pg_isready

# Verify DATABASE_URL is correct
echo $DATABASE_URL

# Recreate test database
dropdb xze_test
createdb xze_test
sqlx migrate run
```

### OpenAPI Feature Not Enabled

**Problem**: Contract tests fail with "feature not enabled" error

**Solution**:
```bash
# Always use --features openapi for contract tests
cargo test --package xze-serve --test contract_tests --features openapi
```

### Tests Timeout

**Problem**: Tests hang or timeout

**Solution**:
```bash
# Increase test timeout
cargo test --all-features -- --ignored --test-threads=1 --nocapture

# Check for deadlocks in database
psql -d xze_test -c "SELECT * FROM pg_stat_activity WHERE datname = 'xze_test';"
```

### Ollama Service Unavailable

**Problem**: Search tests fail with connection errors

**Solution**:
```bash
# Start Ollama service
ollama serve

# Or skip search tests
cargo test --all-features -- --ignored --skip test_search
```

## Test Coverage

### View Test Coverage

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --all-features --out Html --output-dir coverage

# Open coverage report
open coverage/index.html
```

### Check Which Tests Run

```bash
# List all tests without running
cargo test --package xze-serve --test contract_tests --features openapi -- --list

# List only ignored tests
cargo test --package xze-serve --test api_versioning_tests --all-features \
  -- --list --ignored
```

## Best Practices

### Before Committing

Always run these checks:

```bash
# 1. Format code
cargo fmt --all

# 2. Check compilation
cargo check --all-targets --all-features

# 3. Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run all static tests
cargo test --all-features

# 5. Run runtime tests (if database available)
cargo test --all-features -- --ignored --test-threads=1
```

### During Development

Run tests frequently:

```bash
# Watch for changes and run tests automatically
cargo install cargo-watch

# Watch and run tests on file changes
cargo watch -x "test --all-features"
```

### Before Release

Run comprehensive validation:

```bash
# Run all tests with database
DATABASE_URL="postgresql://localhost/xze_test" \
  cargo test --all-features -- --ignored --test-threads=1 --nocapture

# Check for test warnings
cargo test --all-features -- --ignored 2>&1 | grep -i warning

# Verify no flaky tests (run 3 times)
for i in {1..3}; do
  echo "Run $i:"
  cargo test --all-features -- --ignored --test-threads=1 || exit 1
done
```

## Performance

### Run Tests in Parallel

```bash
# Run without --test-threads=1 for faster execution
# (Only safe if tests don't share database state)
cargo test --all-features -- --ignored

# Specify thread count
cargo test --all-features -- --ignored --test-threads=4
```

### Run Subset of Tests

```bash
# Run only fast tests during development
cargo test --all-features

# Run only specific module tests
cargo test --package xze-serve --lib

# Run only integration tests
cargo test --package xze-serve --test '*' --all-features
```

## References

- **Implementation Details**: `docs/explanations/phase7_5_3_api_versioning_tests_implementation.md`
- **Test Summary**: `docs/explanations/phase7_5_3_summary.md`
- **API Documentation**: `docs/reference/openapi_v1.json`
- **Migration Guide**: `docs/how_to/migrate_to_api_v1.md`

## Support

For issues or questions:
1. Check existing tests for examples
2. Review implementation documentation
3. Open an issue on GitHub
4. Contact the development team