# Incremental Loading Implementation Plan

## Overview

This plan implements intelligent incremental loading with file hash tracking for XZe's knowledge base, allowing resume after interruption and selective updates when files change. The implementation follows a phased approach to ensure each step can be verified and tested independently.

This feature enables:

- Fast resume after interrupted loads (skip unchanged files)
- Selective updates when documentation changes (only reload modified files)
- Cleanup of stale database entries (remove deleted files)
- Dry-run mode for safe preview of operations

---

## Architecture Integration

### Crate Structure

```text
xze/
└── crates/
    ├── core/                  # xze-core crate
    │   └── src/
    │       ├── kb/                    # New module for knowledge base
    │       │   ├── mod.rs            # Public API
    │       │   ├── loader.rs         # Incremental document loader
    │       │   ├── hash.rs           # File hashing utilities
    │       │   ├── categorizer.rs    # File change detection
    │       │   ├── store.rs          # Database operations
    │       │   └── error.rs          # KB-specific errors
    │       └── ...
    └── cli/                   # xze-cli crate
        └── src/
            ├── commands/
            │   └── load.rs           # CLI command integration
            └── ...
```

### Dependencies to Add

Add these to `crates/core/Cargo.toml`:

```toml
[dependencies]
# Core dependencies (likely already present)
tokio = { version = "1.35", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "migrate"] }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"

# New dependencies for this feature
sha2 = "0.10"              # SHA-256 hashing
walkdir = "2.4"            # Recursive directory traversal
```

---

## Phase 1: Hash Tracking Infrastructure

### Objectives

- Add `file_hash` column to database schema
- Create hash calculation utilities
- Update database initialization
- NO behavior changes - purely additive

### Time Estimate

2-3 days

### Files to Create

- `crates/core/src/kb/mod.rs` (new module)
- `crates/core/src/kb/hash.rs` (hash utilities)
- `crates/core/src/kb/error.rs` (error types)
- `migrations/YYYYMMDD_add_file_hash.sql` (database migration)

### Files to Modify

- `crates/core/src/lib.rs` (add `pub mod kb`)

### Deliverables

#### 1. Create KB Module Structure

```rust
// crates/core/src/kb/mod.rs
//! Knowledge base loading and management

pub mod error;
pub mod hash;
pub mod loader;
pub mod categorizer;
pub mod store;

pub use error::{KbError, Result};
pub use hash::{FileHasher, calculate_file_hash, calculate_content_hash};
pub use loader::{LoaderConfig, IncrementalLoader, LoadStats};
pub use categorizer::{FileCategory, FileCategorizer};
pub use store::KbStore;
```

#### 2. Implement Hash Utilities

````rust
// crates/core/src/kb/hash.rs
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

use super::error::Result;

