# System Architecture

## Overview

The Pipeline Knowledge Base Server is a Retrieval Augmented Generation (RAG)
application that provides semantic search capabilities over markdown
documentation. The system uses PostgreSQL with pgvector for vector storage,
Ollama for embeddings and LLM operations, and provides both HTTP REST API and
Model Context Protocol (MCP) interfaces.

The system implements a sophisticated multi-stage retrieval pipeline with
intent-aware processing, semantic chunking, and document-type-specific
optimization strategies following the Diataxis documentation framework.

## Feature Summary

| Feature               | Description                                                       | Module(s)                                       |
| --------------------- | ----------------------------------------------------------------- | ----------------------------------------------- |
| Document Loading      | Basic and enhanced loading with metadata extraction               | `load.py`, `load_enhanced.py`                   |
| Semantic Chunking     | Content-aware document segmentation                               | `semantic_chunker.py`, `chunking_strategies.py` |
| Intent Classification | Query intent detection (tutorial, how-to, reference, explanation) | `intent_classifier.py`                          |
| Query Refinement      | Intent-based query enhancement                                    | `query_refiner.py`                              |
| Hybrid Search         | Combined BM25 keyword and vector similarity search                | `hybrid_search.py`                              |
| Multi-Stage Retrieval | Reranking and context expansion pipeline                          | `multi_stage_search.py`                         |
| LLM Reranking         | Relevance scoring using LLM judgment                              | `reranker.py`                                   |
| Context Expansion     | Adjacent chunk retrieval for complete context                     | `context_expander.py`                           |
| Search Optimization   | Grid search parameter tuning with parallel execution              | `search_optimizer.py`                           |
| Quality Monitoring    | Real-time metrics tracking with threshold alerting                | `search_quality_metrics.py`                     |
| A/B Testing           | Statistical experiment framework with export                      | `search_ab_test.py`                             |
| Alerting Integration  | Logging, webhook, and file alert handlers                         | `search_quality_metrics.py`                     |
| HTTP REST API         | FastAPI server with versioned endpoints                           | `server.py`                                     |
| MCP Server            | Model Context Protocol for AI agents                              | `server.py`                                     |
| Web UI                | Gradio interface with metrics dashboard                           | `ui/`                                           |
| Metrics API           | Quality metrics, alerts, and history endpoints                    | `metrics/api.py`                                |
| Database Backup       | PostgreSQL backup and restore operations                          | `backup.py`                                     |
| CLI Interface         | Command-line tools for all operations                             | `main.py`                                       |

## Core Components

### Document Loading and Processing

#### Load Module (`load.py`)

- Purpose: Basic document loading with fixed-size chunking
- Key functionality: Reads markdown files, generates embeddings, stores in
  PostgreSQL
- Chunking strategy: Fixed 1000-character chunks with 200-character overlap
- Use case: Simple, fast loading for general documentation

#### Enhanced Load Module (`load_enhanced.py`)

- Purpose: Advanced document loading with intent-specific chunking and metadata
  extraction
- Key functionality:
  - Intent-specific chunking strategies per document type
  - Semantic chunking based on content meaning
  - Automatic extraction of titles, categories, keywords, code blocks
  - Document type classification (tutorial, how-to, reference, explanation)
- Design decisions:
  - Uses ChunkingStrategyManager for document-type-specific optimization
  - Preserves document structure and context
  - Extracts rich metadata for better retrieval
  - Supports CLI override of chunking strategies
- Integration: Activated via `--enhanced` CLI flag, with optional
  `--chunking-strategy` override

#### Chunking Strategies (`chunking_strategies.py`)

- Purpose: Document-type-specific chunking optimization following Diataxis
  framework
- Key classes: `ChunkingStrategyManager`, `ChunkingConfig`, strategy
  implementations
- Strategies:
  - **TutorialChunkingStrategy**: Step-by-step learning content with larger
    chunks (1200-1800 chars)
  - **HowToChunkingStrategy**: Task-oriented content with balanced chunks
    (800-1200 chars)
  - **ReferenceChunkingStrategy**: Information-dense content with smaller chunks
    (600-1000 chars)
  - **ExplanationChunkingStrategy**: Conceptual content with larger context
    chunks (1000-1500 chars)
