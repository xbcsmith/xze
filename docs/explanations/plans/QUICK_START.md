# Search Features - Quick Start Guide

**Last Updated**: December 2024

## 🎯 Current Status

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 6: CLI Search | ✅ Complete | Semantic search command-line interface |
| Phase 7: API Endpoint | ✅ Complete | REST API for search |
| Phase 8: Production | 📋 Next | Testing, security, monitoring |
| Phase 9: Performance | 📋 Planned | Caching, optimization |
| Phase 10: Advanced | 📋 Planned | Multi-field, aggregations |
| Phase 11: Real-Time | 📋 Planned | WebSocket, streaming |
| Phase 12: Analytics | 📋 Planned | Usage tracking, metrics |

## ✅ What Works Now

### CLI Search
```bash
# Basic search
xze search "how to install"

# With filters
xze search "API endpoints" --max-results 5 --min-similarity 0.7 --category reference

# JSON output
xze search "configuration" --json
```

### REST API
```bash
# Start server
xze serve --host 0.0.0.0 --port 3000

# Search via API
curl "http://localhost:3000/search?q=installation&max_results=5&min_similarity=0.7"

# Category filter
curl "http://localhost:3000/search?q=tutorial&category=tutorial"
```

### Response Format
```json
{
  "query": "installation",
  "results": [{
    "id": 42,
    "source_file": "docs/how_to/installation.md",
    "content": "To install XZe...",
    "similarity": 0.92,
    "title": "Installation Guide",
    "category": "how_to"
  }],
  "total_results": 1
}
```

## 🚀 Next: Phase 8.1 Integration Testing

**Priority**: 🔴 HIGH
**Duration**: 3-4 days
**Goal**: Production-ready testing

### Day 1: Setup Test Infrastructure
```bash
# 1. Create test database
./tests/setup_test_db.sh

# 2. Set environment
export TEST_DATABASE_URL="postgresql://localhost/xze_test"

# 3. Run migrations
sqlx migrate run --database-url $TEST_DATABASE_URL
```

**Files to Create**:
- `tests/common/test_db.rs` - Database utilities
- `tests/common/mod.rs` - Test helpers

### Day 2: Write Integration Tests
**File**: `tests/search_api_tests.rs`

Tests needed:
- ✅ Basic search success
- ✅ Query parameter validation
- ✅ Category filtering
- ✅ Similarity thresholds
- ✅ Error scenarios
- ✅ Empty results handling

```bash
# Run tests
cargo test --test search_api_tests
```

### Day 3: Performance Benchmarks
**File**: `benches/search_bench.rs`

```bash
# Run benchmarks
cargo bench --bench search_bench
```

### Day 4: CI/CD Integration
**File**: `.github/workflows/integration-tests.yml`

```bash
# Test CI locally
act push
```

## 📊 Quick Reference

### Architecture
```
User
  ↓
CLI / REST API
  ↓
xze-serve (Axum)
  ↓
xze-core (search logic)
  ↓
PostgreSQL (pgvector) + Ollama (embeddings)
```

### Key Files

**Core Search**:
- `crates/core/src/semantic/search.rs` - Search implementation
- `crates/core/src/semantic/embeddings.rs` - Embedding generation

**CLI**:
- `crates/cli/src/commands/search.rs` - CLI command

**Server**:
- `crates/serve/src/handlers.rs` - API handlers
- `crates/serve/src/api.rs` - Routes
- `crates/serve/src/server.rs` - Server setup

**Tests**:
- `tests/search_api_tests.rs` - Integration tests (to create)
- `benches/search_bench.rs` - Benchmarks (to create)

### Dependencies
```toml
# Already added:
sqlx = { version = "0.7", features = ["postgres"] }
axum = "0.7"
tower-http = "0.5"

# Need to add for Phase 8:
tower-governor = "0.1"  # Rate limiting
prometheus = "0.13"     # Metrics
```

## 📖 Documentation

### For Users
- `docs/reference/search_api_endpoint.md` - API reference
- `docs/reference/search_command_reference.md` - CLI reference
- `docs/how_to/use_search_api.md` - Usage guide

### For Developers
- `docs/explanations/phase_6_search_integration_implementation.md`
- `docs/explanations/phase_7_search_api_implementation.md`
- `docs/plans/search_features_completion_plan.md` - Full roadmap
- `docs/plans/phase_8_1_integration_testing.md` - Testing guide

## 🎬 Getting Started (5 minutes)

### 1. Try the CLI
```bash
# Search indexed documentation
xze search "installation" --max-results 5
```

### 2. Start the Server
```bash
# Terminal 1: Start server
xze serve

# Terminal 2: Test API
curl "http://localhost:3000/search?q=test"
```

