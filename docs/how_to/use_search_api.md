# How to Use the XZe Search API

## Overview

This guide shows you how to use the XZe server's search API endpoint to query your indexed documentation using semantic search.

## Prerequisites

Before you begin, ensure you have:

1. XZe server running (`xze serve`)
2. PostgreSQL database with indexed documentation chunks
3. Ollama service running with the `nomic-embed-text` model
4. Documentation indexed using `xze index` command

## Quick Start

### 1. Start the XZe Server

```bash
xze serve --host 0.0.0.0 --port 3000
```

The server will start and be accessible at `http://localhost:3000`.

### 2. Perform a Basic Search

```bash
curl "http://localhost:3000/search?q=installation"
```

This returns the top 10 most relevant chunks matching "installation".

## Common Use Cases

### Search for Installation Instructions

Find documentation about installing your software:

```bash
curl "http://localhost:3000/search?q=how+to+install"
```

### Find Configuration Examples

Search for configuration-related content with high precision:

```bash
curl "http://localhost:3000/search?q=configuration+options&min_similarity=0.7"
```

### Get Tutorial Content

Limit search to tutorial category:

```bash
curl "http://localhost:3000/search?q=getting+started&category=tutorial"
```

### Find Reference Documentation

Search only in reference docs:

```bash
curl "http://localhost:3000/search?q=API+methods&category=reference&max_results=5"
```

### Search How-To Guides

Find step-by-step guides:

```bash
curl "http://localhost:3000/search?q=deploy+to+production&category=how_to"
```

### High-Precision Search

Get only highly relevant results:

```bash
curl "http://localhost:3000/search?q=authentication&min_similarity=0.8&max_results=3"
```

## Using with Programming Languages

### Python

Create a reusable search client:

```python
import requests
from typing import Optional, List, Dict, Any

class XzeSearchClient:
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url
        self.search_url = f"{base_url}/search"

    def search(
        self,
        query: str,
        max_results: int = 10,
        min_similarity: float = 0.0,
        category: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Perform a semantic search.

        Args:
            query: Search query string
            max_results: Maximum number of results (default: 10)
            min_similarity: Minimum similarity threshold 0.0-1.0 (default: 0.0)
            category: Filter by category: tutorial, how_to, reference, explanation

        Returns:
            Search response with results
        """
        params = {
            'q': query,
            'max_results': max_results,
            'min_similarity': min_similarity
        }
        if category:
            params['category'] = category

        response = requests.get(self.search_url, params=params)
        response.raise_for_status()
        return response.json()

    def search_best_match(self, query: str) -> Optional[Dict[str, Any]]:
        """Get the single best matching result."""
        results = self.search(query, max_results=1, min_similarity=0.5)
        return results['results'][0] if results['results'] else None

    def search_by_category(
        self,
        query: str,
        category: str,
        min_similarity: float = 0.5
    ) -> List[Dict[str, Any]]:
        """Search within a specific documentation category."""
        results = self.search(
            query,
            category=category,
            min_similarity=min_similarity
        )
        return results['results']

# Usage examples
client = XzeSearchClient()

# Basic search
results = client.search("database connection")
for result in results['results']:
    print(f"[{result['similarity']:.2%}] {result['source_file']}")
    print(f"  {result['title']}")
    print()

# Get best match
best = client.search_best_match("error handling")
if best:
    print(f"Best match: {best['source_file']}")
    print(f"Similarity: {best['similarity']:.2%}")
    print(f"Content preview: {best['content'][:200]}...")

# Search tutorials
tutorials = client.search_by_category("getting started", "tutorial")
print(f"Found {len(tutorials)} tutorial(s)")
```

### JavaScript/Node.js

Create a search client module:

```javascript
const axios = require('axios');

class XzeSearchClient {
    constructor(baseUrl = 'http://localhost:3000') {
        this.baseUrl = baseUrl;
        this.searchUrl = `${baseUrl}/search`;
    }

    async search(query, options = {}) {
        const params = {
            q: query,
            max_results: options.maxResults || 10,
            min_similarity: options.minSimilarity || 0.0,
            ...(options.category && { category: options.category })
        };

        const response = await axios.get(this.searchUrl, { params });
        return response.data;
    }

    async searchBestMatch(query) {
        const results = await this.search(query, {
            maxResults: 1,
            minSimilarity: 0.5
        });
        return results.results[0] || null;
    }

    async searchByCategory(query, category, minSimilarity = 0.5) {
        const results = await this.search(query, {
            category,
            minSimilarity
        });
        return results.results;
    }
}

// Usage examples
const client = new XzeSearchClient();

// Basic search
const results = await client.search("API authentication");
results.results.forEach(result => {
    console.log(`[${(result.similarity * 100).toFixed(1)}%] ${result.source_file}`);
    console.log(`  ${result.title}`);
    console.log();
});

// Get best match
const best = await client.searchBestMatch("logging configuration");
if (best) {
    console.log(`Best match: ${best.source_file}`);
    console.log(`Similarity: ${(best.similarity * 100).toFixed(1)}%`);
    console.log(`Preview: ${best.content.substring(0, 200)}...`);
}

// Search how-to guides
const howtos = await client.searchByCategory("deployment", "how_to");
console.log(`Found ${howtos.length} how-to guide(s)`);
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "net/http"
    "net/url"
)

type SearchClient struct {
    BaseURL string
}

type SearchResult struct {
    ID                  int64    `json:"id"`
    SourceFile          string   `json:"source_file"`
    Content             string   `json:"content"`
    Similarity          float64  `json:"similarity"`
    ChunkIndex          int      `json:"chunk_index"`
    TotalChunks         int      `json:"total_chunks"`
    Title               *string  `json:"title"`
    Category            *string  `json:"category"`
    SentenceRange       [2]int   `json:"sentence_range"`
    AvgChunkSimilarity  float64  `json:"avg_chunk_similarity"`
}

type SearchResponse struct {
    Query        string         `json:"query"`
    Results      []SearchResult `json:"results"`
    TotalResults int            `json:"total_results"`
}

func NewSearchClient(baseURL string) *SearchClient {
    return &SearchClient{BaseURL: baseURL}
}

func (c *SearchClient) Search(query string, maxResults int, minSimilarity float64, category string) (*SearchResponse, error) {
    params := url.Values{}
    params.Add("q", query)
    params.Add("max_results", fmt.Sprintf("%d", maxResults))
    params.Add("min_similarity", fmt.Sprintf("%.2f", minSimilarity))
    if category != "" {
        params.Add("category", category)
    }

    searchURL := fmt.Sprintf("%s/search?%s", c.BaseURL, params.Encode())

    resp, err := http.Get(searchURL)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var searchResp SearchResponse
    if err := json.NewDecoder(resp.Body).Decode(&searchResp); err != nil {
        return nil, err
    }

    return &searchResp, nil
}

func main() {
    client := NewSearchClient("http://localhost:3000")

    // Perform search
    results, err := client.Search("database configuration", 10, 0.5, "")
    if err != nil {
        fmt.Printf("Error: %v\n", err)
        return
    }

    fmt.Printf("Found %d results\n\n", results.TotalResults)
    for _, result := range results.Results {
        fmt.Printf("[%.1f%%] %s\n", result.Similarity*100, result.SourceFile)
        if result.Title != nil {
            fmt.Printf("  %s\n", *result.Title)
        }
        fmt.Println()
    }
}
```

## Advanced Techniques

### Combining Multiple Searches

Search across multiple categories and combine results:

```python
def comprehensive_search(client, query):
    """Search all categories and aggregate results."""
    categories = ['tutorial', 'how_to', 'reference', 'explanation']
    all_results = []

    for category in categories:
        results = client.search(query, category=category, max_results=5)
        all_results.extend(results['results'])

    # Sort by similarity
    all_results.sort(key=lambda x: x['similarity'], reverse=True)
    return all_results[:10]  # Top 10 overall
```

