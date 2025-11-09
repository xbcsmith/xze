# Intent Classification Design for XZe

## Overview

The Intent Classifier is a core component of XZe's AI-powered documentation
system that analyzes user queries and repository characteristics to determine
the appropriate Diataxis documentation category. By understanding what users
need and what documentation a repository requires, XZe can generate more
relevant and contextually appropriate documentation.

## Design Philosophy

### Diataxis Framework Alignment

The classifier is built around the Diataxis documentation framework, which XZe
uses to organize all generated documentation:

- **Tutorial**: Learning-oriented, hands-on guidance for beginners
- **How-to**: Task-oriented, problem-solving guides for specific goals
- **Reference**: Information-oriented, technical specifications and API docs
- **Explanation**: Understanding-oriented, conceptual discussion and architecture

This framework provides a proven structure for organizing and generating
technical documentation based on user needs and repository characteristics.

### LLM-Based Classification

The classifier leverages Large Language Models (LLMs) via Ollama rather than
traditional machine learning or rule-based systems:

1. **Natural Language Understanding**: LLMs excel at understanding nuanced
   language and context
2. **Zero-Shot Learning**: No training data required, works immediately
3. **Interpretable Results**: Provides reasoning for classification decisions
4. **Flexible Adaptation**: Easy to adjust prompts for different programming
   languages and domains
5. **Multi-Intent Detection**: Can identify multiple documentation needs in a
   single query

### Rust Implementation Benefits

Implementing the classifier in Rust provides:

1. **Performance**: Native speed with zero-cost abstractions
2. **Memory Safety**: No runtime crashes from memory issues
3. **Concurrency**: Safe async operations with Tokio
4. **Type Safety**: Compile-time guarantees for intent types
5. **Integration**: Seamless integration with existing XZe components

## Architecture

### Component Structure

```text
┌─────────────────────────────────────────────────────────────┐
│                 IntentClassifier (Rust)                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Config    │  │ AI Analysis  │  │  Moka Cache  │      │
│  │  (YAML)     │─▶│   Service    │◀─│   (async)    │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
│                           │                                 │
│                           ▼                                 │
│              ┌────────────────────────┐                     │
│              │  Classification Logic  │                     │
│              ├────────────────────────┤                     │
│              │ - Prompt Building      │                     │
│              │ - Response Parsing     │                     │
│              │ - Intent Validation    │                     │
│              │ - Multi-Intent Support │                     │
│              │ - Confidence Threshold │                     │
│              └────────────────────────┘                     │
│                           │                                 │
│                           ▼                                 │
│                 ClassificationResult                        │
│      (primary, secondary, confidence, reasoning)            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Core Types

```rust
/// Diataxis documentation category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiátaxisIntent {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}

/// Classification confidence level (0.0 - 1.0)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f32);

/// Multi-intent classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Primary detected intent
    pub primary_intent: DiátaxisIntent,
    /// Secondary intents with confidence scores
    pub secondary_intents: Vec<(DiátaxisIntent, Confidence)>,
    /// Overall confidence in primary classification
    pub confidence: Confidence,
    /// LLM reasoning for classification
    pub reasoning: String,
    /// Processing metadata
    pub metadata: ClassificationMetadata,
}
```

### Data Flow

1. **Query Input**: User query or repository analysis enters through
   `classify()` method
2. **Cache Check**: Check if query has been classified before (async cache)
3. **Prompt Generation**: Build classification prompt with query and examples
4. **LLM Invocation**: Send prompt to Ollama via existing AI service
5. **Response Parsing**: Extract primary intent, secondary intents, confidence,
   and reasoning
6. **Validation**: Ensure intents are valid, clamp confidence to [0.0, 1.0]
7. **Threshold Filtering**: Optionally filter low-confidence results
8. **Cache Storage**: Store result with TTL for future queries
9. **Result Return**: Return `ClassificationResult` to caller

## Implementation Details

### Prompt Engineering

The classification prompt guides the LLM to provide structured output:

```text
Structure:
1. Clear instructions for classification task
2. Definitions of each Diataxis category with characteristics
3. Multiple examples per category
4. The user's query or repository description
5. Expected structured output format (PRIMARY, CONFIDENCE, SECONDARY, REASONING)
```

Key design decisions:

- **Explicit Categories**: Each intent clearly defined with purpose and use
  cases
- **Concrete Examples**: Multiple Rust-specific examples per category
- **Structured Output**: Fixed format for reliable parsing
- **Low Temperature**: 0.1 for consistent, deterministic results
- **Multi-Intent Support**: Prompt asks for secondary intents when applicable

Example prompt template:

```text
You are an expert technical documentation classifier for Rust projects.

