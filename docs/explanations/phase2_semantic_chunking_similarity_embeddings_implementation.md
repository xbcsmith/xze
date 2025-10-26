# Phase 2: Similarity Calculations and Embeddings Implementation

## Overview

This document describes the implementation of Phase 2 of the semantic chunking feature for XZe, which adds similarity calculations and embedding generation capabilities. Phase 2 builds on the core types and sentence splitting from Phase 1 to enable semantic similarity analysis between text segments.

## Components Delivered

- `crates/core/src/semantic/similarity.rs` (349 lines) - Cosine similarity calculations and percentile functions
- `crates/core/src/semantic/embeddings.rs` (238 lines) - Async embedding generation with Ollama integration
- `crates/core/src/semantic/mod.rs` - Updated module exports
- `docs/explanations/phase2_semantic_chunking_similarity_embeddings_implementation.md` (this document)

Total: ~600 lines of new code with comprehensive tests

## Implementation Details

### Component 1: Similarity Calculations

The similarity module provides functions for calculating cosine similarity between embedding vectors, which is fundamental to detecting semantic boundaries in text.

#### Error Handling

```rust
pub enum SimilarityError {
    DimensionMismatch { expected: usize, actual: usize },
    ZeroVector,
    InvalidValue(String),
}
```

These errors handle common failure cases in vector similarity calculations:
- Dimension mismatches when comparing vectors of different sizes
- Zero vectors which have no meaningful direction
- Invalid floating-point values like NaN or infinity

#### Cosine Similarity Function

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, SimilarityError>
```

Calculates the cosine of the angle between two vectors, returning a value from -1.0 (opposite) to 1.0 (identical). The implementation:

1. Validates dimensions match between vectors
2. Computes dot product and magnitudes in a single pass
3. Checks for zero vectors before division
4. Validates result for NaN or infinite values
5. Returns similarity score

Key optimizations:
- Single-pass calculation reduces overhead
- Early validation prevents unnecessary computation
- Explicit NaN/infinity checks ensure result validity

#### Pairwise Similarities

```rust
pub fn pairwise_similarities(embeddings: &[Vec<f32>]) -> Result<Vec<f32>, SimilarityError>
```

Computes similarity scores between consecutive embeddings in a sequence. Given N embeddings, returns N-1 similarity scores. This is used to detect semantic boundaries by finding drops in similarity between adjacent sentences.

#### Percentile Calculation

```rust
pub fn calculate_percentile(values: &[f32], percentile: f32) -> f32
```

Computes percentile values for threshold determination. The semantic chunker uses this to dynamically determine similarity thresholds based on the distribution of similarity scores in a document.

Implementation approach:
- Creates sorted copy of input values
- Uses linear interpolation to find percentile position
- Handles edge cases (0th and 100th percentiles)

### Component 2: Embedding Generation

The embeddings module integrates with the Ollama API to generate vector embeddings for text segments.

#### Error Handling

```rust
pub enum EmbeddingError {
    OllamaError(String),
    EmptyText,
    DimensionMismatch { expected: usize, actual: usize },
    InvalidModel(String),
}
```

Provides specific error types for embedding generation failures and includes conversion from XzeError for seamless integration with existing error handling.

#### Single Text Embedding

```rust
pub async fn generate_embeddings(
    client: &OllamaClient,
    model: &str,
    text: &str,
) -> std::result::Result<Vec<f32>, EmbeddingError>
```

Generates an embedding vector for a single text string using the Ollama API. The function:

1. Validates text is not empty
2. Creates an embedding request with specified model
3. Sends request to Ollama API asynchronously
4. Returns embedding vector on success

Typical embedding dimensions:
- nomic-embed-text: 768 dimensions
- mxbai-embed-large: 1024 dimensions
- sentence-transformers models: 384-768 dimensions

#### Batch Embedding Generation

```rust
pub async fn generate_embeddings_batch(
    client: &OllamaClient,
    model: &str,
    texts: &[String],
    batch_size: usize,
) -> std::result::Result<Vec<Vec<f32>>, EmbeddingError>
```

Efficiently generates embeddings for multiple texts by processing them in batches. This is critical for performance when chunking large documents with many sentences.

Features:
- Processes texts in configurable batch sizes
- Validates dimension consistency across all embeddings
- Provides progress logging for large batches
- Fails fast on first error to avoid wasted API calls

Performance considerations:
- Recommended batch size: 10-50 texts
- Larger batches reduce overhead but increase memory usage
- Smaller batches allow faster failure detection

### Component 3: Module Integration

Updated `crates/core/src/semantic/mod.rs` to expose the new modules:

```rust
pub mod embeddings;
pub mod similarity;

pub use embeddings::{generate_embeddings, generate_embeddings_batch, EmbeddingError};
pub use similarity::{calculate_percentile, cosine_similarity, pairwise_similarities, SimilarityError};
```

This provides convenient access to the core functions while maintaining proper module organization.

## Testing

### Test Coverage

Phase 2 includes 23 comprehensive unit tests covering all major functionality:

#### Similarity Tests (19 tests)

- Identical vectors (similarity = 1.0)
- Orthogonal vectors (similarity = 0.0)
- Opposite vectors (similarity = -1.0)
- Dimension mismatch error handling
- Zero vector error handling
- Empty vector error handling
- Normalized vector calculations
- Pairwise similarity calculations
- Edge cases (single embedding, empty list)
- Percentile calculations (min, max, median, quartiles)
- Unsorted input handling
- Panic conditions (empty slice, invalid percentile)

#### Embedding Tests (4 tests)

- Error type conversions
- Empty text validation
- Dimension mismatch detection
- Invalid model error messages

Integration tests requiring a running Ollama instance are deferred to the integration test suite.

### Test Results

```
test result: ok. 49 passed; 0 failed; 0 ignored
```

All tests pass successfully, including the 26 tests from Phase 1 and 23 new tests from Phase 2.

## Usage Examples

### Basic Similarity Calculation

```rust
use xze_core::semantic::similarity::cosine_similarity;