/// Calculates SHA-256 hash of a file asynchronously
///
/// # Arguments
///
/// * `path` - Path to the file to hash
///
/// # Returns
///
/// Returns the hex-encoded SHA-256 hash (64 characters)
///
/// # Errors
///
/// Returns `KbError::IoError` if file cannot be read
///
/// # Examples
///
/// ```rust
/// use xze_core::kb::hash::calculate_file_hash;
/// use std::path::Path;
///
/// async fn example() {
///     let hash = calculate_file_hash(Path::new("README.md")).await.unwrap();
///     assert_eq!(hash.len(), 64);
/// }
/// ```
pub async fn calculate_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path).await
        .map_err(|e| KbError::IoError(path.to_path_buf(), e))?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = reader.read(&mut buffer).await
            .map_err(|e| KbError::IoError(path.to_path_buf(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Calculates SHA-256 hash of content string
///
/// # Arguments
///
/// * `content` - String content to hash
///
/// # Returns
///
/// Returns the hex-encoded SHA-256 hash (64 characters)
///
/// # Examples
///
/// ```rust
/// use xze_core::kb::hash::calculate_content_hash;
///
/// let hash = calculate_content_hash("Hello, world!");
/// assert_eq!(hash.len(), 64);
/// ```
pub fn calculate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_calculate_file_hash_success() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"Hello, world!").await.unwrap();

        let hash = calculate_file_hash(&file_path).await.unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_calculate_file_hash_nonexistent() {
        let result = calculate_file_hash(Path::new("/nonexistent/file.txt")).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_content_hash_consistent() {
        let content = "Test content";
        let hash1 = calculate_content_hash(content);
        let hash2 = calculate_content_hash(content);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_calculate_content_hash_different_content() {
        let hash1 = calculate_content_hash("content1");
        let hash2 = calculate_content_hash("content2");
        assert_ne!(hash1, hash2);
    }
}
````

#### 3. Define Error Types

```rust
// crates/core/src/kb/error.rs
use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, KbError>;

#[derive(Error, Debug)]
pub enum KbError {
    #[error("Failed to read file {0}: {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Invalid file hash format for {0}")]
    InvalidHash(PathBuf),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

#### 4. Create Database Migration

```sql
-- migrations/YYYYMMDD_add_file_hash.sql

-- Add file_hash column to documents table
ALTER TABLE documents
ADD COLUMN IF NOT EXISTS file_hash TEXT;

-- Add index for hash lookups
CREATE INDEX IF NOT EXISTS idx_documents_file_hash
ON documents(file_hash);

-- Add composite index for efficient queries
CREATE INDEX IF NOT EXISTS idx_documents_source_hash
ON documents(source_file, file_hash);

-- Add index for timestamp-based queries (useful for incremental loads)
CREATE INDEX IF NOT EXISTS idx_documents_updated_at
ON documents(updated_at);
```

### Verification Steps

```bash
# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo test --package xze-core kb::hash::tests

# Check compilation
cargo check --all-targets --all-features

# Apply migration (using sqlx-cli)
sqlx migrate run

# Verify schema
psql $DATABASE_URL -c "\d documents"
# Should see file_hash column with TEXT type

# Verify indexes
psql $DATABASE_URL -c "\di idx_documents_*"
# Should see three new indexes
```

### Success Criteria

- Database has `file_hash` column (TEXT type)
- Three indexes created successfully
- Hash utilities calculate correct SHA-256 hashes (64 chars)
- All unit tests pass with >80% coverage
- No clippy warnings
- Code formatted with rustfmt

---

## Phase 2: File Discovery and Categorization

### Objectives

- Build file analysis system
- Categorize files (skip/update/add/delete)
- Add dry-run reporting
- Still NO database modifications - just analysis

### Time Estimate

3-4 days

### Files to Create

- `crates/core/src/kb/categorizer.rs` (file categorization logic)
- `crates/core/src/kb/store.rs` (database queries)

### Deliverables

#### 1. Implement File Discovery

```rust
// crates/core/src/kb/categorizer.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use tracing::{debug, info};

use super::hash::calculate_file_hash;
use super::error::{KbError, Result};

/// Categories for file change detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    /// File exists in DB with same hash - skip processing
    Skip,
    /// File exists in DB with different hash - needs update
    Update,
    /// File does not exist in DB - needs to be added
    Add,
    /// File in DB but no longer on disk - candidate for deletion
    Delete,
}

/// Result of file categorization
#[derive(Debug, Clone)]
pub struct CategorizedFiles {
    pub skip: Vec<PathBuf>,
    pub update: Vec<PathBuf>,
    pub add: Vec<PathBuf>,
    pub delete: Vec<PathBuf>,
}

impl CategorizedFiles {
    pub fn total_files(&self) -> usize {
        self.skip.len() + self.update.len() + self.add.len() + self.delete.len()
    }

    pub fn files_to_process(&self) -> usize {
        self.update.len() + self.add.len()
    }
}

/// File categorizer for incremental loading
pub struct FileCategorizer {
    /// Current files on disk with their hashes
    current_files: HashMap<PathBuf, String>,
    /// Existing files in database with their hashes
    existing_files: HashMap<PathBuf, String>,
}

impl FileCategorizer {
    /// Discovers files in given paths and calculates their hashes
    ///
    /// # Arguments
    ///
    /// * `paths` - List of file or directory paths to scan
    ///
    /// # Returns
    ///
    /// Returns a map of file paths to their SHA-256 hashes
    ///
    /// # Errors
    ///
    /// Returns `KbError::IoError` if files cannot be accessed
    pub async fn discover_files_with_hashes(paths: &[PathBuf]) -> Result<HashMap<PathBuf, String>> {
        let mut files = HashMap::new();

        for path in paths {
            if path.is_file() {
                let hash = calculate_file_hash(path).await?;
                files.insert(path.clone(), hash);
            } else if path.is_dir() {
                for entry in WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let file_path = entry.path().to_path_buf();
                    let hash = calculate_file_hash(&file_path).await?;
                    files.insert(file_path, hash);
                }
            }
        }

        info!("Discovered {} files on disk", files.len());
        Ok(files)
    }

    /// Creates a new categorizer with current and existing files
    pub fn new(
        current_files: HashMap<PathBuf, String>,
        existing_files: HashMap<PathBuf, String>,
    ) -> Self {
        Self {
            current_files,
            existing_files,
        }
    }

    /// Categorizes files into skip/update/add/delete buckets
    ///
    /// # Returns
    ///
    /// Returns categorized file lists
    pub fn categorize(&self) -> CategorizedFiles {
        let mut result = CategorizedFiles {
            skip: Vec::new(),
            update: Vec::new(),
            add: Vec::new(),
            delete: Vec::new(),
        };

        // Check current files against database
        for (path, current_hash) in &self.current_files {
            match self.existing_files.get(path) {
                Some(existing_hash) if existing_hash == current_hash => {
                    debug!("Skip (unchanged): {}", path.display());
                    result.skip.push(path.clone());
                }
                Some(_) => {
                    debug!("Update (modified): {}", path.display());
                    result.update.push(path.clone());
                }
                None => {
                    debug!("Add (new): {}", path.display());
                    result.add.push(path.clone());
                }
            }
        }

        // Find files in database but not on disk
        for path in self.existing_files.keys() {
            if !self.current_files.contains_key(path) {
                debug!("Delete (removed): {}", path.display());
                result.delete.push(path.clone());
            }
        }

        info!(
            "Categorization complete: {} skip, {} update, {} add, {} delete",
            result.skip.len(),
            result.update.len(),
            result.add.len(),
            result.delete.len()
        );

        result
    }

    /// Logs categorization summary to console
    pub fn log_summary(categorized: &CategorizedFiles) {
        println!("\nFile Analysis:");
        println!("  Skip (unchanged):  {}", categorized.skip.len());
        println!("  Update (modified): {}", categorized.update.len());
        println!("  Add (new):         {}", categorized.add.len());
        println!("  Delete (removed):  {}", categorized.delete.len());
        println!("  Total files:       {}", categorized.total_files());
        println!("  Files to process:  {}\n", categorized.files_to_process());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_new_files() {
        let mut current = HashMap::new();
        current.insert(PathBuf::from("new.md"), "hash123".to_string());

        let existing = HashMap::new();

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.add.len(), 1);
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.update.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_unchanged_files() {
        let mut current = HashMap::new();
        current.insert(PathBuf::from("file.md"), "hash123".to_string());

        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("file.md"), "hash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.skip.len(), 1);
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.update.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_modified_files() {
        let mut current = HashMap::new();
        current.insert(PathBuf::from("file.md"), "hash456".to_string());

        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("file.md"), "hash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.update.len(), 1);
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_deleted_files() {
        let current = HashMap::new();

        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("deleted.md"), "hash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.delete.len(), 1);
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.update.len(), 0);
    }

    #[test]
    fn test_categorize_mixed_scenario() {
        let mut current = HashMap::new();
        current.insert(PathBuf::from("unchanged.md"), "hash1".to_string());
        current.insert(PathBuf::from("modified.md"), "hash_new".to_string());
        current.insert(PathBuf::from("new.md"), "hash3".to_string());

        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("unchanged.md"), "hash1".to_string());
        existing.insert(PathBuf::from("modified.md"), "hash_old".to_string());
        existing.insert(PathBuf::from("deleted.md"), "hash4".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.skip.len(), 1);
        assert_eq!(result.update.len(), 1);
        assert_eq!(result.add.len(), 1);
        assert_eq!(result.delete.len(), 1);
        assert_eq!(result.total_files(), 4);
        assert_eq!(result.files_to_process(), 2);
    }
}
```

#### 2. Implement Database Queries

```rust
// crates/core/src/kb/store.rs
use std::collections::HashMap;
use std::path::PathBuf;
use sqlx::{PgPool, Row};
use tracing::debug;

use super::error::Result;

/// Database operations for knowledge base
pub struct KbStore {
    pool: PgPool,
}

impl KbStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Queries existing files from database
    ///
    /// # Returns
    ///
    /// Returns a map of file paths to their stored hashes
    ///
    /// # Errors
    ///
    /// Returns `KbError::DatabaseError` if query fails
    pub async fn query_existing_files(&self) -> Result<HashMap<PathBuf, String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT source_file, file_hash FROM documents WHERE file_hash IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut files = HashMap::new();
        for row in rows {
            let source_file: String = row.try_get("source_file")?;
            let file_hash: String = row.try_get("file_hash")?;
            files.insert(PathBuf::from(source_file), file_hash);
        }

        debug!("Loaded {} files from database", files.len());
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests with actual database in tests/integration/
    // These are just structure tests

    #[test]
    fn test_kb_store_creation() {
        // This would need a real pool in integration tests
        // Just verifying struct can be instantiated
    }
}
```

### Verification Steps

```bash
# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo test --package xze-core kb::categorizer::tests
cargo test --package xze-core kb::store::tests

# Check compilation
cargo check --all-targets --all-features

# Integration test (manual for now)
# Create test with some docs
mkdir -p /tmp/xze-test/docs
echo "# Test Doc" > /tmp/xze-test/docs/test.md

# Run load (should categorize as "Add")
xze load /tmp/xze-test/docs

# Expected output:
# File Analysis:
#   Skip (unchanged):  0
#   Update (modified): 0
#   Add (new):         1
#   Delete (removed):  0

# Modify file and run again
echo "# Modified" >> /tmp/xze-test/docs/test.md
xze load /tmp/xze-test/docs

# Expected output:
#   Skip (unchanged):  0
#   Update (modified): 1
#   Add (new):         0
#   Delete (removed):  0
```

### Success Criteria

- Correctly identifies new files (first run)
- Correctly identifies unchanged files (second run)
- Correctly identifies modified files (after edit)
- Logs clear summary to console
- All unit tests pass with >80% coverage
- Still performs full load (backward compatible)
- No clippy warnings

---

## Phase 3: Implement Skip Logic (Resume)

### Objectives

- Skip unchanged files (do not reprocess)
- Add `--resume` flag to CLI
- First behavior change - performance improvement
- Remove any truncate-table logic

### Time Estimate

3-4 days

### Files to Create

- `crates/core/src/kb/loader.rs` - Incremental loader implementation

### Files to Modify

- `crates/cli/src/commands/load.rs` (add CLI flag)
- `crates/core/src/kb/store.rs` (add insert methods)

### Deliverables

#### 1. Implement Incremental Loader

```rust
// crates/core/src/kb/loader.rs
use std::path::PathBuf;
use std::time::Instant;
use sqlx::PgPool;
use tracing::{info, warn};

use super::categorizer::{CategorizedFiles, FileCategorizer};
use super::error::Result;
use super::store::KbStore;

/// Configuration for incremental loader
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    pub resume: bool,
    pub update: bool,
    pub cleanup: bool,
    pub dry_run: bool,
    pub force: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            resume: false,
            update: false,
            cleanup: false,
            dry_run: false,
            force: false,
        }
    }
}

