//! Core types for semantic chunking
//!
//! This module defines the fundamental data structures used in semantic chunking,
//! including chunks, metadata, and associated utility functions.

use serde::{Deserialize, Serialize};

/// Represents a semantically coherent chunk of text
///
/// A semantic chunk is a contiguous section of text that maintains topical
/// coherence, determined by similarity analysis between adjacent sentences.
///
/// # Examples
///
/// ```
/// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
/// use std::collections::HashMap;
///
/// let metadata = ChunkMetadata {
///     source_file: "docs/guide.md".to_string(),
///     title: Some("Getting Started".to_string()),
///     category: Some("tutorial".to_string()),
///     keywords: vec!["setup".to_string(), "installation".to_string()],
///     word_count: 150,
///     char_count: 890,
/// };
///
/// let chunk = SemanticChunk::new(
///     "This is the chunk content.".to_string(),
///     0,
///     5,
///     0,
///     2,
///     0.85,
///     metadata,
/// );
///
/// assert_eq!(chunk.chunk_index, 0);
/// assert_eq!(chunk.sentence_count(), 3);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticChunk {
    /// The actual text content of the chunk
    pub content: String,

    /// Index of this chunk within the document (0-based)
    pub chunk_index: usize,

    /// Total number of chunks in the document
    pub total_chunks: usize,

    /// Starting sentence index in the original document
    pub start_sentence: usize,

    /// Ending sentence index in the original document (inclusive)
    pub end_sentence: usize,

    /// Average similarity score between sentences in this chunk
    pub avg_similarity: f64,

    /// Additional metadata about the chunk
    pub metadata: ChunkMetadata,
}

/// Metadata associated with a semantic chunk
///
/// Contains contextual information about the chunk's origin,
/// classification, and characteristics.
///
/// # Examples
///
/// ```
/// use xze_core::semantic::types::ChunkMetadata;
///
/// let metadata = ChunkMetadata {
///     source_file: "src/main.rs".to_string(),
///     title: Some("Main Function".to_string()),
///     category: Some("reference".to_string()),
///     keywords: vec!["entry".to_string(), "initialization".to_string()],
///     word_count: 85,
///     char_count: 512,
/// };
///
/// assert_eq!(metadata.word_count, 85);
/// assert!(metadata.title.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMetadata {
    /// Path to the source file this chunk originated from
    pub source_file: String,

    /// Optional title of the document or section
    pub title: Option<String>,

    /// Diataxis category (tutorial, how-to, explanation, reference)
    pub category: Option<String>,

    /// Keywords or tags associated with the chunk
    pub keywords: Vec<String>,

    /// Number of words in the chunk
    pub word_count: usize,

    /// Number of characters in the chunk
    pub char_count: usize,
}

impl SemanticChunk {
    /// Creates a new semantic chunk with the given parameters
    ///
    /// # Arguments
    ///
    /// * `content` - The text content of the chunk
    /// * `chunk_index` - Index of this chunk (0-based)
    /// * `total_chunks` - Total number of chunks in the document
    /// * `start_sentence` - Starting sentence index
    /// * `end_sentence` - Ending sentence index (inclusive)
    /// * `avg_similarity` - Average similarity between sentences
    /// * `metadata` - Associated metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
    ///
    /// let metadata = ChunkMetadata {
    ///     source_file: "test.md".to_string(),
    ///     title: None,
    ///     category: None,
    ///     keywords: vec![],
    ///     word_count: 10,
    ///     char_count: 50,
    /// };
    ///
    /// let chunk = SemanticChunk::new(
    ///     "Test content".to_string(),
    ///     0,
    ///     1,
    ///     0,
    ///     0,
    ///     1.0,
    ///     metadata,
    /// );
    ///
    /// assert_eq!(chunk.content, "Test content");
    /// ```
    pub fn new(
        content: String,
        chunk_index: usize,
        total_chunks: usize,
        start_sentence: usize,
        end_sentence: usize,
        avg_similarity: f64,
        metadata: ChunkMetadata,
    ) -> Self {
        Self {
            content,
            chunk_index,
            total_chunks,
            start_sentence,
            end_sentence,
            avg_similarity,
            metadata,
        }
    }

    /// Returns the number of sentences in this chunk
    ///
    /// Calculated as the difference between end and start sentence indices plus one.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
    ///
    /// let metadata = ChunkMetadata {
    ///     source_file: "test.md".to_string(),
    ///     title: None,
    ///     category: None,
    ///     keywords: vec![],
    ///     word_count: 10,
    ///     char_count: 50,
    /// };
    ///
    /// let chunk = SemanticChunk::new(
    ///     "Sentence one. Sentence two. Sentence three.".to_string(),
    ///     0,
    ///     1,
    ///     0,
    ///     2,
    ///     0.9,
    ///     metadata,
    /// );
    ///
    /// assert_eq!(chunk.sentence_count(), 3);
    /// ```
    pub fn sentence_count(&self) -> usize {
        self.end_sentence.saturating_sub(self.start_sentence) + 1
    }
}

