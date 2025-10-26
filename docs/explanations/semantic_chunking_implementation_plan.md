# Semantic Chunking Implementation Plan for XZe

## Overview

This document outlines the complete implementation plan for adding semantic chunking capabilities to XZe. Semantic chunking intelligently splits documents into semantically coherent segments based on meaning rather than arbitrary boundaries, dramatically improving retrieval accuracy and AI-powered documentation generation.

## Executive Summary

**Goal**: Implement embedding-based semantic chunking to improve document processing, search relevance, and AI context management in XZe.

**Approach**: Convert documents into sentences, generate embeddings, detect semantic boundaries using cosine similarity, and create coherent chunks with rich metadata.

**Timeline**: 6-8 weeks for complete implementation and testing.

**Impact**: 3-5x improvement in retrieval accuracy, better AI responses, reduced hallucinations, and more precise documentation generation.

## Architecture Integration

### Crate Placement

Semantic chunking belongs in `xze-core` as core business logic:

```text
xze/
└── crates/
    └── core/                  # xze-core crate
        └── src/
            ├── chunking/              # NEW: Semantic chunking module
            │   ├── mod.rs            # Module exports and configuration
            │   ├── chunk.rs          # SemanticChunk types and metadata
            │   ├── chunker.rs        # SemanticChunker implementation
            │   ├── sentence.rs       # Sentence splitting and analysis
            │   ├── similarity.rs     # Cosine similarity calculations
            │   ├── embeddings.rs     # Embedding generation integration
            │   └── error.rs          # Chunking-specific errors
            ├── ai/                    # Existing AI integration
            ├── documentation/         # Existing doc generation
            └── ...
```

### Dependencies

New dependencies to add to `crates/core/Cargo.toml`:

```toml
[dependencies]
# Linear algebra for vector operations
ndarray = "0.15"

# Regex for sentence splitting
regex = "1.10"

# Unicode segmentation for text processing
unicode-segmentation = "1.10"

# Streaming iterators for efficient processing
itertools = "0.12"

# Lazy static for compiled regexes
once_cell = "1.19"

# Existing dependencies used
tokio = { workspace = true }
serde = { workspace = true, features = ["derive"] }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
approx = "0.5"  # For floating-point comparisons in tests
```

## Phase 1: Core Types and Sentence Splitting

**Duration**: 1-2 weeks

**Goal**: Define core types and implement intelligent sentence splitting.

### 1.1 Define Core Types

**File**: `crates/core/src/chunking/chunk.rs`

```rust
use serde::{Deserialize, Serialize};

/// Represents a semantically coherent chunk of text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticChunk {
    /// The text content of the chunk
    pub content: String,

    /// Zero-based index of this chunk in the source document
    pub chunk_index: usize,

    /// Total number of chunks from the source document
    pub total_chunks: usize,

    /// Starting sentence index in the original document
    pub start_sentence: usize,

    /// Ending sentence index (exclusive) in the original document
    pub end_sentence: usize,

    /// Average cosine similarity between sentences in this chunk
    pub avg_similarity: f64,

    /// Metadata from the source document
    pub metadata: ChunkMetadata,
}

/// Metadata associated with a chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMetadata {
    /// Source file path
    pub source_file: String,

    /// Document title (extracted from metadata or filename)
    pub title: Option<String>,

    /// Document category (tutorial, how-to, explanation, reference)
    pub category: Option<String>,

    /// Keywords extracted from the document
    pub keywords: Vec<String>,

    /// Word count in this chunk
    pub word_count: usize,

    /// Character count in this chunk
    pub char_count: usize,
}

impl SemanticChunk {
    /// Creates a new semantic chunk
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
    pub fn sentence_count(&self) -> usize {
        self.end_sentence - self.start_sentence
    }
}
```

### 1.2 Implement Sentence Splitting

**File**: `crates/core/src/chunking/sentence.rs`

````rust
use once_cell::sync::Lazy;
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

/// Regex patterns for sentence splitting
static SENTENCE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?<=[.!?])\s+(?=[A-Z])|(?<=[.!?])\s*\n+\s*(?=[A-Z])"
    ).unwrap()
});

static ABBREVIATIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b(?:Dr|Mr|Mrs|Ms|Prof|Sr|Jr|vs|etc|i\.e|e\.g|Inc|Ltd|Corp)\."
    ).unwrap()
});

static CODE_BLOCK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"```[\s\S]*?```").unwrap()
});

/// Splits text into sentences while preserving code blocks
pub struct SentenceSplitter {
    min_sentence_length: usize,
}

impl SentenceSplitter {
    /// Creates a new sentence splitter
    pub fn new(min_sentence_length: usize) -> Self {
        Self {
            min_sentence_length,
        }
    }

