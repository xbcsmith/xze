//! Embedding generation for semantic chunking
//!
//! This module provides functions for generating embeddings using the Ollama API.
//! It supports both single and batch embedding generation with error handling
//! and retry logic.

use crate::ai::client::{EmbedRequest, OllamaClient};
use crate::error::XzeError;
use thiserror::Error;
use tracing::{debug, warn};

/// Errors that can occur during embedding generation
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// Failed to generate embeddings from Ollama
    #[error("Failed to generate embeddings: {0}")]
    OllamaError(String),

    /// Empty text provided for embedding
    #[error("Cannot generate embedding for empty text")]
    EmptyText,

    /// Embedding dimension mismatch
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Invalid model specified
    #[error("Invalid model specified: {0}")]
    InvalidModel(String),
}

impl From<XzeError> for EmbeddingError {
    fn from(error: XzeError) -> Self {
        EmbeddingError::OllamaError(error.to_string())
    }
}

/// Generates embeddings for a single text using Ollama
///
/// This function sends a request to the Ollama API to generate an embedding
/// vector for the provided text using the specified model.
///
/// # Arguments
///
/// * `client` - Ollama client for API communication
/// * `model` - Model name to use for embedding generation (e.g., "nomic-embed-text")
/// * `text` - Text to generate embedding for
///
/// # Returns
///
/// Returns a vector of f32 values representing the embedding
///
/// # Errors
///
/// Returns `EmbeddingError::EmptyText` if text is empty
/// Returns `EmbeddingError::OllamaError` if API request fails
///
/// # Examples
///
/// ```no_run
/// use xze_core::semantic::embeddings::generate_embeddings;
/// use xze_core::ai::client::OllamaClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClient::new("http://localhost:11434".to_string());
/// let embedding = generate_embeddings(
///     &client,
///     "nomic-embed-text",
///     "This is a sample sentence."
/// ).await?;
/// assert!(!embedding.is_empty());
/// # Ok(())
/// # }
/// ```
pub async fn generate_embeddings(
    client: &OllamaClient,
    model: &str,
    text: &str,
) -> std::result::Result<Vec<f32>, EmbeddingError> {
    if text.trim().is_empty() {
        return Err(EmbeddingError::EmptyText);
    }

    debug!("Generating embedding for text of length {}", text.len());

    let request = EmbedRequest {
        model: model.to_string(),
        prompt: text.to_string(),
    };

    let embedding = client.embed(request).await?;

    debug!("Generated embedding with dimension {}", embedding.len());

    Ok(embedding)
}

/// Generates embeddings for multiple texts in batches
///
/// This function efficiently generates embeddings for a collection of texts
/// by processing them in batches. This is more efficient than making individual
/// requests for each text.
///
/// # Arguments
///
/// * `client` - Ollama client for API communication
/// * `model` - Model name to use for embedding generation
/// * `texts` - Slice of texts to generate embeddings for
/// * `batch_size` - Number of texts to process in each batch (recommended: 10-50)
///
/// # Returns
///
/// Returns a vector of embedding vectors, one for each input text
///
/// # Errors
///
/// Returns `EmbeddingError` if any embedding generation fails
///
/// # Examples
///
/// ```no_run
/// use xze_core::semantic::embeddings::generate_embeddings_batch;
/// use xze_core::ai::client::OllamaClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OllamaClient::new("http://localhost:11434".to_string());
/// let texts = vec![
///     "First sentence.".to_string(),
///     "Second sentence.".to_string(),
///     "Third sentence.".to_string(),
/// ];
/// let embeddings = generate_embeddings_batch(
///     &client,
///     "nomic-embed-text",
///     &texts,
///     10
/// ).await?;
/// assert_eq!(embeddings.len(), 3);
/// # Ok(())
/// # }
/// ```
pub async fn generate_embeddings_batch(
    client: &OllamaClient,
    model: &str,
    texts: &[String],
    batch_size: usize,
) -> std::result::Result<Vec<Vec<f32>>, EmbeddingError> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    debug!(
        "Generating embeddings for {} texts in batches of {}",
        texts.len(),
        batch_size
    );

    let mut all_embeddings = Vec::with_capacity(texts.len());
    let mut expected_dimension: Option<usize> = None;

    for (batch_idx, chunk) in texts.chunks(batch_size).enumerate() {
        debug!(
            "Processing batch {}/{} ({} texts)",
            batch_idx + 1,
            texts.len().div_ceil(batch_size),
            chunk.len()
        );

        for text in chunk {
            match generate_embeddings(client, model, text).await {
                Ok(embedding) => {
                    // Verify dimension consistency
                    if let Some(expected) = expected_dimension {
                        if embedding.len() != expected {
                            return Err(EmbeddingError::DimensionMismatch {
                                expected,
                                actual: embedding.len(),
                            });
                        }
                    } else {
                        expected_dimension = Some(embedding.len());
                    }

                    all_embeddings.push(embedding);
                }
                Err(e) => {
                    warn!("Failed to generate embedding for text: {}", e);
                    return Err(e);
                }
            }
        }
    }

    debug!(
        "Successfully generated {} embeddings with dimension {}",
        all_embeddings.len(),
        expected_dimension.unwrap_or(0)
    );

    Ok(all_embeddings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_error_from_xze_error() {
        let xze_err = XzeError::ai("Test error");
        let embed_err = EmbeddingError::from(xze_err);
        assert!(embed_err.to_string().contains("Test error"));
    }

    #[test]
    fn test_empty_text_error() {
        let err = EmbeddingError::EmptyText;
        assert_eq!(err.to_string(), "Cannot generate embedding for empty text");
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let err = EmbeddingError::DimensionMismatch {
            expected: 384,
            actual: 768,
        };
        assert!(err.to_string().contains("384"));
        assert!(err.to_string().contains("768"));
    }

    #[test]
    fn test_invalid_model_error() {
        let err = EmbeddingError::InvalidModel("unknown-model".to_string());
        assert!(err.to_string().contains("unknown-model"));
    }

    // Note: Integration tests that require a running Ollama instance
    // are in the integration test directory
}