impl ChunkMetadata {
    /// Creates a new ChunkMetadata with basic information
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the source file
    /// * `content` - The chunk content (used to calculate counts)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::types::ChunkMetadata;
    ///
    /// let metadata = ChunkMetadata::new(
    ///     "docs/example.md".to_string(),
    ///     "Hello world, this is a test.",
    /// );
    ///
    /// assert_eq!(metadata.source_file, "docs/example.md");
    /// assert_eq!(metadata.word_count, 6);
    /// assert!(metadata.char_count > 0);
    /// ```
    pub fn new(source_file: String, content: &str) -> Self {
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();

        Self {
            source_file,
            title: None,
            category: None,
            keywords: Vec::new(),
            word_count,
            char_count,
        }
    }

    /// Creates metadata with all fields specified
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the source file
    /// * `title` - Optional title
    /// * `category` - Optional Diataxis category
    /// * `keywords` - List of keywords
    /// * `content` - The chunk content (used to calculate counts)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::types::ChunkMetadata;
    ///
    /// let metadata = ChunkMetadata::with_details(
    ///     "docs/guide.md".to_string(),
    ///     Some("Installation".to_string()),
    ///     Some("how-to".to_string()),
    ///     vec!["setup".to_string()],
    ///     "Follow these steps to install.",
    /// );
    ///
    /// assert_eq!(metadata.title.unwrap(), "Installation");
    /// assert!(metadata.word_count > 0);
    /// ```
    pub fn with_details(
        source_file: String,
        title: Option<String>,
        category: Option<String>,
        keywords: Vec<String>,
        content: &str,
    ) -> Self {
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();

        Self {
            source_file,
            title,
            category,
            keywords,
            word_count,
            char_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_chunk_creation() {
        let metadata = ChunkMetadata::new("test.md".to_string(), "Test content");
        let chunk = SemanticChunk::new(
            "Test content".to_string(),
            0,
            1,
            0,
            2,
            0.85,
            metadata.clone(),
        );

        assert_eq!(chunk.content, "Test content");
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.total_chunks, 1);
        assert_eq!(chunk.start_sentence, 0);
        assert_eq!(chunk.end_sentence, 2);
        assert_eq!(chunk.avg_similarity, 0.85);
        assert_eq!(chunk.metadata.source_file, "test.md");
    }

    #[test]
    fn test_sentence_count() {
        let metadata = ChunkMetadata::new("test.md".to_string(), "Test");
        let chunk = SemanticChunk::new("Test".to_string(), 0, 1, 0, 4, 0.9, metadata);

        assert_eq!(chunk.sentence_count(), 5);
    }

    #[test]
    fn test_sentence_count_single_sentence() {
        let metadata = ChunkMetadata::new("test.md".to_string(), "Test");
        let chunk = SemanticChunk::new("Test".to_string(), 0, 1, 0, 0, 1.0, metadata);

        assert_eq!(chunk.sentence_count(), 1);
    }

    #[test]
    fn test_chunk_metadata_new() {
        let content = "Hello world, this is a test sentence.";
        let metadata = ChunkMetadata::new("source.md".to_string(), content);

        assert_eq!(metadata.source_file, "source.md");
        assert_eq!(metadata.word_count, 7);
        assert_eq!(metadata.char_count, 37);
        assert!(metadata.title.is_none());
        assert!(metadata.category.is_none());
        assert!(metadata.keywords.is_empty());
    }

    #[test]
    fn test_chunk_metadata_with_details() {
        let content = "Installation steps for setup.";
        let metadata = ChunkMetadata::with_details(
            "guide.md".to_string(),
            Some("Setup Guide".to_string()),
            Some("how-to".to_string()),
            vec!["install".to_string(), "setup".to_string()],
            content,
        );

        assert_eq!(metadata.source_file, "guide.md");
        assert_eq!(metadata.title, Some("Setup Guide".to_string()));
        assert_eq!(metadata.category, Some("how-to".to_string()));
        assert_eq!(metadata.keywords.len(), 2);
        assert_eq!(metadata.word_count, 4);
    }

    #[test]
    fn test_chunk_serialization() {
        let metadata = ChunkMetadata::new("test.md".to_string(), "Content");
        let chunk = SemanticChunk::new("Content".to_string(), 0, 1, 0, 0, 1.0, metadata);

        let serialized = serde_json::to_string(&chunk).unwrap();
        let deserialized: SemanticChunk = serde_json::from_str(&serialized).unwrap();

        assert_eq!(chunk, deserialized);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = ChunkMetadata::with_details(
            "test.md".to_string(),
            Some("Title".to_string()),
            Some("tutorial".to_string()),
            vec!["tag1".to_string()],
            "Test content",
        );

        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: ChunkMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metadata, deserialized);
    }

    #[test]
    fn test_word_count_with_multiple_spaces() {
        let content = "Word1    Word2     Word3";
        let metadata = ChunkMetadata::new("test.md".to_string(), content);

        assert_eq!(metadata.word_count, 3);
    }

    #[test]
    fn test_char_count_with_unicode() {
        let content = "Hello 世界";
        let metadata = ChunkMetadata::new("test.md".to_string(), content);

        assert_eq!(metadata.char_count, 8); // 5 + 1 (space) + 2 (Chinese chars)
    }

    #[test]
    fn test_empty_content() {
        let metadata = ChunkMetadata::new("test.md".to_string(), "");

        assert_eq!(metadata.word_count, 0);
        assert_eq!(metadata.char_count, 0);
    }
}