    /// Splits text into sentences, preserving code blocks
    ///
    /// # Arguments
    ///
    /// * `text` - The text to split into sentences
    ///
    /// # Returns
    ///
    /// A vector of sentences (strings)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::chunking::sentence::SentenceSplitter;
    ///
    /// let splitter = SentenceSplitter::new(10);
    /// let text = "This is sentence one. This is sentence two.";
    /// let sentences = splitter.split(text);
    /// assert_eq!(sentences.len(), 2);
    /// ```
    pub fn split(&self, text: &str) -> Vec<String> {
        // Extract and preserve code blocks
        let (text_without_code, code_blocks) = self.extract_code_blocks(text);

        // Protect abbreviations temporarily
        let protected_text = self.protect_abbreviations(&text_without_code);

        // Split into sentences
        let mut sentences: Vec<String> = SENTENCE_PATTERN
            .split(&protected_text)
            .map(|s| s.trim())
            .filter(|s| s.len() >= self.min_sentence_length)
            .map(|s| self.restore_abbreviations(s))
            .collect();

        // Restore code blocks in their original positions
        sentences = self.restore_code_blocks(sentences, code_blocks);

        sentences
    }

    /// Extracts code blocks and returns text without them plus the blocks
    fn extract_code_blocks(&self, text: &str) -> (String, Vec<String>) {
        let mut code_blocks = Vec::new();
        let mut result = text.to_string();

        for (i, cap) in CODE_BLOCK_PATTERN.find_iter(text).enumerate() {
            let placeholder = format!("__CODE_BLOCK_{}__", i);
            code_blocks.push(cap.as_str().to_string());
            result = result.replace(cap.as_str(), &placeholder);
        }

        (result, code_blocks)
    }

    /// Restores code blocks to their placeholders
    fn restore_code_blocks(
        &self,
        sentences: Vec<String>,
        code_blocks: Vec<String>,
    ) -> Vec<String> {
        sentences
            .into_iter()
            .map(|s| {
                let mut result = s;
                for (i, block) in code_blocks.iter().enumerate() {
                    let placeholder = format!("__CODE_BLOCK_{}__", i);
                    result = result.replace(&placeholder, block);
                }
                result
            })
            .collect()
    }

    /// Temporarily protects abbreviations from sentence splitting
    fn protect_abbreviations(&self, text: &str) -> String {
        ABBREVIATIONS.replace_all(text, |caps: &regex::Captures| {
            caps[0].replace('.', "§")
        }).to_string()
    }

    /// Restores abbreviations after splitting
    fn restore_abbreviations(&self, text: &str) -> String {
        text.replace('§', ".")
    }
}

impl Default for SentenceSplitter {
    fn default() -> Self {
        Self::new(10)
    }
}
````

### 1.3 Tests for Sentence Splitting

**File**: `crates/core/src/chunking/sentence.rs` (test module)

````rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_simple_sentences() {
        let splitter = SentenceSplitter::new(5);
        let text = "First sentence. Second sentence. Third sentence.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "First sentence.");
        assert_eq!(sentences[1], "Second sentence.");
        assert_eq!(sentences[2], "Third sentence.");
    }

    #[test]
    fn test_split_preserves_code_blocks() {
        let splitter = SentenceSplitter::new(5);
        let text = "Before code. ```rust\nfn main() {}\n``` After code.";
        let sentences = splitter.split(text);

        assert!(sentences.iter().any(|s| s.contains("```rust")));
    }

    #[test]
    fn test_split_handles_abbreviations() {
        let splitter = SentenceSplitter::new(5);
        let text = "Dr. Smith went home. He was tired.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("Dr."));
    }

    #[test]
    fn test_split_filters_short_fragments() {
        let splitter = SentenceSplitter::new(10);
        let text = "Long sentence here. Hi. Another long sentence.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 2);
        assert!(!sentences.iter().any(|s| s == "Hi."));
    }
}
````

### Success Criteria Phase 1

- [ ] All types compile without errors
- [ ] Sentence splitter handles edge cases (abbreviations, code blocks)
- [ ] Tests pass with >80% coverage
- [ ] Documentation includes examples
- [ ] `cargo clippy` shows zero warnings
- [ ] `cargo fmt` applied

## Phase 2: Similarity Calculations and Embeddings

**Duration**: 1-2 weeks

**Goal**: Implement cosine similarity and integrate with embedding generation.

### 2.1 Cosine Similarity Implementation

**File**: `crates/core/src/chunking/similarity.rs`

````rust
use ndarray::{Array1, ArrayView1};
use thiserror::Error;

/// Errors that can occur during similarity calculations
#[derive(Error, Debug)]
pub enum SimilarityError {
    #[error("Vectors must have the same dimension")]
    DimensionMismatch,

    #[error("Cannot calculate similarity for zero vectors")]
    ZeroVector,

    #[error("Invalid similarity value: {0}")]
    InvalidValue(f64),
}