let sentence1_embedding = vec![0.1, 0.5, 0.8];
let sentence2_embedding = vec![0.2, 0.6, 0.7];

let similarity = cosine_similarity(&sentence1_embedding, &sentence2_embedding)?;
println!("Similarity: {}", similarity);
```

### Detecting Semantic Boundaries

```rust
use xze_core::semantic::similarity::{pairwise_similarities, calculate_percentile};

// Given embeddings for a sequence of sentences
let embeddings = vec![
    vec![0.1, 0.5, 0.8],
    vec![0.2, 0.6, 0.7],
    vec![0.9, 0.1, 0.2],
];

// Calculate similarities between consecutive sentences
let similarities = pairwise_similarities(&embeddings)?;

// Determine threshold at 25th percentile
let threshold = calculate_percentile(&similarities, 0.25);

// Similarities below threshold indicate potential boundaries
for (i, sim) in similarities.iter().enumerate() {
    if *sim < threshold {
        println!("Potential boundary between sentences {} and {}", i, i + 1);
    }
}
```

### Batch Embedding Generation

```rust
use xze_core::semantic::embeddings::generate_embeddings_batch;
use xze_core::ai::client::OllamaClient;

let client = OllamaClient::new("http://localhost:11434".to_string());
let sentences = vec![
    "First sentence about topic A.".to_string(),
    "Second sentence about topic A.".to_string(),
    "Third sentence about topic B.".to_string(),
];

let embeddings = generate_embeddings_batch(
    &client,
    "nomic-embed-text",
    &sentences,
    10
).await?;

println!("Generated {} embeddings", embeddings.len());
```

## Architecture Decisions

### Why Cosine Similarity?

Cosine similarity was chosen over Euclidean distance or dot product because:

1. Scale invariance: Measures angle, not magnitude
2. Normalized range: Always between -1 and 1
3. Semantic meaning: Higher values indicate more similar meaning
4. Common in NLP: Standard metric for embedding similarity

### Why Batch Processing?

Batch embedding generation provides significant benefits:

1. Reduced API overhead: Fewer round-trip requests
2. Better error handling: Detect issues early
3. Memory efficiency: Process chunks at a time
4. Progress tracking: Log batch completion

### Error Handling Strategy

The implementation uses distinct error types for similarity and embedding operations:

- `SimilarityError`: Mathematical/validation errors
- `EmbeddingError`: API and model-related errors

This separation allows callers to handle different failure modes appropriately.

## Integration with Phase 1

Phase 2 builds directly on Phase 1 components:

- `SentenceSplitter` provides text segments for embedding
- `SemanticChunk` will store similarity scores (Phase 3)
- `ChunkMetadata` will include embedding dimensions (Phase 3)

The similarity and embedding functions are designed to integrate seamlessly with the chunker implementation in Phase 3.

## Performance Characteristics

### Similarity Calculations

- Time complexity: O(n) where n is vector dimension
- Space complexity: O(1) - no additional allocations
- Typical execution: < 1 microsecond for 768-dim vectors

### Embedding Generation

- Single embedding: 50-200ms depending on model and hardware
- Batch processing: Approximately linear with batch size
- Memory usage: ~3-4 KB per embedding (768 dimensions)

### Scalability

For a 10,000 word document with average sentence length of 20 words:
- Approximately 500 sentences
- Embedding generation: 25-100 seconds (with batching)
- Similarity calculations: < 1 millisecond
- Total memory: ~2 MB for all embeddings

## Validation Results

All quality checks passed successfully:

- ✅ `cargo fmt --all` - Code formatted
- ✅ `cargo check --all-targets --all-features` - Compiles without errors
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - Zero warnings
- ✅ `cargo test --all-features -p xze-core semantic` - 49 tests passed
- ✅ Documentation complete

## Known Limitations

1. **Ollama Dependency**: Requires running Ollama instance for embeddings
2. **No Caching**: Embeddings are regenerated each time (addressed in Phase 4)
3. **Single Model**: No automatic model fallback (can be added later)
4. **No GPU Detection**: Does not optimize for available hardware

## Next Steps: Phase 3

Phase 3 will implement the semantic chunker that uses these components:

1. Configure chunking parameters (threshold, min/max sentences)
2. Implement boundary detection using similarity scores
3. Create chunks based on detected boundaries
4. Calculate chunk-level statistics
5. Add comprehensive integration tests

The similarity and embedding functions are now ready for integration into the complete semantic chunking pipeline.

## References

- Architecture: `docs/explanations/architecture.md`
- Phase 1: `docs/explanations/phase1_semantic_chunking_core_types_implementation.md`
- Implementation Plan: `docs/explanations/semantic_chunking_implementation_plan.md`
- Ollama API: https://github.com/ollama/ollama/blob/main/docs/api.md

## Troubleshooting

### Dimension Mismatch Errors

If you encounter dimension mismatch errors:

1. Verify all embeddings use the same model
2. Check that model hasn't changed between calls
3. Ensure batch processing doesn't mix models

### Ollama Connection Errors

If embedding generation fails:

1. Verify Ollama is running: `ollama list`
2. Check base URL is correct (default: http://localhost:11434)
3. Ensure model is pulled: `ollama pull nomic-embed-text`
4. Check network connectivity

### Invalid Similarity Values

If cosine similarity returns unexpected values:

1. Verify embeddings are non-zero
2. Check for NaN values in input vectors
3. Ensure vectors have correct dimensions
4. Validate embedding generation succeeded
