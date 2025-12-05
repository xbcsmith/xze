# How-To: Ingest Documentation

This guide explains how to ingest your documentation into the XZe knowledge base so it can be searched and used for RAG.

## Prerequisites

- A running PostgreSQL database with `pgvector` extension.
- A running Ollama instance (or compatible LLM provider).
- `xze` CLI installed.

## Steps

1.  **Prepare your documentation**: Ensure your documentation is in Markdown format (`.md` or `.markdown`). The ingestion process works best with well-structured files (headers, code blocks).

2.  **Run the ingestion command**:

    ```bash
    xze ingest ./path/to/docs --database-url postgres://user:pass@localhost:5432/xze
    ```

    This will:
    - Scan the directory recursively.
    - Classify each file (Tutorial, How-To, Reference, Explanation).
    - Chunk the content based on semantic boundaries.
    - Generate embeddings using the default model (`llama3`).
    - Store chunks and embeddings in the database.

3.  **Verify ingestion**:

    You can verify the ingestion by running a search:

    ```bash
    xze search "test query" --database-url postgres://user:pass@localhost:5432/xze
    ```

## Advanced Options

### Using a different model

If you want to use a specific embedding model:

```bash
xze ingest ./docs --model nomic-embed-text
```

### Dry Run

To see what would happen without writing to the database:

```bash
xze ingest ./docs --dry-run
```

### Performance Tuning

Adjust concurrency for faster processing (default is 4):

```bash
xze ingest ./docs --concurrency 8
```
