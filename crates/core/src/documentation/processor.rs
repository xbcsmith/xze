//! Document processing with semantic chunking integration
//!
//! This module provides functionality to process documents and optionally
//! split them into semantic chunks for improved knowledge base storage
//! and retrieval.
//!
//! # Overview
//!
//! The document processor integrates semantic chunking into the document
//! pipeline, allowing documents to be:
//! - Loaded and validated
//! - Split into semantically coherent chunks
//! - Stored in the knowledge base with embeddings
//! - Retrieved for semantic search
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::documentation::processor::{DocumentProcessor, ProcessingConfig};
//! use xze_core::ai::OllamaClient;
//! use xze_core::kb::store::KbStore;
//! use std::path::Path;
//! use sqlx::PgPool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PgPool::connect("postgresql://localhost/xze").await?;
//! let store = KbStore::new(pool);
//! let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
//!
//! let config = ProcessingConfig::default();
//! let processor = DocumentProcessor::new(store, ollama_client, config);
//!
//! // Process a document with semantic chunking
//! let file_path = Path::new("docs/guide.md");
//! let content = std::fs::read_to_string(file_path)?;
//! let result = processor.process_document_with_chunking(
//!     file_path,
//!     &content,
//!     None,
//! ).await?;
//!
//! println!("Processed {} chunks", result.chunks_created);
//! # Ok(())
//! # }
//! ```

use crate::ai::OllamaClient;
use crate::kb::hash::calculate_content_hash;
use crate::kb::store::KbStore;
use crate::semantic::{ChunkMetadata, ChunkerConfig, SemanticChunk, SemanticChunker};
use crate::{Result, XzeError};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors that can occur during document processing
#[derive(Debug, Error)]
pub enum ProcessingError {
    /// Failed to read document
    #[error("Failed to read document: {0}")]
    ReadError(String),

    /// Failed to generate chunks
    #[error("Failed to generate chunks: {0}")]
    ChunkingError(String),

    /// Failed to store chunks
    #[error("Failed to store chunks: {0}")]
    StorageError(String),

    /// Invalid document format
    #[error("Invalid document format: {0}")]
    InvalidFormat(String),

    /// Document is empty
    #[error("Document is empty")]
    EmptyDocument,
}

impl From<ProcessingError> for XzeError {
    fn from(err: ProcessingError) -> Self {
        XzeError::Generic(anyhow::anyhow!(err.to_string()))
    }
}

/// Configuration for document processing
///
/// Controls how documents are processed, including whether to use
/// semantic chunking and how chunks should be created.
///
/// # Examples
///
/// ```
/// use xze_core::documentation::processor::ProcessingConfig;
/// use xze_core::semantic::chunker::ChunkerConfig;
///
/// // Default configuration with chunking enabled
/// let config = ProcessingConfig::default();
/// assert!(config.enable_chunking);
///
/// // Custom configuration
/// let custom = ProcessingConfig {
///     enable_chunking: true,
///     chunker_config: ChunkerConfig::technical_docs(),
///     skip_existing: false,
///     validate_content: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Enable semantic chunking
    pub enable_chunking: bool,

    /// Configuration for the semantic chunker
    pub chunker_config: ChunkerConfig,

    /// Skip processing if document already exists in KB
    pub skip_existing: bool,

    /// Validate content before processing
    pub validate_content: bool,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            enable_chunking: true,
            chunker_config: ChunkerConfig::default(),
            skip_existing: true,
            validate_content: true,
        }
    }
}

/// Result of document processing
///
/// Contains information about the processing operation including
/// the number of chunks created and whether the document was updated.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Path of the processed file
    pub file_path: String,

    /// Hash of the file content
    pub file_hash: String,

    /// Number of chunks created
    pub chunks_created: usize,

    /// Whether this was a new document (true) or update (false)
    pub is_new: bool,

    /// Total character count in document
    pub total_chars: usize,

    /// Total word count in document
    pub total_words: usize,

    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

impl ProcessingResult {
    /// Create a summary string of the processing result
    pub fn summary(&self) -> String {
        let action = if self.is_new { "Created" } else { "Updated" };
        format!(
            "{} {} with {} chunks ({} words, {} chars) in {}ms",
            action,
            self.file_path,
            self.chunks_created,
            self.total_words,
            self.total_chars,
            self.duration_ms
        )
    }
}

/// Document processor that integrates semantic chunking
///
/// Provides high-level document processing functionality including
/// reading, chunking, and storing documents in the knowledge base.
pub struct DocumentProcessor {
    store: KbStore,
    ollama_client: OllamaClient,
    config: ProcessingConfig,
}