Categories:
1. TUTORIAL: Learning-oriented, teaches concepts through examples
   Examples: "how to get started", "beginner guide", "first rust program"

2. HOWTO: Task-oriented, solves specific problems
   Examples: "how to deploy", "configure logging", "optimize builds"

3. REFERENCE: Information-oriented, technical specifications
   Examples: "API documentation", "trait bounds", "std::vec::Vec"

4. EXPLANATION: Understanding-oriented, conceptual discussion
   Examples: "why ownership", "async architecture", "design decisions"

Query: "{query}"

Respond with:
PRIMARY: <intent>
CONFIDENCE: <0.0-1.0>
SECONDARY: <intent1:confidence1>, <intent2:confidence2>
REASONING: <explanation>
```

### Caching Strategy

The classifier uses the `moka` crate for async caching:

**Cache Features**:

- Async-first design compatible with Tokio
- Time-to-live (TTL) based expiration
- LRU eviction when capacity reached
- Thread-safe for concurrent access
- Configurable size and TTL

**Cache Configuration**:

```rust
Cache::builder()
    .max_capacity(1000)          // Max 1000 entries
    .time_to_live(Duration::from_secs(3600))  // 1 hour TTL
    .build()
```

**Cache Key Normalization**:

Queries are normalized before caching to improve hit rate:

- Trimmed whitespace
- Converted to lowercase
- Consistent formatting

**Cache Benefits**:

- **Performance**: Sub-5ms latency for cache hits vs 100-500ms for LLM calls
- **Cost Reduction**: Fewer Ollama API calls
- **Consistency**: Same query always returns same result (within TTL)
- **Scalability**: Handles high query volume efficiently

### Error Handling

The classifier implements graceful degradation:

```text
Error Type                    → Response Strategy
─────────────────────────────────────────────────────────
Empty Query                   → Return ValidationError
Ollama Unavailable            → Default to all intents
Invalid LLM Response          → Retry once, then default
Parse Error                   → Use safe defaults (Reference)
Invalid Intent in Response    → Map to closest valid intent
Confidence Below Threshold    → Return with warning flag
```

This ensures the system remains functional even when classification fails. The
default fallback is to generate all documentation types, maintaining backward
compatibility.

### Confidence Scoring

Confidence values interpretation:

- **0.9-1.0**: Very high confidence, clear intent signal
- **0.7-0.9**: High confidence, strong classification
- **0.5-0.7**: Medium confidence, reasonable classification
- **0.3-0.5**: Low confidence, uncertain classification
- **0.0-0.3**: Very low confidence, likely needs all doc types

The system clamps confidence values to [0.0, 1.0] range and provides a
configurable threshold (default 0.5) below which results are flagged as
uncertain.

### Multi-Intent Detection

Unlike the original Python implementation, the Rust version supports
multi-intent detection from the start:

```rust
pub struct ClassificationResult {
    pub primary_intent: DiátaxisIntent,
    pub secondary_intents: Vec<(DiátaxisIntent, Confidence)>,
    // ...
}
```

This allows handling queries like:

- "Explain async and show me how to use tokio" → Explanation + HowTo
- "API documentation with examples" → Reference + Tutorial
- "Architecture overview and deployment guide" → Explanation + HowTo

## Design Decisions

### Why LLM Instead of Traditional ML?

**Traditional ML Approach:**

- Requires labeled training data (expensive to create)
- Needs retraining for domain changes
- Limited to patterns seen in training
- Black box predictions
- Language-specific models

**LLM Approach (Chosen):**

- Zero-shot, works immediately
- Adapts to new domains via prompt engineering
- Provides reasoning for transparency
- Leverages pre-trained knowledge
- Naturally handles Rust terminology

### Why Diataxis Categories?

Diataxis provides a well-researched framework that:

1. **Covers User Needs**: Four categories cover most documentation queries
2. **Clear Boundaries**: Each category has distinct characteristics
3. **Proven Effectiveness**: Used successfully by Rust docs, Django, etc.
4. **Simple Implementation**: Four categories are manageable for classification
5. **XZe Alignment**: Matches XZe's existing documentation generation structure

### Why Async Caching with Moka?

**Moka Benefits:**

- Native Rust implementation (no FFI overhead)
- Async-first design (perfect for Tokio)
- Concurrent access patterns (multiple requests)
- Memory-efficient LRU eviction
- TTL-based expiration (prevents stale results)

**Alternatives Considered:**

- `lru` crate: Not async-friendly
- Redis: Overkill for single-node deployment, adds complexity
- No caching: Unacceptable latency for production

### Why Multi-Intent from Start?

The original design planned multi-intent as a future enhancement. XZe
implements it from the start because:

1. **Real-World Queries**: Users often have multiple documentation needs
2. **Repository Complexity**: Large projects need all doc types
3. **Better UX**: More accurate documentation generation
4. **Implementation Cost**: Minimal additional complexity in Rust
5. **Future-Proof**: Avoids breaking API changes later

### Model Selection

Default model: `llama3.2:3b`

**Rationale:**

- **Size**: 3B parameters provides excellent accuracy/speed balance
- **Availability**: Widely available via Ollama
- **Performance**: 100-300ms latency suitable for real-time use
- **Accuracy**: Sufficient for 4-category classification
- **Memory**: Modest GPU requirements (4-6GB VRAM)

**Alternative Models:**

| Model        | Params | Speed  | Accuracy | Use Case                  |
| ------------ | ------ | ------ | -------- | ------------------------- |
| llama3.2:3b  | 3B     | Fast   | Good     | Default, production       |
| llama3.1:8b  | 8B     | Medium | Better   | High-accuracy needs       |
| llama3.1:70b | 70B    | Slow   | Best     | Offline, batch processing |
| mistral:7b   | 7B     | Medium | Good     | Alternative backend       |

## Performance Characteristics

### Latency

**Without cache (cold):**

- 3B model: 100-300ms per query (p95)
- 8B model: 300-800ms per query (p95)
- 70B model: 1-5s per query (p95)

**With cache (hot):**

- All models: <5ms per query (p99)

**Target SLAs:**

- Cold: <500ms (p95)
- Hot: <5ms (p99)
- Timeout: 10s

### Throughput

Depends on:

- Model size (3B vs 8B vs 70B)
- Hardware capabilities (CPU/GPU)
- Concurrent requests (async processing)
- Cache hit rate (warmup period)

**Example (3B model, single GPU):**

- Cold queries: 5-10 QPS
- Cached queries: 1000+ QPS
- Mixed (70% cache hit): 50-100 QPS

### Memory Usage

**Rust Process:**

- Classifier instance: ~500KB
- Cache (1000 entries): ~100KB
- Per-request allocation: ~10KB

**Ollama Process (separate):**

- 3B model: 3-4GB
- 8B model: 8-10GB
- 70B model: 70-80GB

### Accuracy

**Target Metrics:**

- Primary intent accuracy: >85% on test set
- Confidence calibration: Within 10% of actual accuracy
- Multi-intent detection: >75% recall on queries with 2+ intents

## Integration Points

### With AI Analysis Service

The classifier integrates with XZe's existing AI service:

```rust
use xze_core::ai::AIAnalysisService;