/// Statistics from a load operation
#[derive(Debug, Clone, Default)]
pub struct LoadStats {
    pub files_skipped: usize,
    pub files_added: usize,
    pub files_updated: usize,
    pub files_deleted: usize,
    pub chunks_inserted: usize,
    pub chunks_deleted: usize,
    pub duration_secs: f64,
}

impl LoadStats {
    pub fn log_summary(&self) {
        println!("\nLoad Statistics:");
        println!("  Files skipped:     {}", self.files_skipped);
        println!("  Files added:       {}", self.files_added);
        println!("  Files updated:     {}", self.files_updated);
        println!("  Files deleted:     {}", self.files_deleted);
        println!("  Chunks inserted:   {}", self.chunks_inserted);
        println!("  Chunks deleted:    {}", self.chunks_deleted);
        println!("  Duration:          {:.2}s\n", self.duration_secs);
    }
}

/// Incremental document loader with resume capability
pub struct IncrementalLoader {
    store: KbStore,
    config: LoaderConfig,
}

impl IncrementalLoader {
    pub fn new(pool: PgPool, config: LoaderConfig) -> Self {
        Self {
            store: KbStore::new(pool),
            config,
        }
    }

    /// Validates configuration flags for conflicts
    fn validate_config(&self) -> Result<()> {
        if self.config.force && (self.config.resume || self.config.update) {
            warn!("--force flag overrides --resume and --update");
        }
        Ok(())
    }

    /// Loads documents incrementally based on configuration
    ///
    /// # Arguments
    ///
    /// * `paths` - List of file or directory paths to load
    ///
    /// # Returns
    ///
    /// Returns load statistics
    ///
    /// # Errors
    ///
    /// Returns `KbError` if loading fails
    pub async fn load(&self, paths: &[PathBuf]) -> Result<LoadStats> {
        self.validate_config()?;

        let start = Instant::now();
        let mut stats = LoadStats::default();

        if self.config.dry_run {
            println!("\nDRY RUN - No changes will be made\n");
        }

        // Discover current files on disk
        let current_files = FileCategorizer::discover_files_with_hashes(paths).await?;

        // Query existing files from database
        let existing_files = if self.config.force {
            info!("Force mode: treating all files as new");
            Default::default()
        } else {
            self.store.query_existing_files().await?
        };

        // Categorize files
        let categorizer = FileCategorizer::new(current_files.clone(), existing_files);
        let categorized = categorizer.categorize();
        FileCategorizer::log_summary(&categorized);

        if self.config.dry_run {
            stats.duration_secs = start.elapsed().as_secs_f64();
            return Ok(stats);
        }

        // Process files based on configuration
        if self.config.resume {
            // Resume mode: only add new files, skip everything else
            stats.files_skipped = categorized.skip.len() + categorized.update.len();
            stats.files_added = self.process_add_files(&categorized.add, &current_files).await?;
        } else if self.config.update {
            // Update mode: skip unchanged, update modified, add new
            stats.files_skipped = categorized.skip.len();
            stats.files_updated = self.process_update_files(&categorized.update, &current_files).await?;
            stats.files_added = self.process_add_files(&categorized.add, &current_files).await?;

            if self.config.cleanup {
                stats.files_deleted = self.process_delete_files(&categorized.delete).await?;
            }
        } else if self.config.force {
            // Force mode: reload everything
            let all_files: Vec<PathBuf> = current_files.keys().cloned().collect();
            stats.files_added = self.process_add_files(&all_files, &current_files).await?;
        } else {
            // Default mode: full load (backward compatible)
            let all_files: Vec<PathBuf> = current_files.keys().cloned().collect();
            stats.files_added = self.process_add_files(&all_files, &current_files).await?;
        }

        stats.duration_secs = start.elapsed().as_secs_f64();
        stats.log_summary();

        Ok(stats)
    }

    async fn process_add_files(
        &self,
        files: &[PathBuf],
        hashes: &std::collections::HashMap<PathBuf, String>,
    ) -> Result<usize> {
        info!("Processing {} files to add", files.len());

        for file in files {
            let hash = hashes.get(file).expect("hash should exist");
            // TODO: Actual document processing and chunk insertion
            // This will be implemented when integrating with existing doc pipeline
            info!("Would add file: {} (hash: {})", file.display(), hash);
        }

        Ok(files.len())
    }

    async fn process_update_files(
        &self,
        files: &[PathBuf],
        hashes: &std::collections::HashMap<PathBuf, String>,
    ) -> Result<usize> {
        info!("Processing {} files to update", files.len());

        for file in files {
            let hash = hashes.get(file).expect("hash should exist");
            // TODO: Delete old chunks and insert new ones in transaction
            info!("Would update file: {} (hash: {})", file.display(), hash);
        }

        Ok(files.len())
    }

