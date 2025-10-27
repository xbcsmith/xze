# How to Configure Semantic Chunking

## Overview

This guide explains how to configure XZe's semantic chunking system for different use cases. You'll learn how to adjust chunking parameters, choose the right configuration preset, and fine-tune behavior for your specific content type.

## Quick Start

### Using Default Configuration

The simplest approach is to use the default configuration:

```rust
use xze_core::semantic::ChunkerConfig;

let config = ChunkerConfig::default();
```

This provides balanced settings suitable for most documentation.

### Using Presets

For specific content types, use built-in presets:

```rust
// For technical documentation, API references, tutorials
let config = ChunkerConfig::technical_docs();

// For narrative content, guides, blog posts
let config = ChunkerConfig::narrative();
```

## Configuration Parameters

### Similarity Threshold

Controls how similar adjacent sentences must be to remain in the same chunk.

**Parameter**: `similarity_threshold`
**Range**: 0.0 to 1.0
**Default**: 0.5

```rust
let mut config = ChunkerConfig::default();
config.similarity_threshold = 0.6; // Higher = more boundaries
```

**When to adjust**:
- **Increase (0.6-0.8)**: For highly structured content with distinct topics
- **Decrease (0.3-0.4)**: For flowing narrative with gradual topic transitions

### Minimum Chunk Sentences

Minimum number of sentences per chunk.

**Parameter**: `min_chunk_sentences`
**Range**: 1 to max_chunk_sentences
**Default**: 1

```rust
config.min_chunk_sentences = 3; // At least 3 sentences per chunk
```

**When to adjust**:
- **Increase (3-5)**: To ensure chunks have sufficient context
- **Keep at 1**: To allow very focused, granular chunks

### Maximum Chunk Sentences

Maximum number of sentences per chunk.

**Parameter**: `max_chunk_sentences`
**Range**: min_chunk_sentences to unlimited
**Default**: 10

```rust
config.max_chunk_sentences = 15; // Up to 15 sentences per chunk
```

**When to adjust**:
- **Increase (15-20)**: For technical docs with long explanations
- **Decrease (5-8)**: For concise, focused chunks in narrative content

### Similarity Percentile

Which percentile of similarity scores to use as threshold.

**Parameter**: `similarity_percentile`
**Range**: 0.0 to 100.0
**Default**: 75.0

```rust
config.similarity_percentile = 80.0; // Use 80th percentile
```

**How it works**: The system calculates similarity between all adjacent sentence pairs, then uses this percentile of those scores as the boundary threshold.

**When to adjust**:
- **Increase (80-90)**: To create more chunks (stricter boundaries)
- **Decrease (60-70)**: To create fewer, larger chunks

### Minimum Sentence Length

Minimum character count for a sentence to be processed.

**Parameter**: `min_sentence_length`
**Range**: 1 to unlimited
**Default**: 10

```rust
config.min_sentence_length = 20; // Ignore sentences shorter than 20 chars
```

**When to adjust**:
- **Increase (20-30)**: To filter out short fragments and headers
- **Decrease (5-10)**: To preserve all content including brief statements

### Embedding Batch Size

Number of sentences to process in a single embedding API call.

**Parameter**: `embedding_batch_size`
**Range**: 1 to unlimited
**Default**: 32

```rust
config.embedding_batch_size = 64; // Process 64 sentences at once
```

**When to adjust**:
- **Increase (64-128)**: For better performance with large documents
- **Decrease (8-16)**: If hitting API rate limits or memory constraints

### Model Name

The Ollama embedding model to use.

**Parameter**: `model_name`
**Default**: "nomic-embed-text"

```rust
config.model_name = "mxbai-embed-large".to_string();
```

**Available models** (must be pulled in Ollama):
- `nomic-embed-text`: Fast, good quality (recommended)
- `mxbai-embed-large`: Higher quality, slower
- `all-minilm`: Lightweight, faster

## Configuration Presets Explained

### Technical Documentation Preset

```rust
let config = ChunkerConfig::technical_docs();
```

Optimized for:
- API documentation
- Technical tutorials
- Reference material
- Code-heavy content

**Settings**:
- `similarity_threshold`: 0.6 (stricter boundaries)
- `min_chunk_sentences`: 2
- `max_chunk_sentences`: 15 (larger chunks)
- `similarity_percentile`: 70.0
- `min_sentence_length`: 15

**Why these settings**:
- Technical docs have clear topic boundaries
- Code blocks and examples benefit from larger chunks
- Higher threshold prevents mixing unrelated topics

### Narrative Preset

```rust
let config = ChunkerConfig::narrative();
```

Optimized for:
- Guides and walkthroughs
- Blog posts
- Explanatory content
- User manuals

**Settings**:
- `similarity_threshold`: 0.4 (gentler boundaries)
- `min_chunk_sentences`: 1
- `max_chunk_sentences`: 8 (smaller chunks)
- `similarity_percentile`: 80.0
- `min_sentence_length`: 10

**Why these settings**:
- Narrative flows more gradually between topics
- Smaller chunks help with focused retrieval
- Lower threshold accommodates topic transitions

## Use Case Examples

### API Documentation

```rust
let mut config = ChunkerConfig::technical_docs();
config.max_chunk_sentences = 20; // Allow longer API explanations
config.min_sentence_length = 20; // Skip short headers
```

### Tutorial Content

```rust
let mut config = ChunkerConfig::technical_docs();
config.similarity_percentile = 75.0; // More natural step boundaries
config.max_chunk_sentences = 12; // Moderate chunk size
```

