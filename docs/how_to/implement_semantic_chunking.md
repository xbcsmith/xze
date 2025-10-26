# How to Implement Semantic Chunking in XZe

## Overview

This guide provides practical steps for implementing semantic chunking in XZe. Follow these instructions to add intelligent document segmentation based on semantic boundaries.

## Prerequisites

Before starting implementation, ensure you have:

- Rust toolchain installed (latest stable)
- PostgreSQL database set up
- Ollama running locally for embeddings
- XZe development environment configured
- Familiarity with async Rust and tokio

## Quick Start

### Step 1: Set Up Development Environment

```bash
# Clone and navigate to XZe
cd xze

# Install required tools
rustup component add clippy rustfmt

# Verify everything compiles
cargo check --all-targets --all-features
```

### Step 2: Add Dependencies

Add these dependencies to `crates/core/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

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

[dev-dependencies]
# Existing dev dependencies...

# For floating-point comparisons in tests
approx = "0.5"
```

### Step 3: Create Module Structure

```bash
# Create chunking module directory
mkdir -p crates/core/src/chunking

# Create module files
touch crates/core/src/chunking/mod.rs
touch crates/core/src/chunking/chunk.rs
touch crates/core/src/chunking/chunker.rs
touch crates/core/src/chunking/sentence.rs
touch crates/core/src/chunking/similarity.rs
touch crates/core/src/chunking/embeddings.rs
touch crates/core/src/chunking/error.rs
```

### Step 4: Implement Core Types

Start with the foundation in `crates/core/src/chunking/chunk.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticChunk {
    pub content: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub start_sentence: usize,
    pub end_sentence: usize,
    pub avg_similarity: f64,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMetadata {
    pub source_file: String,
    pub title: Option<String>,
    pub category: Option<String>,
    pub keywords: Vec<String>,
    pub word_count: usize,
    pub char_count: usize,
}
```

### Step 5: Run Quality Checks

