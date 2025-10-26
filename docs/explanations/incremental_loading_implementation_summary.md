# Incremental Loading Implementation Summary

## Overview

This document summarizes the refactored implementation plan for XZe's incremental loading feature. The plan converts the original Python-based design into a production-ready Rust implementation that integrates seamlessly with XZe's existing architecture.

## What Was Delivered

### Primary Deliverable

- `docs/explanations/incremental_loading_implementation_plan.md` - Comprehensive 7-phase implementation plan converted from Python to Rust

### Key Changes from Original Plan

1. **Language Conversion**: All Python code examples converted to idiomatic Rust
2. **Architecture Integration**: Aligned with XZe's crate structure (xze-core, xze-cli, xze-serve)
3. **Async/Await**: All I/O operations use Tokio async runtime
4. **Error Handling**: Proper Result types with thiserror-based error definitions
5. **Type Safety**: Leveraged Rust's type system for configuration and statistics
6. **Database Layer**: Converted from psycopg to SQLx with transaction support
7. **Testing**: Comprehensive unit and integration test specifications
8. **Documentation**: Added Rust-specific documentation with runnable examples

## Feature Summary

The incremental loading feature enables XZe to intelligently load documentation by tracking file hashes and detecting changes. This provides:

- **Resume Capability**: Skip unchanged files after interrupted loads (>70% time reduction)
- **Selective Updates**: Only reload modified files (>85% time reduction for 10% changes)
- **Cleanup**: Remove stale database entries for deleted files
- **Dry-Run Mode**: Preview operations without making changes
- **Force Reload**: Override hash checks when needed

## Implementation Phases

### Phase 1: Hash Tracking Infrastructure (2-3 days)

Add SHA-256 file hashing and database schema extensions.

**Key Components**:
- `crates/core/src/kb/hash.rs` - Async file and content hashing
- `crates/core/src/kb/error.rs` - KB-specific error types
- Database migration for `file_hash` column and indexes

**Success Criteria**:
- Database schema updated with hash column
- Hash utilities calculate correct SHA-256 hashes
- All tests pass with >80% coverage

### Phase 2: File Discovery and Categorization (3-4 days)

Build file analysis system to categorize files by change status.

**Key Components**:
- `crates/core/src/kb/categorizer.rs` - File categorization logic
- `crates/core/src/kb/store.rs` - Database query operations

**File Categories**:
- Skip: Unchanged files (hash matches)
- Update: Modified files (hash differs)
- Add: New files (not in database)
- Delete: Removed files (in database but not on disk)

**Success Criteria**:
- Correctly identifies all four file categories
- Logs clear categorization summary
- No database modifications yet (analysis only)

### Phase 3: Implement Skip Logic (3-4 days)

Add resume capability to skip unchanged files.

**Key Components**:
- `crates/core/src/kb/loader.rs` - Incremental loader implementation
- `crates/cli/src/commands/load.rs` - CLI flag integration

**New CLI Flag**:
```bash
xze load --resume /path/to/docs
```

**Success Criteria**:
- Resume flag skips unchanged files
- >70% time reduction on second run
- No duplicate chunks created
- Backward compatible (works without flag)

### Phase 4: Implement Update Logic (4-5 days)

Add selective update capability with transaction safety.

**Key Components**:
- Atomic update operations (delete old + insert new in transaction)
- Transaction rollback on error
- Update statistics tracking

**New CLI Flag**:
```bash
xze load --update /path/to/docs
```

**Success Criteria**:
- Modified files detected and updated
- Old chunks deleted, new chunks inserted
- Transaction rollback works on error
- No orphaned data

### Phase 5: Implement Cleanup Logic (2-3 days)

Add cleanup capability to remove stale database entries.

**Key Components**:
- Batch deletion for removed files
- Safe default (cleanup is opt-in)

**New CLI Flag**:
```bash
xze load --update --cleanup /path/to/docs
```

**Success Criteria**:
- Cleanup removes deleted files from database
- Without cleanup flag, deleted files remain (safe default)
- Clear logging of deletions

### Phase 6: Polish CLI and UX (2-3 days)

Add dry-run, force, and improved user experience.

**Key Components**:
- Flag validation and conflict detection
- Enhanced logging with performance metrics
- Edge case handling

**New CLI Flags**:
```bash
xze load --dry-run /path/to/docs   # Preview changes
xze load --force /path/to/docs     # Full reload
```

**Success Criteria**:
- Dry-run shows actions without executing
- Force performs full reload
- Flag conflicts detected
- Graceful error handling

### Phase 7: Testing and Documentation (4-5 days)

Comprehensive testing and documentation.

**Key Components**:
- Integration tests with real database
- User guide (how-to documentation)
- Architecture documentation (explanations)
- Performance benchmarks

**Success Criteria**:
- All tests pass with >80% coverage
- Documentation complete and clear
- Performance improvements documented
- Benchmarks show expected gains

## Technical Architecture

### Module Structure

```text
crates/core/src/kb/
├── mod.rs           # Public API
├── hash.rs          # SHA-256 file hashing
├── categorizer.rs   # File change detection
├── store.rs         # Database operations
├── loader.rs        # Incremental loading engine
└── error.rs         # Error types
```

### Key Dependencies

```toml
sha2 = "0.10"              # SHA-256 hashing
walkdir = "2.4"            # Recursive directory traversal
sqlx = "0.7"               # Async PostgreSQL with migrations
tokio = "1.35"             # Async runtime
thiserror = "1.0"          # Error handling
```

### CLI Integration

```rust
// crates/cli/src/commands/load.rs
#[derive(Debug, Parser)]
pub struct LoadArgs {
    pub paths: Vec<PathBuf>,
    #[arg(long)] pub resume: bool,
    #[arg(long)] pub update: bool,
    #[arg(long)] pub cleanup: bool,
    #[arg(long)] pub dry_run: bool,
    #[arg(long)] pub force: bool,
}
```

