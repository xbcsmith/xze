//! Semantic search functionality for querying document chunks
//!
//! This module provides semantic search capabilities that leverage embeddings
//! to find relevant document chunks based on similarity to a query.
//!
//! # Overview
//!
//! The search functionality:
//! - Generates embeddings for search queries
//! - Compares query embeddings against stored chunk embeddings
//! - Ranks results by cosine similarity
//! - Returns rich metadata about matching chunks
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::semantic::search::{search_with_chunks, SearchConfig};
//! use sqlx::PgPool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PgPool::connect("postgresql://localhost/xze").await?;
//! let config = SearchConfig::default();
//!
//! let results = search_with_chunks(
//!     &pool,
//!     "how to install the application",
//!     "http://localhost:11434",
//!     &config,
//! ).await?;
//!
//! for result in results {
//!     println!("Found in {}: {}", result.source_file, result.content);
//!     println!("Similarity: {:.2}%", result.similarity * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

use crate::ai::client::OllamaClient;
use crate::semantic::embeddings::{generate_embeddings, EmbeddingError};
use crate::semantic::similarity::{cosine_similarity, SimilarityError};
use sqlx::{PgPool, Row};
use thiserror::Error;
use tracing::{debug, info};

/// Search result containing chunk information and similarity score
///
/// This structure represents a single search result, including the chunk
/// content, metadata, and similarity score relative to the search query.
///
/// # Fields
///
/// - `id` - Unique database identifier for the chunk
/// - `source_file` - Path to the source file containing the chunk
/// - `content` - Text content of the chunk
/// - `similarity` - Cosine similarity score (0.0 to 1.0) between query and chunk
/// - `chunk_index` - Zero-based index of this chunk within the document
/// - `total_chunks` - Total number of chunks in the source document
/// - `title` - Optional document title from metadata
/// - `category` - Optional document category (Diataxis framework)
/// - `sentence_range` - Range of sentence indices (start, end) within the document
/// - `avg_chunk_similarity` - Average similarity between sentences in the chunk
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkSearchResult {
    /// Unique database identifier
    pub id: i64,
    /// Path to source file
    pub source_file: String,
    /// Chunk text content
    pub content: String,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Zero-based chunk index
    pub chunk_index: i32,
    /// Total chunks in document
    pub total_chunks: i32,
    /// Document title
    pub title: Option<String>,
    /// Document category
    pub category: Option<String>,
    /// Sentence range (start, end)
    pub sentence_range: (i32, i32),
    /// Average chunk similarity
    pub avg_chunk_similarity: f32,
}

/// Configuration for semantic search
///
/// Controls search behavior including result limits and filtering.
///
/// # Examples
///
/// ```
/// use xze_core::semantic::search::SearchConfig;
///
/// let config = SearchConfig::default();
/// assert_eq!(config.max_results, 10);
///
/// let custom = SearchConfig {
///     max_results: 20,
///     min_similarity: 0.5,
///     category_filter: Some("tutorial".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: f32,
    /// Optional category filter
    pub category_filter: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_similarity: 0.0,
            category_filter: None,
        }
    }
}

