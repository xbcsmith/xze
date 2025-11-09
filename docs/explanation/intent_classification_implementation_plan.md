# Intent Classification Implementation Plan for XZe

## Overview

This document outlines the implementation plan for an intent classification
system in XZe, adapted from the original Python design to Rust. The classifier
will analyze user queries and documentation needs to determine the appropriate
Diataxis category (Tutorial, How-to, Reference, Explanation), enabling
intelligent documentation generation and retrieval.

## Executive Summary

The intent classifier will be a core component of XZe's AI-powered
documentation system, using Ollama-based LLMs to understand user needs and
guide documentation generation. Unlike the original Python implementation
focused on RAG search, this Rust implementation will integrate deeply with
XZe's documentation generation pipeline.

**Timeline**: 3-4 weeks, 1 developer

**Dependencies**: Existing AI service, Diataxis framework types

**Success Criteria**:
- Sub-200ms classification latency (cached)
- 85%+ classification accuracy on test set
- Zero-downtime integration with pipeline

## Project Context

### XZe's Documentation Philosophy

XZe generates documentation following the Diataxis framework:
- **Tutorial**: Getting started guides, learning-oriented content
- **How-to**: Task-specific guides, problem-solving recipes
- **Reference**: API documentation, technical specifications
- **Explanation**: Architecture overviews, design decisions

### Use Cases in XZe

1. **Documentation Generation**: Determine which doc types to generate for a
   repository
2. **Query Understanding**: Classify user requests for specific documentation
3. **Search Enhancement**: Filter and rank documentation by intent (Phase 4)
4. **CLI Interaction**: Guide users to appropriate documentation
5. **Pipeline Optimization**: Skip generating unused documentation types

## Architecture Design

### Component Structure

```rust
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
│              │ - Prompt Builder       │                     │
│              │ - Response Parser      │                     │
│              │ - Intent Validator     │                     │
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

/// Classification confidence level
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f32); // 0.0 - 1.0

/// Multi-intent classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Primary detected intent
    pub primary_intent: DiátaxisIntent,
    /// Secondary intents (if query has multiple purposes)
    pub secondary_intents: Vec<(DiátaxisIntent, Confidence)>,
    /// Overall confidence in primary classification
    pub confidence: Confidence,
    /// LLM reasoning for classification
    pub reasoning: String,
    /// Processing metadata
    pub metadata: ClassificationMetadata,
}

/// Classification metadata for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    /// Was result from cache?
    pub cached: bool,
    /// Processing duration
    pub duration_ms: u64,
    /// Model used for classification
    pub model: String,
    /// Timestamp of classification
    pub timestamp: DateTime<Utc>,
}

/// Classifier configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ClassifierConfig {
    /// Ollama model to use
    pub model: String,
    /// Temperature for classification (0.0 - 1.0)
    pub temperature: f32,
    /// Minimum confidence threshold (0.0 - 1.0)
    pub confidence_threshold: f32,
    /// Enable multi-intent detection
    pub enable_multi_intent: bool,
    /// Cache size (number of entries)
    pub cache_size: u64,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable classification metrics
    pub enable_metrics: bool,
}
```

### Integration Points

```rust
// With AI Service
impl IntentClassifier {
    pub fn new(ai_service: Arc<AIAnalysisService>, config: ClassifierConfig) -> Self;
    pub async fn classify(&self, query: &str) -> Result<ClassificationResult>;
    pub async fn classify_batch(&self, queries: Vec<String>) -> Result<Vec<ClassificationResult>>;
}

// With Documentation Generator
impl AIDocumentationGenerator {
    /// Determine which doc types to generate based on repository analysis
    async fn determine_doc_priorities(&self, repo: &Repository) -> Vec<DiátaxisIntent>;
}

// With CLI
impl Commands {
    /// Classify a user query
    async fn classify_intent(&self, query: String);
}

// With Pipeline Controller
impl PipelineController {
    /// Submit job with intent-based configuration
    async fn submit_with_intent(&self, repo_id: RepositoryId, intents: Vec<DiátaxisIntent>);
}
```