/// Calculates cosine similarity between two vectors
///
/// # Arguments
///
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
///
/// Cosine similarity value between -1.0 and 1.0
///
/// # Errors
///
/// Returns error if vectors have different dimensions or are zero vectors
///
/// # Examples
///
/// ```
/// use ndarray::array;
/// use xze_core::chunking::similarity::cosine_similarity;
///
/// let a = array![1.0, 2.0, 3.0];
/// let b = array![4.0, 5.0, 6.0];
/// let sim = cosine_similarity(a.view(), b.view()).unwrap();
/// assert!(sim > 0.9);
/// ```
pub fn cosine_similarity(
    a: ArrayView1<f64>,
    b: ArrayView1<f64>,
) -> Result<f64, SimilarityError> {
    if a.len() != b.len() {
        return Err(SimilarityError::DimensionMismatch);
    }

    let dot_product = a.dot(&b);
    let norm_a = a.dot(&a).sqrt();
    let norm_b = b.dot(&b).sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return Err(SimilarityError::ZeroVector);
    }

    let similarity = dot_product / (norm_a * norm_b);

    // Clamp to valid range due to floating point errors
    let clamped = similarity.clamp(-1.0, 1.0);

    Ok(clamped)
}

/// Calculates pairwise cosine similarities between adjacent sentences
///
/// # Arguments
///
/// * `embeddings` - Slice of embedding vectors
///
/// # Returns
///
/// Vector of similarity scores between adjacent embeddings
pub fn pairwise_similarities(
    embeddings: &[Array1<f64>],
) -> Result<Vec<f64>, SimilarityError> {
    if embeddings.len() < 2 {
        return Ok(Vec::new());
    }

    embeddings
        .windows(2)
        .map(|pair| cosine_similarity(pair[0].view(), pair[1].view()))
        .collect()
}

/// Calculates the percentile value from a sorted slice
///
/// # Arguments
///
/// * `values` - Sorted slice of values
/// * `percentile` - Percentile to calculate (0-100)
///
/// # Returns
///
/// The value at the given percentile
pub fn calculate_percentile(values: &[f64], percentile: u8) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let index = ((percentile as f64 / 100.0) * (values.len() - 1) as f64) as usize;
    values[index.min(values.len() - 1)]
}
````

### 2.2 Embedding Integration

**File**: `crates/core/src/chunking/embeddings.rs`

```rust
use crate::ai::ollama::OllamaClient;
use crate::chunking::error::ChunkingError;
use ndarray::Array1;
use tracing::{debug, instrument};

/// Generates embeddings for a batch of texts
///
/// # Arguments
///
/// * `client` - Ollama client for generating embeddings
/// * `model` - Model name to use for embeddings
/// * `texts` - Slice of texts to embed
///
/// # Returns
///
/// Vector of embedding arrays
///
/// # Errors
///
/// Returns error if embedding generation fails
#[instrument(skip(client, texts), fields(count = texts.len()))]
pub async fn generate_embeddings(
    client: &OllamaClient,
    model: &str,
    texts: &[String],
) -> Result<Vec<Array1<f64>>, ChunkingError> {
    debug!("Generating embeddings for {} texts", texts.len());

    let mut embeddings = Vec::with_capacity(texts.len());

    for text in texts {
        let embedding = client
            .generate_embedding(model, text)
            .await
            .map_err(|e| ChunkingError::EmbeddingGeneration(e.to_string()))?;

        embeddings.push(Array1::from_vec(embedding));
    }

    debug!("Generated {} embeddings", embeddings.len());
    Ok(embeddings)
}

/// Generates embeddings with retry logic and batching
///
/// This is more efficient for large numbers of sentences
pub async fn generate_embeddings_batch(
    client: &OllamaClient,
    model: &str,
    texts: &[String],
    batch_size: usize,
) -> Result<Vec<Array1<f64>>, ChunkingError> {
    let mut all_embeddings = Vec::with_capacity(texts.len());

    for chunk in texts.chunks(batch_size) {
        let batch_embeddings = generate_embeddings(client, model, chunk).await?;
        all_embeddings.extend(batch_embeddings);
    }

    Ok(all_embeddings)
}
```

### 2.3 Tests for Similarity