    async fn process_delete_files(&self, files: &[PathBuf]) -> Result<usize> {
        info!("Processing {} files to delete", files.len());

        for file in files {
            // TODO: Delete chunks for file
            info!("Would delete file: {}", file.display());
        }

        Ok(files.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert!(!config.resume);
        assert!(!config.update);
        assert!(!config.cleanup);
        assert!(!config.dry_run);
        assert!(!config.force);
    }

    #[test]
    fn test_load_stats_default() {
        let stats = LoadStats::default();
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.files_added, 0);
        assert_eq!(stats.files_updated, 0);
        assert_eq!(stats.files_deleted, 0);
    }

    // Integration tests with database would go in tests/integration/
}
```

#### 2. Update CLI Command

```rust
// crates/cli/src/commands/load.rs (additions)

// Add to existing load command structure
#[derive(Debug, Parser)]
pub struct LoadArgs {
    /// Paths to load
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Skip unchanged files (incremental load)
    #[arg(long)]
    pub resume: bool,

    // Other existing flags...
}

// In execute function:
pub async fn execute(args: LoadArgs, config: XzeConfig) -> Result<()> {
    let loader_config = LoaderConfig {
        resume: args.resume,
        ..Default::default()
    };

    let loader = IncrementalLoader::new(pool, loader_config);
    let stats = loader.load(&args.paths).await?;

    Ok(())
}
```

### Verification Steps

```bash
# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Build
cargo build --release

# First load (everything is new)
time xze load /path/to/docs
# Note the duration

# Second load WITHOUT --resume (default behavior - full reload)
time xze load /path/to/docs
# Should take similar time

# Third load WITH --resume (new behavior - skip unchanged)
time xze load --resume /path/to/docs
# Should be MUCH faster
# Output should show:
#   Files skipped: N
#   Files added: 0

# Modify a file
echo "# Test change" >> /path/to/docs/test.md

# Load with resume
xze load --resume /path/to/docs
# Output should show:
#   Files skipped: N-1
#   Files added: 1

# Verify database
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents;"
# Should have chunks from all files

# Verify no duplicates
psql $DATABASE_URL -c "SELECT source_file, file_hash, COUNT(*) FROM documents GROUP BY source_file, file_hash HAVING COUNT(*) > 1;"
# Should return 0 rows
```

### Success Criteria

- `--resume` flag skips unchanged files
- Significantly faster on second run with `--resume` (>70% time reduction)
- All files still in database
- No duplicate chunks created
- Backward compatible (works without `--resume`)
- All tests pass with >80% coverage
- No clippy warnings

---

## Phase 4: Implement Update Logic

### Objectives

- Detect modified files
- Delete old chunks and insert new chunks atomically
- Transaction safety per file
- Add `--update` flag

### Time Estimate

4-5 days

### Files to Modify

- `crates/core/src/kb/store.rs` (add delete and transaction methods)
- `crates/core/src/kb/loader.rs` (implement update logic)
- `crates/cli/src/commands/load.rs` (add CLI flag)

### Deliverables

#### 1. Add Database Operations

```rust
// crates/core/src/kb/store.rs (additions)