- Design decisions:
  - Each strategy optimized for document type characteristics
  - Configurable parameters per strategy
  - Fallback to semantic chunking for unknown types
  - CLI override capability for testing and customization

#### Semantic Chunker (`semantic_chunker.py`)

- Purpose: Intelligent document segmentation based on semantic coherence
- Key classes: `SemanticChunker`, `SemanticChunk`
- Algorithm:
  1. Generate embeddings for sentences
  2. Calculate semantic similarity between adjacent sentences
  3. Split when similarity drops below threshold
  4. Respect document structure (headings, code blocks)
- Design decisions:
  - Maintains context continuity within chunks
  - Balances chunk size with semantic coherence
  - Preserves markdown structure
  - Used as fallback by intent-specific strategies

### Search and Retrieval Pipeline

#### Basic Search Service (`search.py`)

- Purpose: Consolidated search functionality for all interfaces
- Key class: `SearchService`
- Methods:
  - `search()`: Semantic similarity search using vector embeddings
  - `search_with_intent()`: Intent-aware search with query refinement
  - `search_hybrid()`: Hybrid BM25 + vector search
  - `search_hybrid_with_intent()`: Combined intent and hybrid search
- Design decisions:
  - Single service class for consistency across CLI, HTTP, and MCP
  - Input sanitization and validation
  - Rich error handling with actionable messages

#### Intent Classification (`intent_classifier.py`)

- Purpose: Classify user query intent to improve retrieval
- Key class: `IntentClassifier`
- Intent categories: Tutorial, How-to, Reference, Explanation (Diataxis
  framework)
- Algorithm:
  1. LLM analyzes query to determine intent
  2. Assigns confidence score
  3. Maps to document type for filtering
- Design decisions:
  - Uses lightweight LLM (llama3.2:3b) for speed
  - Caches results to avoid redundant classifications
  - Provides reasoning for transparency

#### Query Refinement (`query_refiner.py`)

- Purpose: Enhance queries based on detected intent
- Key class: `QueryRefiner`
- Functionality:
  - Adds intent-specific context to queries
  - Expands technical terms
  - Improves retrieval precision
- Integration: Used automatically with intent-aware search

#### Hybrid Search Engine (`hybrid_search.py`)

- Purpose: Combine keyword and semantic search for better results
- Key class: `HybridSearchEngine`
- Algorithm:
  1. BM25 keyword search using PostgreSQL full-text search
  2. Vector similarity search using pgvector
  3. Reciprocal Rank Fusion (RRF) to merge results
- Design decisions:
  - Configurable weights for BM25 vs vector components
  - RRF provides robust score fusion
  - Handles edge cases (no results from one method)

#### Multi-Stage Retrieval Pipeline (`multi_stage_search.py`)

- Purpose: Advanced retrieval with reranking and context expansion
- Key class: `MultiStageSearchPipeline`
- Pipeline stages:
  1. **Initial Retrieval**: Hybrid search (BM25 + vector) retrieves candidates
  2. **LLM Reranking**: Relevance scoring reduces to top results
  3. **Context Expansion**: Adjacent chunks added for complete understanding
- Design decisions:
  - Configurable stage parameters (limits, context window)
  - Optional stages (can skip reranking or expansion)
  - Balances quality vs performance
- Integration: Activated via `--use-multi-stage` CLI flag or
  `/api/v1/search/multi-stage` API endpoint

#### LLM Reranker (`reranker.py`)

- Purpose: Score document relevance using LLM judgment
- Key class: `LLMReranker`
- Algorithm:
  1. LLM evaluates each retrieved document
  2. Assigns 0-100 relevance score
  3. Optionally provides reasoning
  4. Results sorted by relevance
- Design decisions:
  - Uses low temperature (0.1) for consistent scoring
  - Falls back to similarity scores on LLM failure
  - Optional reasoning for explainability

#### Context Expander (`context_expander.py`)

- Purpose: Retrieve adjacent chunks for complete context
- Key class: `ContextExpander`
- Functionality:
  - Fetches N chunks before matched chunk
  - Fetches N chunks after matched chunk
  - Handles document boundaries
  - Deduplicates overlapping expansions
