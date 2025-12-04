# XZe RAG Service Refactoring Plan

## Overview

Refactor XZe from a general-purpose documentation tool into a specialized
**Retrieval-Augmented Generation (RAG) Service** aligned with the `rag_arch.md`
specification. The service will leverage PostgreSQL with `pgvector` for storage,
implement a multi-stage retrieval pipeline (Hybrid Search, Reranking, Context
Expansion), and provide a unique "Diataxis Reorganization" capability to
restructure unstructured documentation into the Diataxis framework.

## Current State Analysis

### Existing Infrastructure

- **Core**: Rust-based `xze-core` with `ai` module (Ollama client, partial
  provider abstraction).
- **Search**: Basic vector search using local embedding cache (no persistent
  vector DB).
- **API**: `xze-serve` (Axum) with basic endpoints.
- **Storage**: No centralized database; relies on file system and in-memory structures.

### Identified Issues

- **Missing Persistence**: No robust vector store (Postgres/pgvector) for
  production-grade RAG.
- **Limited Retrieval**: Single-stage vector search lacks precision (no Hybrid
  BM25, no Reranking).
- **No Classification**: Does not classify content by intent (Tutorial vs How-To).
- **Scope Creep**: Contains unused GitHub/Repository management code that
  belongs to an external service.

## Implementation Phases

### Phase 1: Core Foundation (Storage & Ingestion)

#### Task 1.1 Database Infrastructure

- **Action**: Add `sqlx` (Postgres) and `pgvector` dependencies.
- **Schema**: Create `documents` table:

  ```sql
  CREATE TABLE documents (
      id UUID PRIMARY KEY,
      content TEXT NOT NULL,
      embedding vector(1536),
      diataxis_type VARCHAR(50), -- 'tutorial', 'how-to', 'reference', 'explanation'
      chunk_strategy VARCHAR(50),
      metadata JSONB,
      ts vector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED
  );
  CREATE INDEX ON documents USING hnsw (embedding vector_cosine_ops);
  CREATE INDEX ON documents USING gin (ts);
  ```

- **Structs**: Define `Document` and `DocumentMetadata` in `xze-core/src/storage/models.rs`.

#### Task 1.2 Diataxis Classification

- **Action**: Implement `DiataxisClassifier` in `xze-core/src/ingest/classifier.rs`.
- **Logic**: Use `Provider::complete` with a prompt that outputs a JSON
  classification (`{"type": "tutorial", "confidence": 0.95}`).
- **Integration**: Add `classify_content` method to `IngestionPipeline`.

#### Task 1.3 Ingestion Pipeline

- **Action**: Build `ChunkingStrategyManager` in `xze-core/src/ingest/chunking.rs`.
- **Strategies**:
  - `Tutorial`: Large chunks (1200-1800 chars), preserve headers.
  - `HowTo`: Medium chunks (800-1200 chars), preserve steps.
  - `Reference`: Small chunks (600-1000 chars), strict boundaries.
- **Workflow**:
    1. Read file content.
    2. Run `DiataxisClassifier` on whole file or large sections.
    3. Select Strategy based on classification.
    4. Chunk content.
    5. Generate Embeddings (using `ai::providers`).
    6. Store in Postgres.

#### Task 1.4 Testing Requirements

- **Unit**: Mock `Provider` to test Classifier output parsing.
- **Integration**: Spin up Postgres container (testcontainers), run migrations,
  insert/query vectors.

#### Task 1.5 Deliverables

- `xze-core/src/storage/postgres.rs` (Connection pool, CRUD)
- `xze-core/src/ingest/` module (Classifier, Chunking)
- `migrations/` folder with SQL.

#### Task 1.6 Success Criteria

- `cargo test` passes for storage and ingestion.
- Can ingest a sample markdown file and verify `diataxis_type` in DB.

### Phase 2: Search Pipeline (Retrieval)

#### Task 2.1 Hybrid Search Implementation

