# XZe - AI-Powered Documentation Generator

XZe is an intelligent documentation tool that uses AI-powered semantic chunking to automatically analyze, organize, and generate high-quality documentation from your codebase.

## Features

### Semantic Chunking

XZe's semantic chunking breaks documents into meaningful segments based on topical coherence rather than arbitrary size limits:

- **Intelligent Segmentation**: Analyzes sentence similarity using embeddings to determine natural boundaries
- **Code Preservation**: Maintains code blocks and formatting integrity
- **Adaptive Configuration**: Optimized presets for technical docs, tutorials, and narrative content
- **Search-Optimized**: Improves search relevance by maintaining context within chunks

### AI-Powered Analysis

- **Ollama Integration**: Leverages local LLM models for embedding generation and analysis
- **Semantic Search**: Find relevant documentation using natural language queries
- **Automated Extraction**: Identifies keywords, categories, and metadata automatically

### Git Integration

- **Change Detection**: Tracks documentation changes alongside code
- **Version Awareness**: Maintains documentation history through Git
- **Automated Updates**: Triggers documentation regeneration on commits

### Documentation Pipeline

- **Diataxis Framework**: Organizes documentation into Tutorials, How-To Guides, Explanations, and Reference
- **Multiple Formats**: Supports Markdown, code comments, and structured documentation
- **Database Storage**: PostgreSQL-backed knowledge base for fast retrieval

## Quick Start

### Prerequisites

- Rust 1.70 or later
- PostgreSQL 14 or later
- Ollama with embedding model (e.g., `nomic-embed-text`)

### Installation

```bash
cargo install xze
```

Or build from source:

```bash
git clone https://github.com/yourusername/xze.git
cd xze
cargo build --release
```

### Configuration

Set up your environment:

```bash
export DATABASE_URL="postgresql://localhost/xze"
export OLLAMA_URL="http://localhost:11434"
```

Run database migrations:

```bash
sqlx migrate run
```

### Pull Embedding Model

```bash
ollama pull nomic-embed-text
```

## Usage

### Chunking Documents

Process a single document:

```bash
xze chunk docs/guide.md
```

Process multiple documents:

```bash
xze chunk docs/ --output chunks_report.json
```

Use configuration presets:

```bash
# For technical documentation
xze chunk api_docs/ --strategy technical

# For narrative content
xze chunk tutorials/ --strategy narrative
```

Customize chunking behavior:

```bash
xze chunk docs/ --threshold 0.6 --max-sentences 15
```

Preview without storing:

```bash
xze chunk docs/guide.md --dry-run
```

### Searching Documents

Basic search:

```bash
xze search "how to configure authentication"
```

Filtered search:

```bash
xze search "database setup" --category tutorial --min-similarity 0.7
```

Limit results:

```bash
xze search "installation steps" --max-results 5
```

JSON output:

```bash
xze search "API endpoints" --json
```

Verbose output with full content:

```bash
xze search "configuration" --full-content --verbose
```

## Programmatic Usage

Use XZe as a library in your Rust application:

```rust
use xze_core::semantic::{SemanticChunker, ChunkerConfig, search_with_chunks, SearchConfig};
use xze_core::ai::OllamaClient;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;

    // Create Ollama client
    let ollama = OllamaClient::new("http://localhost:11434");

    // Configure and create chunker
    let config = ChunkerConfig::technical_docs();
    let chunker = SemanticChunker::new(config, ollama);

    // Process document
    let content = std::fs::read_to_string("docs/api.md")?;
    let chunks = chunker.chunk_document(&content, "docs/api.md").await?;

    println!("Created {} chunks", chunks.len());

    // Search chunks
    let search_config = SearchConfig {
        max_results: 5,
        min_similarity: 0.7,
        category_filter: None,
    };

    let results = search_with_chunks(
        &pool,
        "authentication endpoints",
        "http://localhost:11434",
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

## Configuration

### Chunking Configuration

XZe provides several configuration options for semantic chunking:

**Default Configuration**:

```rust
ChunkerConfig::default()
```

**Technical Documentation** (API docs, references, tutorials):

```rust
ChunkerConfig::technical_docs()
// - Similarity threshold: 0.6 (stricter boundaries)
// - Max sentences: 15 (larger chunks)
// - Min sentence length: 15
```

**Narrative Content** (guides, blog posts, explanations):

```rust
ChunkerConfig::narrative()
// - Similarity threshold: 0.4 (gentler boundaries)
// - Max sentences: 8 (smaller chunks)
// - Similarity percentile: 80.0
```

**Custom Configuration**:

```rust
let config = ChunkerConfig {
    similarity_threshold: 0.5,
    min_chunk_sentences: 2,
    max_chunk_sentences: 12,
    similarity_percentile: 75.0,
    min_sentence_length: 10,
    embedding_batch_size: 32,
    model_name: "nomic-embed-text".to_string(),
};
```

### Search Configuration

```rust
let search_config = SearchConfig {
    max_results: 10,           // Maximum results to return
    min_similarity: 0.0,       // Minimum similarity threshold (0.0-1.0)
    category_filter: None,     // Optional category filter
};
```

## Architecture

XZe follows a clean, modular architecture:

```
xze/
├── crates/
│   ├── cli/          # Command-line interface
│   ├── serve/        # Server mode and webhooks
│   └── core/         # Core business logic
│       ├── semantic/ # Semantic chunking
│       ├── ai/       # Ollama integration
│       ├── kb/       # Knowledge base
│       └── git/      # Git integration
├── docs/
│   ├── tutorials/    # Learning-oriented guides
│   ├── how_to/       # Task-oriented recipes
│   ├── explanations/ # Understanding-oriented docs
│   └── reference/    # Information-oriented specs
└── migrations/       # Database migrations
```

## Documentation

### For Users

- [Semantic Chunking Tutorial](docs/tutorials/semantic_chunking_tutorial.md) - Step-by-step guide to using semantic chunking
- [Configuration How-To](docs/how_to/chunking_configuration.md) - Detailed configuration guide
- [Search Command Reference](docs/reference/search_command_reference.md) - Complete CLI reference

### For Developers

- [Semantic Chunking API Reference](docs/reference/semantic_chunking_api.md) - Complete API documentation
- [Implementation Plan](docs/explanations/semantic_chunking_implementation_plan.md) - Technical design and architecture
- [Phase 6 Implementation](docs/explanations/phase_6_search_integration_implementation.md) - Search integration details

### Development Guidelines

- [AGENTS.md](AGENTS.md) - Guidelines for AI agents and contributors

## How It Works

### Semantic Chunking Process

1. **Sentence Splitting**: Documents are split into sentences while preserving code blocks and formatting
2. **Embedding Generation**: Each sentence is converted to a vector embedding using Ollama
3. **Similarity Analysis**: Cosine similarity is calculated between adjacent sentences
4. **Boundary Detection**: Chunk boundaries are identified where similarity drops below threshold
5. **Chunk Creation**: Semantic chunks are created with metadata (keywords, counts, categories)
6. **Storage**: Chunks and embeddings are stored in PostgreSQL for fast retrieval

### Search Process

1. **Query Embedding**: Search query is converted to vector embedding
2. **Similarity Matching**: Query embedding is compared to all chunk embeddings
3. **Ranking**: Results are ranked by cosine similarity score
4. **Filtering**: Results are filtered by minimum similarity and category
5. **Return**: Top-ranked chunks are returned with metadata

## Performance

### Processing Time

- Small document (1-5 KB): ~1-2 seconds
- Medium document (5-50 KB): ~3-10 seconds
- Large document (50-500 KB): ~15-60 seconds

Performance depends on:

- Ollama server response time
- Document complexity
- Embedding batch size
- Database connection speed

### Database Size

Typical storage requirements:

- Chunk text: ~1 KB per chunk
- Embedding vector (768 dimensions): ~3 KB per chunk
- Metadata: ~0.5 KB per chunk
- Total: ~4.5 KB per chunk on average

### Search Performance

- Query embedding generation: ~100-500ms
- Database query and similarity calculation: ~50-200ms per 1000 chunks
- Total search time: ~200-700ms for typical knowledge bases

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Lint with Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features
```

### Running Locally

```bash
# Start Ollama
ollama serve

# Start PostgreSQL
# (system-specific)

# Run XZe
cargo run -- chunk docs/
```

## Troubleshooting

### Ollama Connection Issues

**Error**: `Failed to connect to Ollama at http://localhost:11434`

**Solution**:

1. Verify Ollama is running: `curl http://localhost:11434/api/tags`
2. Start Ollama: `ollama serve`
3. Ensure embedding model is pulled: `ollama pull nomic-embed-text`

### Database Connection Issues

**Error**: `Failed to connect to database`

**Solution**:

1. Verify PostgreSQL is running
2. Check DATABASE_URL environment variable
3. Test connection: `psql $DATABASE_URL -c "SELECT 1"`
4. Run migrations: `sqlx migrate run`

### No Chunks Generated

**Possible causes**:

- Document too short (minimum 2 sentences)
- Sentences too short (check `min_sentence_length`)
- Configuration too restrictive

**Solution**:

```bash
# Use dry-run to debug
xze chunk sample.md --dry-run --verbose

# Try with more permissive settings
xze chunk sample.md --threshold 0.3 --max-sentences 20
```

### Poor Search Results

**Solutions**:

1. Increase minimum similarity: `--min-similarity 0.7`
2. Re-chunk with stricter boundaries: `--threshold 0.6`
3. Verify embedding model is loaded in Ollama
4. Use more specific queries

## Contributing

Contributions are welcome! Please:

1. Read [AGENTS.md](AGENTS.md) for development guidelines
2. Follow Rust best practices and project conventions
3. Ensure all tests pass: `cargo test --all-features`
4. Run quality checks: `cargo fmt && cargo clippy`
5. Add tests for new features
6. Update documentation

## License

[Add your license here]

## Acknowledgments

- Ollama for local LLM inference
- PostgreSQL for robust data storage
- The Rust community for excellent tooling

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/yourusername/xze/issues)
- Discussions: [GitHub Discussions](https://github.com/yourusername/xze/discussions)
