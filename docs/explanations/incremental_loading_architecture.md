# Incremental Loading Architecture

## Overview

The incremental loading system enables XZe to efficiently manage large documentation repositories by processing only files that have changed. This document describes the architecture, components, data flow, and design decisions behind the incremental loading implementation.

## Design Goals

1. **Efficiency**: Avoid reprocessing unchanged files
2. **Correctness**: Ensure database accurately reflects filesystem state
3. **Resumability**: Support interruption and continuation of long-running loads
4. **Flexibility**: Support multiple operational modes (resume, update, cleanup, dry-run)
5. **Transparency**: Provide clear feedback on what operations are performed

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                            │
│                    (crates/cli/commands/load.rs)            │
│  - Parse arguments and flags                                │
│  - Validate flag combinations                               │
│  - Convert to LoaderConfig                                  │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Loader Layer                            │
│                  (crates/core/src/kb/loader.rs)             │
│  - Orchestrate loading workflow                             │
│  - Categorize files (skip/add/update/delete)                │
│  - Execute operations based on config                       │
│  - Collect and report statistics                            │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Component Layer                           │
│                   (crates/core/src/kb/)                     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │     Hash     │  │ Categorizer  │  │    Store     │     │
│  │   Module     │  │   Module     │  │   Module     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                              │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Database Layer                            │
│                      (PostgreSQL)                            │
│  - documents table (file_path, file_hash)                   │
│  - document_chunks table (content, embeddings)              │
└─────────────────────────────────────────────────────────────┘
```

## Components

### Hash Module

**Location**: `crates/core/src/kb/hash.rs`

**Purpose**: Calculate SHA-256 hashes of file content for change detection.

**Key Functions**:

- `calculate_file_hash(path: &Path) -> Result<String>`: Computes hash from file on disk
- `calculate_content_hash(content: &[u8]) -> String`: Computes hash from byte content

**Design Decisions**:

- Uses SHA-256 for strong collision resistance
- Reads entire file into memory (acceptable for documentation files)
- No content normalization (line endings, encoding preserved)
- Returns hex-encoded string for database storage

**Performance Characteristics**:

- O(n) where n is file size
- Limited by disk I/O speed
- Typically < 1ms for small documentation files (< 100KB)

### Categorizer Module

**Location**: `crates/core/src/kb/categorizer.rs`

**Purpose**: Classify files into categories based on hash comparison.

**Key Types**:

```rust
pub enum FileCategory {
    Skip,    // File unchanged, already in database
    Update,  // File exists but hash differs
    Add,     // New file not in database
    Delete,  // File in database but missing from filesystem
}

pub struct CategorizedFiles {
    pub skip: Vec<String>,
    pub update: Vec<String>,
    pub add: Vec<String>,
    pub delete: Vec<String>,
}
```

**Key Functions**:

- `discover_files_with_hashes(paths: &[String]) -> Result<HashMap<String, String>>`: Walks filesystem and computes hashes
- `FileCategorizer::new(current_files, existing_files) -> Self`: Creates categorizer
- `FileCategorizer::categorize() -> CategorizedFiles`: Performs categorization logic

**Categorization Logic**:

```
For each file in filesystem:
  If file in database:
    If hash matches:
      Category = Skip
    Else:
      Category = Update
  Else:
    Category = Add

For each file in database:
  If file not in filesystem:
    Category = Delete
```

**Design Decisions**:

- All filesystem discovery happens upfront (not streamed)
- Hash calculation parallelized via async/await
- Results stored in memory (trade-off: memory vs. latency)
- Categorization is deterministic and repeatable

### Store Module

**Location**: `crates/core/src/kb/store.rs`

**Purpose**: Abstract database operations for file and chunk management.

**Key Functions**:

- `query_existing_files() -> HashMap<String, String>`: Retrieves all files with hashes from database
- `delete_chunks_for_file(file_path: &str)`: Removes all chunks for a specific file
- `insert_file_chunks(file_path: &str, chunks: Vec<Chunk>)`: Inserts document and chunks
- `cleanup_deleted_files(deleted_paths: &[String])`: Removes documents and chunks for deleted files

**Database Schema**:

```sql
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    file_path TEXT UNIQUE NOT NULL,
    file_hash TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE document_chunks (
    id SERIAL PRIMARY KEY,
    document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding VECTOR(1536),  -- Dimension depends on embedding model
    metadata JSONB,
    UNIQUE(document_id, chunk_index)
);
```

**Transaction Model**:

- **Insert operations**: Each file insertion wrapped in implicit transaction
- **Update operations**: Delete chunks + insert new chunks (atomic per file)
- **Delete operations**: Cascade delete removes chunks automatically
- **Cleanup operations**: Each file deleted independently (not batched in single transaction)

**Design Decisions**:

- Uses `sqlx` for compile-time query validation
- Connection pooling for concurrent operations
- Foreign key constraints ensure referential integrity
- Cascade deletes simplify cleanup logic
- File paths are absolute and normalized

### Loader Module

**Location**: `crates/core/src/kb/loader.rs`

**Purpose**: Orchestrate the complete loading workflow.

**Key Types**:

```rust
pub struct LoaderConfig {
    pub resume: bool,    // Skip files already in database
    pub update: bool,    // Update changed files
    pub cleanup: bool,   // Remove deleted files
    pub dry_run: bool,   // Preview without changes
    pub force: bool,     // Treat all files as new
}

