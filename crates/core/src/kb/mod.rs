//! Knowledge base module for incremental loading operations
//!
//! This module provides infrastructure for incremental loading of documentation
//! into the knowledge base. It includes file hashing, categorization, database
//! operations, and incremental loading orchestration.
//!
//! # Architecture
//!
//! The KB module is organized into several sub-modules:
//!
//! - `hash`: File hashing utilities using SHA-256
//! - `error`: Error types for KB operations
//! - `categorizer`: File categorization (Skip, Add, Update, Delete)
//! - `store`: Database operations for file metadata and chunks
//! - `loader`: Incremental loading orchestration
//!
//! # Usage
//!
//! ```no_run
//! use xze_core::kb::{hash, error::Result};
//!
//! # async fn example() -> Result<()> {
//! // Calculate hash of a file
//! let file_hash = hash::calculate_file_hash("/path/to/file.txt").await?;
//! println!("File hash: {}", file_hash);
//!
//! // Calculate hash of content
//! let content = "Some text content";
//! let content_hash = hash::calculate_content_hash(content);
//! println!("Content hash: {}", content_hash);
//! # Ok(())
//! # }
//! ```
//!
//! # Implementation Status
//!
//! - Phase 1 (Complete): Hash tracking infrastructure
//!   - File hashing with SHA-256
//!   - Error types
//!   - Module stubs for future phases
//!
//! - Phase 2 (Planned): File discovery and categorization
//! - Phase 3 (Planned): Skip logic (resume functionality)
//! - Phase 4 (Planned): Update logic
//! - Phase 5 (Planned): Cleanup logic
//! - Phase 6 (Planned): CLI polish
//! - Phase 7 (Planned): Testing and documentation

pub mod categorizer;
pub mod error;
pub mod hash;
pub mod loader;
pub mod store;

// Re-export commonly used types
pub use categorizer::{CategorizedFiles, FileCategorizer, FileCategory};
pub use error::{KbError, Result};
pub use hash::{calculate_content_hash, calculate_file_hash, verify_hash_format};
pub use loader::{IncrementalLoader, LoadStats, LoaderConfig};
pub use store::KbStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that main types are exported
        let _: Result<()> = Ok(());
        let _config = LoaderConfig::default();
        let _stats = LoadStats::default();
    }

    #[tokio::test]
    async fn test_hash_integration() {
        // Test that hash functions work as expected
        let content = "test content";
        let hash = calculate_content_hash(content);
        assert_eq!(hash.len(), 64);

        // Verify hash format validation
        assert!(verify_hash_format(&hash).is_ok());
    }
}
