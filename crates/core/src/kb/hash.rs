//! File hashing utilities for knowledge base operations
//!
//! This module provides functions for calculating SHA-256 hashes of files and content.
//! Hashes are used to detect file changes and determine which files need to be
//! reprocessed during incremental loading operations.
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::kb::hash;
//!
//! # async fn example() -> xze_core::kb::error::Result<()> {
//! // Calculate hash of a file
//! let hash = hash::calculate_file_hash("/path/to/file.txt").await?;
//! println!("File hash: {}", hash);
//!
//! // Calculate hash of content
//! let content = "Hello, world!";
//! let hash = hash::calculate_content_hash(content);
//! println!("Content hash: {}", hash);
//! # Ok(())
//! # }
//! ```

use crate::kb::error::{KbError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, warn};

/// Buffer size for reading files during hash calculation
const BUFFER_SIZE: usize = 8192;

/// Calculate SHA-256 hash of a file
///
/// Reads the file in chunks to efficiently handle large files without
/// loading the entire content into memory.
///
/// # Arguments
///
/// * `path` - Path to the file to hash
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 hash string (64 characters)
///
/// # Errors
///
/// Returns `KbError::Io` if the file cannot be read
/// Returns `KbError::HashCalculation` if hash calculation fails
///
/// # Examples
///
/// ```no_run
/// use xze_core::kb::hash::calculate_file_hash;
///
/// # async fn example() -> xze_core::kb::error::Result<()> {
/// let hash = calculate_file_hash("/path/to/file.txt").await?;
/// assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
/// # Ok(())
/// # }
/// ```
pub async fn calculate_file_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    debug!("Calculating hash for file: {}", path.display());

    // Open file
    let file = File::open(path).await.map_err(|e| {
        warn!("Failed to open file for hashing: {}: {}", path.display(), e);
        KbError::hash_calculation(
            path.display().to_string(),
            format!("Failed to open file: {}", e),
        )
    })?;

    // Create buffered reader for efficient I/O
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    // Read file in chunks and update hash
    loop {
        let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
            warn!(
                "Failed to read file during hashing: {}: {}",
                path.display(),
                e
            );
            KbError::hash_calculation(
                path.display().to_string(),
                format!("Failed to read file: {}", e),
            )
        })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize hash and convert to hex string
    let result = hasher.finalize();
    let hash_string = format!("{:x}", result);

    debug!("Calculated hash for {}: {}", path.display(), hash_string);

    Ok(hash_string)
}