pub struct LoadStats {
    pub files_skipped: usize,
    pub files_added: usize,
    pub files_updated: usize,
    pub files_deleted: usize,
    pub chunks_inserted: usize,
    pub chunks_deleted: usize,
    pub duration_secs: f64,
}
```

**Key Functions**:

- `IncrementalLoader::new(store, config) -> Self`: Creates loader with configuration
- `load(paths: &[String]) -> Result<LoadStats>`: Executes complete loading workflow
- `process_add_files(files: &[String])`: Processes new files
- `process_update_files(files: &[String])`: Updates modified files
- `process_delete_files(files: &[String])`: Removes deleted files

**Workflow**:

```
1. Validate configuration
2. Discover files and compute hashes
3. Query existing files from database
4. Categorize files (skip/add/update/delete)
5. Log operation mode and file counts
6. If dry-run: Log preview and return
7. Process additions (if any)
8. Process updates (if update flag enabled)
9. Process deletions (if cleanup flag enabled)
10. Log completion summary and statistics
```

**Mode Behavior**:

| Mode | Skip | Add | Update | Delete |
|------|------|-----|--------|--------|
| Default | Yes (unchanged) | Yes | No | No |
| --resume | Yes (all existing) | Yes | No | No |
| --update | Yes (unchanged) | Yes | Yes | No |
| --update --cleanup | Yes (unchanged) | Yes | Yes | Yes |
| --force | No | Yes (all) | N/A | Optional (--cleanup) |
| --dry-run | N/A | Preview | Preview | Preview |

## Data Flow

### Full Load Workflow

```
User: xze load /docs
  │
  ▼
CLI: Parse args, create config {resume: false, update: false, cleanup: false}
  │
  ▼
Loader: discover_files_with_hashes(["/docs"])
  │
  ▼
Hash: Walk filesystem, compute SHA-256 for each file
  │
  ▼
Store: query_existing_files()
  │
  ▼
Categorizer: Categorize files
  │
  ├─ Skip: []
  ├─ Add: [all files]
  ├─ Update: []
  └─ Delete: []
  │
  ▼
Loader: process_add_files()
  │
  ├─ Generate chunks for each file
  ├─ Generate embeddings (placeholder)
  └─ Store: insert_file_chunks()
  │
  ▼
Loader: Return LoadStats
  │
  ▼
CLI: Display summary
```

### Incremental Update Workflow

```
User: xze load --update --cleanup /docs
  │
  ▼
CLI: Parse args, create config {resume: false, update: true, cleanup: true}
  │
  ▼
Loader: discover_files_with_hashes(["/docs"])
  │
  ▼
Hash: Walk filesystem, compute SHA-256 for each file
  │
  ▼
Store: query_existing_files()
  │
  ▼
Categorizer: Categorize files
  │
  ├─ Skip: [files with matching hashes]
  ├─ Add: [new files]
  ├─ Update: [files with different hashes]
  └─ Delete: [files in DB but not in filesystem]
  │
  ▼
Loader: process_add_files() for Add category
  │
  ▼
Loader: process_update_files() for Update category
  │
  ├─ For each file:
  │   ├─ Store: delete_chunks_for_file()
  │   ├─ Generate new chunks
  │   └─ Store: insert_file_chunks()
  │
  ▼
Loader: process_delete_files() for Delete category
  │
  └─ Store: cleanup_deleted_files()
  │
  ▼
Loader: Return LoadStats
  │
  ▼