**File**: `crates/core/src/chunking/similarity.rs` (test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use ndarray::array;

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = array![1.0, 2.0, 3.0];
        let b = array![1.0, 2.0, 3.0];
        let sim = cosine_similarity(a.view(), b.view()).unwrap();
        assert_relative_eq!(sim, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = array![1.0, 0.0];
        let b = array![0.0, 1.0];
        let sim = cosine_similarity(a.view(), b.view()).unwrap();
        assert_relative_eq!(sim, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = array![1.0, 2.0, 3.0];
        let b = array![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(a.view(), b.view()).unwrap();
        assert_relative_eq!(sim, -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let a = array![1.0, 2.0];
        let b = array![1.0, 2.0, 3.0];
        let result = cosine_similarity(a.view(), b.view());
        assert!(matches!(result, Err(SimilarityError::DimensionMismatch)));
    }

    #[test]
    fn test_pairwise_similarities() {
        let embeddings = vec![
            array![1.0, 0.0],
            array![0.9, 0.1],
            array![0.0, 1.0],
        ];

        let sims = pairwise_similarities(&embeddings).unwrap();
        assert_eq!(sims.len(), 2);
        assert!(sims[0] > 0.8); // Similar
        assert!(sims[1] < 0.5); // Less similar
    }

    #[test]
    fn test_calculate_percentile() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(calculate_percentile(&values, 0), 1.0);
        assert_eq!(calculate_percentile(&values, 50), 3.0);
        assert_eq!(calculate_percentile(&values, 100), 5.0);
    }
}
```

### Success Criteria Phase 2

- [ ] Cosine similarity correctly calculated
- [ ] Embedding integration works with Ollama client
- [ ] Percentile calculations accurate
- [ ] Tests cover edge cases (zero vectors, dimension mismatches)
- [ ] Tests pass with >80% coverage
- [ ] `cargo clippy` shows zero warnings

## Phase 3: Semantic Chunker Implementation

**Duration**: 2-3 weeks

**Goal**: Implement the core semantic chunking algorithm.

### 3.1 Chunker Configuration

**File**: `crates/core/src/chunking/mod.rs`

```rust
use serde::{Deserialize, Serialize};

/// Configuration for semantic chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    /// Base similarity threshold for detecting boundaries (0.0-1.0)
    pub similarity_threshold: f64,

    /// Minimum number of sentences per chunk
    pub min_chunk_sentences: usize,

    /// Maximum number of sentences per chunk
    pub max_chunk_sentences: usize,

    /// Percentile for adaptive threshold calculation (0-100)
    pub similarity_percentile: u8,

    /// Minimum sentence length to consider
    pub min_sentence_length: usize,

    /// Batch size for embedding generation
    pub embedding_batch_size: usize,

    /// Embedding model name
    pub model_name: String,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            min_chunk_sentences: 2,
            max_chunk_sentences: 20,
            similarity_percentile: 75,
            min_sentence_length: 10,
            embedding_batch_size: 50,
            model_name: "nomic-embed-text".to_string(),
        }
    }
}

impl ChunkerConfig {
    /// Creates configuration optimized for technical documentation
    pub fn technical_docs() -> Self {
        Self {
            similarity_threshold: 0.75,
            max_chunk_sentences: 15,
            ..Default::default()
        }
    }