### 3. Read the Docs
```bash
# View implementation details
cat docs/explanations/phase_7_search_api_implementation.md

# View roadmap
cat docs/plans/search_features_completion_plan.md
```

### 4. Start Phase 8.1
```bash
# Read the guide
cat docs/plans/phase_8_1_integration_testing.md

# Setup test database
./tests/setup_test_db.sh

# Create first test file
touch tests/search_api_tests.rs
```

## 🔧 Common Commands

### Development
```bash
# Build
cargo build

# Test (unit tests)
cargo test

# Test (integration - after Phase 8.1)
cargo test --test search_api_tests

# Benchmark
cargo bench

# Check without building
cargo check
```

### Running Search
```bash
# CLI with verbose output
xze search "query" -v

# CLI with JSON
xze search "query" --json

# API with curl
curl -s "http://localhost:3000/search?q=query" | jq

# API with HTTPie
http GET localhost:3000/search q=="query" max_results==5
```

### Database
```bash
# Connect to database
psql xze

# Check indexed documents
psql xze -c "SELECT COUNT(*) FROM semantic_chunks;"

# View embeddings
psql xze -c "SELECT id, source_file FROM documents LIMIT 5;"
```

## 🐛 Troubleshooting

### Search returns no results
```bash
# Check database has data
psql xze -c "SELECT COUNT(*) FROM semantic_chunks;"

# Check Ollama is running
curl http://localhost:11434/api/tags

# Try with min_similarity=0
xze search "query" --min-similarity 0
```

### Server won't start
```bash
# Check database connection
psql postgresql://localhost/xze -c "SELECT 1;"

# Check port is available
lsof -i :3000

# Check Ollama
curl http://localhost:11434/api/version
```

### Tests fail
```bash
# Check test database exists
psql xze_test -c "SELECT 1;"

# Run migrations
sqlx migrate run --database-url postgresql://localhost/xze_test

# Clean test data
psql xze_test -c "TRUNCATE TABLE semantic_chunks CASCADE;"
```

## 📞 Help & Resources

### Issues?
- Check `docs/how_to/` guides
- Review `docs/reference/` for API details
- Read `docs/explanations/` for architecture

### Want to Contribute?
1. Start with Phase 8.1 (Integration Testing)
2. Follow `docs/plans/phase_8_1_integration_testing.md`
3. Run all tests before submitting PR
4. Update documentation

### Questions?
- Architecture: See `docs/reference/xze-architecture.md`
- Implementation: See phase implementation docs
- Planning: See `docs/plans/search_features_completion_plan.md`

## 🎯 Success Metrics

### Current (Phase 7)
- ✅ CLI search functional
- ✅ API endpoint working
- ✅ Documentation complete
- ✅ Basic error handling
- ⏳ Test coverage: ~40%

### Target (End of Phase 8)
- Test coverage: >80%
- All integration tests passing
- CI/CD pipeline green
- Production deployment ready
- Monitoring operational

### Target (End of Phase 9)
- Cache hit rate: >90%
- p95 latency: <500ms
- Supports: 10,000+ docs
- Database optimized

## 🗓️ Timeline

- **Week 1-2**: Phase 8 (Production Readiness)
- **Week 3-4**: Phase 9 (Performance)
- **Week 5-6**: Phase 10 (Advanced Features Part 1)
- **Week 7-8**: Phase 11 (Real-Time)
- **Week 9**: Phase 10 (Advanced Features Part 2)
- **Week 10**: Phase 12 (Analytics)

**Total**: ~10 weeks to complete all features

## 💡 Pro Tips

1. **Start Small**: Begin with Phase 8.1, don't skip ahead
2. **Test Early**: Write tests as you implement features
3. **Document Always**: Update docs with every PR
4. **Monitor Performance**: Track benchmarks from the start
5. **Use Examples**: Refer to existing search code patterns

## 🚦 Ready to Start?

```bash
# 1. Review current implementation
cat docs/explanations/phase_7_search_api_implementation.md

# 2. Read Phase 8.1 guide
cat docs/plans/phase_8_1_integration_testing.md

# 3. Setup test environment
./tests/setup_test_db.sh

# 4. Start coding!
code tests/search_api_tests.rs
```

---

**Quick Links**:
- [Full Plan](./search_features_completion_plan.md)
- [Phase 8.1 Guide](./phase_8_1_integration_testing.md)
- [API Reference](../reference/search_api_endpoint.md)
- [Architecture](../reference/xze-architecture.md)

**Status**: Phase 7 ✅ Complete → Phase 8 📋 Next → Phase 9-12 🔮 Planned