impl KbStore {
    /// Deletes all chunks for a specific file
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the file whose chunks should be deleted
    ///
    /// # Returns
    ///
    /// Returns the number of chunks deleted
    ///
    /// # Errors
    ///
    /// Returns `KbError::DatabaseError` if deletion fails
    pub async fn delete_chunks_for_file(&self, source_file: &Path) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM documents WHERE source_file = $1"
        )
        .bind(source_file.to_string_lossy().as_ref())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Updates file chunks atomically (delete old + insert new in transaction)
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the file
    /// * `file_hash` - New hash of the file
    /// * `chunks` - Chunks to insert
    ///
    /// # Errors
    ///
    /// Returns `KbError::DatabaseError` if transaction fails
    /// Transaction is rolled back automatically on error
    pub async fn update_file_chunks(
        &self,
        source_file: &Path,
        file_hash: &str,
        chunks: &[DocumentChunk],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete old chunks
        sqlx::query("DELETE FROM documents WHERE source_file = $1")
            .bind(source_file.to_string_lossy().as_ref())
            .execute(&mut *tx)
            .await?;

        // Insert new chunks
        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO documents
                (source_file, file_hash, chunk_id, content, embedding, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(source_file.to_string_lossy().as_ref())
            .bind(file_hash)
            .bind(&chunk.chunk_id)
            .bind(&chunk.content)
            .bind(&chunk.embedding)
            .bind(&chunk.metadata)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Inserts chunks for a new file
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the file
    /// * `file_hash` - Hash of the file
    /// * `chunks` - Chunks to insert
    ///
    /// # Errors
    ///
    /// Returns `KbError::DatabaseError` if insert fails
    pub async fn insert_file_chunks(
        &self,
        source_file: &Path,
        file_hash: &str,
        chunks: &[DocumentChunk],
    ) -> Result<()> {
        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO documents
                (source_file, file_hash, chunk_id, content, embedding, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(source_file.to_string_lossy().as_ref())
            .bind(file_hash)
            .bind(&chunk.chunk_id)
            .bind(&chunk.content)
            .bind(&chunk.embedding)
            .bind(&chunk.metadata)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

// Placeholder for document chunk structure
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub chunk_id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
}
```

#### 2. Update Loader Implementation

```rust
// crates/core/src/kb/loader.rs (update methods)

impl IncrementalLoader {
    async fn process_update_files(
        &self,
        files: &[PathBuf],
        hashes: &std::collections::HashMap<PathBuf, String>,
    ) -> Result<usize> {
        info!("Processing {} files to update", files.len());

        for file in files {
            let hash = hashes.get(file).expect("hash should exist");

            // Generate new chunks (integrate with existing pipeline)
            let chunks = self.generate_chunks(file).await?;

            // Update atomically in transaction
            self.store.update_file_chunks(file, hash, &chunks).await?;

            info!("Updated file: {} ({} chunks)", file.display(), chunks.len());
        }

        Ok(files.len())
    }

    async fn process_add_files(
        &self,
        files: &[PathBuf],
        hashes: &std::collections::HashMap<PathBuf, String>,
    ) -> Result<usize> {
        info!("Processing {} files to add", files.len());

        for file in files {
            let hash = hashes.get(file).expect("hash should exist");

            // Generate chunks
            let chunks = self.generate_chunks(file).await?;

            // Insert chunks
            self.store.insert_file_chunks(file, hash, &chunks).await?;

            info!("Added file: {} ({} chunks)", file.display(), chunks.len());
        }

        Ok(files.len())
    }

    // Placeholder - integrate with existing document pipeline
    async fn generate_chunks(&self, _file: &Path) -> Result<Vec<DocumentChunk>> {
        // TODO: Wire into existing AIDocumentationGenerator or chunking logic
        Ok(Vec::new())
    }
}
```

#### 3. Update CLI

```rust
// crates/cli/src/commands/load.rs (additions)

#[derive(Debug, Parser)]
pub struct LoadArgs {
    /// Paths to load
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Skip unchanged files (incremental load)
    #[arg(long)]
    pub resume: bool,

    /// Update modified files and add new files (skip unchanged)
    #[arg(long)]
    pub update: bool,

    // Other existing flags...
}

pub async fn execute(args: LoadArgs, config: XzeConfig) -> Result<()> {
    let loader_config = LoaderConfig {
        resume: args.resume,
        update: args.update,
        ..Default::default()
    };

    let loader = IncrementalLoader::new(pool, loader_config);
    let stats = loader.load(&args.paths).await?;

    Ok(())
}
```

### Verification Steps

```bash
# Initial load
xze load /path/to/docs

# Record chunk count for a file
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents WHERE source_file = '/path/to/docs/test.md';"
# Example: 5 chunks

# Modify the file (add content)
echo -e "\n\n## New Section\n\nNew content." >> /path/to/docs/test.md

# Update
xze load --update /path/to/docs

# Check chunk count
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents WHERE source_file = '/path/to/docs/test.md';"
# Should be different (e.g., 6 or 7 chunks)

# Verify hash changed
psql $DATABASE_URL -c "SELECT DISTINCT file_hash FROM documents WHERE source_file = '/path/to/docs/test.md';"
# Should show only 1 hash (new one)

# Verify no orphaned chunks
psql $DATABASE_URL -c "SELECT source_file, file_hash, COUNT(*) FROM documents GROUP BY source_file, file_hash ORDER BY source_file;"
# Each file should have exactly 1 hash

# Test transaction rollback (simulate error)
# Temporarily modify code to panic mid-transaction, verify rollback

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --package xze-core kb::
```

### Success Criteria

- Modified files are detected correctly
- Old chunks are deleted atomically
- New chunks are inserted successfully
- File hash is updated in database
- No duplicate hashes per file
- Transaction rollback works on error
- All tests pass with >80% coverage
- No clippy warnings

---

## Phase 5: Implement Cleanup Logic

### Objectives

- Remove chunks for deleted files
- Add `--cleanup` flag
- Optional but completes the feature

### Time Estimate

2-3 days

### Files to Modify

- `crates/core/src/kb/store.rs` (add cleanup method)
- `crates/core/src/kb/loader.rs` (implement cleanup)
- `crates/cli/src/commands/load.rs` (add CLI flag)

### Deliverables

#### 1. Add Cleanup Method

```rust
// crates/core/src/kb/store.rs (additions)

impl KbStore {
    /// Removes chunks for files that no longer exist on disk
    ///
    /// # Arguments
    ///
    /// * `files_to_delete` - List of file paths to remove from database
    ///
    /// # Returns
    ///
    /// Returns the total number of chunks deleted
    ///
    /// # Errors
    ///
    /// Returns `KbError::DatabaseError` if deletion fails
    pub async fn cleanup_deleted_files(&self, files_to_delete: &[PathBuf]) -> Result<u64> {
        let mut total_deleted = 0;

        for file in files_to_delete {
            let result = self.delete_chunks_for_file(file).await?;
            total_deleted += result;
            info!("Cleaned up {} chunks for deleted file: {}", result, file.display());
        }

        Ok(total_deleted)
    }
}
```

#### 2. Implement in Loader

```rust
// crates/core/src/kb/loader.rs (update process_delete_files)

impl IncrementalLoader {
    async fn process_delete_files(&self, files: &[PathBuf]) -> Result<usize> {
        info!("Processing {} files to delete", files.len());

        let chunks_deleted = self.store.cleanup_deleted_files(files).await?;
        info!("Deleted {} chunks total", chunks_deleted);

        Ok(files.len())
    }
}
```

#### 3. Update CLI

```rust
// crates/cli/src/commands/load.rs (additions)

#[derive(Debug, Parser)]
pub struct LoadArgs {
    /// Paths to load
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Skip unchanged files (incremental load)
    #[arg(long)]
    pub resume: bool,

    /// Update modified files and add new files (skip unchanged)
    #[arg(long)]
    pub update: bool,

    /// Remove chunks for files no longer on disk
    #[arg(long)]
    pub cleanup: bool,

    // Other existing flags...
}

pub async fn execute(args: LoadArgs, config: XzeConfig) -> Result<()> {
    let loader_config = LoaderConfig {
        resume: args.resume,
        update: args.update,
        cleanup: args.cleanup,
        ..Default::default()
    };

    let loader = IncrementalLoader::new(pool, loader_config);
    let stats = loader.load(&args.paths).await?;

    Ok(())
}
```

### Verification Steps

```bash
# Load files
xze load /path/to/docs

# Count chunks
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents;"
# Note the count

# Delete a file from disk
rm /path/to/docs/some_file.md

# Load without cleanup (default safe behavior)
xze load --update /path/to/docs

# File still in database
psql $DATABASE_URL -c "SELECT DISTINCT source_file FROM documents WHERE source_file LIKE '%some_file.md';"
# Should still show the file

# Load with cleanup
xze load --update --cleanup /path/to/docs

# File removed from database
psql $DATABASE_URL -c "SELECT DISTINCT source_file FROM documents WHERE source_file LIKE '%some_file.md';"
# Should return 0 rows

# Verify count decreased
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents;"
# Should be less than before

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --package xze-core kb::
```

### Success Criteria

- `--cleanup` removes deleted files from database
- Without `--cleanup`, deleted files remain (safe default)
- Correct count of removed chunks logged
- Clear logging of deletions
- All tests pass with >80% coverage
- No clippy warnings

---

## Phase 6: Polish CLI and User Experience

### Objectives

- Add `--dry-run` flag
- Add `--force` flag (explicit full reload)
- Improve logging and user feedback
- Handle edge cases gracefully

### Time Estimate

2-3 days

### Files to Modify

- `crates/core/src/kb/loader.rs` (add dry-run and force logic)
- `crates/cli/src/commands/load.rs` (add CLI flags and validation)

### Deliverables

#### 1. Update CLI Arguments

```rust
// crates/cli/src/commands/load.rs (complete implementation)

#[derive(Debug, Parser)]
pub struct LoadArgs {
    /// Paths to load (files or directories)
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Skip unchanged files (incremental load)
    #[arg(long)]
    pub resume: bool,

    /// Update modified files and add new files (skip unchanged)
    #[arg(long)]
    pub update: bool,

    /// Remove chunks for files no longer on disk
    #[arg(long)]
    pub cleanup: bool,

    /// Show what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Force full reload (ignore existing data)
    #[arg(long)]
    pub force: bool,
}

impl LoadArgs {
    /// Validates argument combinations
    pub fn validate(&self) -> Result<()> {
        if self.force && (self.resume || self.update) {
            warn!("--force overrides --resume and --update flags");
        }

        if self.resume && self.update {
            return Err(anyhow::anyhow!(
                "Cannot use --resume and --update together. Use --update for incremental updates."
            ));
        }

        if self.cleanup && !self.update {
            return Err(anyhow::anyhow!(
                "--cleanup requires --update flag"
            ));
        }

        if self.paths.is_empty() {
            return Err(anyhow::anyhow!(
                "At least one path must be provided"
            ));
        }

        for path in &self.paths {
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Path does not exist: {}",
                    path.display()
                ));
            }
        }

        Ok(())
    }
}