After each implementation step, verify code quality:

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Run linter (must show zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features
```

## Implementation Phases

### Phase 1: Sentence Splitting (Week 1-2)

**Goal**: Implement intelligent sentence splitting that handles edge cases.

**Files to Create**:
- `crates/core/src/chunking/sentence.rs`

**Key Features**:
- Regex-based sentence detection
- Code block preservation
- Abbreviation handling
- Minimum length filtering

**Testing**:
```bash
cargo test --package xze-core --lib chunking::sentence::tests
```

**Success Criteria**:
- Handles abbreviations (Dr., Mr., etc.)
- Preserves code blocks intact
- Filters short fragments
- All tests pass with >80% coverage

### Phase 2: Similarity Calculations (Week 3-4)

**Goal**: Implement cosine similarity and embedding integration.

**Files to Create**:
- `crates/core/src/chunking/similarity.rs`
- `crates/core/src/chunking/embeddings.rs`

**Key Features**:
- Cosine similarity between vectors
- Pairwise similarity calculations
- Percentile-based thresholds
- Embedding generation with Ollama

**Testing**:
```bash
cargo test --package xze-core --lib chunking::similarity::tests
```

**Success Criteria**:
- Accurate similarity calculations
- Handles dimension mismatches gracefully
- Integration with Ollama client works
- Edge cases covered (zero vectors, etc.)

### Phase 3: Core Chunker (Week 5-7)

**Goal**: Implement the main semantic chunking algorithm.

**Files to Create**:
- `crates/core/src/chunking/chunker.rs`
- `crates/core/src/chunking/mod.rs`
- `crates/core/src/chunking/error.rs`

**Key Features**:
- Boundary detection algorithm
- Adaptive threshold calculation
- Chunk size constraints
- Configuration management

**Testing**:
```bash
cargo test --package xze-core --lib chunking::chunker::tests
```

**Success Criteria**:
- Creates semantically coherent chunks
- Respects min/max sentence limits
- Adaptive threshold works correctly
- Comprehensive error handling

### Phase 4: Database Integration (Week 8-9)

**Goal**: Store and retrieve chunks from PostgreSQL.

**Create Migration**:

```bash
# Create migration file
touch migrations/$(date +%Y%m%d)_add_semantic_chunks.sql
```

**Migration Content**:

```sql
ALTER TABLE documents
ADD COLUMN IF NOT EXISTS chunk_index INTEGER,
ADD COLUMN IF NOT EXISTS total_chunks INTEGER,
ADD COLUMN IF NOT EXISTS start_sentence INTEGER,
ADD COLUMN IF NOT EXISTS end_sentence INTEGER,
ADD COLUMN IF NOT EXISTS chunk_similarity REAL,
ADD COLUMN IF NOT EXISTS title TEXT,
ADD COLUMN IF NOT EXISTS category TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_chunk_index
ON documents(chunk_index);

CREATE INDEX IF NOT EXISTS idx_documents_source_chunk
ON documents(source_file, chunk_index);

UPDATE documents
SET chunk_index = 0,
    total_chunks = 1,
    start_sentence = 0,
    end_sentence = 0,
    chunk_similarity = 1.0
WHERE chunk_index IS NULL;

ALTER TABLE documents
ALTER COLUMN chunk_index SET NOT NULL,
ALTER COLUMN total_chunks SET NOT NULL;
```

**Testing**:
```bash
# Run migration
sqlx migrate run

# Test database operations
cargo test --package xze-core --test chunking_integration
```

**Success Criteria**:
- Migration runs without errors
- Chunks stored and retrieved correctly
- Indexes improve query performance
- Backward compatibility maintained

### Phase 5: Pipeline Integration (Week 10-11)

**Goal**: Integrate chunking into document processing pipeline.

**Files to Modify**:
- Update document loader to use chunking
- Add CLI commands for chunking

**CLI Integration** (`crates/cli/src/commands/chunk.rs`):

```rust
use clap::Args;

#[derive(Args, Debug)]
pub struct ChunkArgs {
    #[arg(short, long)]
    input: String,

    #[arg(short, long, default_value = "default")]
    strategy: String,

    #[arg(long)]
    threshold: Option<f64>,

    #[arg(long)]
    dry_run: bool,
}
```

**Testing**:
```bash
# Test CLI commands
cargo run -- chunk --input docs/ --strategy technical --dry-run
```

**Success Criteria**:
- Documents processed with chunking
- CLI commands functional
- Existing workflows unaffected
- Performance acceptable

### Phase 6: Search Enhancement (Week 12)

**Goal**: Update search to leverage chunk metadata.

**Files to Modify**:
- Update search module to return chunk context
- Enhance result formatting

**Testing**:
```bash
# Test search with chunks
cargo run -- search "test query" --show-chunks
```

**Success Criteria**:
- Search returns chunk metadata
- Results include context information
- Performance comparable to pre-chunking
- Backward compatibility maintained

### Phase 7: Documentation (Week 13-14)

**Goal**: Create comprehensive documentation.

**Documents to Create**:

```bash
touch docs/explanations/semantic_chunking_architecture.md
touch docs/how_to/configure_semantic_chunking.md
touch docs/reference/semantic_chunking_api.md
touch docs/tutorials/semantic_chunking_quick_start.md
```

**Testing**:
```bash
# Verify documentation completeness
cargo doc --no-deps --open

# Check for broken links
markdown-link-check docs/**/*.md
```

**Success Criteria**:
- All API documented with examples
- Migration guide complete
- Troubleshooting section comprehensive
- Examples are runnable

## Configuration Examples

### Default Configuration

```rust
use xze_core::chunking::{ChunkerConfig, SemanticChunker};
use xze_core::ai::ollama::OllamaClient;

let config = ChunkerConfig::default();
let client = OllamaClient::new("http://localhost:11434");
let chunker = SemanticChunker::new(config, client);
```

### Technical Documentation

```rust
let config = ChunkerConfig::technical_docs();
let chunker = SemanticChunker::new(config, client);
```

### Custom Configuration

```rust
let config = ChunkerConfig {
    similarity_threshold: 0.75,
    min_chunk_sentences: 3,
    max_chunk_sentences: 15,
    similarity_percentile: 80,
    min_sentence_length: 15,
    embedding_batch_size: 100,
    model_name: "nomic-embed-text".to_string(),
};

config.validate().expect("Invalid configuration");
let chunker = SemanticChunker::new(config, client);
```

## Testing Strategy

### Unit Tests

Test each module independently:

```bash
# Test sentence splitting
cargo test --package xze-core --lib chunking::sentence

# Test similarity calculations
cargo test --package xze-core --lib chunking::similarity

# Test chunker
cargo test --package xze-core --lib chunking::chunker
```

### Integration Tests

Test end-to-end workflows:

```bash
# Create integration test file
touch crates/core/tests/semantic_chunking_integration.rs
```

### Test Coverage

Ensure >80% coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --package xze-core --out Html
```

## Debugging Tips

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- chunk --input docs/
```

### Trace Embedding Generation

```rust
use tracing::{debug, info};

#[instrument(skip(self, content))]
pub async fn chunk_document(&self, content: &str) -> Result<Vec<SemanticChunk>> {
    info!("Starting chunking");
    debug!("Content length: {}", content.len());
    // ...
}
```

### Inspect Chunk Boundaries

```rust
for (i, chunk) in chunks.iter().enumerate() {
    println!("Chunk {}: sentences {}-{}, similarity: {:.3}",
        i,
        chunk.start_sentence,
        chunk.end_sentence,
        chunk.avg_similarity
    );
}
```

## Common Issues and Solutions

### Issue: Chunks Too Small

**Symptom**: Many chunks with only 2-3 sentences

**Solution**: Adjust configuration

```rust
let config = ChunkerConfig {
    similarity_threshold: 0.75,  // Increase threshold
    min_chunk_sentences: 5,       // Increase minimum
    ..Default::default()
};
```

### Issue: Processing Too Slow

**Symptom**: Takes >30 seconds per document

**Solution**: Optimize embedding generation

```rust
let config = ChunkerConfig {
    embedding_batch_size: 100,    // Larger batches
    model_name: "nomic-embed-text".to_string(),  // Faster model
    ..Default::default()
};
```

### Issue: Similarity Errors

**Symptom**: "Vector dimension mismatch" errors

**Solution**: Verify consistent embedding model

```rust
// Ensure same model used throughout
let model = "nomic-embed-text";
let config = ChunkerConfig {
    model_name: model.to_string(),
    ..Default::default()
};
```

## Quality Checklist

Before submitting each phase:

### Code Quality
- [ ] `cargo fmt --all` applied
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes with >80% coverage

### Documentation
- [ ] All public functions have doc comments
- [ ] Examples in doc comments are runnable
- [ ] Filenames use lowercase_with_underscores.md
- [ ] No emojis in documentation

### Testing
- [ ] Unit tests cover success cases
- [ ] Unit tests cover failure cases
- [ ] Unit tests cover edge cases
- [ ] Integration tests verify end-to-end workflow

### Performance
- [ ] Processing time acceptable (<30s typical docs)
- [ ] Memory usage reasonable
- [ ] Database queries optimized

## Next Steps

After completing implementation:

1. **Performance Benchmarking**
   ```bash
   cargo bench --package xze-core
   ```

2. **User Acceptance Testing**
   - Test with real documentation
   - Gather feedback on chunk quality
   - Tune configuration based on results

3. **Production Deployment**
   - Create release branch
   - Run full test suite
   - Deploy with migration plan

4. **Monitoring**
   - Add metrics for chunk sizes
   - Monitor processing times
   - Track search relevance improvements

## Resources

- Full implementation plan: `docs/explanations/semantic_chunking_implementation_plan.md`
- Implementation summary: `docs/explanations/semantic_chunking_implementation_summary.md`
- Agent guidelines: `AGENTS.md`
- XZe architecture: `docs/explanations/architecture.md`

## Getting Help

If you encounter issues:

1. Check this guide's troubleshooting section
2. Review the full implementation plan
3. Run diagnostics: `cargo check --all-targets --all-features`
4. Enable debug logging: `RUST_LOG=debug`
5. Open an issue with error details

## Conclusion

This guide provides a practical roadmap for implementing semantic chunking in XZe. Follow the phases sequentially, validate at each step, and maintain high code quality standards throughout. The result will be a robust, performant semantic chunking system that significantly improves document processing and retrieval accuracy.
