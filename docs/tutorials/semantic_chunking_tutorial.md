# Semantic Chunking Tutorial

## Introduction

This tutorial will guide you through using XZe's semantic chunking feature to intelligently segment documents into semantically coherent chunks. By the end of this tutorial, you will understand how to configure, process, and search documents using semantic chunking.

## What is Semantic Chunking?

Semantic chunking breaks documents into meaningful segments based on topical coherence rather than arbitrary size limits. It analyzes sentence similarity using embeddings to determine natural boundaries in your content.

### Benefits

- Maintains context within chunks
- Preserves code blocks and formatting
- Improves search relevance
- Adapts to content type (technical docs vs. narrative)

## Prerequisites

Before starting, ensure you have:

- XZe installed and configured
- PostgreSQL database running
- Ollama server running with embedding model (e.g., `nomic-embed-text`)
- Sample documentation files to process

## Step 1: Understanding the Configuration

Semantic chunking uses several configurable parameters:

### Basic Configuration

```rust
use xze_core::semantic::ChunkerConfig;

let config = ChunkerConfig::default();
```

The default configuration uses:
- Similarity threshold: 0.5 (percentile-based)
- Minimum chunk sentences: 1
- Maximum chunk sentences: 10
- Similarity percentile: 75th percentile
- Minimum sentence length: 10 characters
- Batch size: 32 sentences
- Model: "nomic-embed-text"

### Configuration Presets

For technical documentation:

```rust
let config = ChunkerConfig::technical_docs();
```

For narrative content:

```rust
let config = ChunkerConfig::narrative();
```

## Step 2: Processing Your First Document

Let's process a simple markdown document:

### Create Sample Content

Create a file `sample.md`:

```markdown
# Getting Started

This guide will help you get started with the system.
First, you need to install the required dependencies.

## Installation

Run the following command to install:

```bash
cargo install xze
```

After installation, verify it works by running the help command.

## Configuration

Create a configuration file in your project root.
The configuration uses YAML format.
Here is an example configuration:

```yaml
database_url: postgresql://localhost/xze
ollama_url: http://localhost:11434
```

Save this as config.yaml in your project.
```

### Process the Document

Use the XZe CLI to process the document:

```bash
xze chunk sample.md --output chunks.json
```

This will:
1. Split the document into sentences
2. Generate embeddings for each sentence
3. Calculate similarity between adjacent sentences
4. Create chunks at natural boundaries
5. Store chunks in the database
6. Output chunk information to `chunks.json`

### Understanding the Output

The output shows chunk information:

```json
{
  "chunks": [
    {
      "content": "This guide will help you get started...",
      "chunk_index": 0,
      "total_chunks": 4,
      "start_sentence": 0,
      "end_sentence": 2,
      "avg_similarity": 0.82,
      "metadata": {
        "source_file": "sample.md",
        "title": "Getting Started",
        "word_count": 45,
        "char_count": 234
      }
    }
  ]
}
```

## Step 3: Customizing Chunking Behavior

### Adjusting Similarity Threshold

For more granular chunks (more boundaries):

```bash
xze chunk sample.md --threshold 0.6
```

For larger chunks (fewer boundaries):

```bash
xze chunk sample.md --threshold 0.4
```

### Limiting Chunk Size

Control maximum sentences per chunk:

```bash
xze chunk sample.md --max-sentences 5
```

### Dry Run Mode

Preview chunking without storing:

```bash
xze chunk sample.md --dry-run
```

## Step 4: Searching Chunked Documents

After processing documents, you can search them semantically:

### Basic Search

```bash
xze search "how to install the system"
```

This will:
1. Generate an embedding for your query
2. Compare it to all chunk embeddings
3. Return the most relevant chunks ranked by similarity

### Filtered Search

Search within a specific category:

```bash
xze search "configuration examples" --category tutorial
```

Limit results:

```bash
xze search "installation steps" --max-results 5
```

Set minimum similarity threshold:

```bash
xze search "database setup" --min-similarity 0.7
```

### Output Formats

Human-readable format (default):

```bash
xze search "getting started"
```

JSON output:

```bash
xze search "getting started" --json
```

Verbose output with full content:

```bash
xze search "configuration" --full-content --verbose
```

## Step 5: Processing Multiple Documents

Process an entire documentation directory:

```bash
xze chunk docs/ --output chunks_report.json
```

This recursively processes all markdown files in the directory.

## Step 6: Integrating with Your Workflow

### Using in a CI/CD Pipeline

Create a script to process documentation on commit:

```bash
#!/bin/bash
# process_docs.sh

export DATABASE_URL="postgresql://localhost/xze"
export OLLAMA_URL="http://localhost:11434"

echo "Processing documentation..."
xze chunk docs/ --output build/chunks.json

echo "Verifying chunks..."
xze search "test query" --max-results 1

echo "Documentation processing complete"
```

### Programmatic Usage

Use XZe as a library in your Rust application:

```rust
use xze_core::semantic::{SemanticChunker, ChunkerConfig};
use xze_core::ai::OllamaClient;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;

    // Create Ollama client
    let ollama_url = std::env::var("OLLAMA_URL")?;
    let ollama = OllamaClient::new(&ollama_url);

    // Configure chunker
    let config = ChunkerConfig::technical_docs();
    let chunker = SemanticChunker::new(config, ollama);

    // Read document
    let content = std::fs::read_to_string("sample.md")?;

    // Process document
    let chunks = chunker.chunk_document(&content, "sample.md").await?;

    println!("Created {} chunks", chunks.len());

    // Store chunks in database
    for chunk in chunks {
        // Store using xze_core::kb::store::store_chunks
        println!("Chunk {}: {} sentences",
                 chunk.chunk_index,
                 chunk.sentence_count());
    }

    Ok(())
}
```

## Common Use Cases

### Technical Documentation

Best for API docs, tutorials, and reference material:

```rust
let config = ChunkerConfig::technical_docs();
```

Characteristics:
- Higher similarity threshold (0.6)
- Larger chunks (up to 15 sentences)
- Preserves code blocks
- Better for structured content

### Narrative Content

Best for guides, blog posts, and explanatory content:

```rust
let config = ChunkerConfig::narrative();
```

Characteristics:
- Lower similarity threshold (0.4)
- Smaller chunks (up to 8 sentences)
- Follows topical flow
- Better for conversational content

## Troubleshooting

### Ollama Connection Issues

If you see connection errors:

```bash
Error: Failed to connect to Ollama at http://localhost:11434
```

Verify Ollama is running:

```bash
curl http://localhost:11434/api/tags
```

Start Ollama if needed:

```bash
ollama serve
```

### Database Connection Issues

If database connection fails:

```bash
Error: Failed to connect to database
```

Verify PostgreSQL is running and DATABASE_URL is correct:

```bash
psql $DATABASE_URL -c "SELECT 1"
```

### No Chunks Generated

If chunking produces no results, check:

1. Document has enough content (minimum 2 sentences)
2. Sentences meet minimum length requirement (10 chars by default)
3. Configuration thresholds are not too restrictive

### Poor Search Results

If search returns irrelevant results:

1. Increase `--min-similarity` threshold
2. Verify documents were chunked with appropriate configuration
3. Check that query is specific enough
4. Ensure embedding model is loaded in Ollama

## Next Steps

Now that you understand basic semantic chunking:

- Read the Configuration Guide for advanced options
- Explore the API Reference for programmatic usage
- Review the How-To guides for specific tasks
- Experiment with different configurations for your content type

## Summary

In this tutorial, you learned:

- What semantic chunking is and why it's useful
- How to configure chunking behavior
- How to process documents from CLI and code
- How to search chunked documents
- How to integrate chunking into your workflow
- Common troubleshooting steps

Semantic chunking improves search relevance by maintaining topical coherence within chunks, making it easier to find exactly what you need in your documentation.