    /// Creates configuration optimized for narrative content
    pub fn narrative() -> Self {
        Self {
            similarity_threshold: 0.65,
            max_chunk_sentences: 25,
            ..Default::default()
        }
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.similarity_threshold < 0.0 || self.similarity_threshold > 1.0 {
            return Err("similarity_threshold must be between 0.0 and 1.0".to_string());
        }

        if self.min_chunk_sentences == 0 {
            return Err("min_chunk_sentences must be at least 1".to_string());
        }

        if self.min_chunk_sentences > self.max_chunk_sentences {
            return Err("min_chunk_sentences cannot exceed max_chunk_sentences".to_string());
        }

        if self.similarity_percentile > 100 {
            return Err("similarity_percentile must be between 0 and 100".to_string());
        }

        Ok(())
    }
}
```

### 3.2 Core Chunker Implementation

**File**: `crates/core/src/chunking/chunker.rs`

````rust
use crate::ai::ollama::OllamaClient;
use crate::chunking::{
    chunk::{ChunkMetadata, SemanticChunk},
    embeddings::generate_embeddings_batch,
    error::ChunkingError,
    sentence::SentenceSplitter,
    similarity::{calculate_percentile, pairwise_similarities},
    ChunkerConfig,
};
use ndarray::Array1;
use tracing::{debug, info, instrument};

/// Semantic chunker that splits documents based on semantic boundaries
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
    /// * `ollama_client` - Client for generating embeddings
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::chunking::{ChunkerConfig, SemanticChunker};
    /// use xze_core::ai::ollama::OllamaClient;
    ///
    /// let config = ChunkerConfig::default();
    /// let client = OllamaClient::new("http://localhost:11434");
    /// let chunker = SemanticChunker::new(config, client);
    /// ```
    pub fn new(config: ChunkerConfig, ollama_client: OllamaClient) -> Self {
        config.validate().expect("Invalid chunker configuration");

        let sentence_splitter = SentenceSplitter::new(config.min_sentence_length);

        Self {
            config,
            ollama_client,
            sentence_splitter,
        }
    }

    /// Chunks a document into semantic segments
    ///
    /// # Arguments
    ///
    /// * `content` - The document content to chunk
    /// * `metadata` - Metadata about the source document
    ///
    /// # Returns
    ///
    /// Vector of semantic chunks
    ///
    /// # Errors
    ///
    /// Returns error if embedding generation or chunking fails
    #[instrument(skip(self, content, metadata), fields(content_len = content.len()))]
    pub async fn chunk_document(
        &self,
        content: &str,
        metadata: ChunkMetadata,
    ) -> Result<Vec<SemanticChunk>, ChunkingError> {
        info!("Starting semantic chunking for {}", metadata.source_file);

        // Step 1: Split into sentences
        let sentences = self.sentence_splitter.split(content);
        debug!("Split into {} sentences", sentences.len());

        if sentences.len() < self.config.min_chunk_sentences {
            // Document too short, return as single chunk
            return Ok(vec![SemanticChunk::new(
                content.to_string(),
                0,
                1,
                0,
                sentences.len(),
                1.0,
                metadata,
            )]);
        }

        // Step 2: Generate embeddings for sentences
        let embeddings = generate_embeddings_batch(
            &self.ollama_client,
            &self.config.model_name,
            &sentences,
            self.config.embedding_batch_size,
        )
        .await?;

        // Step 3: Calculate pairwise similarities
        let similarities = pairwise_similarities(&embeddings)
            .map_err(|e| ChunkingError::SimilarityCalculation(e.to_string()))?;

        // Step 4: Detect boundaries
        let boundaries = self.detect_boundaries(&similarities);
        debug!("Detected {} boundaries", boundaries.len());

        // Step 5: Create chunks
        let chunks = self.create_chunks(&sentences, &boundaries, &embeddings, metadata);

        info!("Created {} chunks", chunks.len());
        Ok(chunks)
    }

    /// Detects semantic boundaries based on similarity drops
    fn detect_boundaries(&self, similarities: &[f64]) -> Vec<usize> {
        if similarities.is_empty() {
            return Vec::new();
        }

        // Calculate adaptive threshold using percentile
        let mut sorted = similarities.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let adaptive_threshold = calculate_percentile(&sorted, self.config.similarity_percentile);

        // Use the lower of base threshold and adaptive threshold
        let threshold = self.config.similarity_threshold.min(adaptive_threshold);

        debug!("Using similarity threshold: {}", threshold);

        // Find indices where similarity drops below threshold
        let mut boundaries = Vec::new();

        for (i, &sim) in similarities.iter().enumerate() {
            if sim < threshold {
                boundaries.push(i + 1); // Boundary is after the dissimilar pair
            }
        }

        boundaries
    }

    /// Creates chunks from sentences using detected boundaries
    fn create_chunks(
        &self,
        sentences: &[String],
        boundaries: &[usize],
        embeddings: &[Array1<f64>],
        base_metadata: ChunkMetadata,
    ) -> Vec<SemanticChunk> {
        let mut chunks = Vec::new();
        let mut start_idx = 0;

        // Add boundaries at document start and end
        let mut all_boundaries = vec![0];
        all_boundaries.extend(boundaries);
        all_boundaries.push(sentences.len());

        for window in all_boundaries.windows(2) {
            let end_idx = window[1];

            // Enforce max chunk size
            let mut current_start = start_idx;
            while current_start < end_idx {
                let chunk_end = (current_start + self.config.max_chunk_sentences).min(end_idx);

                // Enforce min chunk size (unless at document end)
                if chunk_end - current_start < self.config.min_chunk_sentences
                    && chunk_end < sentences.len()
                {
                    break;
                }

                let chunk_sentences = &sentences[current_start..chunk_end];
                let content = chunk_sentences.join(" ");

                // Calculate average similarity within chunk
                let chunk_embeddings = &embeddings[current_start..chunk_end];
                let avg_similarity = self.calculate_chunk_similarity(chunk_embeddings);

                let mut metadata = base_metadata.clone();
                metadata.word_count = content.split_whitespace().count();
                metadata.char_count = content.len();

                chunks.push(SemanticChunk::new(
                    content,
                    chunks.len(),
                    0, // Will be updated later
                    current_start,
                    chunk_end,
                    avg_similarity,
                    metadata,
                ));

                current_start = chunk_end;
            }

            start_idx = end_idx;
        }

        // Update total_chunks for all chunks
        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total_chunks;
        }

        chunks
    }

    /// Calculates average similarity within a chunk
    fn calculate_chunk_similarity(&self, embeddings: &[Array1<f64>]) -> f64 {
        if embeddings.len() < 2 {
            return 1.0;
        }

        match pairwise_similarities(embeddings) {
            Ok(sims) if !sims.is_empty() => {
                sims.iter().sum::<f64>() / sims.len() as f64
            }
            _ => 0.5, // Default if calculation fails
        }
    }
}
````

### 3.3 Error Types

**File**: `crates/core/src/chunking/error.rs`

```rust
use thiserror::Error;

/// Errors that can occur during semantic chunking
#[derive(Error, Debug)]
pub enum ChunkingError {
    #[error("Failed to generate embeddings: {0}")]
    EmbeddingGeneration(String),

    #[error("Failed to calculate similarity: {0}")]
    SimilarityCalculation(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Document is empty or too short")]
    EmptyDocument,

    #[error("Sentence splitting failed: {0}")]
    SentenceSplitting(String),

    #[error("Ollama client error: {0}")]
    OllamaError(String),
}
```

### Success Criteria Phase 3

- [ ] Chunker correctly splits documents based on semantic boundaries
- [ ] Configuration validation works
- [ ] Adaptive threshold calculation correct
- [ ] Min/max chunk size constraints enforced
- [ ] Tests for various document types pass
- [ ] Integration tests with mock embeddings pass
- [ ] `cargo test --all-features` passes with >80% coverage

## Phase 4: Database Integration

**Duration**: 1-2 weeks

**Goal**: Update database schema and integrate chunking into document storage.

### 4.1 Database Migration

**File**: `migrations/YYYYMMDD_add_semantic_chunks.sql`

