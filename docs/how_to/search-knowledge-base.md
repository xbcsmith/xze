# How-To: Search the Knowledge Base

This guide explains how to perform advanced semantic searches using the XZe CLI.

## Basic Search

To perform a simple semantic search:

```bash
xze search "how to configure logging"
```

This uses vector similarity to find the most relevant chunks.

## Improving Relevance with Reranking

For higher accuracy, enable the **Reranker**. This uses an LLM to re-evaluate the top results from the vector search and order them by actual relevance to the query.

```bash
xze search "how to configure logging" --rerank
```

*Note: Reranking is slower than basic search but produces significantly better results.*

## Getting More Context

Sometimes a single chunk isn't enough. Use **Context Expansion** to retrieve surrounding chunks (before and after the match).

```bash
xze search "error handling" --expand --context-window 2
```

This will fetch 2 chunks before and 2 chunks after each result, providing a fuller picture of the content.

## Filtering by Category

You can filter results by Diataxis category:

```bash
xze search "api endpoints" --category reference
```

## Outputting JSON

For integration with other tools, output results as JSON:

```bash
xze search "deployment" --json > results.json
```
