//! Semantic chunking implementation
//!
//! This module provides the core semantic chunking functionality that splits
//! documents into semantically coherent chunks based on similarity analysis.
//!
//! # Overview
//!
//! The semantic chunker uses sentence embeddings and cosine similarity to detect
//! topical boundaries in documents. It groups consecutive sentences with high
//! similarity into chunks while respecting configurable constraints.
//!
//! # Architecture
//!
//! The chunking process involves:
//! 1. Split document into sentences (preserving code blocks)
//! 2. Generate embeddings for each sentence
//! 3. Calculate pairwise similarity between consecutive sentences
//! 4. Detect boundaries using percentile-based thresholding
//! 5. Create chunks from sentence groups
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::semantic::chunker::{SemanticChunker, ChunkerConfig};
//! use xze_core::ai::OllamaClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
//! let config = ChunkerConfig::technical_docs();
//! let chunker = SemanticChunker::new(config, ollama_client);
//!
//! let text = "First paragraph about topic A. More on topic A.
//!             New paragraph about topic B. Continues topic B.";
//! let chunks = chunker.chunk_document(text, None).await?;
//!
//! for chunk in chunks {
//!     println!("Chunk {}: {} sentences", chunk.chunk_index, chunk.sentence_count());
//! }
//! # Ok(())
//! # }
//! ```

use crate::ai::OllamaClient;
use crate::semantic::{
    calculate_percentile, generate_embeddings_batch, pairwise_similarities, ChunkMetadata,
    EmbeddingError, SemanticChunk, SentenceSplitter, SimilarityError,
};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Configuration for semantic chunking behavior
///
/// Controls how the chunker splits documents and detects semantic boundaries.
///
/// # Examples
///
/// ```
/// use xze_core::semantic::chunker::ChunkerConfig;
///
/// // Use default configuration
/// let config = ChunkerConfig::default();
/// assert_eq!(config.min_chunk_sentences, 3);
///
/// // Use preset for technical documentation
/// let tech_config = ChunkerConfig::technical_docs();
/// assert_eq!(tech_config.similarity_threshold, 0.75);
///
/// // Custom configuration
/// let custom = ChunkerConfig {
///     similarity_threshold: 0.8,
///     min_chunk_sentences: 5,
///     max_chunk_sentences: 50,
///     ..Default::default()
/// };
/// assert!(custom.validate().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Minimum similarity score to keep sentences in the same chunk (0.0-1.0)
    pub similarity_threshold: f32,

    /// Minimum number of sentences per chunk
    pub min_chunk_sentences: usize,

    /// Maximum number of sentences per chunk
    pub max_chunk_sentences: usize,

    /// Percentile to use for dynamic threshold calculation (0.0-1.0)
    pub similarity_percentile: f32,

    /// Minimum sentence length in characters to avoid fragments
    pub min_sentence_length: usize,

    /// Batch size for embedding generation
    pub embedding_batch_size: usize,

    /// Model name for embedding generation
    pub model_name: String,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            min_chunk_sentences: 3,
            max_chunk_sentences: 30,
            similarity_percentile: 0.5,
            min_sentence_length: 10,
            embedding_batch_size: 32,
            model_name: "nomic-embed-text".to_string(),
        }
    }
}

impl ChunkerConfig {
    /// Preset configuration optimized for technical documentation
    ///
    /// Uses higher similarity threshold and allows larger chunks to keep
    /// technical concepts together.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::chunker::ChunkerConfig;
    ///
    /// let config = ChunkerConfig::technical_docs();
    /// assert_eq!(config.similarity_threshold, 0.75);
    /// assert_eq!(config.max_chunk_sentences, 40);
    /// ```
    pub fn technical_docs() -> Self {
        Self {
            similarity_threshold: 0.75,
            max_chunk_sentences: 40,
            ..Default::default()
        }
    }