pub async fn execute(args: LoadArgs, config: XzeConfig) -> Result<()> {
    args.validate()?;

    let loader_config = LoaderConfig {
        resume: args.resume,
        update: args.update,
        cleanup: args.cleanup,
        dry_run: args.dry_run,
        force: args.force,
    };

    let pool = establish_connection(&config.database_url).await?;
    let loader = IncrementalLoader::new(pool, loader_config);
    let stats = loader.load(&args.paths).await?;

    Ok(())
}
```

#### 2. Enhanced Logging

```rust
// crates/core/src/kb/loader.rs (enhance logging)

impl IncrementalLoader {
    pub async fn load(&self, paths: &[PathBuf]) -> Result<LoadStats> {
        self.validate_config()?;
        self.log_mode();

        let start = Instant::now();
        let mut stats = LoadStats::default();

        // Existing implementation with enhanced logging...

        stats.duration_secs = start.elapsed().as_secs_f64();
        self.log_completion(&stats);

        Ok(stats)
    }

    fn log_mode(&self) {
        println!("\n========================================");
        if self.config.dry_run {
            println!("MODE: DRY RUN (no changes will be made)");
        } else if self.config.force {
            println!("MODE: FORCE (full reload)");
        } else if self.config.update {
            println!("MODE: UPDATE (incremental with updates)");
        } else if self.config.resume {
            println!("MODE: RESUME (skip unchanged files)");
        } else {
            println!("MODE: FULL LOAD (default)");
        }
        println!("========================================\n");
    }

    fn log_completion(&self, stats: &LoadStats) {
        println!("\n========================================");
        println!("LOAD COMPLETE");
        println!("========================================");
        stats.log_summary();

        if stats.files_skipped > 0 {
            let skip_pct = (stats.files_skipped as f64
                / (stats.files_skipped + stats.files_to_process()) as f64) * 100.0;
            println!("Performance: Skipped {:.1}% of files", skip_pct);
        }
    }
}

impl LoadStats {
    pub fn files_to_process(&self) -> usize {
        self.files_added + self.files_updated
    }

    pub fn log_summary(&self) {
        println!("\nLoad Statistics:");
        println!("  Files skipped:     {}", self.files_skipped);
        println!("  Files added:       {}", self.files_added);
        println!("  Files updated:     {}", self.files_updated);
        println!("  Files deleted:     {}", self.files_deleted);
        println!("  Chunks inserted:   {}", self.chunks_inserted);
        println!("  Chunks deleted:    {}", self.chunks_deleted);
        println!("  Duration:          {:.2}s", self.duration_secs);

        if self.duration_secs > 0.0 && self.files_to_process() > 0 {
            let files_per_sec = self.files_to_process() as f64 / self.duration_secs;
            println!("  Throughput:        {:.1} files/sec", files_per_sec);
        }
        println!();
    }
}
```

### Verification Steps

```bash
# Test dry run
xze load --update --cleanup --dry-run /path/to/docs

# Expected output:
# MODE: DRY RUN (no changes will be made)
# File Analysis:
#   Skip (unchanged):  X
#   Update (modified): Y
#   Add (new):         Z
#   Delete (removed):  W
# (No actual changes)

# Verify no database changes
psql $DATABASE_URL -c "SELECT COUNT(*) FROM documents;"
# Count should be unchanged

# Test force reload
xze load --force /path/to/docs
# Should reload all files

# Test conflicting flags
xze load --resume --force /path/to/docs
# Should show warning but proceed with force

xze load --resume --update /path/to/docs
# Should show error and exit

xze load --cleanup /path/to/docs
# Should show error (cleanup requires update)

# Test with nonexistent path
xze load /nonexistent/path
# Should show clear error message

# Test with empty directory
mkdir /tmp/empty
xze load /tmp/empty
# Should handle gracefully

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --package xze-core kb::
cargo test --package xze-cli
```

### Success Criteria

- `--dry-run` shows actions without executing
- `--force` performs full reload correctly
- Clear, informative logging throughout
- Graceful error handling for all edge cases
- Flag conflict detection and validation
- Helpful error messages for users
- All tests pass with >80% coverage
- No clippy warnings

---

## Phase 7: Testing and Documentation

### Objectives

- Add comprehensive unit tests
- Add integration tests with real database
- Create user documentation
- Performance benchmarking
- Update API documentation

### Time Estimate

4-5 days

### Files to Create

- `crates/core/tests/integration/kb_loader.rs` (integration tests)
- `docs/how_to/incremental_loading.md` (user guide)
- `docs/reference/cli_reference.md` (update CLI docs)
- `docs/explanation/incremental_loading_design.md` (architecture doc)

### Deliverables

#### 1. Integration Tests

```rust
// crates/core/tests/integration/kb_loader.rs
use sqlx::PgPool;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use xze_core::kb::{IncrementalLoader, LoaderConfig};

async fn setup_test_db() -> PgPool {
    // Setup test database with migrations
    // Use sqlx test features or testcontainers
    todo!()
}

#[tokio::test]
async fn test_resume_after_full_load() {
    let pool = setup_test_db().await;
    let dir = TempDir::new().unwrap();

    // Create test files
    let file1 = dir.path().join("test1.md");
    fs::write(&file1, "# Test 1").await.unwrap();

    // First load
    let config = LoaderConfig::default();
    let loader = IncrementalLoader::new(pool.clone(), config);
    let stats1 = loader.load(&[dir.path().to_path_buf()]).await.unwrap();
    assert_eq!(stats1.files_added, 1);

    // Second load with resume
    let config = LoaderConfig {
        resume: true,
        ..Default::default()
    };
    let loader = IncrementalLoader::new(pool.clone(), config);
    let stats2 = loader.load(&[dir.path().to_path_buf()]).await.unwrap();
    assert_eq!(stats2.files_skipped, 1);
    assert_eq!(stats2.files_added, 0);
}

