# Semantic Chunking API Reference

## Overview

This document provides a complete API reference for XZe's semantic chunking system. All types and functions are part of the `xze_core::semantic` module.

## Module Structure

```
xze_core::semantic
├── types        - Core data structures
├── splitter     - Sentence splitting
├── similarity   - Similarity calculations
├── embeddings   - Embedding generation
├── chunker      - Main chunking logic
└── search       - Semantic search
```

## Core Types

### SemanticChunk

Represents a semantically coherent chunk of text.

```rust
pub struct SemanticChunk {
    pub content: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub start_sentence: usize,
    pub end_sentence: usize,
    pub avg_similarity: f32,
    pub metadata: ChunkMetadata,
}
```

#### Fields

- **content**: The actual text content of the chunk
- **chunk_index**: Zero-based index of this chunk in the document
- **total_chunks**: Total number of chunks in the document
- **start_sentence**: Index of the first sentence in this chunk
- **end_sentence**: Index of the last sentence (exclusive)
- **avg_similarity**: Average similarity score within this chunk
- **metadata**: Additional metadata about the chunk

#### Methods

##### new

```rust
pub fn new(
    content: String,
    chunk_index: usize,
    total_chunks: usize,
    start_sentence: usize,
    end_sentence: usize,
    avg_similarity: f32,
    metadata: ChunkMetadata,
) -> Self
```

Creates a new semantic chunk.

**Parameters:**

- All struct fields as separate parameters

**Returns:** New `SemanticChunk` instance

**Example:**

```rust
use xze_core::semantic::{SemanticChunk, ChunkMetadata};

let metadata = ChunkMetadata::new("docs/guide.md".to_string(), "Example content");
let chunk = SemanticChunk::new(
    "This is the chunk content.".to_string(),
    0,
    3,
    0,
    2,
    0.85,
    metadata,
);
```

##### sentence_count

```rust
pub fn sentence_count(&self) -> usize
```

Returns the number of sentences in this chunk.

**Returns:** Count of sentences (end_sentence - start_sentence)

**Example:**

```rust
let count = chunk.sentence_count();
println!("Chunk contains {} sentences", count);
```

### ChunkMetadata

Metadata associated with a chunk.

```rust
pub struct ChunkMetadata {
    pub source_file: String,
    pub title: Option<String>,
    pub category: Option<String>,
    pub keywords: Vec<String>,
    pub word_count: usize,
    pub char_count: usize,
}
```

#### Fields

- **source_file**: Path to the source file
- **title**: Optional document title
- **category**: Optional category classification
- **keywords**: Extracted keywords from content
- **word_count**: Number of words in the chunk
- **char_count**: Number of characters in the chunk

#### Methods

##### new

```rust
pub fn new(source_file: String, content: &str) -> Self
```

Creates metadata from content, automatically extracting keywords and counts.

**Parameters:**

- `source_file`: Path to source file
- `content`: The text content to analyze

**Returns:** New `ChunkMetadata` instance

**Example:**

```rust
use xze_core::semantic::ChunkMetadata;

let metadata = ChunkMetadata::new(
    "docs/api.md".to_string(),
    "The API provides endpoints for authentication.",
);

assert!(metadata.word_count > 0);
assert!(metadata.keywords.len() > 0);
```

##### with_details

```rust
pub fn with_details(
    source_file: String,
    title: Option<String>,
    category: Option<String>,
    keywords: Vec<String>,
    content: &str,
) -> Self
```

Creates metadata with all fields specified.

**Parameters:**

- `source_file`: Path to source file
- `title`: Optional document title
- `category`: Optional Diataxis category
- `keywords`: List of keywords
- `content`: The text content to analyze

**Returns:** New `ChunkMetadata` instance

**Example:**

```rust
use xze_core::semantic::ChunkMetadata;

let metadata = ChunkMetadata::with_details(
    "docs/api.md".to_string(),
    Some("API Guide".to_string()),
    Some("reference".to_string()),
    vec!["api".to_string(), "authentication".to_string()],
    "The API provides endpoints for authentication.",
);

assert_eq!(metadata.title, Some("API Guide".to_string()));
assert_eq!(metadata.keywords.len(), 2);
```

## Sentence Splitting

### SentenceSplitter

Splits text into sentences while preserving code blocks.

```rust
pub struct SentenceSplitter {
    min_sentence_length: usize,
}
```

#### Methods

##### new

```rust
pub fn new(min_sentence_length: usize) -> Self
```

Creates a new sentence splitter.

**Parameters:**

- `min_sentence_length`: Minimum character length for a sentence

**Returns:** New `SentenceSplitter` instance

**Example:**

