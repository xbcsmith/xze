//! Error types for knowledge base operations
//!
//! This module provides specialized error types for KB operations including
//! file hashing, database operations, and file categorization.

use thiserror::Error;

/// Result type alias for KB operations
pub type Result<T> = std::result::Result<T, KbError>;

/// Error types for knowledge base operations
///
/// This enum covers all possible error conditions that can occur during
/// KB operations including file I/O, database operations, hashing, and
/// validation errors.
///
/// # Examples
///
/// ```
/// use xze_core::kb::error::{KbError, Result};
///
/// fn validate_path(path: &str) -> Result<()> {
///     if path.is_empty() {
///         return Err(KbError::InvalidPath {
///             path: path.to_string(),
///             reason: "Path cannot be empty".to_string(),
///         });
///     }
///     Ok(())
/// }
///
/// # fn main() {
/// let result = validate_path("");
/// assert!(result.is_err());
/// # }
/// ```
#[derive(Error, Debug)]
pub enum KbError {
    /// I/O errors when reading or writing files
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Database operation errors
    #[error("Database error: {0}")]
    Database(String),

    /// Invalid file hash format or content
    #[error("Invalid hash: {hash} - {reason}")]
    InvalidHash { hash: String, reason: String },

    /// File not found at expected location
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    /// Configuration validation errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid file path
    #[error("Invalid path: {path} - {reason}")]
    InvalidPath { path: String, reason: String },

    /// File categorization errors
    #[error("Categorization error: {0}")]
    Categorization(String),

    /// Hash calculation errors
    #[error("Hash calculation failed for {path}: {reason}")]
    HashCalculation { path: String, reason: String },

    /// File system traversal errors
    #[error("Traversal error: {0}")]
    Traversal(String),

    /// Loader operation errors
    #[error("Loader error: {0}")]
    Loader(String),

    /// Chunk processing errors
    #[error("Chunk processing error: {0}")]
    ChunkProcessing(String),

    /// Transaction errors
    #[error("Transaction error: {0}")]
    Transaction(String),
}

impl KbError {
    /// Create a database error
    pub fn database<S: Into<String>>(message: S) -> Self {
        Self::Database(message.into())
    }

    /// Create an invalid hash error
    pub fn invalid_hash<S1: Into<String>, S2: Into<String>>(hash: S1, reason: S2) -> Self {
        Self::InvalidHash {
            hash: hash.into(),
            reason: reason.into(),
        }
    }

    /// Create a file not found error
    pub fn file_not_found<S: Into<String>>(path: S) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config(message.into())
    }

    /// Create an invalid path error
    pub fn invalid_path<S1: Into<String>, S2: Into<String>>(path: S1, reason: S2) -> Self {
        Self::InvalidPath {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a categorization error
    pub fn categorization<S: Into<String>>(message: S) -> Self {
        Self::Categorization(message.into())
    }

    /// Create a hash calculation error
    pub fn hash_calculation<S1: Into<String>, S2: Into<String>>(path: S1, reason: S2) -> Self {
        Self::HashCalculation {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a traversal error
    pub fn traversal<S: Into<String>>(message: S) -> Self {
        Self::Traversal(message.into())
    }

    /// Create a loader error
    pub fn loader<S: Into<String>>(message: S) -> Self {
        Self::Loader(message.into())
    }

    /// Create a chunk processing error
    pub fn chunk_processing<S: Into<String>>(message: S) -> Self {
        Self::ChunkProcessing(message.into())
    }

    /// Create a transaction error
    pub fn transaction<S: Into<String>>(message: S) -> Self {
        Self::Transaction(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation_database() {
        let err = KbError::database("connection failed");
        assert!(matches!(err, KbError::Database(_)));
        assert_eq!(err.to_string(), "Database error: connection failed");
    }

    #[test]
    fn test_error_creation_invalid_hash() {
        let err = KbError::invalid_hash("abc123", "too short");
        assert!(matches!(err, KbError::InvalidHash { .. }));
        assert!(err.to_string().contains("abc123"));
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn test_error_creation_file_not_found() {
        let err = KbError::file_not_found("/path/to/file.txt");
        assert!(matches!(err, KbError::FileNotFound { .. }));
        assert_eq!(err.to_string(), "File not found: /path/to/file.txt");
    }

    #[test]
    fn test_error_creation_config() {
        let err = KbError::config("invalid configuration");
        assert!(matches!(err, KbError::Config(_)));
        assert_eq!(
            err.to_string(),
            "Configuration error: invalid configuration"
        );
    }

    #[test]
    fn test_error_creation_invalid_path() {
        let err = KbError::invalid_path("", "path is empty");
        assert!(matches!(err, KbError::InvalidPath { .. }));
        assert!(err.to_string().contains("path is empty"));
    }

    #[test]
    fn test_error_creation_categorization() {
        let err = KbError::categorization("failed to categorize files");
        assert!(matches!(err, KbError::Categorization(_)));
        assert_eq!(
            err.to_string(),
            "Categorization error: failed to categorize files"
        );
    }

    #[test]
    fn test_error_creation_hash_calculation() {
        let err = KbError::hash_calculation("/path/file.txt", "permission denied");
        assert!(matches!(err, KbError::HashCalculation { .. }));
        assert!(err.to_string().contains("/path/file.txt"));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_error_creation_traversal() {
        let err = KbError::traversal("directory not accessible");
        assert!(matches!(err, KbError::Traversal(_)));
        assert_eq!(err.to_string(), "Traversal error: directory not accessible");
    }

    #[test]
    fn test_error_creation_loader() {
        let err = KbError::loader("load operation failed");
        assert!(matches!(err, KbError::Loader(_)));
        assert_eq!(err.to_string(), "Loader error: load operation failed");
    }

    #[test]
    fn test_error_creation_chunk_processing() {
        let err = KbError::chunk_processing("failed to process chunk");
        assert!(matches!(err, KbError::ChunkProcessing(_)));
        assert_eq!(
            err.to_string(),
            "Chunk processing error: failed to process chunk"
        );
    }

    #[test]
    fn test_error_creation_transaction() {
        let err = KbError::transaction("transaction rolled back");
        assert!(matches!(err, KbError::Transaction(_)));
        assert_eq!(
            err.to_string(),
            "Transaction error: transaction rolled back"
        );
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let kb_err: KbError = io_err.into();
        assert!(matches!(kb_err, KbError::Io(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_result() -> Result<i32> {
            Ok(42)
        }

        let result = returns_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_with_error() {
        fn returns_error() -> Result<()> {
            Err(KbError::config("test error"))
        }

        let result = returns_error();
        assert!(result.is_err());
    }
}