## Implementation Phases

### Phase 1: Core Classification Engine (Week 1)

**Priority**: Critical

**Objective**: Implement basic single-intent classification with caching

#### Tasks

1. **Create intent classification module** (2 days)
   - [ ] Create `crates/core/src/ai/intent.rs`
   - [ ] Define `DiátaxisIntent` enum with From/Display traits
   - [ ] Implement `Confidence` newtype with validation
   - [ ] Create `ClassificationResult` struct
   - [ ] Add configuration types and defaults
   - [ ] Add module to `crates/core/src/ai/mod.rs`

2. **Implement prompt engineering** (2 days)
   - [ ] Create `PromptBuilder` for classification prompts
   - [ ] Define Diataxis category descriptions
   - [ ] Add example queries for each category
   - [ ] Implement structured output parsing
   - [ ] Add prompt templates to config
   - [ ] Test prompt variations for accuracy

3. **Build classification logic** (2 days)
   - [ ] Implement `IntentClassifier::new()`
   - [ ] Create `classify()` async method
   - [ ] Add LLM response parsing
   - [ ] Implement intent validation
   - [ ] Add confidence clamping (0.0 - 1.0)
   - [ ] Handle parsing errors gracefully

4. **Add caching layer** (1 day)
   - [ ] Add `moka` dependency to Cargo.toml
   - [ ] Implement async cache with TTL
   - [ ] Add cache key normalization
   - [ ] Implement `clear_cache()` method
   - [ ] Add cache hit/miss metrics
   - [ ] Test cache behavior under load

**Deliverables**:
- `crates/core/src/ai/intent.rs` - Core classification
- Unit tests (>80% coverage)
- Integration tests with mock LLM
- Documentation in code

**Success Criteria**:
- Classification completes in <500ms (cold)
- Cache hits complete in <5ms
- Valid results for all test queries

---

### Phase 2: Multi-Intent Detection (Week 2)

**Priority**: High

**Objective**: Support queries with multiple documentation needs

#### Tasks

1. **Enhance prompt for multi-intent** (1 day)
   - [ ] Update prompt template for multiple intents
   - [ ] Add examples of multi-intent queries
   - [ ] Define output format for secondary intents
   - [ ] Test prompt with various query types

2. **Implement multi-intent parsing** (2 days)
   - [ ] Parse primary + secondary intents
   - [ ] Extract confidence per intent
   - [ ] Sort secondary intents by confidence
   - [ ] Validate intent combinations
   - [ ] Add fallback for parsing failures

3. **Add confidence threshold logic** (1 day)
   - [ ] Implement threshold filtering
   - [ ] Skip low-confidence classifications
   - [ ] Add "uncertain" intent type
   - [ ] Provide confidence-based recommendations
   - [ ] Test threshold edge cases

4. **Optimize prompt and parsing** (1 day)
   - [ ] Benchmark different prompt variations
   - [ ] Profile parsing performance
   - [ ] Optimize string allocations
   - [ ] Add fast path for single-intent
   - [ ] Document performance characteristics

**Deliverables**:
- Enhanced classification with multiple intents
- Confidence threshold configuration
- Performance benchmarks
- Updated documentation

**Success Criteria**:
- Correctly identifies 2+ intents when present
- Maintains <500ms latency with multi-intent
- Threshold filtering works as expected

---

### Phase 3: Integration & CLI (Week 3)

**Priority**: High

**Objective**: Integrate with XZe components and add CLI interface

#### Tasks

1. **Integrate with AI service** (1 day)
   - [ ] Update `AIAnalysisService` to include classifier
   - [ ] Add `classify_query()` method
   - [ ] Share model configuration
   - [ ] Reuse Ollama client connection
   - [ ] Test integration with real models

