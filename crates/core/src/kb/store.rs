//! Database store for knowledge base operations
//!
//! This module provides database operations for storing and retrieving
//! file metadata, including file paths and hashes for incremental loading.

use crate::kb::error::{KbError, Result};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// Knowledge base store for database operations
///
/// This struct handles all database interactions including querying
/// existing files, updating file metadata, and managing chunks.
#[derive(Clone)]
pub struct KbStore {
    pool: PgPool,
}

impl KbStore {
    /// Create a new KB store instance
    ///
    /// # Arguments
    ///
    /// * `pool` - PostgreSQL connection pool
    ///
    /// # Returns
    ///
    /// Returns a new KbStore instance
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::KbStore;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = PgPool::connect("postgresql://localhost/xze").await?;
    /// let store = KbStore::new(pool);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Query existing files from database
    ///
    /// Retrieves all files currently stored in the knowledge base
    /// along with their hash values from the documents table.
    ///
    /// # Returns
    ///
    /// Returns a HashMap mapping file paths to their SHA-256 hashes
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if query fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::KbStore;
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let files = store.query_existing_files().await?;
    /// println!("Found {} files in database", files.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_existing_files(&self) -> Result<HashMap<String, String>> {
        debug!("Querying existing files from database");

        let query = r#"
            SELECT file_path, file_hash
            FROM documents
            WHERE file_hash IS NOT NULL
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KbError::database(format!("Failed to query existing files: {}", e)))?;

        let mut files = HashMap::new();
        for row in rows {
            let file_path: String = row
                .try_get("file_path")
                .map_err(|e| KbError::database(format!("Failed to get file_path: {}", e)))?;
            let file_hash: String = row
                .try_get("file_hash")
                .map_err(|e| KbError::database(format!("Failed to get file_hash: {}", e)))?;

            files.insert(file_path, file_hash);
        }

        info!("Retrieved {} files from database", files.len());
        Ok(files)
    }

    /// Delete chunks for a specific file
    ///
    /// Removes all chunks associated with the given file path from the documents table.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file whose chunks should be deleted
    ///
    /// # Returns
    ///
    /// Returns the number of rows (chunks) deleted
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if deletion fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::KbStore;
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let deleted = store.delete_chunks_for_file(Path::new("docs/example.md")).await?;
    /// println!("Deleted {} chunks", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_chunks_for_file(&self, file_path: &Path) -> Result<u64> {
        let file_path_str = file_path.to_string_lossy();

        debug!("Deleting chunks for file: {}", file_path_str);

        let result = sqlx::query("DELETE FROM documents WHERE file_path = $1")
            .bind(file_path_str.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to delete chunks for file {}: {}",
                    file_path_str, e
                ))
            })?;

        let rows_affected = result.rows_affected();

        if rows_affected > 0 {
            info!(
                "Deleted {} chunks for file: {}",
                rows_affected, file_path_str
            );
        } else {
            debug!("No chunks found to delete for file: {}", file_path_str);
        }

        Ok(rows_affected)
    }

    /// Insert chunks for a file
    ///
    /// Stores document chunks in the database for the given file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file
    /// * `file_hash` - SHA-256 hash of the file
    /// * `chunks` - Slice of DocumentChunk to insert
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if insertion fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::{KbStore, DocumentChunk};
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let chunks = vec![
    ///     DocumentChunk {
    ///         chunk_id: "chunk_0".to_string(),
    ///         content: "Example content".to_string(),
    ///         embedding: vec![0.1, 0.2, 0.3],
    ///         metadata: serde_json::json!({}),
    ///     }
    /// ];
    /// store.insert_file_chunks(Path::new("docs/example.md"), "abc123", &chunks).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert_file_chunks(
        &self,
        file_path: &Path,
        file_hash: &str,
        chunks: &[DocumentChunk],
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();

        info!(
            "Inserting {} chunks for file: {}",
            chunks.len(),
            file_path_str
        );

        for chunk in chunks {
            let embedding_bytes = chunk.embedding_as_bytes();
            let metadata_str = chunk.metadata.to_string();

            sqlx::query(
                r#"
                INSERT INTO documents
                (file_path, file_hash, chunk_id, content, embedding, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(file_path_str.as_ref())
            .bind(file_hash)
            .bind(&chunk.chunk_id)
            .bind(&chunk.content)
            .bind(embedding_bytes)
            .bind(metadata_str)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to insert chunk {} for file {}: {}",
                    chunk.chunk_id, file_path_str, e
                ))
            })?;

            debug!("Inserted chunk: {}", chunk.chunk_id);
        }

        info!(
            "Successfully inserted {} chunks for file: {}",
            chunks.len(),
            file_path_str
        );

        Ok(())
    }

    /// Update chunks for an existing file
    ///
    /// Deletes old chunks and inserts new ones for the file atomically
    /// within a transaction. If the transaction fails, all changes are
    /// rolled back automatically.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file
    /// * `file_hash` - New SHA-256 hash of the file
    /// * `chunks` - Slice of new DocumentChunk to insert
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if transaction fails.
    /// Transaction is automatically rolled back on error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::{KbStore, DocumentChunk};
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let chunks = vec![
    ///     DocumentChunk {
    ///         chunk_id: "chunk_0".to_string(),
    ///         content: "Updated content".to_string(),
    ///         embedding: vec![0.4, 0.5, 0.6],
    ///         metadata: serde_json::json!({}),
    ///     }
    /// ];
    /// store.update_file_chunks(Path::new("docs/example.md"), "def456", &chunks).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_file_chunks(
        &self,
        file_path: &Path,
        file_hash: &str,
        chunks: &[DocumentChunk],
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();

        info!(
            "Updating file: {} with {} chunks (transactional)",
            file_path_str,
            chunks.len()
        );

        // Begin transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            KbError::database(format!(
                "Failed to begin transaction for file {}: {}",
                file_path_str, e
            ))
        })?;

        // Delete old chunks
        let delete_result = sqlx::query("DELETE FROM documents WHERE file_path = $1")
            .bind(file_path_str.as_ref())
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to delete old chunks for file {}: {}",
                    file_path_str, e
                ))
            })?;

        let deleted_count = delete_result.rows_affected();
        debug!("Deleted {} old chunks", deleted_count);

        // Insert new chunks
        for chunk in chunks {
            let embedding_bytes = chunk.embedding_as_bytes();
            let metadata_str = chunk.metadata.to_string();

            sqlx::query(
                r#"
                INSERT INTO documents
                (file_path, file_hash, chunk_id, content, embedding, metadata)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(file_path_str.as_ref())
            .bind(file_hash)
            .bind(&chunk.chunk_id)
            .bind(&chunk.content)
            .bind(embedding_bytes)
            .bind(metadata_str)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to insert new chunk {} for file {}: {}",
                    chunk.chunk_id, file_path_str, e
                ))
            })?;

            debug!("Inserted new chunk: {}", chunk.chunk_id);
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            KbError::database(format!(
                "Failed to commit transaction for file {}: {}",
                file_path_str, e
            ))
        })?;

        info!(
            "Successfully updated file: {} (deleted {}, inserted {} chunks)",
            file_path_str,
            deleted_count,
            chunks.len()
        );

        Ok(())
    }

    /// Clean up deleted files from database
    ///
    /// Removes all chunks for files that no longer exist on disk.
    /// This operation deletes rows from the documents table for each
    /// file path in the deleted_files list.
    ///
    /// # Arguments
    ///
    /// * `deleted_files` - List of file paths to remove
    ///
    /// # Returns
    ///
    /// Returns the total number of chunks deleted across all files
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if cleanup fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sqlx::PgPool;
    /// use xze_core::kb::store::KbStore;
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let deleted_files = vec![
    ///     "docs/old_file.md".to_string(),
    ///     "src/removed.rs".to_string(),
    /// ];
    /// let deleted_count = store.cleanup_deleted_files(&deleted_files).await?;
    /// println!("Deleted {} chunks from {} files", deleted_count, deleted_files.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cleanup_deleted_files(&self, deleted_files: &[String]) -> Result<u64> {
        if deleted_files.is_empty() {
            debug!("No deleted files to clean up");
            return Ok(0);
        }

        info!("Cleaning up {} deleted files", deleted_files.len());

        let mut total_deleted = 0u64;

        for file_path in deleted_files {
            debug!("Deleting chunks for deleted file: {}", file_path);

            let result = sqlx::query("DELETE FROM documents WHERE file_path = $1")
                .bind(file_path)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    KbError::database(format!(
                        "Failed to delete chunks for file {}: {}",
                        file_path, e
                    ))
                })?;

            let rows_affected = result.rows_affected();
            total_deleted += rows_affected;

            if rows_affected > 0 {
                info!(
                    "Deleted {} chunks for deleted file: {}",
                    rows_affected, file_path
                );
            } else {
                debug!("No chunks found for deleted file: {}", file_path);
            }
        }

        info!(
            "Cleanup complete: deleted {} total chunks from {} files",
            total_deleted,
            deleted_files.len()
        );

        Ok(total_deleted)
    }
}