```rust
use xze_core::semantic::SentenceSplitter;

let splitter = SentenceSplitter::new(10);
```

##### split

```rust
pub fn split(&self, text: &str) -> Vec<String>
```

Splits text into sentences.

**Parameters:**

- `text`: Text to split

**Returns:** Vector of sentence strings

**Features:**

- Preserves code blocks (triple backtick blocks)
- Handles abbreviations (Dr., Mr., etc.)
- Filters sentences shorter than minimum length
- Handles various sentence terminators (., !, ?)

**Example:**

```rust
let splitter = SentenceSplitter::new(10);
let text = "First sentence. Second sentence. Third sentence.";
let sentences = splitter.split(text);

assert_eq!(sentences.len(), 3);
assert_eq!(sentences[0], "First sentence.");
```

##### default

```rust
impl Default for SentenceSplitter
```

Creates splitter with default minimum length of 10 characters.

**Example:**

```rust
let splitter = SentenceSplitter::default();
```

## Similarity Calculations

### cosine_similarity

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, SimilarityError>
```

Calculates cosine similarity between two vectors.

**Parameters:**

- `a`: First vector
- `b`: Second vector

**Returns:** `Result<f32, SimilarityError>` - Similarity score from -1.0 to 1.0

**Errors:**

- `SimilarityError::DimensionMismatch`: Vectors have different lengths
- `SimilarityError::ZeroVector`: One or both vectors are zero vectors

**Example:**

```rust
use xze_core::semantic::cosine_similarity;

let vec1 = vec![1.0, 2.0, 3.0];
let vec2 = vec![4.0, 5.0, 6.0];

let similarity = cosine_similarity(&vec1, &vec2)?;
println!("Similarity: {}", similarity);
```

### pairwise_similarities

```rust
pub fn pairwise_similarities(embeddings: &[Vec<f32>]) -> Result<Vec<f32>, SimilarityError>
```

Calculates similarity between adjacent pairs of vectors.

**Parameters:**

- `embeddings`: Slice of embedding vectors

**Returns:** `Result<Vec<f32>, SimilarityError>` - Vector of similarity scores

**Note:** Returns n-1 scores for n embeddings (each adjacent pair).

**Example:**

```rust
use xze_core::semantic::pairwise_similarities;

let embeddings = vec![
    vec![1.0, 0.0, 0.0],
    vec![0.9, 0.1, 0.0],
    vec![0.0, 1.0, 0.0],
];

let similarities = pairwise_similarities(&embeddings)?;
assert_eq!(similarities.len(), 2); // n-1 pairs
```

### calculate_percentile

```rust
pub fn calculate_percentile(values: &[f32], percentile: f32) -> f32
```

Calculates a percentile value from a dataset.

**Parameters:**

- `values`: Slice of values
- `percentile`: Percentile to calculate (0.0 to 100.0)

**Returns:** The value at the given percentile

**Example:**

```rust
use xze_core::semantic::calculate_percentile;

let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let p75 = calculate_percentile(&values, 75.0);
```

### SimilarityError

Error type for similarity calculations.

```rust
pub enum SimilarityError {
    DimensionMismatch { expected: usize, got: usize },
    ZeroVector(String),
    InvalidValue(String),
}
```

## Embeddings

### generate_embeddings

```rust
pub async fn generate_embeddings(
    client: &OllamaClient,
    text: &str,
    model: &str,
) -> Result<Vec<f32>, EmbeddingError>
```

Generates embedding vector for text.

**Parameters:**

- `client`: Ollama client instance
- `text`: Text to embed
- `model`: Model name (e.g., "nomic-embed-text")

**Returns:** `Result<Vec<f32>, EmbeddingError>` - Embedding vector

**Example:**

```rust
use xze_core::semantic::generate_embeddings;
use xze_core::ai::OllamaClient;

let client = OllamaClient::new("http://localhost:11434");
let embedding = generate_embeddings(&client, "Sample text", "nomic-embed-text").await?;
```

### generate_embeddings_batch

```rust
pub async fn generate_embeddings_batch(
    client: &OllamaClient,
    texts: &[String],
    model: &str,
) -> Result<Vec<Vec<f32>>, EmbeddingError>
```

Generates embeddings for multiple texts.

**Parameters:**

- `client`: Ollama client instance
- `texts`: Slice of texts to embed
- `model`: Model name

**Returns:** `Result<Vec<Vec<f32>>, EmbeddingError>` - Vector of embedding vectors

**Example:**

```rust
use xze_core::semantic::generate_embeddings_batch;

