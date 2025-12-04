# XZe Architecture Reference

## 1. System Architecture Overview

### 1.1 Introduction

XZe is a specialized **Retrieval-Augmented Generation (RAG) Service** designed
to ingest, analyze, and reorganize documentation. Unlike general-purpose tools,
XZe focuses on **Diataxis-aware processing**: it classifies unstructured content
into Diataxis categories (Tutorials, How-To Guides, Reference, Explanation) and
provides advanced retrieval and reorganization capabilities.

The system is built on an **Event-Driven Architecture**, integrating with external systems via Redpanda (Kafka) to automate documentation improvement workflows.

### 1.2 High-Level Architecture

XZe operates as a stateless service backed by PostgreSQL. It exposes both a REST API for synchronous operations and a Redpanda consumer for asynchronous event processing.

```mermaid
graph TD
    subgraph "External Systems"
        RepoService[Repository Service]
        User[User / CLI]
    end

    subgraph "XZe Service"
        API[REST API (Axum)]
        Consumer[Event Consumer]
        Producer[Event Producer]
        
        subgraph "Core Logic"
            Ingest[Ingestion Pipeline]
            Search[Search Pipeline]
            Export[Diataxis Exporter]
        end
        
        subgraph "AI & Analysis"
            Classifier[Diataxis Classifier]
            Reranker[LLM Reranker]
            Embedder[Vector Embedder]
        end
    end

    subgraph "Storage"
        DB[(PostgreSQL + pgvector)]
    end

    RepoService -- "RepoUpdateEvent" --> Consumer
    User -- "Search / Export" --> API
    
    Consumer --> Ingest
    API --> Search
    API --> Export
    
    Ingest --> Classifier
    Ingest --> Embedder
    Ingest --> DB
    
    Search --> DB
    Search --> Reranker
    
    Export --> DB
    Export --> Producer
    Producer -- "AnalysisComplete" --> RepoService
```

### 1.3 Core Design Principles

#### RAG-First

- Optimized for semantic retrieval and generation.
- Uses **PostgreSQL with pgvector** for robust, scalable vector and metadata storage.
- Implements **Hybrid Search** (BM25 + Vector) and **Multi-Stage Retrieval** (Reranking, Expansion).

#### Diataxis-Centric

- Understands documentation structure.
- **Classifies** content by intent (Tutorial vs How-To) during ingestion.
- **Reorganizes** content into Diataxis structure via export capabilities.

#### Event-Driven

- Decoupled from repository management.
- Reacts to `RepoUpdateEvent` to trigger analysis.
- Emits `AnalysisComplete` or `SuggestionEvent` results.

## 2. Component Design

### 2.1 Storage Layer (PostgreSQL)

**Crate**: `xze-core` (storage module)

**Technology**: PostgreSQL 15+ with `pgvector` extension.

**Schema**:

- **`documents` Table**:
  - `id`: UUID
  - `content`: Text content
  - `embedding`: Vector(1536) (HNSW index)
  - `ts`: tsvector (GIN index for BM25)
  - `diataxis_type`: Enum (Tutorial, HowTo, Reference, Explanation)
  - `metadata`: JSONB (Source file, line numbers, git commit)

**Responsibilities**:

- Persist document chunks and embeddings.
- Perform fast vector similarity search.
- Perform full-text keyword search.
- Store quality metrics and job state.

### 2.2 Ingestion Pipeline

**Crate**: `xze-core` (ingest module)

**Workflow**:

1. **Read**: Load markdown files from source (temp dir).
2. **Classify**: `DiataxisClassifier` uses LLM to determine content type (Tutorial, How-To, etc.).
3. **Chunk**: `ChunkingStrategyManager` selects strategy based on classification:
    - *Tutorial*: Large chunks, preserve headers.
    - *How-To*: Step-based chunking.
    - *Reference*: Small, strict chunks.
4. **Embed**: Generate embeddings via AI Provider.
5. **Store**: Write to PostgreSQL.

### 2.3 Search Pipeline (Multi-Stage)

**Crate**: `xze-core` (search module)

**Stages**:

1. **Hybrid Retrieval**:
    - Run Vector Search (Cosine Similarity).
    - Run Keyword Search (BM25 via Postgres `tsvector`).
    - Combine scores using **Reciprocal Rank Fusion (RRF)**.
2. **LLM Reranking**:
    - Top K results sent to LLM.
    - Scored 0-100 for relevance to query.
    - Re-sorted.
3. **Context Expansion**:
    - Fetch adjacent chunks (`index - 1`, `index + 1`) from DB to provide full context.

### 2.4 Diataxis Exporter (Reorganization)

**Crate**: `xze-core` (export module)

**Purpose**: "The Last Feature" - Convert sprawling docs into a structured Diataxis layout.

**Workflow**:

1. **Query**: Select all documents for a specific Diataxis type (e.g., `WHERE diataxis_type = 'tutorial'`).
2. **Reconstruct**: Assemble chunks back into coherent documents (if needed).
3. **Layout**: Write files to structured directories:
    - `/tutorials/`
    - `/how-to-guides/`
    - `/reference/`
    - `/explanation/`
4. **Output**: Generate a clean artifact for the user or external service.

### 2.5 Event Integration

**Crate**: `xze-core` (events module)

**Technology**: Redpanda (Kafka Protocol).

**Events**:

- **Consumer**: `RepoUpdateEvent`
  - Payload: `{ "repo_url": "...", "commit_sha": "..." }`
  - Action: Trigger Ingestion Pipeline.
- **Producer**: `AnalysisCompleteEvent`
  - Payload: `{ "status": "success", "metrics": { ... } }`
- **Producer**: `SuggestionEvent`
  - Payload: `{ "file": "...", "suggestion": "..." }`

## 3. API Layer (xze-serve)

**Crate**: `xze-serve`

**Endpoints**:

- `POST /api/v1/ingest`: Trigger manual ingestion.
- `POST /api/v1/search/hybrid`: Perform hybrid search.
- `POST /api/v1/search/multi-stage`: Perform full multi-stage retrieval.
- `POST /api/v1/export`: Trigger Diataxis export.
- `GET /api/v1/metrics`: Retrieve quality metrics (MRR, NDCG).

## 4. AI Provider Abstraction

**Crate**: `xze-core` (ai module)

**Purpose**: Unified interface for LLM operations (Embeddings, Classification, Reranking).

**Supported Providers**:

- Ollama (Local)
- OpenAI
- Anthropic
- Copilot

**Factory**: `ProviderFactory` instantiates providers based on configuration, enabling runtime switching.

## 5. Quality Monitoring

**Crate**: `xze-core` (metrics module)

**Metrics**:

- **MRR (Mean Reciprocal Rank)**: Accuracy of the first result.
- **NDCG**: Quality of the ranking order.
- **Latency**: End-to-end search time.

**Storage**: Metrics persisted in PostgreSQL for historical analysis.