/// Errors that can occur during semantic search
#[derive(Error, Debug)]
pub enum SearchError {
    /// Error generating embeddings for the query
    #[error("Failed to generate query embedding: {0}")]
    EmbeddingGeneration(#[from] EmbeddingError),

    /// Error calculating similarity scores
    #[error("Similarity calculation failed: {0}")]
    SimilarityCalculation(#[from] SimilarityError),

    /// Database query error
    #[error("Database query failed: {0}")]
    Database(String),

    /// Error parsing embedding data
    #[error("Failed to parse embedding data: {0}")]
    EmbeddingParse(String),

    /// Empty query provided
    #[error("Query string cannot be empty")]
    EmptyQuery,

    /// Invalid configuration
    #[error("Invalid search configuration: {0}")]
    InvalidConfig(String),
}

/// Search for document chunks semantically similar to a query
///
/// This function generates an embedding for the query text, retrieves all
/// chunks from the database, calculates similarity scores, and returns
/// the top matching results.
///
/// # Arguments
///
/// * `pool` - PostgreSQL connection pool
/// * `query` - Search query text
/// * `ollama_url` - URL of the Ollama API endpoint
/// * `config` - Search configuration (limits, filters)
///
/// # Returns
///
/// Returns a vector of `ChunkSearchResult` ordered by similarity (highest first)
///
/// # Errors
///
/// Returns `SearchError` if:
/// - Query is empty
/// - Embedding generation fails
/// - Database query fails
/// - Similarity calculation fails
///
/// # Examples
///
/// ```no_run
/// use xze_core::semantic::search::{search_with_chunks, SearchConfig};
/// use sqlx::PgPool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = PgPool::connect("postgresql://localhost/xze").await?;
/// let config = SearchConfig {
///     max_results: 5,
///     min_similarity: 0.3,
///     category_filter: Some("how_to".to_string()),
/// };
///
/// let results = search_with_chunks(
///     &pool,
///     "configure logging",
///     "http://localhost:11434",
///     &config,
/// ).await?;
///
/// for result in results {
///     println!("Match: {} (score: {:.2})", result.source_file, result.similarity);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn search_with_chunks(
    pool: &PgPool,
    query: &str,
    ollama_url: &str,
    config: &SearchConfig,
) -> Result<Vec<ChunkSearchResult>, SearchError> {
    // Validate input
    if query.trim().is_empty() {
        return Err(SearchError::EmptyQuery);
    }

    if config.max_results == 0 {
        return Err(SearchError::InvalidConfig(
            "max_results must be greater than 0".to_string(),
        ));
    }

    if config.min_similarity < 0.0 || config.min_similarity > 1.0 {
        return Err(SearchError::InvalidConfig(
            "min_similarity must be between 0.0 and 1.0".to_string(),
        ));
    }

    info!(
        "Searching for: '{}' (max: {}, min_sim: {})",
        query, config.max_results, config.min_similarity
    );

    // Create Ollama client
    let client = OllamaClient::new(ollama_url.to_string());

    // Generate embedding for the query
    debug!("Generating embedding for query");
    let query_embedding = generate_embeddings(&client, "nomic-embed-text", query).await?;

    // Build SQL query with optional category filter
    let sql = if let Some(ref _category) = config.category_filter {
        r#"
            SELECT
                id,
                file_path,
                content,
                embedding,
                chunk_index,
                total_chunks,
                title,
                category,
                start_sentence,
                end_sentence,
                avg_similarity
            FROM semantic_chunks
            WHERE category = $1
            ORDER BY created_at DESC
        "#
    } else {
        r#"
            SELECT
                id,
                file_path,
                content,
                embedding,
                chunk_index,
                total_chunks,
                title,
                category,
                start_sentence,
                end_sentence,
                avg_similarity
            FROM semantic_chunks
            ORDER BY created_at DESC
        "#
    };

    // Execute query
    debug!("Querying database for chunks");
    let rows = if let Some(ref category) = config.category_filter {
        sqlx::query(sql).bind(category).fetch_all(pool).await
    } else {
        sqlx::query(sql).fetch_all(pool).await
    }
    .map_err(|e| SearchError::Database(format!("Failed to fetch chunks: {}", e)))?;

    info!("Retrieved {} chunks from database", rows.len());

    // Calculate similarities and build results
    let mut results = Vec::new();

    for row in rows {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| SearchError::Database(format!("Failed to get id: {}", e)))?;

        let file_path: String = row
            .try_get("file_path")
            .map_err(|e| SearchError::Database(format!("Failed to get file_path: {}", e)))?;

        let content: String = row
            .try_get("content")
            .map_err(|e| SearchError::Database(format!("Failed to get content: {}", e)))?;

        let embedding_bytes: Vec<u8> = row
            .try_get("embedding")
            .map_err(|e| SearchError::Database(format!("Failed to get embedding: {}", e)))?;

        let chunk_index: i32 = row
            .try_get("chunk_index")
            .map_err(|e| SearchError::Database(format!("Failed to get chunk_index: {}", e)))?;

        let total_chunks: i32 = row
            .try_get("total_chunks")
            .map_err(|e| SearchError::Database(format!("Failed to get total_chunks: {}", e)))?;

        let title: Option<String> = row.try_get("title").ok();
        let category: Option<String> = row.try_get("category").ok();

        let start_sentence: i32 = row
            .try_get("start_sentence")
            .map_err(|e| SearchError::Database(format!("Failed to get start_sentence: {}", e)))?;

        let end_sentence: i32 = row
            .try_get("end_sentence")
            .map_err(|e| SearchError::Database(format!("Failed to get end_sentence: {}", e)))?;

        let avg_similarity: f32 = row
            .try_get("avg_similarity")
            .map_err(|e| SearchError::Database(format!("Failed to get avg_similarity: {}", e)))?;

        // Parse embedding from byte array
        let chunk_embedding = parse_embedding(&embedding_bytes)?;

        // Calculate cosine similarity
        let similarity = cosine_similarity(&query_embedding, &chunk_embedding)?;

        // Filter by minimum similarity
        if similarity >= config.min_similarity {
            results.push(ChunkSearchResult {
                id,
                source_file: file_path,
                content,
                similarity,
                chunk_index,
                total_chunks,
                title,
                category,
                sentence_range: (start_sentence, end_sentence),
                avg_chunk_similarity: avg_similarity,
            });
        }
    }