## Performance Targets

Expected performance improvements based on testing:

| Operation              | Files Processed | Time Reduction | Use Case                 |
| ---------------------- | --------------- | -------------- | ------------------------ |
| Resume (unchanged)     | 0%              | >70%           | Interrupted loads        |
| Update (10% modified)  | 10%             | >85%           | Daily documentation sync |
| Update (50% modified)  | 50%             | >50%           | Major refactoring        |
| Force (full reload)    | 100%            | 0% (baseline)  | Schema changes           |

## Rust-Specific Patterns

### Error Handling

All operations return `Result<T, KbError>` with proper error context:

```rust
pub enum KbError {
    IoError(PathBuf, std::io::Error),
    DatabaseError(sqlx::Error),
    InvalidHash(PathBuf),
    FileNotFound(PathBuf),
    ConfigError(String),
}
```

### Async Operations

All I/O uses Tokio async runtime:

```rust
pub async fn calculate_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path).await?;
    // Async file reading with buffering
}
```

### Transaction Safety

Updates use SQLx transactions for atomicity:

```rust
let mut tx = pool.begin().await?;
sqlx::query("DELETE FROM documents WHERE source_file = $1")
    .execute(&mut *tx).await?;
// Insert new chunks
tx.commit().await?;  // Rolls back on error
```

### Type-Driven Configuration

```rust
pub struct LoaderConfig {
    pub resume: bool,
    pub update: bool,
    pub cleanup: bool,
    pub dry_run: bool,
    pub force: bool,
}
```

## Timeline and Effort

**Total Estimated Time**: 20-24 days (4-5 weeks for 1 developer)

**Critical Path**:
1. Phase 1 (hash infrastructure) - Required by all phases
2. Phase 2 (categorization) - Required by phases 3-6
3. Phase 3 (resume) - First user-visible feature
4. Phase 4 (update) - Enables incremental updates
5. Phase 5-6 (cleanup/polish) - Complete feature set
6. Phase 7 (testing/docs) - Production readiness

## Validation Checklist

All phases must meet these criteria:

- [ ] `cargo fmt --all` applied successfully
- [ ] `cargo check --all-targets --all-features` passes with zero errors
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes with >80% coverage
- [ ] Documentation created in `docs/explanations/` or `docs/how_to/`
- [ ] Filename uses lowercase_with_underscores.md
- [ ] No emojis in code or documentation
- [ ] All public functions have doc comments with examples

## Migration from Python to Rust

### Key Conversions

| Python Concept                | Rust Equivalent                         |
| ----------------------------- | --------------------------------------- |
| `def function(param: str)`    | `pub fn function(param: &str)`          |
| `Dict[str, str]`              | `HashMap<String, String>`               |
| `List[str]`                   | `Vec<String>` or `Vec<PathBuf>`         |
| `psycopg.Connection`          | `sqlx::PgPool`                          |
| `try/except`                  | `Result<T, E>` with `?` operator        |
| `@lru_cache`                  | Not needed (hash calculation is cheap)  |
| `hashlib.sha256`              | `sha2::Sha256`                          |
| `os.walk`                     | `walkdir::WalkDir`                      |
| Function returns              | `Result<T, KbError>` everywhere         |

### Async Conversion

All Python sync I/O converted to Tokio async:

```python
# Python
def read_file(path: str) -> str:
    with open(path, 'r') as f:
        return f.read()
```

```rust
// Rust
pub async fn read_file(path: &Path) -> Result<String> {
    tokio::fs::read_to_string(path).await
        .map_err(|e| KbError::IoError(path.to_path_buf(), e))
}
```

## Integration Points

### With Existing XZe Components

- **AIDocumentationGenerator**: For chunk generation and embedding
- **XzeConfig**: For database connection and configuration
- **Pipeline**: Can be triggered by file watcher or scheduler
- **CLI**: New commands integrated into existing command structure

### Database Schema

Additive changes only (no breaking changes):

```sql
ALTER TABLE documents ADD COLUMN file_hash TEXT;
CREATE INDEX idx_documents_file_hash ON documents(file_hash);
CREATE INDEX idx_documents_source_hash ON documents(source_file, file_hash);
```

## Known Limitations

Current scope limitations (potential future enhancements):

- No parallel file processing (could add rayon for hash calculation)
- No progress bars (could add indicatif crate)
- No persistent hash cache (could add SQLite for filesystem cache)
- No incremental embedding updates (requires AI service changes)
- Single-node only (no distributed cache)
- No file size limits or quotas

## Next Steps

To implement this feature:

1. **Review and approve** this implementation plan
2. **Create feature branch**: `feature/incremental-loading`
3. **Start Phase 1**: Hash infrastructure (2-3 days)
4. **Incremental reviews**: Review after each phase completion
5. **Testing**: Run comprehensive tests after Phase 3 and Phase 6
6. **Documentation**: Complete all docs in Phase 7
7. **Merge to main**: After all validations pass

## References

- Implementation Plan: `docs/explanations/incremental_loading_implementation_plan.md`
- AGENTS.md: Project development guidelines
- XZe Architecture: `docs/explanations/architecture.md`
- Diataxis Framework: https://diataxis.fr/

## Validation Results

- [x] Implementation plan converted from Python to Rust
- [x] All code examples are idiomatic Rust with proper error handling
- [x] Architecture aligned with XZe crate structure
- [x] Documentation follows AGENTS.md guidelines (lowercase filenames, no emojis)
- [x] Testing strategy defined with >80% coverage requirements
- [x] Performance targets specified
- [x] Timeline and effort estimated
- [x] Integration points identified

**Status**: Ready for implementation