let texts = vec![
    "First sentence".to_string(),
    "Second sentence".to_string(),
];
let embeddings = generate_embeddings_batch(&client, &texts, "nomic-embed-text").await?;
```

### EmbeddingError

Error type for embedding operations.

```rust
pub enum EmbeddingError {
    OllamaError(String),
    InvalidResponse(String),
    EmptyInput,
}
```

## Chunker

### ChunkerConfig

Configuration for semantic chunking.

```rust
pub struct ChunkerConfig {
    pub similarity_threshold: f32,
    pub min_chunk_sentences: usize,
    pub max_chunk_sentences: usize,
    pub similarity_percentile: f32,
    pub min_sentence_length: usize,
    pub embedding_batch_size: usize,
    pub model_name: String,
}
```

#### Methods

##### default

```rust
impl Default for ChunkerConfig
```

Returns default configuration.

**Default values:**

- `similarity_threshold`: 0.5
- `min_chunk_sentences`: 1
- `max_chunk_sentences`: 10
- `similarity_percentile`: 75.0
- `min_sentence_length`: 10
- `embedding_batch_size`: 32
- `model_name`: "nomic-embed-text"

##### technical_docs

```rust
pub fn technical_docs() -> Self
```

Returns configuration optimized for technical documentation.

**Settings:**

- `similarity_threshold`: 0.6
- `min_chunk_sentences`: 2
- `max_chunk_sentences`: 15
- `similarity_percentile`: 70.0
- `min_sentence_length`: 15

##### narrative

```rust
pub fn narrative() -> Self
```

Returns configuration optimized for narrative content.

**Settings:**

- `similarity_threshold`: 0.4
- `min_chunk_sentences`: 1
- `max_chunk_sentences`: 8
- `similarity_percentile`: 80.0
- `min_sentence_length`: 10

##### validate

```rust
pub fn validate(&self) -> Result<(), ChunkingError>
```

Validates configuration parameters.

**Returns:** `Result<(), ChunkingError>`

**Validation rules:**

- Similarity threshold between 0.0 and 1.0
- Min chunk sentences at least 1
- Max chunk sentences >= min chunk sentences
- Similarity percentile between 0.0 and 100.0
- Positive min sentence length and batch size

### SemanticChunker

Main chunking engine.

```rust
pub struct SemanticChunker {
    config: ChunkerConfig,
    ollama_client: OllamaClient,
    sentence_splitter: SentenceSplitter,
}
```

#### Methods

##### new

```rust
pub fn new(config: ChunkerConfig, ollama_client: OllamaClient) -> Self
```

Creates a new semantic chunker.

**Parameters:**

- `config`: Chunker configuration
- `ollama_client`: Ollama client for embeddings

**Returns:** New `SemanticChunker` instance

**Example:**

```rust
use xze_core::semantic::{SemanticChunker, ChunkerConfig};
use xze_core::ai::OllamaClient;

let config = ChunkerConfig::technical_docs();
let client = OllamaClient::new("http://localhost:11434");
let chunker = SemanticChunker::new(config, client);
```

##### chunk_document

```rust
pub async fn chunk_document(
    &self,
    content: &str,
    source_file: &str,
) -> Result<Vec<SemanticChunk>, ChunkingError>
```

Chunks a document into semantic segments.

**Parameters:**

- `content`: Document text to chunk
- `source_file`: Path to source file

**Returns:** `Result<Vec<SemanticChunk>, ChunkingError>`

**Process:**

1. Splits content into sentences
2. Generates embeddings for each sentence
3. Calculates pairwise similarities
4. Detects boundaries based on similarity threshold
5. Creates chunks with metadata

**Example:**

```rust
let content = std::fs::read_to_string("docs/guide.md")?;
let chunks = chunker.chunk_document(&content, "docs/guide.md").await?;

