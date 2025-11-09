# Search API Endpoint Reference

## Overview

The XZe server provides a RESTful API endpoint for performing semantic search across indexed documentation chunks. This endpoint leverages embeddings-based similarity matching to find relevant content based on natural language queries.

## Endpoint

```
GET /search
```

## Query Parameters

| Parameter        | Type    | Required | Default | Description                                                                               |
| ---------------- | ------- | -------- | ------- | ----------------------------------------------------------------------------------------- |
| `q`              | string  | Yes      | -       | The search query string. Must not be empty.                                               |
| `max_results`    | integer | No       | `10`    | Maximum number of results to return. Must be greater than 0.                              |
| `min_similarity` | float   | No       | `0.0`   | Minimum similarity threshold (0.0 to 1.0). Results below this threshold are filtered out. |
| `category`       | string  | No       | -       | Filter results by Diátaxis category: `tutorial`, `how_to`, `reference`, or `explanation`. |

## Request Examples

### Basic Search

```bash
curl "http://localhost:3000/search?q=how+to+install"
```

### Search with Result Limit

```bash
curl "http://localhost:3000/search?q=configuration+options&max_results=5"
```

### Search with Similarity Threshold

```bash
curl "http://localhost:3000/search?q=API+endpoints&min_similarity=0.7"
```

### Search with Category Filter

```bash
curl "http://localhost:3000/search?q=getting+started&category=tutorial"
```

### Complex Search Query

```bash
curl "http://localhost:3000/search?q=deployment+guide&max_results=3&min_similarity=0.5&category=how_to"
```

## Response Format

### Success Response (200 OK)

```json
{
  "query": "how to install",
  "results": [
    {
      "id": 42,
      "source_file": "docs/how_to/installation.md",
      "content": "To install XZe, you can use cargo install or download...",
      "similarity": 0.92,
      "chunk_index": 0,
      "total_chunks": 5,
      "title": "Installation Guide",
      "category": "how_to",
      "sentence_range": [0, 8],
      "avg_chunk_similarity": 0.85
    },
    {
      "id": 73,
      "source_file": "docs/tutorial/getting_started.md",
      "content": "Before installing XZe, ensure you have Rust 1.70 or later...",
      "similarity": 0.87,
      "chunk_index": 1,
      "total_chunks": 12,
      "title": "Getting Started with XZe",
      "category": "tutorial",
      "sentence_range": [5, 12],
      "avg_chunk_similarity": 0.78
    }
  ],
  "total_results": 2,
  "config": {
    "max_results": 10,
    "min_similarity": 0.0,
    "category_filter": null
  }
}
```

### Response Fields

#### Top-Level Fields

- **query** (string): The original search query
- **results** (array): Array of search result items, sorted by similarity (descending)
- **total_results** (integer): Number of results returned
- **config** (object): The search configuration used for this query

#### Result Item Fields

- **id** (integer): Unique database identifier for the chunk
- **source_file** (string): Path to the source documentation file
- **content** (string): Text content of the matching chunk
- **similarity** (float): Cosine similarity score between query and chunk (0.0 to 1.0)
- **chunk_index** (integer): Zero-based index of this chunk within its document
- **total_chunks** (integer): Total number of chunks in the source document
- **title** (string | null): Document title from metadata
- **category** (string | null): Diátaxis category (`tutorial`, `how_to`, `reference`, `explanation`)
- **sentence_range** (array): Two-element array [start, end] indicating sentence indices
- **avg_chunk_similarity** (float): Average semantic similarity between sentences within the chunk

#### Config Fields

- **max_results** (integer): Maximum results limit applied
- **min_similarity** (float): Minimum similarity threshold applied
- **category_filter** (string | null): Category filter applied, if any

### Error Responses

#### 400 Bad Request - Empty Query

```json
{
  "error": "Query string cannot be empty",
  "details": null
}
```

#### 400 Bad Request - Invalid Configuration

```json
{
  "error": "max_results must be greater than 0",
  "details": "InvalidConfig: max_results must be greater than 0"
}
```

#### 502 Bad Gateway - Embedding Generation Failed

```json
{
  "error": "Failed to generate query embedding",
  "details": "EmbeddingGeneration: Connection to Ollama failed"
}
```

#### 500 Internal Server Error - Database Error

```json
{
  "error": "Database error occurred",
  "details": "Database: Connection pool exhausted"
}
```

## Similarity Scoring

The API uses cosine similarity to measure how closely a document chunk matches the search query:

- **0.9 - 1.0**: Extremely relevant, nearly exact semantic match
- **0.7 - 0.9**: Highly relevant, strong semantic similarity
- **0.5 - 0.7**: Moderately relevant, some semantic overlap
- **0.3 - 0.5**: Loosely relevant, weak semantic connection
- **0.0 - 0.3**: Minimally relevant, little to no semantic connection

### Recommended Thresholds

- **General search**: `min_similarity=0.0` (return all results, let similarity score guide)
- **High precision**: `min_similarity=0.7` (only very relevant results)
- **Balanced**: `min_similarity=0.5` (filter out weakly related results)

## Usage Examples

### Python

```python
import requests

def search_docs(query, max_results=10, min_similarity=0.0, category=None):
    params = {
        'q': query,
        'max_results': max_results,
        'min_similarity': min_similarity
    }
    if category:
        params['category'] = category

    response = requests.get('http://localhost:3000/search', params=params)
    response.raise_for_status()
    return response.json()

# Basic search
results = search_docs("how to configure logging")
for result in results['results']:
    print(f"[{result['similarity']:.2f}] {result['source_file']}")
    print(f"  {result['content'][:100]}...")

# Filtered search
tutorials = search_docs(
    "getting started",
    max_results=5,
    min_similarity=0.6,
    category="tutorial"
)
```

### JavaScript/Node.js

```javascript
const axios = require("axios");

async function searchDocs(query, options = {}) {
  const params = {
    q: query,
    max_results: options.maxResults || 10,
    min_similarity: options.minSimilarity || 0.0,
    ...(options.category && { category: options.category }),
  };

  const response = await axios.get("http://localhost:3000/search", { params });
  return response.data;
}

// Basic search
const results = await searchDocs("API endpoints");
results.results.forEach((result) => {
  console.log(`[${result.similarity.toFixed(2)}] ${result.source_file}`);
  console.log(`  ${result.content.substring(0, 100)}...`);
});

// High-precision search
const precise = await searchDocs("deployment strategies", {
  maxResults: 3,
  minSimilarity: 0.7,
  category: "how_to",
});
```

### cURL with jq

```bash
# Pretty-print top 3 results
curl -s "http://localhost:3000/search?q=configuration&max_results=3" | \
  jq '.results[] | {file: .source_file, similarity: .similarity, title: .title}'

# Extract only high-similarity matches
curl -s "http://localhost:3000/search?q=installation&min_similarity=0.8" | \
  jq '.results | length'

# Search within a specific category
curl -s "http://localhost:3000/search?q=examples&category=tutorial" | \
  jq '.results[] | .source_file'
```

## Performance Considerations

### Query Response Time

Typical response times depend on:

- Database size: More chunks = longer search time
- Embedding generation: ~100-500ms for query embedding
- Similarity calculation: O(n) where n = number of chunks
- Network latency to Ollama service

Expected performance:

- Small corpus (< 1,000 chunks): 200-500ms
- Medium corpus (1,000-10,000 chunks): 500ms-2s
- Large corpus (> 10,000 chunks): 2-5s

### Optimization Tips

1. **Use category filters** when possible to reduce search space
2. **Set appropriate max_results** - lower values return faster
3. **Use min_similarity thresholds** to reduce post-processing
4. **Ensure Ollama service is local** to minimize network latency
5. **Index database properly** - ensure embeddings are indexed

## Integration with CLI

The server API provides the same functionality as the CLI search command:

```bash
# CLI equivalent
xze search "how to install" --max-results 10

# API equivalent
curl "http://localhost:3000/search?q=how+to+install&max_results=10"
```

Both use the same underlying search implementation from `xze-core`.

## Error Handling

### Client-Side Error Handling

```python
import requests

try:
    response = requests.get('http://localhost:3000/search',
                          params={'q': 'test query'})
    response.raise_for_status()
    data = response.json()

    if data['total_results'] == 0:
        print("No results found. Try:")
        print("- Using different keywords")
        print("- Lowering min_similarity threshold")
        print("- Removing category filters")

except requests.exceptions.HTTPError as e:
    if e.response.status_code == 400:
        error = e.response.json()
        print(f"Invalid request: {error['error']}")
    elif e.response.status_code == 502:
        print("Ollama service unavailable")
    else:
        print(f"Server error: {e}")

except requests.exceptions.ConnectionError:
    print("Cannot connect to XZe server")
```

## See Also

- [Search Command Reference](./search_command_reference.md) - CLI search interface
- [Semantic Chunking API](./semantic_chunking_api.md) - Underlying chunking system
- [Server Configuration](../how_to/server_configuration.md) - Server setup guide
- [Phase 6 Implementation](../explanation/phase_6_search_integration_implementation.md) - Technical details
