# CLI Command Reference

This reference documents the command-line interface (CLI) for the XZe tool.

## `xze ingest`

Ingests documentation into the knowledge base for RAG (Retrieval-Augmented Generation).

### Usage

```bash
xze ingest [OPTIONS] <PATH>
```

### Arguments

- `<PATH>`: Path to the documentation directory to ingest.

### Options

- `--database-url <URL>`: PostgreSQL connection string. Can also be set via `DATABASE_URL` environment variable.
- `--ollama-url <URL>`: URL of the Ollama server (default: `http://localhost:11434`).
- `--model <NAME>`: Ollama model to use for embeddings (default: `llama3`).
- `--concurrency <N>`: Maximum number of concurrent file processing tasks (default: `4`).
- `--dry-run`: Process files but do not write to the database.
- `-h, --help`: Print help.

---

## `xze search`

Searches the knowledge base using semantic similarity, with optional hybrid search, reranking, and context expansion.

### Usage

```bash
xze search [OPTIONS] <QUERY>
```

### Arguments

- `<QUERY>`: The search query text.

### Options

- `-n, --max-results <N>`: Maximum number of results to return (default: `10`).
- `--rerank`: Enable LLM-based reranking of results for better relevance.
- `--expand`: Enable context expansion (fetch surrounding chunks).
- `--context-window <N>`: Number of chunks to fetch before and after the matching chunk (default: `1`).
- `--database-url <URL>`: PostgreSQL connection string.
- `--ollama-url <URL>`: URL of the Ollama server (default: `http://localhost:11434`).
- `--model <NAME>`: Ollama model to use (default: `llama3`).
- `--json`: Output results in JSON format.
- `--full-content`: Show full chunk content instead of truncating.
- `-v, --verbose`: Show detailed metadata (similarity scores, chunk indices).
- `-h, --help`: Print help.

---

## `xze reorg`

Reorganizes documentation files according to the Diataxis framework (Tutorials, How-To, Reference, Explanation).

### Usage

```bash
xze reorg [OPTIONS] <PATH>
```

### Arguments

- `<PATH>`: Path to the documentation directory to reorganize.

### Options

- `--execute`: Execute the reorganization plan (move files). If not specified, only prints the plan.
- `--dry-run`: Simulate execution without modifying the filesystem.
- `--ollama-url <URL>`: URL of the Ollama server (default: `http://localhost:11434`).
- `--model <NAME>`: Ollama model to use for classification (default: `llama3`).
- `-h, --help`: Print help.