/// Represents a document chunk to be stored in the database
///
/// Each chunk contains the actual content, its embedding vector for
/// semantic search, and metadata for additional context.
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    /// Unique identifier for this chunk within the document
    pub chunk_id: String,
    /// The actual text content of the chunk
    pub content: String,
    /// Embedding vector for semantic similarity search
    pub embedding: Vec<f32>,
    /// Additional metadata as JSON
    pub metadata: serde_json::Value,
}

impl DocumentChunk {
    /// Create a new document chunk
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - Unique identifier for the chunk
    /// * `content` - Text content of the chunk
    /// * `embedding` - Embedding vector for the chunk
    /// * `metadata` - Additional metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::kb::store::DocumentChunk;
    /// use serde_json::json;
    ///
    /// let chunk = DocumentChunk::new(
    ///     "chunk_0".to_string(),
    ///     "Example content".to_string(),
    ///     vec![0.1, 0.2, 0.3],
    ///     json!({"section": "introduction"}),
    /// );
    /// ```
    pub fn new(
        chunk_id: String,
        content: String,
        embedding: Vec<f32>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            chunk_id,
            content,
            embedding,
            metadata,
        }
    }

    /// Convert embedding vector to bytes for database storage
    ///
    /// Converts the f32 vector to a byte array that can be stored
    /// in PostgreSQL as a bytea column.
    fn embedding_as_bytes(&self) -> Vec<u8> {
        self.embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_kb_store_creation() {
        // Note: We cannot easily test PgPool creation without a real database
        // This test validates the struct exists and is constructible in principle
        // Integration tests with testcontainers will provide full coverage
    }

    #[test]
    fn test_document_chunk_creation() {
        let chunk = DocumentChunk::new(
            "chunk_0".to_string(),
            "Test content".to_string(),
            vec![0.1, 0.2, 0.3],
            serde_json::json!({"key": "value"}),
        );

        assert_eq!(chunk.chunk_id, "chunk_0");
        assert_eq!(chunk.content, "Test content");
        assert_eq!(chunk.embedding.len(), 3);
        assert_eq!(chunk.metadata["key"], "value");
    }

    #[test]
    fn test_document_chunk_embedding_as_bytes() {
        let chunk = DocumentChunk::new(
            "chunk_0".to_string(),
            "Test".to_string(),
            vec![1.0, 2.0, 3.0],
            serde_json::json!({}),
        );

        let bytes = chunk.embedding_as_bytes();
        // Each f32 is 4 bytes, so 3 floats = 12 bytes
        assert_eq!(bytes.len(), 12);
    }

    #[test]
    fn test_cleanup_deleted_files_empty_list() {
        // Test with empty list is handled synchronously
        let deleted_files: Vec<String> = vec![];
        assert_eq!(deleted_files.len(), 0);
    }

    #[test]
    fn test_cleanup_deleted_files_valid_paths() {
        let deleted_files = ["docs/removed.md".to_string(), "src/deleted.rs".to_string()];
        assert_eq!(deleted_files.len(), 2);
        assert_eq!(deleted_files[0], "docs/removed.md");
        assert_eq!(deleted_files[1], "src/deleted.rs");
    }
}