impl AIAnalysisService {
    /// Classify user query
    pub async fn classify_query(&self, query: &str) -> Result<ClassificationResult>;

    /// Analyze repository to determine needed doc types
    pub async fn analyze_documentation_needs(
        &self,
        repo: &Repository
    ) -> Result<Vec<DiátaxisIntent>>;
}
```

### With Documentation Generator

Guides documentation generation:

```rust
impl AIDocumentationGenerator {
    /// Generate only relevant documentation types
    async fn generate_for_intents(
        &self,
        repo: &Repository,
        intents: Vec<DiátaxisIntent>
    ) -> Result<Vec<Document>>;
}
```

### With Pipeline Controller

Configures pipeline jobs:

```rust
impl PipelineController {
    /// Submit job with intent-based filtering
    pub async fn submit_with_intents(
        &self,
        repo_id: RepositoryId,
        intents: Vec<DiátaxisIntent>
    ) -> Result<JobId>;
}
```

### With CLI

Provides user-facing classification:

```rust
// xze classify <query>
pub async fn handle_classify_command(query: String) -> Result<()>;
```

### With Server Mode (Phase 4)

Enables intent-aware search and retrieval:

```rust
// REST API endpoint
POST /api/v1/classify
{
    "query": "how to deploy a rust app"
}