2. **Add CLI commands** (2 days)
   - [ ] Create `xze classify <query>` command
   - [ ] Add `--verbose` flag for reasoning
   - [ ] Display confidence levels
   - [ ] Show secondary intents
   - [ ] Add JSON output format
   - [ ] Include examples in help text

3. **Pipeline integration** (2 days)
   - [ ] Add intent-based job configuration
   - [ ] Update `JobConfig` with intent list
   - [ ] Filter documentation generation by intent
   - [ ] Add intent to job metadata
   - [ ] Test end-to-end pipeline flow

4. **Configuration management** (1 day)
   - [ ] Add classifier config to `XzeConfig`
   - [ ] Support environment variables
   - [ ] Add config validation
   - [ ] Document all configuration options
   - [ ] Provide sensible defaults

**Deliverables**:
- CLI classify command
- Pipeline integration
- Configuration support
- Integration test suite

**Success Criteria**:
- CLI command works with live models
- Pipeline generates filtered documentation
- Configuration is flexible and documented

---

### Phase 4: Monitoring & Optimization (Week 4)

**Priority**: Medium

**Objective**: Add observability, metrics, and performance tuning

#### Tasks

1. **Add metrics and telemetry** (2 days)
   - [ ] Track classification latency
   - [ ] Monitor cache hit rate
   - [ ] Count classifications per intent
   - [ ] Track confidence distribution
   - [ ] Add model performance metrics
   - [ ] Export metrics for Prometheus

2. **Performance optimization** (2 days)
   - [ ] Profile hot paths
   - [ ] Optimize prompt construction
   - [ ] Reduce allocations in parsing
   - [ ] Implement batch classification
   - [ ] Add parallel processing for batches
   - [ ] Benchmark before/after

3. **Error handling improvements** (1 day)
   - [ ] Add retry logic for transient failures
   - [ ] Implement circuit breaker pattern
   - [ ] Add graceful degradation
   - [ ] Improve error messages
   - [ ] Log classification failures

4. **Documentation and examples** (1 day)
   - [ ] Write usage guide in `docs/how_to/`
   - [ ] Add explanation doc in `docs/explanation/`
   - [ ] Create reference doc in `docs/reference/`
   - [ ] Add code examples
   - [ ] Document best practices

**Deliverables**:
- Metrics and monitoring
- Performance optimizations
- Comprehensive documentation
- Example applications

**Success Criteria**:
- Metrics are accurate and useful
- 20%+ performance improvement
- Documentation is clear and complete

---

## Technical Specifications

### Prompt Engineering

#### Classification Prompt Template

```text
You are an expert technical documentation classifier. Your task is to analyze
user queries and determine which type of documentation they need according to
the Diataxis framework.

Documentation Categories:

1. TUTORIAL: Learning-oriented content for beginners
   - Teaching a concept through hands-on examples
   - Guiding users through their first experience
   - Building confidence and skills
   Examples: "how to get started", "beginner guide", "first project"

2. HOWTO: Task-oriented problem-solving guides
   - Solving a specific problem
   - Step-by-step instructions for a goal
   - Practical recipes and workflows
   Examples: "how to deploy", "configure X", "set up Y"

3. REFERENCE: Information-oriented technical specifications
   - Looking up API details
   - Technical specifications
   - Parameter documentation
   Examples: "API documentation", "function parameters", "configuration options"

4. EXPLANATION: Understanding-oriented conceptual discussion
   - Understanding architecture or design
   - Exploring concepts and tradeoffs
   - Background and context
   Examples: "why does X work this way", "architecture overview", "design decisions"

Query: "{query}"

Analyze this query and respond in the following format:
PRIMARY: <intent>
CONFIDENCE: <0.0-1.0>
SECONDARY: <intent1:confidence1>, <intent2:confidence2> (if applicable)
REASONING: <brief explanation>

Your response:
```

#### Parsing Strategy