#[tokio::test]
async fn test_update_modified_files() {
    let pool = setup_test_db().await;
    let dir = TempDir::new().unwrap();

    // Create and load file
    let file = dir.path().join("test.md");
    fs::write(&file, "# Original").await.unwrap();

    let config = LoaderConfig::default();
    let loader = IncrementalLoader::new(pool.clone(), config);
    loader.load(&[dir.path().to_path_buf()]).await.unwrap();

    // Modify file
    fs::write(&file, "# Modified").await.unwrap();

    // Update
    let config = LoaderConfig {
        update: true,
        ..Default::default()
    };
    let loader = IncrementalLoader::new(pool.clone(), config);
    let stats = loader.load(&[dir.path().to_path_buf()]).await.unwrap();

    assert_eq!(stats.files_updated, 1);

    // Verify only one hash exists for file
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT file_hash) FROM documents WHERE source_file = $1"
    )
    .bind(file.to_string_lossy().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_cleanup_deleted_files() {
    let pool = setup_test_db().await;
    let dir = TempDir::new().unwrap();

    // Create and load file
    let file = dir.path().join("to_delete.md");
    fs::write(&file, "# Delete me").await.unwrap();

    let config = LoaderConfig::default();
    let loader = IncrementalLoader::new(pool.clone(), config);
    loader.load(&[dir.path().to_path_buf()]).await.unwrap();

    // Delete file from disk
    fs::remove_file(&file).await.unwrap();

    // Load with cleanup
    let config = LoaderConfig {
        update: true,
        cleanup: true,
        ..Default::default()
    };
    let loader = IncrementalLoader::new(pool.clone(), config);
    let stats = loader.load(&[dir.path().to_path_buf()]).await.unwrap();

    assert_eq!(stats.files_deleted, 1);

    // Verify file removed from database
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM documents WHERE source_file = $1"
    )
    .bind(file.to_string_lossy().as_ref())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_dry_run_mode() {
    let pool = setup_test_db().await;
    let dir = TempDir::new().unwrap();

    let file = dir.path().join("test.md");
    fs::write(&file, "# Test").await.unwrap();

    // Get initial count
    let initial: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Dry run
    let config = LoaderConfig {
        dry_run: true,
        ..Default::default()
    };
    let loader = IncrementalLoader::new(pool.clone(), config);
    loader.load(&[dir.path().to_path_buf()]).await.unwrap();

    // Verify no changes
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(initial, after);
}
```

#### 2. User Guide

````markdown
<!-- docs/how_to/incremental_loading.md -->

# Incremental Loading Guide

## Overview

XZe supports incremental loading to speed up document processing by skipping
unchanged files and selectively updating modified ones.

## Quick Start

### Full Load (Default)

```bash
xze load /path/to/docs
```

Loads all documents, replacing existing data.

### Resume After Interruption

```bash
xze load --resume /path/to/docs
```

Skips files that are already in the database with matching hashes. Use this to
resume after an interrupted load operation.

### Incremental Updates

```bash
xze load --update /path/to/docs
```

Updates only modified files, adds new files, skips unchanged files.

### Incremental Updates with Cleanup

```bash
xze load --update --cleanup /path/to/docs
```

Same as update mode, but also removes database entries for files that have been
deleted from disk.

### Dry Run

```bash
xze load --update --cleanup --dry-run /path/to/docs
```

Shows what would be done without making any changes.

### Force Full Reload

```bash
xze load --force /path/to/docs
```

Reloads all files regardless of hash status.

## Use Cases

### Scenario 1: Initial Load

```bash
xze load /path/to/large/codebase
```

First time loading a large codebase. Use default mode.

### Scenario 2: Load Interrupted

```bash
xze load --resume /path/to/large/codebase
```

Network failed halfway through. Resume skips already-loaded files.

### Scenario 3: Daily Documentation Updates

```bash
xze load --update --cleanup /path/to/docs
```

Run nightly to keep documentation in sync with source files.

### Scenario 4: Preview Changes

```bash
xze load --update --cleanup --dry-run /path/to/docs
```

See what would change without committing.

## Performance Tips

- Use `--resume` for interrupted loads (fastest)
- Use `--update` for incremental updates (fast)
- Use `--force` only when schema changes (slow)
- Run `--cleanup` periodically, not on every load

## Troubleshooting

### Issue: Files Not Being Updated

**Symptom**: Modified files show as "skip" instead of "update"

**Solution**: File hash might not have changed. Check for:

- Whitespace-only changes (hash unchanged)
- File system timestamp vs content
- Use `--force` to reload anyway

### Issue: Database Growing Too Large

**Symptom**: Database size increasing over time

**Solution**: Run cleanup regularly:

```bash
xze load --update --cleanup /path/to/docs
```

### Issue: Slow Performance

**Symptom**: Load takes a long time even with `--resume`

**Solution**:

- Check database indexes exist
- Run `VACUUM ANALYZE` on PostgreSQL
- Consider batch size tuning (future feature)

## Technical Details

### Hash Algorithm

XZe uses SHA-256 for file hashing. Hash is calculated from file content only
(not metadata like timestamps).

### Database Schema

```sql
ALTER TABLE documents ADD COLUMN file_hash TEXT;
CREATE INDEX idx_documents_file_hash ON documents(file_hash);
CREATE INDEX idx_documents_source_hash ON documents(source_file, file_hash);
```

### Transaction Safety

Update operations use database transactions to ensure atomicity. If an update
fails halfway through, all changes are rolled back.
````

#### 3. Architecture Documentation

````markdown
<!-- docs/explanation/incremental_loading_design.md -->

# Incremental Loading Architecture

## Overview

Incremental loading uses SHA-256 file hashing to detect changes and selectively
update the knowledge base, dramatically improving performance for large
repositories.

## Components

### Hash Module

Calculates SHA-256 hashes for files and content strings using async I/O.

Key functions:

- `calculate_file_hash(path: &Path) -> Result<String>`
- `calculate_content_hash(content: &str) -> String`

### Categorizer Module

Analyzes files and categorizes them for processing.

Key types:

- `FileCategory`: Skip, Update, Add, Delete
- `CategorizedFiles`: Grouped file lists
- `FileCategorizer`: Analysis engine

### Store Module

Database operations with transaction support.

Key methods:

- `query_existing_files() -> HashMap<PathBuf, String>`
- `insert_file_chunks()`
- `update_file_chunks()` (transactional)
- `delete_chunks_for_file()`
- `cleanup_deleted_files()`

### Loader Module

Orchestrates incremental loading based on configuration.

Key types:

- `LoaderConfig`: Configuration flags
- `LoadStats`: Performance metrics
- `IncrementalLoader`: Main loader engine

## Data Flow

```text
1. Discover files on disk → Calculate hashes
2. Query existing files from database
3. Categorize files (skip/update/add/delete)
4. Process based on mode:
   - Resume: Add new files only
   - Update: Add new + update modified
   - Force: Reload all
5. Return statistics
```
````

## Transaction Model

Updates use PostgreSQL transactions for atomicity:

```rust
BEGIN;
  DELETE FROM documents WHERE source_file = 'file.md';
  INSERT INTO documents VALUES (...);
COMMIT;
```

If any operation fails, the entire transaction rolls back.

## Performance Characteristics

- Hash calculation: O(n) where n = file size
- File discovery: O(m) where m = number of files
- Categorization: O(m) hash lookups
- Skip mode: Near-zero database writes
- Update mode: Writes only for changed files

## Error Handling

All operations return `Result<T, KbError>`:

- `IoError`: File system errors
- `DatabaseError`: SQL errors
- `InvalidHash`: Malformed hash data
- `ConfigError`: Invalid configuration

Errors propagate up with context, allowing graceful degradation.

````

### Verification Steps

```bash
# Run all unit tests
cargo test --package xze-core kb::

# Run integration tests
cargo test --package xze-core --test integration

# Check code coverage
cargo tarpaulin --package xze-core --out Html --output-dir coverage

# Verify coverage >80%
open coverage/index.html

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Build release
cargo build --release

# Run benchmark suite
cargo bench --package xze-core --bench kb_loader

# Verify documentation
cargo doc --package xze-core --no-deps --open

# Integration test with real database
export DATABASE_URL="postgres://localhost/xze_test"
sqlx migrate run
cargo test --package xze-core --test integration -- --nocapture

# Performance benchmark
time xze load /path/to/large/repo              # Note baseline
time xze load --resume /path/to/large/repo     # Should be >70% faster
time xze load --update /path/to/large/repo     # Should be similar to resume

# Modify some files and retest
echo "# Change" >> /path/to/large/repo/README.md
time xze load --update /path/to/large/repo     # Should process only changed files
````

### Success Criteria

- All unit tests pass with >80% code coverage
- All integration tests pass
- Benchmark shows >70% time reduction with `--resume`
- Documentation complete and comprehensive
- API docs generated successfully
- No clippy warnings or format issues
- Performance metrics documented

---

## Summary of CLI Flags

After all phases complete, XZe supports these loading modes:

```bash
# Full reload (default - original behavior)
xze load /path/to/docs

# Resume interrupted load (skip unchanged files)
xze load --resume /path/to/docs

# Update changed files (skip unchanged, reload modified, add new)
xze load --update /path/to/docs

# Update with cleanup (also remove deleted files)
xze load --update --cleanup /path/to/docs

# Dry run (show what would be done without making changes)
xze load --update --cleanup --dry-run /path/to/docs

# Force full reload (ignore existing data)
xze load --force /path/to/docs
```

---

## Implementation Timeline

Total estimated time: **20-24 days** (approximately 4-5 weeks for 1 developer)

| Phase | Description                       | Estimated Time | Dependencies |
| ----- | --------------------------------- | -------------- | ------------ |
| 1     | Hash Tracking Infrastructure      | 2-3 days       | None         |
| 2     | File Discovery and Categorization | 3-4 days       | Phase 1      |
| 3     | Implement Skip Logic (Resume)     | 3-4 days       | Phase 2      |
| 4     | Implement Update Logic            | 4-5 days       | Phase 3      |
| 5     | Implement Cleanup Logic           | 2-3 days       | Phase 4      |
| 6     | Polish CLI and UX                 | 2-3 days       | Phase 5      |
| 7     | Testing and Documentation         | 4-5 days       | All phases   |

---

## Implementation Notes

### Critical Requirements

- Each phase builds on the previous one
- All phases maintain backward compatibility
- Database schema changes are additive only (no breaking changes)
- Transaction safety is critical for update operations
- Performance improvements should be measurable after Phase 3
- Testing must cover edge cases (empty directories, permission errors, symlinks, etc.)
- All code must pass `cargo clippy -- -D warnings`
- All tests must achieve >80% code coverage
- Documentation must follow Diataxis framework

### Dependencies to Add

Add to `crates/core/Cargo.toml`:

```toml
[dependencies]
sha2 = "0.10"
walkdir = "2.4"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "migrate"] }
tokio = { version = "1.35", features = ["full"] }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"

[dev-dependencies]
tempfile = "3.8"
testcontainers = "0.15"  # For integration tests with real PostgreSQL
```

### Rust Patterns to Follow

1. **Error Handling**: Use `Result<T, KbError>` everywhere, never `unwrap()` without justification
2. **Async/Await**: All I/O operations are async with tokio
3. **Documentation**: Every public item has doc comments with examples
4. **Testing**: Each module has comprehensive unit tests in `#[cfg(test)]` blocks
5. **Transactions**: Use SQLx transactions for atomic database operations
6. **Logging**: Use `tracing` macros (`info!`, `debug!`, `warn!`, `error!`)
7. **Configuration**: Use structured types with `Default` implementations
8. **Type Safety**: Leverage Rust's type system (newtype pattern, enums, etc.)

### Integration with Existing XZe Architecture

- Wire into existing `AIDocumentationGenerator` for chunk generation
- Reuse existing database connection pooling from `xze-core`
- Integrate with existing CLI framework in `xze-cli`
- Follow existing error handling patterns throughout XZe
- Use existing configuration types from `XzeConfig`
- Respect crate boundaries (core never depends on cli/serve)

### Migration Strategy

1. **Phase 1-2**: Pure additions, no behavior change
2. **Phase 3**: Introduce `--resume` flag (opt-in)
3. **Phase 4-6**: Add more flags (all opt-in)
4. **Phase 7**: Document and stabilize
5. **Future**: Consider making `--update` the default (breaking change)

### Performance Targets

- **Resume mode**: >70% time reduction on unchanged files
- **Update mode**: Process only changed files (<10% of full load time for 1% file changes)
- **Hash calculation**: <1ms per file for typical documentation files
- **Database queries**: <100ms for existing file lookup (even with 10k+ files)
- **Memory usage**: O(n) where n = number of files (not file sizes)

### Known Limitations and Future Work

- No parallel file processing (could use rayon for hash calculation)
- No progress bars or streaming output (could add with indicatif crate)
- No persistent cache for hashes (could add SQLite cache for filesystem)
- No incremental embedding updates (requires AI service changes)
- No distributed cache support (single-node only)
- No file size limits or quotas
- No rate limiting for database operations

### Rollout Plan

1. Implement Phase 1-3 behind feature flag
2. Beta test with internal documentation
3. Gather performance metrics
4. Implement Phase 4-6
5. Write comprehensive documentation
6. Release as stable feature
7. Monitor performance and gather feedback
8. Iterate based on user needs

---

## References

- XZe Architecture: `docs/explanation/architecture.md`
- CLI Reference: `docs/reference/cli_reference.md`
- Database Schema: `migrations/`
- AGENTS.md: Project development guidelines
- Diataxis Framework: https://diataxis.fr/

---

## Appendix: Example Benchmark Results

Expected performance improvements (with 1000 files, 10% modified):

```text
Operation              Duration    Files Processed    Performance
------------------------------------------------------------------
Full Load              120s        1000              Baseline
Resume (unchanged)     5s          0                 96% faster
Update (10% modified)  15s         100               87.5% faster
Force Reload           120s        1000              Same as baseline
Dry Run                2s          0                 98% faster
```

Hash calculation overhead: ~2s for 1000 files (amortized <2ms per file)

```

```