- Design decisions:
  - Configurable context window (default: 1 before, 1 after)
  - Preserves chunk ordering
  - Efficient database queries

### Search Optimization and Quality

#### Search Optimizer (`search_optimizer.py`)

- Purpose: Systematic parameter tuning using grid search
- Key classes: `SearchOptimizer`, `HybridSearchOptimizer`, `MultiStageSearchOptimizer`
- Features:
  - Grid search over parameter combinations
  - Parallel async execution with configurable workers
  - Result caching and recommendation system
  - Support for multiple optimization metrics (MRR, NDCG, Precision, Recall, Latency)
- Design decisions:
  - Full async implementation for FastAPI integration
  - Thread-safe result aggregation for parallel execution
  - Graceful fallback to sequential on parallel failures
  - Use-case-specific configuration presets

#### Quality Metrics (`search_quality_metrics.py`)

- Purpose: Track and evaluate search quality over time
- Key classes: `SearchQualityMetrics`, `QualityMonitor`
- Metrics:
  - Mean Reciprocal Rank (MRR)
  - Normalized Discounted Cumulative Gain (NDCG@K)
  - Precision@K and Recall@K
  - Mean Average Precision (MAP)
  - F1@K
- Features:
  - Real-time quality tracking with history
  - Threshold-based alerting
  - Configurable retention and alert thresholds
- Design decisions:
  - Full async implementation
  - 15-minute alert deduplication window
  - Pluggable alert handlers

#### Alerting Integration (`search_quality_metrics.py`)

- Purpose: Notify operators of quality degradation
- Key classes: `BaseAlertHandler`, `LoggingAlertHandler`, `WebhookAlertHandler`, `FileAlertHandler`
- Handler types:
  - **LoggingAlertHandler**: Structured logging at appropriate severity levels
  - **WebhookAlertHandler**: HTTP POST to external systems with retry/backoff
  - **FileAlertHandler**: JSONL file persistence for audit trails
- Features:
  - 15-minute deduplication window prevents alert storms
  - Factory function for handler creation
  - Configurable via environment variables
- Webhook payload format:

```json
{
  "event_type": "quality_alert",
  "timestamp": "2025-01-15T10:30:00.000000",
  "alert": {
    "severity": "warning",
    "metric": "mrr",
    "current_value": 0.45,
    "threshold": 0.5,
    "message": "MRR dropped below threshold"
  }
}
```

#### A/B Testing (`search_ab_test.py`)

- Purpose: Statistical experimentation for search configurations
- Key class: `SearchABTest`
- Features:
  - Variant assignment with configurable traffic splits
  - Statistical significance testing (Welch's t-test, Cohen's d)
  - Result export to JSON and CSV
  - Date range filtering for exports
- Export contents:
  - Experiment configuration and metadata
  - Raw data points per variant
  - Aggregated statistics (mean, variance, sample size)
  - Statistical analysis results
- Design decisions:
  - Full async implementation
  - Both raw and aggregated data in exports
  - CLI commands for experiment management

### Server Interfaces

#### FastAPI HTTP Server (`server.py`)

- Purpose: REST API for programmatic access
- Endpoints (v1):
  - `POST /api/v1/search`: Basic semantic search (JSON body)
  - `GET /api/v1/search`: Basic semantic search (query parameters)
  - `POST /api/v1/search/intent`: Intent-aware search
  - `POST /api/v1/search/hybrid`: Hybrid BM25 + vector search
  - `POST /api/v1/search/hybrid/intent`: Combined hybrid and intent search
  - `POST /api/v1/search/multi-stage`: Multi-stage retrieval pipeline
  - `POST /api/v1/search/multi-stage/intent`: Intent-aware multi-stage pipeline
  - `GET /api/v1/documents/{id}`: Document retrieval by ID
  - `GET /health`: Health check
- Metrics Endpoints:
  - `GET /api/v1/metrics/quality`: Current quality metrics
  - `GET /api/v1/metrics/quality/alerts`: Quality alerts with filtering
  - `GET /api/v1/metrics/quality/history`: Metrics history with pagination
- Deprecated Endpoints (remove 2026-01-01):
  - Legacy root-level endpoints (`/search`, `/search/intent`, etc.) still work
    but are deprecated
  - All deprecated endpoints include deprecation headers pointing to v1
    replacements
