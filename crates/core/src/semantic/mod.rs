//! Semantic chunking module for intelligent text segmentation
//!
//! This module provides functionality for breaking documents into semantically
//! coherent chunks based on similarity analysis between adjacent sentences.
//!
//! # Overview
//!
//! Semantic chunking improves upon traditional fixed-size chunking by:
//! - Maintaining topical coherence within chunks
//! - Using sentence embeddings and similarity scores
//! - Preserving code blocks and formatting
//! - Respecting natural document boundaries
//!
//! # Components
//!
//! - [`types`] - Core data structures (SemanticChunk, ChunkMetadata)
//! - [`splitter`] - Sentence splitting with code preservation
//!
//! # Examples
//!
//! ```
//! use xze_core::semantic::{splitter::SentenceSplitter, types::ChunkMetadata};
//!
//! // Split text into sentences
//! let splitter = SentenceSplitter::new(10);
//! let text = "First sentence. Second sentence. Third sentence.";
//! let sentences = splitter.split(text);
//!
//! assert_eq!(sentences.len(), 3);
//!
//! // Create chunk metadata
//! let metadata = ChunkMetadata::new(
//!     "docs/guide.md".to_string(),
//!     "Example content for documentation",
//! );
//!
//! assert!(metadata.word_count > 0);
//! ```

pub mod embeddings;
pub mod similarity;
pub mod splitter;
pub mod types;

// Re-export commonly used types
pub use embeddings::{generate_embeddings, generate_embeddings_batch, EmbeddingError};
pub use similarity::{
    calculate_percentile, cosine_similarity, pairwise_similarities, SimilarityError,
};
pub use splitter::SentenceSplitter;
pub use types::{ChunkMetadata, SemanticChunk};