impl DocumentProcessor {
    /// Create a new document processor
    ///
    /// # Arguments
    ///
    /// * `store` - Knowledge base store for persisting chunks
    /// * `ollama_client` - Ollama client for generating embeddings
    /// * `config` - Processing configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::documentation::processor::{DocumentProcessor, ProcessingConfig};
    /// use xze_core::ai::OllamaClient;
    /// use xze_core::kb::store::KbStore;
    /// use sqlx::PgPool;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = PgPool::connect("postgresql://localhost/xze").await?;
    /// let store = KbStore::new(pool);
    /// let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    /// let config = ProcessingConfig::default();
    ///
    /// let processor = DocumentProcessor::new(store, ollama_client, config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(store: KbStore, ollama_client: OllamaClient, config: ProcessingConfig) -> Self {
        Self {
            store,
            ollama_client,
            config,
        }
    }

    /// Process a document with semantic chunking
    ///
    /// This is the main entry point for processing documents. It:
    /// 1. Validates the document content
    /// 2. Checks if the document already exists (optional)
    /// 3. Generates semantic chunks
    /// 4. Stores chunks in the knowledge base
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the document file
    /// * `content` - Content of the document
    /// * `metadata` - Optional metadata to associate with chunks
    ///
    /// # Returns
    ///
    /// Returns a `ProcessingResult` with statistics about the operation
    ///
    /// # Errors
    ///
    /// Returns `ProcessingError` if:
    /// - Document is empty or invalid
    /// - Chunking fails
    /// - Storage operation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::documentation::processor::{DocumentProcessor, ProcessingConfig};
    /// use xze_core::ai::OllamaClient;
    /// use xze_core::kb::store::KbStore;
    /// use xze_core::semantic::types::ChunkMetadata;
    /// use std::path::Path;
    /// use sqlx::PgPool;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await?;
    /// # let store = KbStore::new(pool);
    /// # let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    /// # let config = ProcessingConfig::default();
    /// let processor = DocumentProcessor::new(store, ollama_client, config);
    ///
    /// let file_path = Path::new("docs/guide.md");
    /// let content = std::fs::read_to_string(file_path)?;
    ///
    /// let metadata = Some(ChunkMetadata {
    ///     source_file: "guide.md".to_string(),
    ///     title: Some("User Guide".to_string()),
    ///     category: Some("tutorial".to_string()),
    ///     keywords: vec!["guide".to_string(), "tutorial".to_string()],
    ///     word_count: 0,
    ///     char_count: 0,
    /// });
    ///
    /// let result = processor.process_document_with_chunking(
    ///     file_path,
    ///     &content,
    ///     metadata,
    /// ).await?;
    ///
    /// println!("{}", result.summary());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn process_document_with_chunking(
        &self,
        file_path: &Path,
        content: &str,
        metadata: Option<ChunkMetadata>,
    ) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        let file_path_str = file_path.to_string_lossy().to_string();

        info!("Processing document: {}", file_path_str);

        // Validate content
        if self.config.validate_content {
            self.validate_content(content)?;
        }

        // Calculate file hash
        let file_hash = calculate_content_hash(content);
        debug!("Document hash: {}", file_hash);

        // Check if document exists and skip if configured
        let is_new = if self.config.skip_existing {
            let existing = self
                .store
                .query_existing_files()
                .await
                .map_err(|e| ProcessingError::StorageError(e.to_string()))?;
            let file_path_key = file_path_str.clone();
            if let Some(existing_hash) = existing.get(&file_path_key) {
                if existing_hash == &file_hash {
                    info!("Document unchanged, skipping: {}", file_path_str);
                    return Ok(ProcessingResult {
                        file_path: file_path_str,
                        file_hash,
                        chunks_created: 0,
                        is_new: false,
                        total_chars: content.len(),
                        total_words: content.split_whitespace().count(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
            !existing.contains_key(&file_path_key)
        } else {
            true
        };

        // Delete existing chunks if updating
        if !is_new {
            info!("Deleting existing chunks for: {}", file_path_str);
            self.store
                .delete_semantic_chunks_for_file(file_path)
                .await
                .map_err(|e| ProcessingError::StorageError(e.to_string()))?;
        }

        // Generate chunks if enabled
        let chunks = if self.config.enable_chunking {
            let chunker = SemanticChunker::new(
                self.config.chunker_config.clone(),
                self.ollama_client.clone(),
            );

            debug!("Generating semantic chunks");
            let mut generated_chunks = chunker
                .chunk_document(content, metadata.clone())
                .await
                .map_err(|e| ProcessingError::ChunkingError(e.to_string()))?;

            // Update metadata if provided
            if let Some(meta) = metadata {
                for chunk in &mut generated_chunks {
                    chunk.metadata = meta.clone();
                    // Update counts based on actual chunk content
                    chunk.metadata.word_count = chunk.content.split_whitespace().count();
                    chunk.metadata.char_count = chunk.content.len();
                }
            }

            generated_chunks
        } else {
            // Create a single chunk for the entire document
            let meta = metadata.unwrap_or_else(|| ChunkMetadata {
                source_file: file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                title: None,
                category: None,
                keywords: vec![],
                word_count: content.split_whitespace().count(),
                char_count: content.len(),
            });

            vec![SemanticChunk {
                content: content.to_string(),
                chunk_index: 0,
                total_chunks: 1,
                start_sentence: 0,
                end_sentence: 0,
                avg_similarity: 0.0,
                metadata: meta,
                embedding: vec![],
            }]
        };

        let chunks_created = chunks.len();
        info!("Generated {} chunks", chunks_created);

        // Store chunks in knowledge base
        debug!("Storing chunks in knowledge base");
        self.store
            .store_semantic_chunks(file_path, &file_hash, &chunks)
            .await
            .map_err(|e| ProcessingError::StorageError(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = ProcessingResult {
            file_path: file_path_str,
            file_hash,
            chunks_created,
            is_new,
            total_chars: content.len(),
            total_words: content.split_whitespace().count(),
            duration_ms,
        };

        info!("{}", result.summary());

        Ok(result)
    }

    /// Validate document content
    ///
    /// Performs basic validation on document content to ensure it's
    /// suitable for processing.
    fn validate_content(&self, content: &str) -> Result<()> {
        if content.trim().is_empty() {
            return Err(ProcessingError::EmptyDocument.into());
        }

        // Check for minimum content length (at least 10 characters)
        if content.len() < 10 {
            warn!("Document is very short: {} characters", content.len());
        }

        // Check for valid UTF-8 (already guaranteed by &str)
        // Additional validation could be added here

        Ok(())
    }

    /// Process multiple documents in batch
    ///
    /// Convenience method for processing multiple documents. Each document
    /// is processed independently, and errors for individual documents do not
    /// stop processing of other documents.
    ///
    /// # Arguments
    ///
    /// * `documents` - Vector of tuples containing (file_path, content, metadata)
    ///
    /// # Returns
    ///
    /// Returns a vector of results, one for each document. Failed documents
    /// will have an error logged but won't stop batch processing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::documentation::processor::{DocumentProcessor, ProcessingConfig};
    /// use xze_core::ai::OllamaClient;
    /// use xze_core::kb::store::KbStore;
    /// use std::path::Path;
    /// use sqlx::PgPool;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = PgPool::connect("postgresql://localhost/xze").await?;
    /// # let store = KbStore::new(pool);
    /// # let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    /// # let config = ProcessingConfig::default();
    /// let processor = DocumentProcessor::new(store, ollama_client, config);
    ///
    /// let documents = vec![
    ///     (Path::new("docs/guide.md"), "Guide content".to_string(), None),
    ///     (Path::new("docs/api.md"), "API content".to_string(), None),
    /// ];
    ///
    /// let results = processor.process_documents_batch(documents).await;
    /// println!("Processed {} documents", results.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn process_documents_batch(
        &self,
        documents: Vec<(&Path, String, Option<ChunkMetadata>)>,
    ) -> Vec<Result<ProcessingResult>> {
        let total = documents.len();
        info!("Processing batch of {} documents", total);

        let mut results = Vec::with_capacity(total);

        for (index, (file_path, content, metadata)) in documents.into_iter().enumerate() {
            debug!(
                "Processing document {}/{}: {:?}",
                index + 1,
                total,
                file_path
            );

            match self
                .process_document_with_chunking(file_path, &content, metadata)
                .await
            {
                Ok(result) => {
                    results.push(Ok(result));
                }
                Err(e) => {
                    warn!("Failed to process document {:?}: {}", file_path, e);
                    results.push(Err(e));
                }
            }
        }

        let successful = results.iter().filter(|r| r.is_ok()).count();
        info!(
            "Batch processing complete: {}/{} successful",
            successful, total
        );

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_config_default() {
        let config = ProcessingConfig::default();
        assert!(config.enable_chunking);
        assert!(config.skip_existing);
        assert!(config.validate_content);
    }

    #[test]
    fn test_processing_result_summary() {
        let result = ProcessingResult {
            file_path: "docs/guide.md".to_string(),
            file_hash: "abc123".to_string(),
            chunks_created: 5,
            is_new: true,
            total_chars: 1000,
            total_words: 200,
            duration_ms: 500,
        };

        let summary = result.summary();
        assert!(summary.contains("Created"));
        assert!(summary.contains("docs/guide.md"));
        assert!(summary.contains("5 chunks"));
        assert!(summary.contains("200 words"));
    }

    #[test]
    fn test_validate_content_empty() {
        let config = ProcessingConfig::default();
        let pool = sqlx::PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let store = KbStore::new(pool);
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let processor = DocumentProcessor::new(store, ollama_client, config);

        let result = processor.validate_content("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_content_valid() {
        let config = ProcessingConfig::default();
        let pool = sqlx::PgPool::connect_lazy("postgresql://localhost/test").unwrap();
        let store = KbStore::new(pool);
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let processor = DocumentProcessor::new(store, ollama_client, config);

        let result = processor.validate_content("This is valid content with enough text.");
        assert!(result.is_ok());
    }

    #[test]
    fn test_processing_error_display() {
        let error = ProcessingError::EmptyDocument;
        assert_eq!(error.to_string(), "Document is empty");

        let error = ProcessingError::ChunkingError("test error".to_string());
        assert_eq!(error.to_string(), "Failed to generate chunks: test error");
    }
}