Response:
{
    "primary_intent": "howto",
    "confidence": 0.89,
    "secondary_intents": [
        {"intent": "reference", "confidence": 0.45}
    ],
    "reasoning": "Query explicitly asks 'how to', indicating task-oriented need"
}
```

## Testing Strategy

### Unit Tests

Focus on:

- Intent enum validation and conversion
- Confidence clamping and validation
- Response parsing with various formats
- Cache behavior (hit/miss/eviction)
- Error handling and fallbacks
- Multi-intent detection logic

### Integration Tests

Focus on:

- Ollama connectivity and model availability
- End-to-end classification with real models
- Cache persistence and TTL behavior
- Concurrent request handling
- Timeout and retry logic

### Performance Tests

Focus on:

- Latency measurements (cold and hot)
- Cache effectiveness (hit rate over time)
- Throughput limits (QPS under load)
- Memory usage under sustained load
- Model comparison benchmarks

### Accuracy Tests

Focus on:

- Test dataset classification accuracy
- Confidence calibration
- Multi-intent detection recall/precision
- Edge case handling
- Cross-model consistency

## Security Considerations

### Input Validation

- **Query Length**: Limit to reasonable size (e.g., 1000 chars)
- **Character Sanitization**: Handle special characters safely
- **Rate Limiting**: Prevent abuse in server mode
- **Resource Limits**: Cap concurrent classifications

### Prompt Injection

The classifier is potentially vulnerable to prompt injection attacks. Mitigations:

- Use system/user message separation when model supports it
- Validate output format strictly (regex parsing)
- Log suspicious classification patterns
- Limit query complexity
- Set aggressive timeouts

### Data Privacy

- **No Persistent Storage**: Queries cached in memory only
- **TTL Expiration**: Automatic cleanup after 1 hour
- **No Logging**: Queries not logged by default (configurable)
- **Local Processing**: All data stays on user's machine
- **Disable Cache Option**: For sensitive deployments

### Resource Exhaustion

- **Request Limits**: Cap concurrent requests
- **Timeout Protection**: 10s max per classification
- **Memory Bounds**: Fixed cache size
- **CPU Limits**: Use tokio task limits

## Monitoring and Observability

### Metrics

Track via Prometheus (optional):

```rust
// Classification metrics
classification_total{intent, cached}
classification_duration_seconds{cached}
classification_confidence{intent}
cache_hit_rate
cache_size_bytes
classification_errors_total{error_type}
```

### Logging

Structured logging with tracing:

```rust
info!("Classified query",
    intent = ?result.primary_intent,
    confidence = result.confidence.0,
    cached = result.metadata.cached,
    duration_ms = result.metadata.duration_ms
);
```

### Health Checks

```rust
pub async fn health_check() -> HealthStatus {
    // Check Ollama availability
    // Check cache health
    // Return status
}
```

## Future Enhancements

### Planned Improvements (Phase 2)

1. **Context-Aware Classification**: Use repository metadata for better accuracy
2. **Feedback Loop**: Learn from user corrections and regeneration requests
3. **Confidence Tuning**: Adjust thresholds based on observed accuracy
4. **Batch Optimization**: Classify multiple queries in single LLM call
5. **Model Switching**: Fallback to smaller/faster model on timeout

### Research Directions (Long-term)

1. **Fine-Tuned Models**: Train on Rust documentation corpus
2. **Hybrid Approach**: Combine LLM with rule-based heuristics
3. **Active Learning**: Continuously improve from production data
4. **Multilingual Support**: Classify non-English queries
5. **Domain Adaptation**: Specialize for different programming languages
6. **Custom Categories**: Allow user-defined intent types beyond Diataxis
7. **Intent Prediction**: Predict needs before user asks

## Comparison with Original Design

### Improvements Over Python Implementation

1. **Multi-Intent**: Built-in from start (not future work)
2. **Performance**: Native Rust speed (2-3x faster)
3. **Type Safety**: Compile-time guarantees (fewer runtime errors)
4. **Async Native**: Tokio integration (better concurrency)
5. **Memory Safety**: No GC pauses or memory leaks
6. **Integration**: Seamless with XZe components

### Maintained from Original

1. **LLM-Based**: Still uses Ollama for classification
2. **Diataxis**: Same four-category framework
3. **Caching**: Similar strategy with improved implementation
4. **Prompt Engineering**: Adapted prompts for Rust context
5. **Graceful Degradation**: Same error handling philosophy

### Key Architectural Differences

| Aspect       | Python Original     | Rust XZe         |
| ------------ | ------------------- | ---------------- |
| Language     | Python 3.11+        | Rust 1.70+       |
| Async        | asyncio             | Tokio            |
| Cache        | functools.lru_cache | moka async cache |
| Types        | Dynamic             | Static           |
| Use Case     | RAG search          | Doc generation   |
| Multi-Intent | Future              | Built-in         |
| Integration  | FastAPI             | XZe pipeline     |

## Conclusion

The Intent Classifier provides a robust, LLM-based solution for understanding
documentation needs in XZe. Its design prioritizes:

- **Accuracy** through careful prompt engineering and multi-intent support
- **Performance** through intelligent async caching and native Rust speed
- **Reliability** through graceful error handling and fallback strategies
- **Maintainability** through strong typing and clear abstractions
- **Integration** through seamless XZe component interaction

The Diataxis-aligned approach ensures that classification maps naturally to
XZe's documentation generation structure, enabling more efficient and relevant
documentation creation for Rust projects.

By implementing this in Rust from the start with multi-intent support, XZe
provides a production-ready intent classification system that improves upon the
original Python design while maintaining its core benefits of LLM-based
understanding and framework alignment.