/// Calculate SHA-256 hash of string content
///
/// Calculates the hash of the given string content. This is useful for
/// hashing in-memory content or verifying content matches a known hash.
///
/// # Arguments
///
/// * `content` - String content to hash
///
/// # Returns
///
/// Returns a hex-encoded SHA-256 hash string (64 characters)
///
/// # Examples
///
/// ```
/// use xze_core::kb::hash::calculate_content_hash;
///
/// let content = "Hello, world!";
/// let hash = calculate_content_hash(content);
/// assert_eq!(hash.len(), 64);
///
/// // Same content produces same hash
/// let hash2 = calculate_content_hash(content);
/// assert_eq!(hash, hash2);
/// ```
pub fn calculate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Verify that a hash string is valid SHA-256 format
///
/// Checks if the provided string is a valid hex-encoded SHA-256 hash
/// (64 hexadecimal characters).
///
/// # Arguments
///
/// * `hash` - Hash string to validate
///
/// # Returns
///
/// Returns `Ok(())` if the hash is valid
///
/// # Errors
///
/// Returns `KbError::InvalidHash` if the hash format is invalid
///
/// # Examples
///
/// ```
/// use xze_core::kb::hash::verify_hash_format;
///
/// // Valid hash (64 hex characters)
/// let valid = "a".repeat(64);
/// assert!(verify_hash_format(&valid).is_ok());
///
/// // Invalid hash (too short)
/// let invalid = "abc123";
/// assert!(verify_hash_format(&invalid).is_err());
/// ```
pub fn verify_hash_format(hash: &str) -> Result<()> {
    if hash.len() != 64 {
        return Err(KbError::invalid_hash(
            hash,
            format!("Expected 64 characters, got {}", hash.len()),
        ));
    }

    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(KbError::invalid_hash(
            hash,
            "Hash contains non-hexadecimal characters",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_calculate_file_hash_success() {
        // Create temporary file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"test content";
        std::io::Write::write_all(&mut temp_file, content).unwrap();

        let hash = calculate_file_hash(temp_file.path()).await.unwrap();

        // Verify hash format
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_calculate_file_hash_nonexistent() {
        let result = calculate_file_hash("/nonexistent/file.txt").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            KbError::HashCalculation { .. }
        ));
    }

    #[tokio::test]
    async fn test_calculate_file_hash_consistent() {
        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"consistent content";
        std::io::Write::write_all(&mut temp_file, content).unwrap();

        // Calculate hash twice
        let hash1 = calculate_file_hash(temp_file.path()).await.unwrap();
        let hash2 = calculate_file_hash(temp_file.path()).await.unwrap();

        // Hashes should be identical
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_calculate_file_hash_different_content() {
        // Create two files with different content
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();

        std::io::Write::write_all(&mut file1, b"content 1").unwrap();
        std::io::Write::write_all(&mut file2, b"content 2").unwrap();

        let hash1 = calculate_file_hash(file1.path()).await.unwrap();
        let hash2 = calculate_file_hash(file2.path()).await.unwrap();

        // Hashes should be different
        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_calculate_file_hash_large_file() {
        // Create temporary file with large content (larger than buffer)
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = File::create(temp_file.path()).await.unwrap();

        // Write 1MB of data
        let chunk = vec![0u8; 1024];
        for _ in 0..1024 {
            file.write_all(&chunk).await.unwrap();
        }
        file.flush().await.unwrap();
        drop(file);

        let hash = calculate_file_hash(temp_file.path()).await.unwrap();

        // Verify hash format
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_calculate_file_hash_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let hash = calculate_file_hash(temp_file.path()).await.unwrap();

        // Empty file should still produce valid hash
        assert_eq!(hash.len(), 64);

        // Verify it matches known empty file hash
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_calculate_content_hash_consistent() {
        let content = "test content";
        let hash1 = calculate_content_hash(content);
        let hash2 = calculate_content_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_calculate_content_hash_different_content() {
        let hash1 = calculate_content_hash("content 1");
        let hash2 = calculate_content_hash("content 2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_content_hash_empty_string() {
        let hash = calculate_content_hash("");

        // Verify it matches known empty string hash
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_calculate_content_hash_unicode() {
        let content = "Hello, 世界! 🌍";
        let hash = calculate_content_hash(content);

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_verify_hash_format_valid() {
        let valid_hash = "a".repeat(64);
        assert!(verify_hash_format(&valid_hash).is_ok());

        let valid_hash = "0123456789abcdef".repeat(4);
        assert!(verify_hash_format(&valid_hash).is_ok());
    }

    #[test]
    fn test_verify_hash_format_invalid_length() {
        let too_short = "abc123";
        let result = verify_hash_format(too_short);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KbError::InvalidHash { .. }));

        let too_long = "a".repeat(65);
        let result = verify_hash_format(&too_long);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_hash_format_invalid_characters() {
        let invalid = "g".repeat(64); // 'g' is not a hex digit
        let result = verify_hash_format(&invalid);
        assert!(result.is_err());

        let invalid = "x".repeat(32) + &"a".repeat(32);
        let result = verify_hash_format(&invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_hash_format_uppercase() {
        // Uppercase hex digits should be valid
        let uppercase = "A".repeat(64);
        assert!(verify_hash_format(&uppercase).is_ok());

        let mixed = "aAbBcCdDeEfF".repeat(5) + "aAbB";
        assert!(verify_hash_format(&mixed).is_ok());
    }
}