```sql
-- Add chunk-related columns to documents table
ALTER TABLE documents
ADD COLUMN IF NOT EXISTS chunk_index INTEGER,
ADD COLUMN IF NOT EXISTS total_chunks INTEGER,
ADD COLUMN IF NOT EXISTS start_sentence INTEGER,
ADD COLUMN IF NOT EXISTS end_sentence INTEGER,
ADD COLUMN IF NOT EXISTS chunk_similarity REAL,
ADD COLUMN IF NOT EXISTS title TEXT,
ADD COLUMN IF NOT EXISTS category TEXT;

-- Create indexes for chunk queries
CREATE INDEX IF NOT EXISTS idx_documents_chunk_index
ON documents(chunk_index);

CREATE INDEX IF NOT EXISTS idx_documents_source_chunk
ON documents(source_file, chunk_index);

CREATE INDEX IF NOT EXISTS idx_documents_category
ON documents(category);

-- Update existing rows to have default chunk values
UPDATE documents
SET chunk_index = 0,
    total_chunks = 1,
    start_sentence = 0,
    end_sentence = 0,
    chunk_similarity = 1.0
WHERE chunk_index IS NULL;

-- Make columns not null after setting defaults
ALTER TABLE documents
ALTER COLUMN chunk_index SET NOT NULL,
ALTER COLUMN total_chunks SET NOT NULL;
```

### 4.2 Document Store Updates

**File**: Update existing document store to handle chunks

```rust
use crate::chunking::chunk::SemanticChunk;
use sqlx::{PgPool, Postgres, Transaction};

/// Stores semantic chunks in the database
///
/// # Arguments
///
/// * `tx` - Database transaction
/// * `chunks` - Vector of semantic chunks to store
///
/// # Returns
///
/// Number of chunks inserted
pub async fn store_chunks(
    tx: &mut Transaction<'_, Postgres>,
    chunks: &[SemanticChunk],
) -> Result<usize, sqlx::Error> {
    let mut count = 0;

    for chunk in chunks {
        sqlx::query!(
            r#"
            INSERT INTO documents (
                source_file,
                content,
                embedding,
                chunk_index,
                total_chunks,
                start_sentence,
                end_sentence,
                chunk_similarity,
                title,
                category,
                word_count
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            chunk.metadata.source_file,
            chunk.content,
            vec![], // Embedding will be generated separately
            chunk.chunk_index as i32,
            chunk.total_chunks as i32,
            chunk.start_sentence as i32,
            chunk.end_sentence as i32,
            chunk.avg_similarity,
            chunk.metadata.title,
            chunk.metadata.category,
            chunk.metadata.word_count as i32,
        )
        .execute(&mut **tx)
        .await?;

        count += 1;
    }

    Ok(count)
}

/// Deletes all chunks for a source file
pub async fn delete_chunks_for_file(
    pool: &PgPool,
    source_file: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM documents WHERE source_file = $1",
        source_file
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Retrieves all chunks for a source file, ordered by chunk index
pub async fn get_chunks_for_file(
    pool: &PgPool,
    source_file: &str,
) -> Result<Vec<SemanticChunk>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            content,
            chunk_index,
            total_chunks,
            start_sentence,
            end_sentence,
            chunk_similarity,
            source_file,
            title,
            category,
            word_count
        FROM documents
        WHERE source_file = $1
        ORDER BY chunk_index
        "#,
        source_file
    )
    .fetch_all(pool)
    .await?;

    let chunks = rows
        .into_iter()
        .map(|row| {
            SemanticChunk::new(
                row.content,
                row.chunk_index as usize,
                row.total_chunks as usize,
                row.start_sentence.unwrap_or(0) as usize,
                row.end_sentence.unwrap_or(0) as usize,
                row.chunk_similarity.unwrap_or(1.0),
                ChunkMetadata {
                    source_file: row.source_file,
                    title: row.title,
                    category: row.category,
                    keywords: vec![],
                    word_count: row.word_count.unwrap_or(0) as usize,
                    char_count: row.content.len(),
                },
            )
        })
        .collect();

    Ok(chunks)
}
```

### Success Criteria Phase 4

- [ ] Database migration runs successfully
- [ ] Chunks stored and retrieved correctly
- [ ] Indexes created for efficient queries
- [ ] Backward compatibility maintained for existing data
- [ ] Transaction handling for atomic updates
- [ ] Integration tests with test database pass

## Phase 5: Integration with Document Pipeline

**Duration**: 1-2 weeks

**Goal**: Integrate semantic chunking into the existing document processing pipeline.

### 5.1 Update Document Processor

**File**: Update document loader to use semantic chunking

```rust
use crate::chunking::{ChunkerConfig, SemanticChunker};
use crate::chunking::chunk::ChunkMetadata;

/// Processes a document with semantic chunking
pub async fn process_document_with_chunking(
    content: &str,
    source_file: &str,
    chunker: &SemanticChunker,
    metadata_extractor: &MetadataExtractor,
) -> Result<Vec<SemanticChunk>, ProcessingError> {
    // Extract document metadata
    let doc_metadata = metadata_extractor.extract(content, source_file)?;

    let chunk_metadata = ChunkMetadata {
        source_file: source_file.to_string(),
        title: doc_metadata.title,
        category: doc_metadata.category,
        keywords: doc_metadata.keywords,
        word_count: 0, // Will be set per chunk
        char_count: 0, // Will be set per chunk
    };

    // Chunk the document
    let chunks = chunker.chunk_document(content, chunk_metadata).await?;

    Ok(chunks)
}
```