- Design decisions:
  - Pydantic models for request/response validation
  - Consistent error handling with HTTP status codes
  - OpenAPI documentation at `/docs`

#### FastMCP Server (`server.py`)

- Purpose: Model Context Protocol interface for AI agents
- Tools:
  - `search_kb`: Semantic search tool
  - `get_document`: Document retrieval tool
- Design decisions:
  - Same underlying SearchService as HTTP API
  - Tool-based interface for LLM integration
  - Structured output for agent consumption

#### Embedded Gradio UI (`ui/`)

- Purpose: Browser-based search interface with metrics dashboard
- Features:
  - Natural language search
  - Real-time results with markdown rendering
  - Source file display
  - Quality metrics dashboard tab
  - Auto-refresh capability with configurable intervals (15s, 30s, 60s, 5m)
  - Alert history display
- Integration: Mounted at `/ui` endpoint on HTTP server
- Design decisions:
  - Optional (can disable with `--no-ui` flag)
  - Self-contained Gradio app
  - Uses SearchService for consistency
  - Timer-based auto-refresh with error handling

### Database and Storage

#### PostgreSQL with pgvector

- Purpose: Store documents, embeddings, and metadata
- Schema:
  - `documents` table: Main document storage
  - Columns: id, source_file, content, embedding, chunk_index, total_chunks,
    title, category, doc_type, keywords, code_blocks, chunking_strategy,
    created_at
  - Vector index: HNSW for fast similarity search
  - Full-text index: GIN for BM25 keyword search
- Design decisions:
  - pgvector for native vector operations
  - PostgreSQL for reliability and ACID compliance
  - Indexes optimized for hybrid search
  - Chunking strategy tracking for analysis

#### Backup and Restore (`backup.py`)

- Purpose: Database backup and restore operations
- Functionality:
  - `pg_dump` integration for full backups
  - `pg_restore` for recovery
  - Custom format for compression
- Design decisions:
  - Uses native PostgreSQL tools for reliability
  - Validates DSN before operations
  - Provides actionable error messages

### Configuration and Utilities

#### Configuration (`config.py`)

- Purpose: Centralized configuration management
- Sources: Environment variables, defaults
- Core Settings:
  - `pgvector_dsn`: Database connection string
  - `ollama_url`: Ollama server URL
  - `ollama_model`: Embedding model name
  - `intent_model`: Intent classification model name
- Optimization Settings:
  - `search_optimization_parallel_enabled`: Enable parallel optimization
  - `search_optimization_max_workers`: Worker count for parallel execution
- Quality Monitoring Settings:
  - `quality_monitoring_history_size`: Metrics history retention
  - `quality_mrr_threshold`: MRR alert threshold
  - `quality_ndcg_at_10_threshold`: NDCG@10 alert threshold
  - `quality_precision_at_10_threshold`: Precision@10 alert threshold
  - `quality_recall_at_10_threshold`: Recall@10 alert threshold
- Alerting Settings:
  - `quality_alert_handlers`: Enabled handler types (logging, webhook, file)
  - `quality_alert_webhook_url`: Webhook endpoint URL
  - `quality_alert_webhook_timeout`: Webhook request timeout
  - `quality_alert_log_file`: File path for alert persistence
  - `quality_alert_deduplication_window`: Deduplication window in seconds
- Dashboard Settings:
  - `quality_dashboard_auto_refresh_enabled`: Enable auto-refresh
  - `quality_dashboard_auto_refresh_default_interval`: Default refresh interval
  - `quality_dashboard_max_refresh_rate`: Rate limiting for refreshes
- Design decisions:
  - Environment-first for 12-factor app compliance
  - Sensible defaults for development
  - Validation on initialization

#### Input Sanitization (`sanitize.py`)

- Purpose: Validate and sanitize user inputs
- Functionality:
  - SQL injection prevention
  - Query length limits
  - Limit bounds checking
- Design decisions:
  - Applied at service layer for security
  - Clear validation error messages
  - Optional (can disable for internal calls)

#### Database Migrations (`migrations/`)

- Purpose: Schema evolution and upgrades
- Migrations:
  - `add_intent_fields.py`: Add intent-related columns
  - `add_chunking_strategy_field.py`: Add chunking strategy tracking