### Blog Posts

```rust
let config = ChunkerConfig::narrative();
// Use as-is, optimized for flowing content
```

### Reference Manual

```rust
let mut config = ChunkerConfig::technical_docs();
config.similarity_threshold = 0.7; // Very strict boundaries
config.min_chunk_sentences = 1; // Allow single-sentence definitions
config.max_chunk_sentences = 10; // Keep focused
```

### Mixed Content (Docs + Code)

```rust
let mut config = ChunkerConfig::technical_docs();
config.similarity_threshold = 0.55; // Balanced
config.max_chunk_sentences = 15; // Accommodate code blocks
config.embedding_batch_size = 64; // Better performance
```

## CLI Configuration

### Using Command-Line Options

```bash
# Custom threshold
xze chunk input.md --threshold 0.6

# Custom max sentences
xze chunk input.md --max-sentences 15

# Combined options
xze chunk input.md --threshold 0.6 --max-sentences 12
```

### Environment Variables

Set defaults via environment:

```bash
export XZE_SIMILARITY_THRESHOLD=0.6
export XZE_MAX_CHUNK_SENTENCES=15
export XZE_OLLAMA_MODEL=nomic-embed-text

xze chunk input.md
```

## Validation and Testing

### Validate Configuration

The configuration validates itself automatically:

```rust
let config = ChunkerConfig {
    similarity_threshold: 1.5, // Invalid!
    ..Default::default()
};

// This will return an error
match config.validate() {
    Ok(_) => println!("Valid configuration"),
    Err(e) => println!("Invalid: {}", e),
}
```

**Validation rules**:
- `similarity_threshold` must be between 0.0 and 1.0
- `min_chunk_sentences` must be at least 1
- `max_chunk_sentences` must be >= `min_chunk_sentences`
- `similarity_percentile` must be between 0.0 and 100.0
- `min_sentence_length` must be positive
- `embedding_batch_size` must be positive

### Test Your Configuration

Use dry-run mode to preview chunking:

```bash
xze chunk sample.md --dry-run --threshold 0.6
```

This shows chunk boundaries without storing to database.

## Performance Tuning

### For Large Documents

```rust
let mut config = ChunkerConfig::default();
config.embedding_batch_size = 128; // Larger batches
config.max_chunk_sentences = 20; // Fewer chunks
```

### For Many Small Documents

```rust
let mut config = ChunkerConfig::default();
config.embedding_batch_size = 16; // Smaller batches
config.min_chunk_sentences = 2; // Ensure minimum content
```

### For Real-Time Processing

```rust
let mut config = ChunkerConfig::default();
config.embedding_batch_size = 32; // Balanced
config.max_chunk_sentences = 10; // Moderate size
config.model_name = "nomic-embed-text".to_string(); // Fastest model
```

## Troubleshooting Configuration Issues

### Chunks Too Large

**Symptom**: Chunks contain too many unrelated topics

**Solution**:
```rust
config.similarity_threshold = 0.65; // Increase threshold
config.max_chunk_sentences = 8; // Reduce max size
config.similarity_percentile = 80.0; // Higher percentile
```

### Chunks Too Small

**Symptom**: Every sentence becomes its own chunk

**Solution**:
```rust
config.similarity_threshold = 0.4; // Lower threshold
config.min_chunk_sentences = 3; // Enforce minimum
config.similarity_percentile = 70.0; // Lower percentile
```

### Missing Content

**Symptom**: Some sentences not appearing in chunks

**Solution**:
```rust
config.min_sentence_length = 5; // Lower minimum length
```

Check sentence splitting with:
```bash
xze chunk input.md --dry-run --verbose
```

### Poor Search Results

**Symptom**: Search returns irrelevant chunks

**Solution**:
```rust
// Re-chunk with stricter boundaries
config.similarity_threshold = 0.7;
config.max_chunk_sentences = 10;
```

Then re-process documents.

## Best Practices

### Start with Presets

Always start with a preset, then adjust:

```rust
let mut config = ChunkerConfig::technical_docs();
// Make incremental adjustments
config.max_chunk_sentences = 12;
```

### Document Your Configuration

When using custom configuration, document why:

```rust
// Custom config for API reference documentation
// - Higher threshold to separate endpoints clearly
// - Larger chunks to include complete examples
let mut config = ChunkerConfig::technical_docs();
config.similarity_threshold = 0.65;
config.max_chunk_sentences = 18;
```

### Test Before Deploying

Always test configuration changes on sample content:

1. Use dry-run mode
2. Check chunk boundaries make sense
3. Verify search results are relevant
4. Compare with previous configuration

### Version Your Configuration

Store configuration in version control:

```rust
// config/chunking.rs
pub fn production_config() -> ChunkerConfig {
    let mut config = ChunkerConfig::technical_docs();
    config.similarity_threshold = 0.6;
    config.max_chunk_sentences = 15;
    config
}
```

## Summary

Configuration options in order of impact:

1. **similarity_threshold**: Primary control for chunk boundaries
2. **max_chunk_sentences**: Limits chunk size
3. **similarity_percentile**: Fine-tunes boundary detection
4. **min_chunk_sentences**: Ensures minimum context
5. **embedding_batch_size**: Performance optimization

Start with a preset, test with dry-run, adjust incrementally, and always validate.

## See Also

- Semantic Chunking Tutorial for hands-on walkthrough
- API Reference for programmatic configuration
- Search Command Reference for using chunked documents