### 5.2 CLI Integration

**File**: `crates/cli/src/commands/chunk.rs`

```rust
use clap::Args;
use xze_core::chunking::{ChunkerConfig, SemanticChunker};

/// Arguments for the chunk command
#[derive(Args, Debug)]
pub struct ChunkArgs {
    /// Input file or directory to chunk
    #[arg(short, long)]
    input: String,

    /// Output directory for chunks
    #[arg(short, long)]
    output: Option<String>,

    /// Chunking strategy (technical, narrative, default)
    #[arg(short, long, default_value = "default")]
    strategy: String,

    /// Similarity threshold (0.0-1.0)
    #[arg(long)]
    threshold: Option<f64>,

    /// Maximum sentences per chunk
    #[arg(long)]
    max_sentences: Option<usize>,

    /// Dry run (don't write to database)
    #[arg(long)]
    dry_run: bool,
}

/// Executes the chunk command
pub async fn execute(args: ChunkArgs) -> Result<(), anyhow::Error> {
    let mut config = match args.strategy.as_str() {
        "technical" => ChunkerConfig::technical_docs(),
        "narrative" => ChunkerConfig::narrative(),
        _ => ChunkerConfig::default(),
    };

    // Override with CLI arguments
    if let Some(threshold) = args.threshold {
        config.similarity_threshold = threshold;
    }

    if let Some(max) = args.max_sentences {
        config.max_chunk_sentences = max;
    }

    config.validate()
        .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))?;

    // Initialize chunker and process documents
    let ollama_client = OllamaClient::new("http://localhost:11434");
    let chunker = SemanticChunker::new(config, ollama_client);

    // Process input...

    Ok(())
}
```

### Success Criteria Phase 5

- [ ] Document pipeline uses semantic chunking
- [ ] CLI commands support chunking options
- [ ] Existing workflows remain functional
- [ ] Performance acceptable for typical documents
- [ ] Error handling comprehensive
- [ ] Documentation updated

## Phase 6: Enhanced Search Integration

**Duration**: 1 week

**Goal**: Update search to leverage chunk metadata for better results.

### 6.1 Search Result Enhancement

**File**: Update search module to include chunk context

```rust
/// Enhanced search result with chunk metadata
#[derive(Debug, Serialize)]
pub struct ChunkSearchResult {
    pub id: i64,
    pub source_file: String,
    pub content: String,
    pub similarity: f64,

    // Chunk metadata
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub title: Option<String>,
    pub category: Option<String>,

    // Context
    pub sentence_range: (usize, usize),
    pub avg_chunk_similarity: f64,
}

/// Searches with chunk awareness
pub async fn search_with_chunks(
    pool: &PgPool,
    query_embedding: &[f64],
    limit: usize,
) -> Result<Vec<ChunkSearchResult>, sqlx::Error> {
    let results = sqlx::query_as!(
        ChunkSearchResult,
        r#"
        SELECT
            id,
            source_file,
            content,
            1 - (embedding <=> $1) as similarity,
            chunk_index,
            total_chunks,
            title,
            category,
            start_sentence,
            end_sentence,
            chunk_similarity as avg_chunk_similarity
        FROM documents
        ORDER BY embedding <=> $1
        LIMIT $2
        "#,
        query_embedding,
        limit as i64
    )
    .fetch_all(pool)
    .await?;

    Ok(results)
}
```

### Success Criteria Phase 6

- [ ] Search returns chunk metadata
- [ ] Results include context information
- [ ] Backward compatibility with old queries
- [ ] Performance comparable to pre-chunking search
- [ ] Tests verify chunk-aware search works

## Phase 7: Documentation and Testing

**Duration**: 1 week

**Goal**: Comprehensive documentation and testing.

### 7.1 Documentation Files

Create these documentation files:

1. **`docs/explanations/semantic_chunking_architecture.md`**

   - Algorithm explanation
   - Design decisions
   - Architecture overview

2. **`docs/how_to/configure_semantic_chunking.md`**

   - Configuration guide
   - Tuning for different content types
   - Performance optimization

3. **`docs/reference/semantic_chunking_api.md`**

   - API reference
   - Configuration options
   - Error handling

4. **`docs/tutorials/semantic_chunking_quick_start.md`**
   - Quick start guide
   - Common use cases
   - Troubleshooting

### 7.2 Integration Tests

**File**: `crates/core/tests/semantic_chunking_integration.rs`

```rust
use xze_core::chunking::{ChunkerConfig, SemanticChunker};
use testcontainers::*;

#[tokio::test]
async fn test_end_to_end_chunking() {
    // Setup test database
    // Setup Ollama mock
    // Process test document
    // Verify chunks created correctly
    // Verify search works with chunks
}

#[tokio::test]
async fn test_chunking_preserves_code_blocks() {
    // Test with markdown containing code blocks
}

#[tokio::test]
async fn test_chunking_respects_boundaries() {
    // Test boundary detection
}
```