    /// Preset configuration optimized for narrative text
    ///
    /// Uses lower similarity threshold and smaller chunks to respect
    /// narrative flow and paragraph boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::chunker::ChunkerConfig;
    ///
    /// let config = ChunkerConfig::narrative();
    /// assert_eq!(config.similarity_threshold, 0.65);
    /// assert_eq!(config.max_chunk_sentences, 20);
    /// ```
    pub fn narrative() -> Self {
        Self {
            similarity_threshold: 0.65,
            max_chunk_sentences: 20,
            similarity_percentile: 0.4,
            ..Default::default()
        }
    }

    /// Validates configuration parameters
    ///
    /// # Errors
    ///
    /// Returns [`ChunkingError::InvalidConfiguration`] if any parameter is invalid:
    /// - similarity_threshold not in [0.0, 1.0]
    /// - similarity_percentile not in [0.0, 1.0]
    /// - min_chunk_sentences is 0
    /// - max_chunk_sentences less than min_chunk_sentences
    /// - min_sentence_length is 0
    /// - embedding_batch_size is 0
    /// - model_name is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::chunker::ChunkerConfig;
    ///
    /// let valid_config = ChunkerConfig::default();
    /// assert!(valid_config.validate().is_ok());
    ///
    /// let invalid_config = ChunkerConfig {
    ///     similarity_threshold: 1.5,
    ///     ..Default::default()
    /// };
    /// assert!(invalid_config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), ChunkingError> {
        if !(0.0..=1.0).contains(&self.similarity_threshold) {
            return Err(ChunkingError::InvalidConfiguration(
                "similarity_threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.similarity_percentile) {
            return Err(ChunkingError::InvalidConfiguration(
                "similarity_percentile must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.min_chunk_sentences == 0 {
            return Err(ChunkingError::InvalidConfiguration(
                "min_chunk_sentences must be greater than 0".to_string(),
            ));
        }

        if self.max_chunk_sentences < self.min_chunk_sentences {
            return Err(ChunkingError::InvalidConfiguration(
                "max_chunk_sentences must be >= min_chunk_sentences".to_string(),
            ));
        }

        if self.min_sentence_length == 0 {
            return Err(ChunkingError::InvalidConfiguration(
                "min_sentence_length must be greater than 0".to_string(),
            ));
        }

        if self.embedding_batch_size == 0 {
            return Err(ChunkingError::InvalidConfiguration(
                "embedding_batch_size must be greater than 0".to_string(),
            ));
        }

        if self.model_name.is_empty() {
            return Err(ChunkingError::InvalidConfiguration(
                "model_name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Semantic chunker for splitting documents into coherent segments
///
/// Uses sentence embeddings and similarity analysis to detect topical boundaries
/// and create semantically coherent chunks.
///
/// # Examples
///
/// ```no_run
/// use xze_core::semantic::chunker::{SemanticChunker, ChunkerConfig};
/// use xze_core::semantic::ChunkMetadata;
/// use xze_core::ai::OllamaClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
/// let config = ChunkerConfig::default();
/// let chunker = SemanticChunker::new(config, ollama_client);
///
/// let text = "Introduction to the topic. More details about it.
///             Next section begins here. Continues the new section.";
///
/// let metadata = ChunkMetadata::new("doc.md".to_string(), text);
/// let chunks = chunker.chunk_document(text, Some(metadata)).await?;
///
/// println!("Created {} chunks", chunks.len());
/// # Ok(())
/// # }
/// ```
pub struct SemanticChunker {
    config: ChunkerConfig,
    ollama_client: OllamaClient,
    sentence_splitter: SentenceSplitter,
}

impl SemanticChunker {
    /// Creates a new semantic chunker
    ///
    /// # Arguments
    ///
    /// * `config` - Chunking configuration
    /// * `ollama_client` - Ollama client for embedding generation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::semantic::chunker::{SemanticChunker, ChunkerConfig};
    /// use xze_core::ai::OllamaClient;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    /// let config = ChunkerConfig::technical_docs();
    /// let chunker = SemanticChunker::new(config, ollama_client);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: ChunkerConfig, ollama_client: OllamaClient) -> Self {
        let sentence_splitter = SentenceSplitter::new(config.min_sentence_length);
        Self {
            config,
            ollama_client,
            sentence_splitter,
        }
    }

    /// Chunks a document into semantically coherent segments
    ///
    /// # Arguments
    ///
    /// * `text` - The document text to chunk
    /// * `metadata` - Optional metadata to attach to chunks
    ///
    /// # Returns
    ///
    /// Returns a vector of [`SemanticChunk`] instances representing the document segments.
    ///
    /// # Errors
    ///
    /// Returns [`ChunkingError`] if:
    /// - Document is empty ([`ChunkingError::EmptyDocument`])
    /// - Configuration is invalid ([`ChunkingError::InvalidConfiguration`])
    /// - Embedding generation fails ([`ChunkingError::EmbeddingGeneration`])
    /// - Similarity calculation fails ([`ChunkingError::SimilarityCalculation`])
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::semantic::chunker::{SemanticChunker, ChunkerConfig};
    /// use xze_core::ai::OllamaClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    /// let chunker = SemanticChunker::new(ChunkerConfig::default(), ollama_client);
    ///
    /// let text = "First topic sentence. Another sentence about it.
    ///             New topic starts here. Continues new topic.";
    /// let chunks = chunker.chunk_document(text, None).await?;
    ///
    /// assert!(!chunks.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chunk_document(
        &self,
        text: &str,
        metadata: Option<ChunkMetadata>,
    ) -> Result<Vec<SemanticChunk>, ChunkingError> {
        // Validate configuration
        self.config.validate()?;

        // Split into sentences
        let sentences = self.sentence_splitter.split(text);

        if sentences.is_empty() {
            return Err(ChunkingError::EmptyDocument);
        }

        info!(
            "Split document into {} sentences for chunking",
            sentences.len()
        );

        // Generate embeddings for all sentences
        let embeddings = generate_embeddings_batch(
            &self.ollama_client,
            &self.config.model_name,
            &sentences,
            self.config.embedding_batch_size,
        )
        .await
        .map_err(ChunkingError::EmbeddingGeneration)?;

        debug!("Generated embeddings for {} sentences", embeddings.len());

        // Calculate pairwise similarities
        let similarities =
            pairwise_similarities(&embeddings).map_err(ChunkingError::SimilarityCalculation)?;

        // Detect chunk boundaries
        let boundaries = self.detect_boundaries(&similarities);

        debug!("Detected {} chunk boundaries", boundaries.len());

        // Create chunks from boundaries
        let chunks = self.create_chunks(&sentences, &embeddings, &boundaries, metadata);

        info!("Created {} semantic chunks", chunks.len());

        Ok(chunks)
    }

    /// Detects chunk boundaries based on similarity scores
    ///
    /// Uses percentile-based thresholding to identify where to split chunks.
    /// A boundary is placed where similarity drops below the threshold.
    ///
    /// # Arguments
    ///
    /// * `similarities` - Pairwise similarity scores between consecutive sentences
    ///
    /// # Returns
    ///
    /// Returns a vector of indices where chunk boundaries should be placed.
    fn detect_boundaries(&self, similarities: &[f32]) -> Vec<usize> {
        if similarities.is_empty() {
            return vec![];
        }

        // Calculate dynamic threshold based on percentile
        let threshold = calculate_percentile(similarities, self.config.similarity_percentile)
            .min(self.config.similarity_threshold);

        debug!(
            "Using similarity threshold: {:.3} (percentile: {:.2})",
            threshold, self.config.similarity_percentile
        );

        let mut boundaries = vec![0]; // Start with first sentence

        for (i, &similarity) in similarities.iter().enumerate() {
            if similarity < threshold {
                boundaries.push(i + 1);
            }
        }

        boundaries
    }

    /// Creates chunks from sentences and boundary positions
    ///
    /// Groups consecutive sentences between boundaries into chunks, respecting
    /// min/max chunk size constraints.
    ///
    /// # Arguments
    ///
    /// * `sentences` - All sentences in the document
    /// * `embeddings` - Embeddings for all sentences
    /// * `boundaries` - Indices where chunks should start
    /// * `metadata` - Optional metadata to attach to chunks
    ///
    /// # Returns
    ///
    /// Returns a vector of [`SemanticChunk`] instances.
    fn create_chunks(
        &self,
        sentences: &[String],
        embeddings: &[Vec<f32>],
        boundaries: &[usize],
        metadata: Option<ChunkMetadata>,
    ) -> Vec<SemanticChunk> {
        let mut chunks = Vec::new();
        let mut current_start = 0;

        // Use default metadata if none provided
        let default_metadata = ChunkMetadata::new("unknown".to_string(), "");
        let chunk_metadata = metadata.unwrap_or(default_metadata);

        for &boundary in boundaries.iter().skip(1) {
            let chunk_sentences = &sentences[current_start..boundary];

            // Enforce minimum chunk size
            if chunk_sentences.len() < self.config.min_chunk_sentences {
                continue;
            }

            // Enforce maximum chunk size by splitting if needed
            if chunk_sentences.len() > self.config.max_chunk_sentences {
                // Split into smaller chunks
                let mut sub_start = current_start;
                while sub_start < boundary {
                    let sub_end = (sub_start + self.config.max_chunk_sentences).min(boundary);
                    let sub_sentences = &sentences[sub_start..sub_end];

                    if sub_sentences.len() >= self.config.min_chunk_sentences {
                        let avg_similarity =
                            self.calculate_chunk_similarity(embeddings, sub_start, sub_end) as f64;

                        let content = sub_sentences.join(" ");
                        let chunk = SemanticChunk::new(
                            content,
                            chunks.len(),
                            0, // Total chunks updated later
                            sub_start,
                            sub_end - 1,
                            avg_similarity,
                            chunk_metadata.clone(),
                        );
                        chunks.push(chunk);
                    }

                    sub_start = sub_end;
                }
            } else {
                // Create normal chunk
                let avg_similarity =
                    self.calculate_chunk_similarity(embeddings, current_start, boundary) as f64;

                let content = chunk_sentences.join(" ");
                let chunk = SemanticChunk::new(
                    content,
                    chunks.len(),
                    0, // Total chunks updated later
                    current_start,
                    boundary - 1,
                    avg_similarity,
                    chunk_metadata.clone(),
                );
                chunks.push(chunk);
            }

            current_start = boundary;
        }

        // Handle remaining sentences
        if current_start < sentences.len() {
            let remaining_sentences = &sentences[current_start..];
            if remaining_sentences.len() >= self.config.min_chunk_sentences {
                let avg_similarity =
                    self.calculate_chunk_similarity(embeddings, current_start, sentences.len())
                        as f64;

                let content = remaining_sentences.join(" ");
                let chunk = SemanticChunk::new(
                    content,
                    chunks.len(),
                    0,
                    current_start,
                    sentences.len() - 1,
                    avg_similarity,
                    chunk_metadata.clone(),
                );
                chunks.push(chunk);
            } else if !chunks.is_empty() {
                // Merge with last chunk if too small
                warn!(
                    "Merging {} remaining sentences with last chunk",
                    remaining_sentences.len()
                );
                if let Some(last_chunk) = chunks.last_mut() {
                    let merged_content =
                        format!("{} {}", last_chunk.content, remaining_sentences.join(" "));
                    last_chunk.content = merged_content;
                    last_chunk.end_sentence = sentences.len() - 1;
                }
            }
        }

        // Update total_chunks for all chunks
        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total_chunks;
        }

        chunks
    }

    /// Calculates average similarity within a chunk
    ///
    /// Computes the mean cosine similarity between consecutive sentence pairs
    /// within the specified range.
    ///
    /// # Arguments
    ///
    /// * `embeddings` - All sentence embeddings
    /// * `start` - Start index (inclusive)
    /// * `end` - End index (exclusive)
    ///
    /// # Returns
    ///
    /// Returns the average similarity score, or 1.0 if the range contains only one sentence.
    fn calculate_chunk_similarity(&self, embeddings: &[Vec<f32>], start: usize, end: usize) -> f32 {
        if end - start <= 1 {
            return 1.0; // Single sentence has perfect similarity
        }

        let chunk_embeddings = &embeddings[start..end];
        let similarities = pairwise_similarities(chunk_embeddings).unwrap_or_default();

        if similarities.is_empty() {
            return 1.0;
        }

        similarities.iter().sum::<f32>() / similarities.len() as f32
    }
}

/// Errors that can occur during semantic chunking
#[derive(Error, Debug)]
pub enum ChunkingError {
    /// Failed to generate embeddings for sentences
    #[error("Embedding generation failed: {0}")]
    EmbeddingGeneration(#[from] EmbeddingError),

    /// Failed to calculate similarity between sentences
    #[error("Similarity calculation failed: {0}")]
    SimilarityCalculation(#[from] SimilarityError),

    /// Invalid configuration parameters
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Document is empty or contains no valid sentences
    #[error("Document is empty or contains no valid sentences")]
    EmptyDocument,

    /// Sentence splitting failed
    #[error("Sentence splitting failed: {0}")]
    SentenceSplitting(String),

    /// Ollama client error
    #[error("Ollama client error: {0}")]
    OllamaError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_config_default() {
        let config = ChunkerConfig::default();
        assert_eq!(config.similarity_threshold, 0.7);
        assert_eq!(config.min_chunk_sentences, 3);
        assert_eq!(config.max_chunk_sentences, 30);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chunker_config_technical_docs() {
        let config = ChunkerConfig::technical_docs();
        assert_eq!(config.similarity_threshold, 0.75);
        assert_eq!(config.max_chunk_sentences, 40);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chunker_config_narrative() {
        let config = ChunkerConfig::narrative();
        assert_eq!(config.similarity_threshold, 0.65);
        assert_eq!(config.max_chunk_sentences, 20);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_chunker_config_validation_invalid_threshold() {
        let config = ChunkerConfig {
            similarity_threshold: 1.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_chunker_config_validation_invalid_percentile() {
        let config = ChunkerConfig {
            similarity_percentile: -0.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_chunker_config_validation_zero_min_sentences() {
        let config = ChunkerConfig {
            min_chunk_sentences: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_chunker_config_validation_max_less_than_min() {
        let config = ChunkerConfig {
            min_chunk_sentences: 10,
            max_chunk_sentences: 5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_chunker_config_validation_zero_batch_size() {
        let config = ChunkerConfig {
            embedding_batch_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_chunker_config_validation_empty_model_name() {
        let config = ChunkerConfig {
            model_name: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_detect_boundaries_empty_similarities() {
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let config = ChunkerConfig::default();
        let chunker = SemanticChunker::new(config, ollama_client);

        let boundaries = chunker.detect_boundaries(&[]);
        assert!(boundaries.is_empty());
    }

    #[test]
    fn test_detect_boundaries_with_low_similarity() {
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let config = ChunkerConfig {
            similarity_threshold: 0.5,
            similarity_percentile: 0.5,
            ..Default::default()
        };
        let chunker = SemanticChunker::new(config, ollama_client);

        let similarities = vec![0.9, 0.8, 0.3, 0.7, 0.2, 0.85];
        let boundaries = chunker.detect_boundaries(&similarities);

        // Should have boundary at start and where similarity < threshold
        assert!(boundaries.contains(&0));
        assert!(boundaries.len() > 1);
    }

    #[test]
    fn test_calculate_chunk_similarity_single_sentence() {
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let config = ChunkerConfig::default();
        let chunker = SemanticChunker::new(config, ollama_client);

        let embeddings: Vec<Vec<f32>> = vec![vec![1.0, 0.0, 0.0]];
        let similarity = chunker.calculate_chunk_similarity(&embeddings, 0, 1);
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_calculate_chunk_similarity_multiple_sentences() {
        let ollama_client = OllamaClient::new("http://localhost:11434".to_string());
        let config = ChunkerConfig::default();
        let chunker = SemanticChunker::new(config, ollama_client);

        let embeddings: Vec<Vec<f32>> = vec![
            vec![1.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
        ];
        let similarity = chunker.calculate_chunk_similarity(&embeddings, 0, 3);
        assert_eq!(similarity, 1.0); // All identical vectors
    }
}