CLI: Display summary
```

## Performance Characteristics

### Time Complexity

- **File discovery**: O(n) where n = number of files
- **Hash calculation**: O(n * m) where m = average file size
- **Database query**: O(d) where d = files in database
- **Categorization**: O(n + d)
- **Processing**: O(p * c) where p = files to process, c = chunks per file

### Space Complexity

- **In-memory**: O(n) for file paths and hashes
- **Database**: O(d * c) for documents and chunks

### Bottlenecks

1. **Disk I/O**: Reading files for hashing and chunking
2. **Database writes**: Insert operations for chunks
3. **Chunk generation**: CPU-bound text processing
4. **Embedding generation**: GPU/API calls (currently placeholder)

### Optimization Strategies

1. **Parallel hashing**: Use async/await for concurrent file reads
2. **Batch insertions**: Group chunk insertions where possible
3. **Connection pooling**: Reuse database connections
4. **Incremental processing**: Only process changed files
5. **Skip logic**: Avoid reprocessing unchanged files

## Error Handling

### Error Types

```rust
pub enum KbError {
    IoError(String),           // File system errors
    DatabaseError(String),     // Database operation errors
    InvalidHash(String),       // Hash calculation errors
    FileNotFound(String),      // Missing file errors
    ConfigError(String),       // Invalid configuration
}
```

### Error Recovery

- **File read errors**: Skip file and log error, continue with others
- **Database errors**: Fail fast (transaction rollback where applicable)
- **Hash errors**: Skip file and log error
- **Configuration errors**: Fail at validation before processing

### Transactional Guarantees

- **Single file operations**: Atomic (success or rollback)
- **Multiple file operations**: Currently independent (partial success possible)
- **Dry-run mode**: No transactions, read-only operations

### Future Improvements

- Batch deletions in single transaction for cleanup
- Retry logic for transient database errors
- Checkpoint/resume for very large loads
- Progress persistence to allow true resume after crash

## Design Decisions

### Why SHA-256?

- Strong collision resistance
- Fast computation
- Widely supported
- Sufficient for file change detection

### Why Hash Entire File?

- Simple and deterministic
- Ensures exact change detection
- Acceptable performance for documentation files (< 100KB typical)
- Alternative (mtime) is unreliable across systems

### Why Eager Discovery?

- Simplifies categorization logic
- Enables accurate progress reporting
- Memory footprint acceptable for documentation repositories
- Alternative (streaming) would complicate transaction model

### Why Per-File Transactions?

- Simplifies error handling
- Allows partial progress
- Reduces lock contention
- Trade-off: Consistency vs. resumability

### Why Separate Skip/Update/Add Categories?

- Clear user feedback
- Enables dry-run preview
- Simplifies testing
- Allows mode-specific logic

## Testing Strategy

### Unit Tests

- Hash calculation correctness
- Categorization logic
- Configuration validation
- Statistics aggregation

### Integration Tests

Location: `crates/core/tests/kb_incremental_loading_tests.rs`

Scenarios:
- Resume after full load
- Update modified files
- Cleanup deleted files
- Dry run mode
- Force full reload
- Mixed operations (add + update + delete)

### Test Infrastructure

- Uses testcontainers for PostgreSQL (when available)
- Temporary directories for file operations
- Helper functions for file creation/modification/deletion
- Database cleanup between tests

## Future Enhancements

### Short Term

1. Batch database operations for better performance
2. Configurable verbosity levels
3. Progress bars for long operations
4. Parallel chunk processing

### Medium Term

1. Incremental embedding updates (avoid regenerating unchanged chunks)
2. Checkpoint/resume for crash recovery
3. File-level locking for concurrent loads
4. Compressed hash storage

### Long Term

1. Distributed loading across multiple workers
2. Content-aware chunking (preserve semantic boundaries)
3. Smart embedding caching (reuse similar chunks)
4. Real-time incremental updates (file watcher integration)

## Security Considerations

1. **Path validation**: Normalize and validate file paths to prevent directory traversal
2. **SQL injection**: Use parameterized queries (sqlx compile-time validation)
3. **Resource limits**: Consider adding file size limits and rate limiting
4. **Permissions**: Respect filesystem permissions during discovery

## Monitoring and Observability

### Metrics to Track

- Files processed per second
- Average hash calculation time
- Database operation latency
- Chunk generation throughput
- Memory usage during discovery

### Logging Levels

- INFO: Operation mode, file counts, summary statistics
- DEBUG: Per-file operations, categorization decisions
- TRACE: Hash values, database queries

### Key Log Messages

```
INFO: Starting incremental load (mode: update+cleanup)
INFO: Discovered 1234 files (567 new, 89 modified, 12 deleted, 566 unchanged)
INFO: Processing complete (added: 567, updated: 89, deleted: 12, skipped: 566)
```

## References

- Hash module implementation: `crates/core/src/kb/hash.rs`
- Categorizer implementation: `crates/core/src/kb/categorizer.rs`
- Store implementation: `crates/core/src/kb/store.rs`
- Loader implementation: `crates/core/src/kb/loader.rs`
- CLI integration: `crates/cli/src/commands/load.rs`
- Integration tests: `crates/core/tests/kb_incremental_loading_tests.rs`
- User guide: `docs/how_to/incremental_loading_guide.md`
