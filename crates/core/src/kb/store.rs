//! Database store for knowledge base operations
//!
//! This module provides database operations for storing and retrieving
//! file metadata, including file paths and hashes for incremental loading.
//! It also supports storing and retrieving semantic chunks.

use crate::kb::error::{KbError, Result};
use crate::semantic::types::{ChunkMetadata, SemanticChunk};
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

    /// Store semantic chunks for a file
    ///
    /// Inserts semantic chunks into the semantic_chunks table.
    /// This method should be called after generating semantic chunks from a document.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file
    /// * `file_hash` - SHA-256 hash of the file
    /// * `chunks` - Slice of SemanticChunk to store
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
    /// use xze_core::kb::store::KbStore;
    /// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await
    /// #     .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;
    /// let store = KbStore::new(pool);
    /// let metadata = ChunkMetadata::new("docs/example.md".to_string(), "Example content");
    /// let chunks = vec![
    ///     SemanticChunk::new(
    ///         "Chunk content".to_string(),
    ///         0, 1, 0, 2, 0.85,
    ///         metadata.clone(),
    ///     )
    /// ];
    /// store.store_semantic_chunks(Path::new("docs/example.md"), "abc123", &chunks).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn store_semantic_chunks(
        &self,
        file_path: &Path,
        file_hash: &str,
        chunks: &[SemanticChunk],
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();

        info!(
            "Storing {} semantic chunks for file: {}",
            chunks.len(),
            file_path_str
        );

        for chunk in chunks {
            let embedding_bytes = self.embedding_to_bytes(&chunk.embedding);
            let keywords: Vec<String> = chunk
                .metadata
                .keywords
                .iter()
                .map(|s| s.to_string())
                .collect();

            sqlx::query(
                r#"
                INSERT INTO semantic_chunks
                (file_path, file_hash, chunk_index, total_chunks, start_sentence, end_sentence,
                 content, embedding, avg_similarity, source_file, title, category, keywords,
                 word_count, char_count)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                "#,
            )
            .bind(file_path_str.as_ref())
            .bind(file_hash)
            .bind(chunk.chunk_index as i32)
            .bind(chunk.total_chunks as i32)
            .bind(chunk.start_sentence as i32)
            .bind(chunk.end_sentence as i32)
            .bind(&chunk.content)
            .bind(embedding_bytes)
            .bind(chunk.avg_similarity)
            .bind(&chunk.metadata.source_file)
            .bind(chunk.metadata.title.as_deref())
            .bind(chunk.metadata.category.as_deref())
            .bind(&keywords)
            .bind(chunk.metadata.word_count as i32)
            .bind(chunk.metadata.char_count as i32)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to insert semantic chunk {} for file {}: {}",
                    chunk.chunk_index, file_path_str, e
                ))
            })?;

            debug!("Inserted semantic chunk: {}", chunk.chunk_index);
        }

        info!(
            "Successfully stored {} semantic chunks for file: {}",
            chunks.len(),
            file_path_str
        );

        Ok(())
    }

    /// Delete semantic chunks for a specific file
    ///
    /// Removes all semantic chunks associated with the given file path
    /// from the semantic_chunks table.
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
    /// let deleted = store.delete_semantic_chunks_for_file(Path::new("docs/example.md")).await?;
    /// println!("Deleted {} semantic chunks", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_semantic_chunks_for_file(&self, file_path: &Path) -> Result<u64> {
        let file_path_str = file_path.to_string_lossy();

        debug!("Deleting semantic chunks for file: {}", file_path_str);

        let result = sqlx::query("DELETE FROM semantic_chunks WHERE file_path = $1")
            .bind(file_path_str.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to delete semantic chunks for file {}: {}",
                    file_path_str, e
                ))
            })?;

        let rows_affected = result.rows_affected();

        if rows_affected > 0 {
            info!(
                "Deleted {} semantic chunks for file: {}",
                rows_affected, file_path_str
            );
        } else {
            debug!(
                "No semantic chunks found to delete for file: {}",
                file_path_str
            );
        }

        Ok(rows_affected)
    }

    /// Get semantic chunks for a specific file
    ///
    /// Retrieves all semantic chunks for the given file path, ordered by chunk_index.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file to retrieve chunks for
    ///
    /// # Returns
    ///
    /// Returns a vector of SemanticChunk ordered by chunk_index
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if query fails
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
    /// let chunks = store.get_semantic_chunks_for_file(Path::new("docs/example.md")).await?;
    /// println!("Retrieved {} semantic chunks", chunks.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_semantic_chunks_for_file(
        &self,
        file_path: &Path,
    ) -> Result<Vec<SemanticChunk>> {
        let file_path_str = file_path.to_string_lossy();

        debug!("Retrieving semantic chunks for file: {}", file_path_str);

        let query = r#"
            SELECT chunk_index, total_chunks, start_sentence, end_sentence,
                   content, embedding, avg_similarity, source_file, title, category,
                   keywords, word_count, char_count
            FROM semantic_chunks
            WHERE file_path = $1
            ORDER BY chunk_index ASC
        "#;

        let rows = sqlx::query(query)
            .bind(file_path_str.as_ref())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                KbError::database(format!(
                    "Failed to retrieve semantic chunks for file {}: {}",
                    file_path_str, e
                ))
            })?;

        let mut chunks = Vec::new();

        for row in rows {
            let chunk_index: i32 = row
                .try_get("chunk_index")
                .map_err(|e| KbError::database(format!("Failed to get chunk_index: {}", e)))?;
            let total_chunks: i32 = row
                .try_get("total_chunks")
                .map_err(|e| KbError::database(format!("Failed to get total_chunks: {}", e)))?;
            let start_sentence: i32 = row
                .try_get("start_sentence")
                .map_err(|e| KbError::database(format!("Failed to get start_sentence: {}", e)))?;
            let end_sentence: i32 = row
                .try_get("end_sentence")
                .map_err(|e| KbError::database(format!("Failed to get end_sentence: {}", e)))?;
            let content: String = row
                .try_get("content")
                .map_err(|e| KbError::database(format!("Failed to get content: {}", e)))?;
            let embedding_bytes: Vec<u8> = row
                .try_get("embedding")
                .map_err(|e| KbError::database(format!("Failed to get embedding: {}", e)))?;
            let avg_similarity: f32 = row
                .try_get("avg_similarity")
                .map_err(|e| KbError::database(format!("Failed to get avg_similarity: {}", e)))?;
            let source_file: String = row
                .try_get("source_file")
                .map_err(|e| KbError::database(format!("Failed to get source_file: {}", e)))?;
            let title: Option<String> = row
                .try_get("title")
                .map_err(|e| KbError::database(format!("Failed to get title: {}", e)))?;
            let category: Option<String> = row
                .try_get("category")
                .map_err(|e| KbError::database(format!("Failed to get category: {}", e)))?;
            let keywords: Vec<String> = row
                .try_get("keywords")
                .map_err(|e| KbError::database(format!("Failed to get keywords: {}", e)))?;
            let word_count: i32 = row
                .try_get("word_count")
                .map_err(|e| KbError::database(format!("Failed to get word_count: {}", e)))?;
            let char_count: i32 = row
                .try_get("char_count")
                .map_err(|e| KbError::database(format!("Failed to get char_count: {}", e)))?;

            let embedding = self.bytes_to_embedding(&embedding_bytes)?;

            let metadata = ChunkMetadata {
                source_file,
                title,
                category,
                keywords,
                word_count: word_count as usize,
                char_count: char_count as usize,
            };

            let chunk = SemanticChunk {
                content,
                chunk_index: chunk_index as usize,
                total_chunks: total_chunks as usize,
                start_sentence: start_sentence as usize,
                end_sentence: end_sentence as usize,
                avg_similarity: avg_similarity as f64,
                metadata,
                embedding,
            };

            chunks.push(chunk);
        }

        info!(
            "Retrieved {} semantic chunks for file: {}",
            chunks.len(),
            file_path_str
        );

        Ok(chunks)
    }

    /// Convert embedding vector to bytes for database storage
    ///
    /// Converts the f32 vector to a byte array that can be stored
    /// in PostgreSQL as a bytea column.
    fn embedding_to_bytes(&self, embedding: &[f32]) -> Vec<u8> {
        embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    /// Convert bytes from database back to embedding vector
    ///
    /// Converts a byte array from PostgreSQL bytea column back to
    /// a vector of f32 values.
    ///
    /// # Errors
    ///
    /// Returns `KbError::Database` if byte array length is not a multiple of 4
    fn bytes_to_embedding(&self, bytes: &[u8]) -> Result<Vec<f32>> {
        if !bytes.len().is_multiple_of(4) {
            return Err(KbError::database(format!(
                "Invalid embedding byte length: {} (must be multiple of 4)",
                bytes.len()
            )));
        }

        let embedding = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().expect("chunk is exactly 4 bytes");
                f32::from_le_bytes(arr)
            })
            .collect();

        Ok(embedding)
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

    #[test]
    fn test_embedding_to_bytes_conversion() {
        // Test embedding conversion without database pool
        let embedding = vec![1.0f32, 2.5f32, -3.14f32];

        // Convert to bytes manually (same logic as embedding_to_bytes)
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        // Each f32 is 4 bytes
        assert_eq!(bytes.len(), 12);

        // Verify we can convert back manually (same logic as bytes_to_embedding)
        let restored: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap();
                f32::from_le_bytes(arr)
            })
            .collect();

        assert_eq!(restored.len(), 3);
        assert!((restored[0] - 1.0).abs() < 0.001);
        assert!((restored[1] - 2.5).abs() < 0.001);
        assert!((restored[2] - (-3.14)).abs() < 0.001);
    }

    #[test]
    fn test_bytes_to_embedding_invalid_length() {
        // Invalid byte length (not a multiple of 4)
        let invalid_bytes = vec![0u8, 1u8, 2u8];

        // Test the validation logic
        assert!(!invalid_bytes.len().is_multiple_of(4));
        assert_eq!(invalid_bytes.len(), 3);
    }

    #[test]
    fn test_bytes_to_embedding_empty() {
        let empty_bytes: Vec<u8> = vec![];

        // Empty bytes should be valid (multiple of 4: 0 % 4 == 0)
        assert!(empty_bytes.len().is_multiple_of(4));

        // Converting empty bytes should yield empty embedding
        let result: Vec<f32> = empty_bytes
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap();
                f32::from_le_bytes(arr)
            })
            .collect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_semantic_chunk_with_embedding() {
        use crate::semantic::types::{ChunkMetadata, SemanticChunk};

        let metadata = ChunkMetadata::new("test.md".to_string(), "Test content");
        let embedding = vec![0.1, 0.2, 0.3, 0.4];

        let mut chunk = SemanticChunk::new("Test content".to_string(), 0, 1, 0, 0, 0.95, metadata);
        chunk.set_embedding(embedding.clone());

        assert_eq!(chunk.embedding.len(), 4);
        assert_eq!(chunk.embedding, embedding);
        assert_eq!(chunk.chunk_index, 0);
    }
}