### Progressive Search with Fallback

Start with high precision, fall back to broader search:

```python
def smart_search(client, query):
    """Try high-precision first, fall back if no results."""
    # Try high precision
    results = client.search(query, min_similarity=0.8, max_results=5)

    if results['total_results'] == 0:
        # Fall back to medium precision
        results = client.search(query, min_similarity=0.5, max_results=10)

    if results['total_results'] == 0:
        # Fall back to broad search
        results = client.search(query, min_similarity=0.0, max_results=20)

    return results
```

### Search with Context

Extract surrounding context from the document:

```python
def search_with_context(client, query):
    """Get search results with adjacent chunks for context."""
    results = client.search(query, max_results=5)

    for result in results['results']:
        chunk_idx = result['chunk_index']
        total = result['total_chunks']

        print(f"Match in: {result['source_file']}")
        print(f"Position: Chunk {chunk_idx + 1} of {total}")

        # Show if there's context before/after
        if chunk_idx > 0:
            print("  (Previous chunk available)")
        if chunk_idx < total - 1:
            print("  (Next chunk available)")
        print()
```

## Troubleshooting

### No Results Found

If your search returns no results:

1. **Lower the similarity threshold**:
   ```bash
   curl "http://localhost:3000/search?q=your+query&min_similarity=0.0"
   ```

2. **Remove category filters**:
   ```bash
   # Instead of: &category=tutorial
   # Try without the category parameter
   ```

3. **Try different keywords**:
   ```bash
   # Instead of: "install"
   # Try: "installation", "setup", "getting started"
   ```

4. **Increase max results**:
   ```bash
   curl "http://localhost:3000/search?q=your+query&max_results=50"
   ```

### Server Connection Error

If you get connection errors:

```bash
# Check if server is running
curl http://localhost:3000/health

# If not, start the server
xze serve
```

### Slow Response Times

If searches are slow:

1. Use category filters to narrow the search space
2. Reduce max_results
3. Ensure Ollama is running locally
4. Check database indices are created

### Empty Content in Results

If results have empty or truncated content, the chunks may not have been indexed properly:

```bash
# Re-index your documentation
xze index /path/to/docs --force
```

## Best Practices

### 1. Use Appropriate Similarity Thresholds

- **Exploratory search**: `min_similarity=0.0` (see everything)
- **General use**: `min_similarity=0.5` (balanced)
- **High precision**: `min_similarity=0.7` (very relevant only)

### 2. Leverage Category Filters

Category filters significantly improve both relevance and performance:

```bash
# Looking for step-by-step instructions? Use how_to
curl "http://localhost:3000/search?q=deploy&category=how_to"

# Learning concepts? Use tutorial
curl "http://localhost:3000/search?q=architecture&category=tutorial"

# Need API details? Use reference
curl "http://localhost:3000/search?q=functions&category=reference"
```

### 3. Optimize Result Counts

Don't request more results than you need:

```bash
# Good: Request only what you'll use
curl "http://localhost:3000/search?q=config&max_results=5"

# Wasteful: Requesting many results you won't use
curl "http://localhost:3000/search?q=config&max_results=100"
```

### 4. Handle Errors Gracefully

Always handle potential errors in your client code:

```python
try:
    results = client.search(query)
    if results['total_results'] == 0:
        # Handle no results case
        print("Try rephrasing your query")
except requests.HTTPError as e:
    # Handle HTTP errors
    print(f"Search failed: {e}")
except requests.ConnectionError:
    # Handle connection errors
    print("Cannot connect to server")
```

## See Also

- [Search API Endpoint Reference](../reference/search_api_endpoint.md) - Complete API documentation
- [Search Command Reference](../reference/search_command_reference.md) - CLI search interface
- [Server Configuration](./server_configuration.md) - Configure the XZe server
- [Indexing Documentation](./index_documentation.md) - How to index your docs