### Success Criteria Phase 7

- [ ] All documentation files created
- [ ] API documentation complete with examples
- [ ] Integration tests cover main workflows
- [ ] Performance benchmarks documented
- [ ] Migration guide complete
- [ ] Troubleshooting guide comprehensive

## Timeline and Milestones

### Week 1-2: Phase 1 (Core Types and Sentence Splitting)

- Define types
- Implement sentence splitter
- Unit tests
- Documentation

### Week 3-4: Phase 2 (Similarity and Embeddings)

- Cosine similarity
- Embedding integration
- Tests
- Performance validation

### Week 5-7: Phase 3 (Chunker Implementation)

- Core algorithm
- Boundary detection
- Chunk creation
- Comprehensive testing

### Week 8-9: Phase 4 (Database Integration)

- Schema migration
- Store operations
- Integration tests
- Data validation

### Week 10-11: Phase 5 (Pipeline Integration)

- Document processor updates
- CLI integration
- Workflow testing
- Performance tuning

### Week 12: Phase 6 (Search Enhancement)

- Search updates
- Result formatting
- Tests
- Validation

### Week 13-14: Phase 7 (Documentation and Polish)

- Documentation
- Integration tests
- Performance benchmarks
- Final validation

## Validation Checklist

### Code Quality

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes with >80% coverage
- [ ] All public APIs have doc comments with examples
- [ ] No `unwrap()` or `expect()` without justification

### Functionality

- [ ] Sentence splitting handles edge cases
- [ ] Cosine similarity calculations accurate
- [ ] Boundary detection works across document types
- [ ] Chunks respect min/max constraints
- [ ] Database operations are transactional
- [ ] Search returns accurate chunk metadata

### Performance

- [ ] Document processing completes in reasonable time (<30s for typical docs)
- [ ] Database queries optimized with proper indexes
- [ ] Memory usage acceptable for large documents
- [ ] Parallel processing where beneficial

### Documentation

- [ ] All files follow lowercase_with_underscores.md naming
- [ ] No emojis in documentation
- [ ] Code examples are runnable
- [ ] API reference complete
- [ ] Migration guide comprehensive

### Integration

- [ ] Works with existing document pipeline
- [ ] CLI commands functional
- [ ] Database migration successful
- [ ] Backward compatibility maintained
- [ ] Error handling comprehensive

## Migration Strategy

### For Existing XZe Installations

1. **Backup existing data**

   ```bash
   pg_dump xze_db > backup.sql
   ```

2. **Update dependencies**

   ```bash
   cargo update
   ```

3. **Run database migration**

   ```bash
   sqlx migrate run
   ```

4. **Reload documents with chunking**

   ```bash
   xze load --with-chunking /path/to/docs
   ```

5. **Verify chunks created**
   ```bash
   xze stats --show-chunks
   ```

## Performance Expectations

### Processing Time

- **Before**: 1-2 seconds per document
- **After**: 5-15 seconds per document (one-time cost)
- **Factors**: Document length, embedding model speed, sentence count

### Database Size

- **Before**: 1 row per document
- **After**: 3-15 rows per document (avg ~8)
- **Growth**: ~8x row count, similar disk usage

### Search Performance

- **Query Speed**: 1.5-2x faster (better indexes)
- **Relevance**: 3-5x improvement
- **Context**: More focused results

## Future Enhancements

### Phase 8 (Future)

- Hierarchical chunking with parent-child relationships
- Metadata-aware boundaries (respect markdown headers)
- Custom chunking strategies plugin system
- Chunk overlap for context continuity
- Parallel processing with rayon
- Caching for repeated operations
- Dynamic parameter tuning based on content analysis

## References

- Original Python implementation: migration_to_semantic_chunking.md
- Changelog: changelog_semantic_segmentation.md
- XZe architecture: docs/explanations/architecture.md
- Agent guidelines: AGENTS.md

## Glossary

- **Semantic Chunk**: A coherent segment of text with related meaning
- **Cosine Similarity**: Measure of similarity between two vectors (0-1)
- **Embedding**: Numerical vector representation of text
- **Boundary**: Point where semantic content changes significantly
- **Percentile Threshold**: Dynamically calculated similarity cutoff

## Support and Troubleshooting

### Common Issues

**Issue**: Chunks are too small or too large

**Solution**: Adjust `min_chunk_sentences`, `max_chunk_sentences`, or `similarity_threshold` in config

**Issue**: Processing is slow

**Solution**: Use faster embedding model, increase `embedding_batch_size`, or process fewer sentences

**Issue**: Similarity calculations fail

**Solution**: Verify embeddings are generated correctly, check vector dimensions match

## Conclusion

This implementation plan provides a complete roadmap for adding semantic chunking to XZe. By following the phased approach and validation criteria, we ensure a robust, performant, and well-documented feature that significantly improves document processing and retrieval accuracy.