```rust
fn parse_classification_response(response: &str) -> Result<ClassificationResult> {
    // Extract structured fields using regex
    let primary_re = Regex::new(r"PRIMARY:\s*(\w+)")?;
    let confidence_re = Regex::new(r"CONFIDENCE:\s*([\d.]+)")?;
    let secondary_re = Regex::new(r"SECONDARY:\s*(.+)")?;
    let reasoning_re = Regex::new(r"REASONING:\s*(.+)")?;

    // Parse and validate each field
    // Return structured result or error
}
```

### Caching Strategy

#### Cache Implementation

```rust
use moka::future::Cache;

pub struct IntentCache {
    cache: Cache<String, ClassificationResult>,
}

impl IntentCache {
    pub fn new(config: &ClassifierConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.cache_size)
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .build();
        Self { cache }
    }

    pub async fn get(&self, key: &str) -> Option<ClassificationResult> {
        self.cache.get(&normalize_key(key)).await
    }

    pub async fn insert(&self, key: String, result: ClassificationResult) {
        self.cache.insert(normalize_key(&key), result).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
    }
}

fn normalize_key(query: &str) -> String {
    query.trim().to_lowercase()
}
```

### Error Handling

```rust
#[derive(Error, Debug)]
pub enum ClassificationError {
    #[error("Empty query provided")]
    EmptyQuery,

    #[error("LLM service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Failed to parse classification response: {0}")]
    ParseError(String),

    #[error("Invalid intent: {0}")]
    InvalidIntent(String),

    #[error("Confidence below threshold: {actual} < {threshold}")]
    LowConfidence { actual: f32, threshold: f32 },
}

impl From<ClassificationError> for XzeError {
    fn from(err: ClassificationError) -> Self {
        XzeError::AiService {
            message: err.to_string(),
        }
    }
}
```

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Cold Classification | <500ms | 95th percentile |
| Cache Hit Latency | <5ms | 99th percentile |
| Cache Hit Rate | >70% | After warmup |
| Accuracy | >85% | Test dataset |
| Memory Usage | <50MB | Steady state |
| Throughput (cached) | >1000 QPS | Single instance |

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_validation() {
        assert!(DiátaxisIntent::from_str("tutorial").is_ok());
        assert!(DiátaxisIntent::from_str("invalid").is_err());
    }

    #[test]
    fn test_confidence_clamping() {
        assert_eq!(Confidence::new(1.5), Confidence(1.0));
        assert_eq!(Confidence::new(-0.5), Confidence(0.0));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let classifier = create_test_classifier();
        let query = "how to get started";

        let result1 = classifier.classify(query).await.unwrap();
        let result2 = classifier.classify(query).await.unwrap();

        assert!(result1.metadata.cached == false);
        assert!(result2.metadata.cached == true);
    }

    #[tokio::test]
    async fn test_multi_intent_detection() {
        let classifier = create_test_classifier();
        let query = "explain the API and how to use it";

        let result = classifier.classify(query).await.unwrap();

        assert_eq!(result.primary_intent, DiátaxisIntent::Reference);
        assert!(!result.secondary_intents.is_empty());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires Ollama running
async fn test_real_classification() {
    let ai_service = create_test_ai_service();
    let config = ClassifierConfig::default();
    let classifier = IntentClassifier::new(ai_service, config);

    let test_queries = vec![
        ("how to install rust", DiátaxisIntent::HowTo),
        ("getting started with cargo", DiátaxisIntent::Tutorial),
        ("std::vec::Vec API", DiátaxisIntent::Reference),
        ("why use async/await", DiátaxisIntent::Explanation),
    ];

    for (query, expected) in test_queries {
        let result = classifier.classify(query).await.unwrap();
        assert_eq!(result.primary_intent, expected);
        assert!(result.confidence.0 > 0.5);
    }
}
```

### Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_classification(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let classifier = create_bench_classifier();

    c.bench_function("classify_cached", |b| {
        b.to_async(&rt).iter(|| async {
            classifier.classify(black_box("how to deploy")).await
        });
    });

    c.bench_function("classify_cold", |b| {
        b.to_async(&rt).iter(|| async {
            classifier.clear_cache().await;
            classifier.classify(black_box("how to deploy")).await
        });
    });
}
```

## Configuration

### Default Configuration

```yaml
# config/default.yaml
intent_classifier:
  # Model configuration
  model: "llama3.2:3b"
  temperature: 0.1

  # Classification settings
  confidence_threshold: 0.5
  enable_multi_intent: true

  # Cache settings
  cache_size: 1000
  cache_ttl_seconds: 3600

  # Monitoring
  enable_metrics: true
```

### Environment Variables

```bash
# Override configuration via environment
XZE_CLASSIFIER_MODEL=llama3.1:8b
XZE_CLASSIFIER_CONFIDENCE_THRESHOLD=0.7
XZE_CLASSIFIER_CACHE_SIZE=5000
XZE_CLASSIFIER_ENABLE_MULTI_INTENT=true
```

## Dependencies

### New Crate Dependencies

```toml
[dependencies]
# Async caching
moka = { version = "0.12", features = ["future"] }

# Regex for parsing
regex = "1.10"

# Metrics (optional)
prometheus = { version = "0.13", optional = true }
```

### Existing Dependencies

- `tokio` - Async runtime
- `serde` - Serialization
- `chrono` - Timestamps
- `tracing` - Logging
- Existing AI service and types

## Usage Examples

### CLI Usage

```bash
# Classify a single query
xze classify "how to get started with rust"
# Output:
# Intent: Tutorial
# Confidence: 0.92
# Reasoning: Query asks about getting started, which is learning-oriented

# Verbose output with secondary intents
xze classify --verbose "explain the API and show me how to use it"
# Output:
# Primary Intent: Reference (0.85)
# Secondary Intents:
#   - HowTo (0.72)
#   - Explanation (0.65)
# Reasoning: Query has multiple purposes...

# JSON output for scripting
xze classify --json "cargo build errors"
```

### Programmatic Usage

```rust
use xze_core::ai::intent::{IntentClassifier, ClassifierConfig};

// Initialize classifier
let ai_service = Arc::new(AIAnalysisService::new(...));
let config = ClassifierConfig::default();
let classifier = IntentClassifier::new(ai_service, config);

// Classify single query
let result = classifier.classify("how to deploy").await?;
println!("Intent: {:?}", result.primary_intent);
println!("Confidence: {}", result.confidence);

// Classify batch
let queries = vec![
    "getting started guide".to_string(),
    "API reference".to_string(),
];
let results = classifier.classify_batch(queries).await?;

// Use in pipeline
let job_config = JobConfig {
    doc_categories: result.to_doc_types(),
    ..Default::default()
};
```

### Integration with Documentation Generator

```rust
impl AIDocumentationGenerator {
    async fn generate_for_repository(&self, repo: &Repository) -> Result<Vec<Document>> {
        // Analyze repository to determine documentation needs
        let analysis = self.analyze_repository_intent(repo).await?;

        // Generate only relevant documentation types
        let mut documents = Vec::new();

        if analysis.needs_tutorial() {
            documents.push(self.generate_tutorial(repo).await?);
        }

        if analysis.needs_howto() {
            documents.extend(self.generate_howto_guides(repo).await?);
        }

        if analysis.needs_reference() {
            documents.push(self.generate_api_reference(repo).await?);
        }

        if analysis.needs_explanation() {
            documents.push(self.generate_architecture_explanation(repo).await?);
        }

        Ok(documents)
    }
}
```

## Migration Path

### For Existing XZe Users

1. **Opt-in Feature**: Classification is optional, existing pipelines continue
   working
2. **Gradual Rollout**: Enable classification per repository via config
3. **Backward Compatibility**: All existing documentation types still generated
   by default
4. **Performance**: No impact when disabled, minimal impact when enabled

### Deployment Strategy

1. **Phase 1**: Deploy with classification disabled by default
2. **Phase 2**: Enable for new repositories
3. **Phase 3**: Migrate existing repositories opt-in
4. **Phase 4**: Enable by default with opt-out

## Risk Assessment

### High Risk

1. **LLM Availability**: Ollama must be running
   - **Mitigation**: Graceful degradation, generate all doc types on failure

2. **Classification Accuracy**: Wrong intent leads to missing documentation
   - **Mitigation**: Multi-intent detection, low threshold defaults

### Medium Risk

1. **Performance Impact**: Classification adds latency
   - **Mitigation**: Aggressive caching, async processing

2. **Prompt Drift**: LLM behavior changes over time
   - **Mitigation**: Versioned prompts, regression tests

### Low Risk

1. **Cache Invalidation**: Stale results after model changes
   - **Mitigation**: TTL-based expiration, manual clear command

## Success Metrics

### Key Performance Indicators

1. **Accuracy**: >85% correct classification on test set
2. **Latency**: <500ms cold, <5ms cached (p95)
3. **Cache Hit Rate**: >70% after warmup
4. **Pipeline Efficiency**: 30% reduction in unnecessary doc generation
5. **User Satisfaction**: Positive feedback on CLI classify command

### Monitoring Dashboard

Track:
- Classifications per intent (distribution)
- Average confidence scores
- Cache hit/miss ratio
- Classification latency histogram
- Error rate and types
- Model performance over time

## Future Enhancements

### Phase 5+ (Beyond Initial Release)

1. **Custom Intent Types**: Allow users to define domain-specific categories
2. **Intent History**: Track and learn from user patterns
3. **Active Learning**: Improve classification from user feedback
4. **Fine-Tuned Models**: Train specialized models on Rust documentation
5. **Multilingual Support**: Classify queries in multiple languages
6. **Context-Aware**: Use repository metadata for better classification
7. **Intent Prediction**: Predict user needs before they ask
8. **A/B Testing**: Compare different prompts and models
9. **Distributed Caching**: Redis integration for multi-node deployments
10. **Real-Time Feedback**: Learn from user corrections

## References

- Original Design: `docs/explanation/intent_classification_design.md`
- Diataxis Framework: https://diataxis.fr/
- Phase 3.1 Completion: `docs/explanation/phase3_1_completion.md`
- AI Service: `crates/core/src/ai/mod.rs`
- Pipeline Controller: `crates/core/src/pipeline/controller.rs`
- AGENTS.md: Development guidelines

## Appendix A: Test Dataset

### Sample Queries for Validation

```rust
// Tutorial queries
"how to get started with rust"
"beginner guide to cargo"
"first rust program"
"learning rust basics"

// How-to queries
"how to deploy a rust application"
"configure logging in rust"
"set up CI/CD pipeline"
"optimize rust build times"

// Reference queries
"std::vec::Vec API documentation"
"tokio runtime parameters"
"cargo.toml configuration options"
"rust trait bounds syntax"

// Explanation queries
"why does rust use ownership"
"understanding async/await in rust"
"rust memory model explained"
"architectural decisions in tokio"

// Multi-intent queries
"explain async and show me how to use tokio"
"what is a trait and how do I implement one"
"architecture of rust compiler and building it"
```

## Appendix B: Prompt Variations

### Alternative Prompt Styles

For experimentation and A/B testing:

**Style 1: Concise**
- Shorter descriptions
- Fewer examples
- Faster processing

**Style 2: Detailed**
- Comprehensive explanations
- Many examples
- Higher accuracy

**Style 3: Structured**
- XML/JSON output format
- Easier parsing
- More reliable extraction

## Conclusion

This implementation plan provides a comprehensive roadmap for adding intent
classification to XZe. The Rust-based implementation will integrate seamlessly
with existing components while providing the flexibility and performance
required for production use.

By following this plan, XZe will gain intelligent documentation generation
capabilities, reducing unnecessary work and improving user experience through
intent-aware processing.

The phased approach ensures incremental value delivery, with each phase
building on the previous one while maintaining system stability and code
quality standards defined in AGENTS.md.