- **Action**: Implement `search_hybrid` in `xze-core/src/search/hybrid.rs`.
- **Logic**:
    1. **Keyword**: `SELECT ... FROM documents WHERE ts @@ plainto_tsquery($1)`.
    2. **Vector**: `SELECT ... ORDER BY embedding <=> $1 LIMIT 50`.
    3. **RRF Fusion**: `score = 1 / (k + rank_bm25) + 1 / (k + rank_vector)`.
- **Config**: Allow tuning weights for BM25 vs Vector.

#### Task 2.2 Multi-Stage Pipeline

- **Action**: Create `SearchPipeline` struct.
- **Stage 1 (Retrieval)**: Call `search_hybrid`.
- **Stage 2 (Reranking)**: Implement `LLMReranker`.
  - Input: Query + Top 20 Docs.
  - Prompt: "Rate relevance 0-100".
  - Output: Re-sorted list.
- **Stage 3 (Expansion)**: Implement `ContextExpander`.
  - Query DB for `chunk_index - 1` and `chunk_index + 1` for top results.

#### Task 2.3 API Updates

- **Action**: Update `xze-serve/src/api/search.rs`.
- **Endpoints**:
  - `POST /api/v1/search/hybrid`
  - `POST /api/v1/search/multi-stage`
- **OpenAPI**: Update `utoipa` definitions.

#### Task 2.4 Testing Requirements

- **Benchmark**: Compare precision of Vector-only vs Hybrid vs Multi-stage on a
  test dataset.
- **Unit**: Test RRF math and Reranker parsing.

#### Task 2.5 Deliverables

- `xze-core/src/search/` (Hybrid, Pipeline, Reranker)
- Updated `xze-serve` routes.

#### Task 2.6 Success Criteria

- Multi-stage search returns higher relevance results than basic vector search.

### Phase 3: Diataxis Reorganization (The "Last Feature")

#### Task 3.1 Export Logic

- **Action**: Implement `DiataxisExporter` in `xze-core/src/export/`.
- **Logic**:
    1. `SELECT * FROM documents WHERE diataxis_type = 'tutorial'`.
    2. Group by `source_file` (or reconstruct if chunks are scattered).
    3. Generate new file path: `output_dir/tutorials/{original_filename}`.
    4. Repeat for How-To, Reference, Explanation.

#### Task 3.2 CLI Export Command

- **Action**: Add `export` subcommand to `xze-cli`.
- **Command**: `xze docs export --out ./new-docs`
- **Output**:

    ```text
    ./new-docs/
      tutorials/
      how-to-guides/
      reference/
      explanation/
    ```

#### Task 3.3 Testing Requirements

- **End-to-End**: Ingest a "messy" folder -> Run Export -> Verify clean structure.

#### Task 3.4 Deliverables

- `xze-core/src/export/`
- `xze-cli` update.

#### Task 3.5 Success Criteria

- Exported directory follows strict Diataxis structure.

### Phase 4: Quality & Integration

#### Task 4.1 Quality Metrics

- **Action**: Implement `SearchQualityMetrics` struct.
- **Metrics**: Track MRR (Mean Reciprocal Rank) and NDCG.
- **Storage**: Store metrics in Postgres `metrics` table.

#### Task 4.2 Redpanda Integration

- **Action**: Add `rdkafka` dependency.
- **Consumer**: Listen to `repo.update` topic.
- **Producer**: Emit `analysis.complete` topic.
- **Workflow**:
    1. **Event**: `{"repo_url": "...", "commit": "..."}`
    2. **Action**: `git clone` (temp) -> Ingest Pipeline -> Export Pipeline.
    3. **Result**: Push new structure to `diataxis-branch` or emit result event.

#### Task 4.3 Cleanup

- **Action**: Delete `crates/core/src/repository/` (old logic).
- **Action**: Delete `crates/core/src/git/` (old logic).

#### Task 4.4 Testing Requirements

- **Load Test**: Simulate 100 concurrent events.
- **Resilience**: Kill consumer mid-process, verify restart/resume.

#### Task 4.5 Deliverables

- `xze-core/src/events/`
- `xze-core/src/metrics/`

#### Task 4.6 Success Criteria

- Full end-to-end flow: Event -> Reorganized Repo.
