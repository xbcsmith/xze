// SPDX-License-Identifier: MIT OR Apache-2.0
//! PostgreSQL storage with pgvector support

use super::models::{DiataxisType, Document};
use pgvector::Vector;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Storage errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    VectorDimensionMismatch { expected: usize, actual: usize },

    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// PostgreSQL storage for documents with vector embeddings
#[derive(Clone)]
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage instance
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::storage::PostgresStorage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::PostgresStorage;
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// storage.migrate().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Database(sqlx::Error::Migrate(Box::new(e))))
    }

    /// Insert a document chunk
    ///
    /// # Arguments
    ///
    /// * `document` - Document to insert
    ///
    /// # Errors
    ///
    /// Returns `StorageError::VectorDimensionMismatch` if embedding dimension is not 1536
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::{PostgresStorage, Document, DiataxisType};
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let doc = Document::new("test.md".into(), "content".into(), 0, 1)
    ///     .with_embedding(vec![0.1; 1536])
    ///     .with_diataxis_type(DiataxisType::Tutorial);
    /// storage.insert(doc).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert(&self, document: Document) -> Result<Uuid, StorageError> {
        let embedding = document.embedding.as_ref().ok_or_else(|| {
            StorageError::InvalidConfig("Document must have an embedding".to_string())
        })?;

        if embedding.len() != 1536 {
            return Err(StorageError::VectorDimensionMismatch {
                expected: 1536,
                actual: embedding.len(),
            });
        }

        let embedding_vector = Vector::from(embedding.clone());
        let diataxis_type_str = document.diataxis_type.map(|t| t.to_string());

        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO documents (
                source_file, content, embedding, chunk_index, total_chunks,
                diataxis_type, chunk_strategy, title, category, code_blocks
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(&document.source_file)
        .bind(&document.content)
        .bind(embedding_vector)
        .bind(document.chunk_index)
        .bind(document.total_chunks)
        .bind(diataxis_type_str)
        .bind(&document.chunk_strategy)
        .bind(&document.title)
        .bind(&document.category)
        .bind(document.code_blocks)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Search documents by vector similarity
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - Query vector
    /// * `limit` - Maximum number of results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::PostgresStorage;
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let query = vec![0.1; 1536];
    /// let results = storage.search_by_vector(&query, 10).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_by_vector(
        &self,
        query_embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>, StorageError> {
        if query_embedding.len() != 1536 {
            return Err(StorageError::VectorDimensionMismatch {
                expected: 1536,
                actual: query_embedding.len(),
            });
        }

        let query_vector = Vector::from(query_embedding.to_vec());

        let rows = sqlx::query(
            r#"
            SELECT id, source_file, content, chunk_index, total_chunks,
                   diataxis_type, chunk_strategy, title, category, code_blocks,
                   created_at, updated_at,
                   embedding <=> $1 AS distance
            FROM documents
            ORDER BY embedding <=> $1
            LIMIT $2
            "#,
        )
        .bind(query_vector)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut documents = Vec::new();
        for row in rows {
            let diataxis_type: Option<String> = row.try_get("diataxis_type")?;
            let diataxis_type = diataxis_type.and_then(|s| s.parse::<DiataxisType>().ok());

            documents.push(Document {
                id: row.try_get("id")?,
                source_file: row.try_get("source_file")?,
                content: row.try_get("content")?,
                embedding: None,
                chunk_index: row.try_get("chunk_index")?,
                total_chunks: row.try_get("total_chunks")?,
                diataxis_type,
                chunk_strategy: row.try_get("chunk_strategy")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                keywords: Vec::new(),
                code_blocks: row.try_get("code_blocks")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(documents)
    }

    /// Get documents by Diataxis type
    ///
    /// # Arguments
    ///
    /// * `diataxis_type` - Type to filter by
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::{PostgresStorage, DiataxisType};
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let tutorials = storage.get_by_diataxis_type(DiataxisType::Tutorial).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_by_diataxis_type(
        &self,
        diataxis_type: DiataxisType,
    ) -> Result<Vec<Document>, StorageError> {
        let type_str = diataxis_type.to_string();

        let rows = sqlx::query(
            r#"
            SELECT id, source_file, content, chunk_index, total_chunks,
                   diataxis_type, chunk_strategy, title, category, code_blocks,
                   created_at, updated_at
            FROM documents
            WHERE diataxis_type = $1
            ORDER BY source_file, chunk_index
            "#,
        )
        .bind(type_str)
        .fetch_all(&self.pool)
        .await?;

        let mut documents = Vec::new();
        for row in rows {
            let diataxis_type: Option<String> = row.try_get("diataxis_type")?;
            let diataxis_type = diataxis_type.and_then(|s| s.parse::<DiataxisType>().ok());

            documents.push(Document {
                id: row.try_get("id")?,
                source_file: row.try_get("source_file")?,
                content: row.try_get("content")?,
                embedding: None,
                chunk_index: row.try_get("chunk_index")?,
                total_chunks: row.try_get("total_chunks")?,
                diataxis_type,
                chunk_strategy: row.try_get("chunk_strategy")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                keywords: Vec::new(),
                code_blocks: row.try_get("code_blocks")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(documents)
    }

    /// Perform hybrid search using Reciprocal Rank Fusion (RRF)
    ///
    /// Combines vector similarity search with full-text search (BM25)
    ///
    /// # Arguments
    ///
    /// * `query_text` - Text query for full-text search
    /// * `query_embedding` - Vector query for similarity search
    /// * `limit` - Maximum number of results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::PostgresStorage;
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let query = "how to install";
    /// let embedding = vec![0.1; 1536];
    /// let results = storage.search_hybrid(query, &embedding, 10).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_hybrid(
        &self,
        query_text: &str,
        query_embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>, StorageError> {
        if query_embedding.len() != 1536 {
            return Err(StorageError::VectorDimensionMismatch {
                expected: 1536,
                actual: query_embedding.len(),
            });
        }

        let query_vector = Vector::from(query_embedding.to_vec());
        // Fetch more candidates for fusion to work effectively
        let candidate_limit = limit * 2;

        let rows = sqlx::query(
            r#"
            WITH vector_results AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY embedding <=> $1) as vec_rank
                FROM documents
                ORDER BY embedding <=> $1
                LIMIT $2
            ),
            text_results AS (
                SELECT id, ROW_NUMBER() OVER (ORDER BY ts_rank(to_tsvector('english', content), plainto_tsquery('english', $3)) DESC) as text_rank
                FROM documents
                WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $3)
                LIMIT $2
            )
            SELECT d.id, d.source_file, d.content, d.chunk_index, d.total_chunks,
                   d.diataxis_type, d.chunk_strategy, d.title, d.category, d.code_blocks,
                   d.created_at, d.updated_at,
                   COALESCE(1.0 / (60 + v.vec_rank), 0.0) + COALESCE(1.0 / (60 + t.text_rank), 0.0) as score
            FROM documents d
            LEFT JOIN vector_results v ON d.id = v.id
            LEFT JOIN text_results t ON d.id = t.id
            WHERE v.id IS NOT NULL OR t.id IS NOT NULL
            ORDER BY score DESC
            LIMIT $4
            "#,
        )
        .bind(query_vector)
        .bind(candidate_limit)
        .bind(query_text)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut documents = Vec::new();
        for row in rows {
            let diataxis_type: Option<String> = row.try_get("diataxis_type")?;
            let diataxis_type = diataxis_type.and_then(|s| s.parse::<DiataxisType>().ok());

            documents.push(Document {
                id: row.try_get("id")?,
                source_file: row.try_get("source_file")?,
                content: row.try_get("content")?,
                embedding: None, // We don't return the embedding to save bandwidth
                chunk_index: row.try_get("chunk_index")?,
                total_chunks: row.try_get("total_chunks")?,
                diataxis_type,
                chunk_strategy: row.try_get("chunk_strategy")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                keywords: Vec::new(),
                code_blocks: row.try_get("code_blocks")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(documents)
    }

    /// Get surrounding chunks for context expansion
    ///
    /// # Arguments
    ///
    /// * `source_file` - Source file path
    /// * `chunk_index` - Center chunk index
    /// * `window` - Number of chunks before and after to fetch
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::PostgresStorage;
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let chunks = storage.get_surrounding_chunks("test.md", 5, 1).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_surrounding_chunks(
        &self,
        source_file: &str,
        chunk_index: i32,
        window: i32,
    ) -> Result<Vec<Document>, StorageError> {
        let start_index = (chunk_index - window).max(0);
        let end_index = chunk_index + window;

        let rows = sqlx::query(
            r#"
            SELECT id, source_file, content, chunk_index, total_chunks,
                   diataxis_type, chunk_strategy, title, category, code_blocks,
                   created_at, updated_at
            FROM documents
            WHERE source_file = $1 AND chunk_index BETWEEN $2 AND $3
            ORDER BY chunk_index
            "#,
        )
        .bind(source_file)
        .bind(start_index)
        .bind(end_index)
        .fetch_all(&self.pool)
        .await?;

        let mut documents = Vec::new();
        for row in rows {
            let diataxis_type: Option<String> = row.try_get("diataxis_type")?;
            let diataxis_type = diataxis_type.and_then(|s| s.parse::<DiataxisType>().ok());

            documents.push(Document {
                id: row.try_get("id")?,
                source_file: row.try_get("source_file")?,
                content: row.try_get("content")?,
                embedding: None,
                chunk_index: row.try_get("chunk_index")?,
                total_chunks: row.try_get("total_chunks")?,
                diataxis_type,
                chunk_strategy: row.try_get("chunk_strategy")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                keywords: Vec::new(),
                code_blocks: row.try_get("code_blocks")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(documents)
    }

    /// Delete all documents from a source file
    ///
    /// # Arguments
    ///
    /// * `source_file` - Source file path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use xze_core::storage::PostgresStorage;
    /// # let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// storage.delete_by_source_file("test.md").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_by_source_file(&self, source_file: &str) -> Result<u64, StorageError> {
        let result = sqlx::query("DELETE FROM documents WHERE source_file = $1")
            .bind(source_file)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_builder() {
        let doc = Document::new("test.md".to_string(), "content".to_string(), 0, 1)
            .with_embedding(vec![0.1; 1536])
            .with_diataxis_type(DiataxisType::Tutorial);

        assert_eq!(doc.source_file, "test.md");
        assert_eq!(doc.diataxis_type, Some(DiataxisType::Tutorial));
        assert_eq!(doc.embedding.as_ref().unwrap().len(), 1536);
    }
}