- Design decisions:
  - Idempotent migrations (safe to run multiple times)
  - Backwards compatible when possible
  - SQL-based for clarity

### Command-Line Interface

#### CLI Commands (`main.py`)

- Purpose: Command-line tools for all system operations
- Commands:
  - `load`: Load documents with optional enhanced processing
  - `search`: Search the knowledge base
  - `start`: Start HTTP or MCP server
  - `backup`: Database backup operations
  - `restore`: Database restore operations
  - `optimize`: Run parameter optimization
  - `quality`: Display quality metrics and alerts
  - `ab-test`: Manage A/B test experiments
- Optimization Command Options:
  - `--metric`: Optimization metric (mrr, ndcg, precision, recall, latency)
  - `--search-type`: Search type (hybrid, multi-stage)
  - `--output`: Results output file
  - `--parallel`: Enable parallel execution
  - `--workers`: Number of parallel workers
  - `--eval-dir`: Evaluation files directory
- Quality Command Options:
  - `--show-metrics`: Display current metrics
  - `--show-alerts`: Display active alerts
  - `--show-history`: Display metrics history
  - `--export`: Export to JSON/CSV
  - `--format`: Output format (table, json, csv)
- A/B Test Command Options:
  - `list`: List all experiments
  - `create`: Create new experiment
  - `start <id>`: Start experiment
  - `stop <id>`: Stop experiment
  - `results <id>`: Show results
  - `export <id>`: Export experiment data

## Data Flow

### Document Loading Flow (Enhanced)

1. User provides markdown files via CLI with optional `--enhanced` and
   `--chunking-strategy` flags
2. Load module reads and processes files
3. Document type classification determines intent (tutorial, how-to, reference,
   explanation)
4. ChunkingStrategyManager selects appropriate chunking strategy based on
   document type
5. Intent-specific chunker splits content using optimized parameters
6. Metadata extractor analyzes content for titles, categories, keywords, code
   blocks
7. Ollama generates embeddings for each chunk
8. Documents, embeddings, and chunking strategy metadata stored in PostgreSQL
9. Verification confirms successful loading with strategy tracking

### Search Flow (Multi-Stage with Intent)

1. User submits query via CLI, HTTP API, or MCP
2. Input sanitization validates query
3. Intent classifier determines query intent (if enabled)
4. Query refiner enhances query based on intent (if enabled)
5. **Stage 1**: Hybrid search retrieves initial candidates
   - BM25 keyword search with intent-based document type filtering
   - Vector similarity search using pgvector
   - RRF fusion of results
6. **Stage 2**: LLM reranker scores relevance (if enabled)
   - Each result evaluated by LLM with intent context
   - Sorted by relevance score
   - Top K results retained
7. **Stage 3**: Context expander retrieves adjacent chunks (if enabled)
   - Fetches before/after chunks respecting chunking strategy boundaries
   - Deduplicates overlapping context
8. Results formatted and returned to user with chunking strategy information
9. Optional AI summarization of results
10. Quality metrics recorded if monitoring enabled

### API Request Flow

1. HTTP request arrives at FastAPI endpoint
2. Pydantic validates request body
3. SearchService executes appropriate search method with intent awareness
4. Results serialized to JSON with metadata
5. Quality metrics updated via QualityMonitor
6. Response returned with status code
7. Errors handled with appropriate HTTP status

### Alerting Flow

1. QualityMonitor evaluates metrics against thresholds
2. Alert generated if threshold breached
3. Deduplication check (15-minute window)
4. If not duplicate, alert dispatched to configured handlers:
   - LoggingAlertHandler: Logs at appropriate severity
   - WebhookAlertHandler: POST to external endpoint with retry
   - FileAlertHandler: Appends to JSONL file
5. Alert history retained for dashboard display

## Configuration Options

### Chunking Strategy Configuration

- `strategy`: Strategy type (tutorial, howto, reference, explanation, semantic)
- `min_chunk_size`: Minimum characters per chunk (varies by strategy)
- `max_chunk_size`: Maximum characters per chunk (varies by strategy)
- `overlap_size`: Character overlap between chunks (varies by strategy)
- `preserve_structure`: Maintain markdown structure (default: true)