for chunk in chunks {
    println!("Chunk {}/{}: {} sentences",
        chunk.chunk_index + 1,
        chunk.total_chunks,
        chunk.sentence_count());
}
```

### ChunkingError

Error type for chunking operations.

```rust
pub enum ChunkingError {
    EmbeddingGeneration(String),
    SimilarityCalculation(String),
    InvalidConfiguration(String),
    EmptyDocument,
    SentenceSplitting(String),
    OllamaError(String),
}
```

## Search

### ChunkSearchResult

Search result containing chunk information.

```rust
pub struct ChunkSearchResult {
    pub id: i64,
    pub source_file: String,
    pub content: String,
    pub similarity: f32,
    pub chunk_index: i32,
    pub total_chunks: i32,
    pub title: Option<String>,
    pub category: Option<String>,
    pub sentence_range: (i32, i32),
    pub avg_chunk_similarity: f32,
}
```

#### Fields

- **id**: Database ID of the chunk
- **source_file**: Path to source file
- **content**: Chunk text content
- **similarity**: Similarity score to query (0.0 to 1.0)
- **chunk_index**: Index of chunk in document
- **total_chunks**: Total chunks in document
- **title**: Optional document title
- **category**: Optional category
- **sentence_range**: (start, end) sentence indices
- **avg_chunk_similarity**: Average similarity within chunk

### SearchConfig

Configuration for semantic search.

```rust
pub struct SearchConfig {
    pub max_results: usize,
    pub min_similarity: f32,
    pub category_filter: Option<String>,
}
```

#### Methods

##### default

```rust
impl Default for SearchConfig
```

Default search configuration.

**Defaults:**

- `max_results`: 10
- `min_similarity`: 0.0
- `category_filter`: None

### search_with_chunks

```rust
pub async fn search_with_chunks(
    pool: &PgPool,
    query: &str,
    ollama_url: &str,
    config: SearchConfig,
) -> Result<Vec<ChunkSearchResult>, SearchError>
```

Searches chunks semantically based on query.

**Parameters:**

- `pool`: Database connection pool
- `query`: Search query text
- `ollama_url`: Ollama server URL
- `config`: Search configuration

**Returns:** `Result<Vec<ChunkSearchResult>, SearchError>` - Ranked results

**Process:**

1. Generates embedding for query
2. Fetches chunks from database (optionally filtered)
3. Calculates similarity between query and each chunk
4. Filters by minimum similarity
5. Sorts by similarity (descending)
6. Returns top results

**Example:**

```rust
use xze_core::semantic::{search_with_chunks, SearchConfig};
use sqlx::PgPool;

let pool = PgPool::connect(&database_url).await?;
let config = SearchConfig {
    max_results: 5,
    min_similarity: 0.7,
    category_filter: Some("tutorial".to_string()),
};

let results = search_with_chunks(
    &pool,
    "how to configure authentication",
    "http://localhost:11434",
    config,
).await?;

for result in results {
    println!("Similarity: {:.2} - {}", result.similarity, result.source_file);
}
```

### SearchError

Error type for search operations.

```rust
pub enum SearchError {
    DatabaseError(String),
    EmbeddingError(String),
    InvalidQuery(String),
    NoResults,
}
```

## Complete Example

```rust
use xze_core::semantic::{
    SemanticChunker, ChunkerConfig, search_with_chunks, SearchConfig
};
use xze_core::ai::OllamaClient;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let database_url = std::env::var("DATABASE_URL")?;
    let ollama_url = "http://localhost:11434";

    let pool = PgPool::connect(&database_url).await?;
    let client = OllamaClient::new(ollama_url);

    // Configure chunker
    let config = ChunkerConfig::technical_docs();
    let chunker = SemanticChunker::new(config, client);

    // Chunk document
    let content = std::fs::read_to_string("docs/api.md")?;
    let chunks = chunker.chunk_document(&content, "docs/api.md").await?;

    println!("Created {} chunks", chunks.len());

    // Store chunks (using kb::store module)
    // ... storage code ...

    // Search chunks
    let search_config = SearchConfig {
        max_results: 5,
        min_similarity: 0.7,
        category_filter: None,
    };

    let results = search_with_chunks(
        &pool,
        "authentication endpoints",
        ollama_url,
        search_config,
    ).await?;

    for result in results {
        println!("Found: {} (similarity: {:.2})",
            result.source_file,
            result.similarity);
    }

    Ok(())
}
```

## Best Practices

### Configuration

1. Start with a preset configuration
2. Validate before use
3. Test on sample documents
4. Adjust incrementally

### Error Handling

Always handle errors appropriately:

```rust
match chunker.chunk_document(&content, path).await {
    Ok(chunks) => process_chunks(chunks),
    Err(ChunkingError::EmptyDocument) => {
        eprintln!("Document is empty");
    }
    Err(ChunkingError::EmbeddingGeneration(msg)) => {
        eprintln!("Embedding failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Chunking error: {}", e);
    }
}
```

### Performance

1. Use batch processing for multiple documents
2. Adjust `embedding_batch_size` based on document size
3. Consider caching embeddings for frequently accessed content
4. Use connection pooling for database operations

### Testing

Write tests for custom configurations:

```rust
#[tokio::test]
async fn test_custom_chunker() {
    let mut config = ChunkerConfig::default();
    config.max_chunk_sentences = 5;

    assert!(config.validate().is_ok());

    // Test chunking behavior
}
```

## See Also

- Semantic Chunking Tutorial for hands-on guide
- Configuration How-To for detailed parameter tuning
- Search Command Reference for CLI usage