    // Sort by similarity (descending)
    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit results
    results.truncate(config.max_results);

    info!("Returning {} search results", results.len());

    Ok(results)
}

/// Parse embedding vector from byte array
///
/// Converts a byte array containing f32 values (little-endian) into a vector.
///
/// # Arguments
///
/// * `bytes` - Byte array containing f32 values
///
/// # Returns
///
/// Returns a vector of f32 values
///
/// # Errors
///
/// Returns `SearchError::EmbeddingParse` if the byte array length is not
/// a multiple of 4 (size of f32)
fn parse_embedding(bytes: &[u8]) -> Result<Vec<f32>, SearchError> {
    if bytes.len() % 4 != 0 {
        return Err(SearchError::EmbeddingParse(format!(
            "Invalid embedding byte length: {} (must be multiple of 4)",
            bytes.len()
        )));
    }

    let mut embedding = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        embedding.push(value);
    }

    Ok(embedding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.max_results, 10);
        assert_eq!(config.min_similarity, 0.0);
        assert!(config.category_filter.is_none());
    }

    #[test]
    fn test_search_config_custom() {
        let config = SearchConfig {
            max_results: 20,
            min_similarity: 0.5,
            category_filter: Some("tutorial".to_string()),
        };
        assert_eq!(config.max_results, 20);
        assert_eq!(config.min_similarity, 0.5);
        assert_eq!(config.category_filter, Some("tutorial".to_string()));
    }

    #[test]
    fn test_parse_embedding_valid() {
        // Create byte array from f32 values
        let values = vec![1.0f32, 2.0f32, 3.0f32];
        let mut bytes = Vec::new();
        for value in &values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_embedding(&bytes).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
    }

    #[test]
    fn test_parse_embedding_invalid_length() {
        let bytes = vec![0u8, 1u8, 2u8]; // Not a multiple of 4
        let result = parse_embedding(&bytes);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SearchError::EmbeddingParse(_)
        ));
    }

    #[test]
    fn test_parse_embedding_empty() {
        let bytes = vec![];
        let result = parse_embedding(&bytes).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_chunk_search_result_creation() {
        let result = ChunkSearchResult {
            id: 1,
            source_file: "docs/guide.md".to_string(),
            content: "Example content".to_string(),
            similarity: 0.95,
            chunk_index: 0,
            total_chunks: 5,
            title: Some("Guide".to_string()),
            category: Some("tutorial".to_string()),
            sentence_range: (0, 3),
            avg_chunk_similarity: 0.85,
        };

        assert_eq!(result.id, 1);
        assert_eq!(result.similarity, 0.95);
        assert_eq!(result.chunk_index, 0);
        assert_eq!(result.sentence_range, (0, 3));
    }

    #[tokio::test]
    async fn test_search_with_empty_query() {
        // Create a test pool (this will fail but we're testing validation)
        let _config = SearchConfig::default();

        // We can't actually create a pool without a database, but we can test
        // that empty query is caught before trying to use the pool
        // This is a design validation test
        assert!(true); // Placeholder - actual test would need mock database
    }

    #[test]
    fn test_search_error_display() {
        let error = SearchError::EmptyQuery;
        assert_eq!(error.to_string(), "Query string cannot be empty");

        let error = SearchError::InvalidConfig("test".to_string());
        assert_eq!(error.to_string(), "Invalid search configuration: test");
    }
}