### Search Pipeline Configuration

- `initial_limit`: Number of results in stage 1 (default: 20)
- `rerank_top_k`: Results after stage 2 (default: 10)
- `final_limit`: Final results returned (default: 5)
- `chunks_before`: Context chunks before match (default: 1)
- `chunks_after`: Context chunks after match (default: 1)
- `bm25_weight`: Weight for BM25 component (default: 0.5)
- `vector_weight`: Weight for vector component (default: 0.5)
- `use_reranking`: Enable LLM reranking (default: true)
- `use_expansion`: Enable context expansion (default: true)
- `include_reasoning`: Include LLM reasoning (default: false)

### Model Configuration

- Embedding model: `nomic-embed-text:latest` (1536 dimensions)
- Intent model: `llama3.2:3b` (fast, lightweight)
- Generation model: `llama3.1:8b` (summarization, reranking)

## Performance Considerations

### Optimization Strategies

- **Intent-specific chunking**: Optimizes chunk sizes per document type for
  better retrieval
- **Embedding caching**: Reuses embeddings for repeated queries
- **Intent caching**: Avoids redundant intent classifications
- **Batch operations**: Processes multiple documents efficiently
- **Parallel optimization**: Async parallel grid search with configurable workers
- **Database indexes**: HNSW for vectors, GIN for full-text, B-tree for chunking
  strategy
- **Connection pooling**: Reuses database connections

### Scalability

- Horizontal: Multiple server instances with load balancer
- Vertical: Increase database resources for larger datasets
- Strategy-aware caching: Cache by chunking strategy for better hit rates
- Async: Full async implementation for concurrent request handling
- Parallel optimization: Configurable worker count for faster parameter tuning

## Security

- Input sanitization prevents SQL injection
- No authentication (intended for internal/trusted networks)
- Environment variables for sensitive configuration
- Docker isolation for deployment
- API key authentication for metrics endpoints (optional)

## Deployment

### Docker Compose

- Multi-container setup: PostgreSQL, Ollama, API server
- Volume mounts for persistence
- Network isolation
- Health checks for reliability

### Standalone

- Virtual environment for Python dependencies
- Manual PostgreSQL and Ollama setup
- Systemd service for production

## Implementation Status

### Completed Features (Phases 1-6)

- [x] Basic document loading and search
- [x] Enhanced metadata extraction
- [x] Intent classification and query refinement
- [x] Hybrid BM25 + vector search
- [x] Multi-stage retrieval pipeline
- [x] Intent-specific chunking strategies
- [x] LLM reranking and context expansion
- [x] HTTP REST API and MCP server
- [x] Embedded Gradio UI
- [x] Database backup and restore
- [x] Search parameter optimization with grid search
- [x] Quality metrics collection and monitoring
- [x] A/B testing framework
- [x] Quality API endpoints (metrics, alerts, history)
- [x] Parallel async optimization
- [x] CLI commands for optimization, quality, and A/B tests
- [x] Full async conversion for Phase 5 modules
- [x] Dashboard auto-refresh capability
- [x] A/B test export (JSON/CSV)
- [x] Alerting integration (logging, webhook, file handlers)
- [x] Comprehensive testing and documentation

### Current Capabilities

- Document-type-aware chunking with four optimized strategies
- Intent-based query processing and refinement
- Multi-stage retrieval with reranking and context expansion
- Hybrid search combining keyword and semantic approaches
- Rich metadata extraction and storage
- Multiple interface options (CLI, HTTP API, MCP, Web UI)
- Production-ready database operations
- Systematic parameter tuning with parallel execution
- Real-time quality monitoring with threshold alerting
- Statistical A/B testing with export capabilities
- Flexible alerting to multiple destinations

## Future Enhancements

### Planned Improvements (Phase 7+)

- Redis caching for distributed systems
- Query analytics and optimization
- Multi-modal support (images, diagrams)
- User authentication and authorization
- Rate limiting and quotas
- Streaming responses for large results

### Extensibility

- Plugin architecture for custom chunking strategies
- Custom reranking models
- Alternative vector stores (Qdrant, Weaviate)
- Additional LLM providers (OpenAI, Anthropic)
- Custom intent classification models
- Custom alert handlers
