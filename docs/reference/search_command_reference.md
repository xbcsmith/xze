# Search Command Reference

## Overview

The `xze search` command provides semantic search capabilities for finding relevant document chunks using natural language queries. Results are ranked by similarity to the query using cosine similarity of embeddings.

## Synopsis

```bash
xze search [OPTIONS] <QUERY>
```

## Arguments

### Required

**`<QUERY>`**
- Search query text (natural language)
- The query will be embedded and compared against all chunks
- Must be non-empty
- Examples: "how to install", "error handling best practices"

## Options

### Result Control

**`-n, --max-results <N>`**
- Maximum number of results to return
- Default: 10
- Must be greater than 0
- Results are sorted by similarity (highest first)

**`-s, --min-similarity <THRESHOLD>`**
- Minimum similarity threshold (0.0 to 1.0)
- Default: 0.0 (no filtering)
- Only results above this threshold are returned
- Higher values = more strict matching
- Recommended ranges:
  - 0.0-0.3: Very broad matching
  - 0.3-0.6: Moderate relevance
  - 0.6-0.8: High relevance
  - 0.8-1.0: Very strict matching

**`-c, --category <CATEGORY>`**
- Filter results by document category
- Optional (searches all categories if not specified)
- Valid values:
  - `tutorial` - Learning-oriented guides
  - `how_to` - Task-oriented procedures
  - `explanation` - Understanding-oriented discussions
  - `reference` - Information-oriented specifications

### Connection Settings

**`--ollama-url <URL>`**
- Ollama server URL for generating embeddings
- Default: `http://localhost:11434`
- Must be accessible and have nomic-embed-text model

**`--database-url <URL>`**
- PostgreSQL connection string
- Required (or set DATABASE_URL environment variable)
- Format: `postgresql://[user[:password]@][host][:port][/dbname]`

### Output Control

**`--json`**
- Output results in JSON format
- Default: human-readable format
- Useful for scripting and automation

**`--full-content`**
- Show complete chunk content
- Default: content is truncated to 300 characters
- Use for detailed examination of results

**`-v, --verbose`**
- Show additional metadata
- Includes: chunk indices, sentence ranges, internal similarity scores
- Useful for debugging and understanding results

## Examples

### Basic Usage

```bash
# Simple search
xze search "how to configure logging" --database-url postgresql://localhost/xze

# Using environment variable for database
export DATABASE_URL=postgresql://localhost/xze
xze search "error handling"
```

### Limiting Results

```bash
# Get top 5 results
xze search "installation steps" --max-results 5

# Only high-confidence matches
xze search "API endpoints" --min-similarity 0.7

# Combine both
xze search "deployment guide" --max-results 3 --min-similarity 0.5
```

### Category Filtering

```bash
# Search only tutorials
xze search "getting started" --category tutorial

# Search only how-to guides
xze search "configure database" --category how_to

# Search only reference documentation
xze search "API methods" --category reference

# Search only explanations
xze search "architecture overview" --category explanation
```

### Output Formats

```bash
# JSON output for scripting
xze search "monitoring" --json > results.json

# Verbose output with metadata
xze search "testing" --verbose

# Full content without truncation
xze search "security" --full-content

# Combine options
xze search "performance" --json --verbose --full-content
```

### Advanced Examples

```bash
# High-precision search in specific category
xze search "REST API authentication" \
  --category reference \
  --min-similarity 0.6 \
  --max-results 10

# Broad exploratory search
xze search "best practices" \
  --min-similarity 0.3 \
  --max-results 20 \
  --verbose

# Scripting with jq
xze search "configuration" --json | \
  jq -r '.results[].source_file' | \
  sort -u
```

## Output Format

### Human-Readable Format

```
Found 3 result(s) for: 'how to install'

================================================================================

1. docs/how_to/installation.md (Similarity: 92.45%)
   Title: Installation Guide
   Category: how_to

   Content:
   To install XZe, first ensure you have Rust installed...

--------------------------------------------------------------------------------

2. docs/tutorials/getting_started.md (Similarity: 78.32%)
   Title: Getting Started with XZe
   Category: tutorial

   Content:
   This tutorial will guide you through installing and...

--------------------------------------------------------------------------------

Showing 2 of 2 results
```

### JSON Format

```json
{
  "query": "how to install",
  "result_count": 2,
  "max_results": 10,
  "min_similarity": 0.0,
  "category_filter": null,
  "results": [
    {
      "id": 42,
      "source_file": "docs/how_to/installation.md",
      "content": "To install XZe, first ensure...",
      "similarity": 0.9245,
      "chunk_index": 1,
      "total_chunks": 5,
      "title": "Installation Guide",
      "category": "how_to",
      "sentence_range": {
        "start": 5,
        "end": 12
      },
      "avg_chunk_similarity": 0.85
    }
  ]
}
```

## Exit Status

- **0** - Success
- **1** - Error (invalid arguments, connection failure, search failure)

## Environment Variables

**`DATABASE_URL`**
- PostgreSQL connection string
- Alternative to --database-url flag
- Example: `postgresql://user:pass@localhost:5432/xze`

**`OLLAMA_URL`**
- Can be set to override default Ollama URL
- Alternative to --ollama-url flag

**`RUST_LOG`**
- Control logging level
- Values: error, warn, info, debug, trace
- Example: `RUST_LOG=debug xze search "query"`

## Prerequisites

1. PostgreSQL database with semantic_chunks table
2. Ollama server running with nomic-embed-text model
3. Document chunks in database (from `xze chunk` command)

## Common Use Cases

### Finding Installation Instructions

```bash
xze search "how to install" --category how_to --max-results 3
```

### API Reference Lookup

```bash
xze search "API methods for user management" --category reference
```

### Understanding Concepts

```bash
xze search "architecture overview" --category explanation
```

### Learning Tutorials

```bash
xze search "getting started guide" --category tutorial
```

### Troubleshooting

```bash
xze search "error: connection timeout" --min-similarity 0.5
```

## Performance Tips

1. Use category filters to reduce search space
2. Set appropriate min_similarity to filter irrelevant results
3. Limit max_results for faster responses
4. For large knowledge bases, consider higher similarity thresholds

## Troubleshooting

### No Results Found

Try:
- Lowering --min-similarity threshold
- Removing --category filter
- Using different keywords or phrasing
- Verifying chunks exist in database with SQL:
  ```sql
  SELECT COUNT(*) FROM semantic_chunks;
  ```

### Database Connection Failed

Check:
- DATABASE_URL is set correctly
- PostgreSQL is running
- Network connectivity
- Database credentials

### Ollama Connection Failed

Check:
- Ollama server is running: `curl http://localhost:11434`
- nomic-embed-text model is available: `ollama list`
- --ollama-url is correct

### Slow Performance

Consider:
- Reducing --max-results
- Adding --category filter
- Setting higher --min-similarity
- For large datasets (>10,000 chunks), consider pgvector extension

## Related Commands

- `xze chunk` - Chunk documents for searching
- `xze load` - Load documents into knowledge base
- `xze analyze` - Analyze and generate documentation

## See Also

- Search Implementation: `docs/explanations/phase_6_search_integration_implementation.md`
- Semantic Chunking: `docs/explanations/semantic_chunking_implementation_plan.md`
- CLI Commands: `docs/reference/cli_commands.md`
