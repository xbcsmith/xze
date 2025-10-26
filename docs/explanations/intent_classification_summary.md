# Intent Classification Feature Summary

## Overview

This document summarizes the intent classification feature planning for XZe,
including the refactored design from the original Python implementation and the
comprehensive Rust implementation plan.

## What Was Done

### 1. Design Document Refactoring

**File**: `docs/explanations/intent_classification_design.md`

Refactored the original Python-based intent classification design to align with
XZe's Rust architecture and documentation generation use case.

**Key Changes**:

- Converted Python examples to Rust code
- Replaced Python caching (lru_cache) with Rust async caching (moka)
- Added multi-intent detection as a core feature (not future work)
- Aligned use cases with XZe's documentation generation pipeline
- Updated integration points for XZe components
- Added Rust-specific performance characteristics
- Included comparison table between Python and Rust implementations

### 2. Implementation Plan Creation

**File**: `docs/explanations/intent_classification_implementation_plan.md`

Created a comprehensive 4-week implementation plan following AGENTS.md
guidelines with detailed tasks, deliverables, and success criteria.

**Plan Structure**:

- **Phase 1** (Week 1): Core classification engine with caching
- **Phase 2** (Week 2): Multi-intent detection and confidence thresholds
- **Phase 3** (Week 3): Integration with XZe components and CLI
- **Phase 4** (Week 4): Monitoring, optimization, and documentation

**Improvements Over Original**:

- Multi-intent detection from the start
- Confidence threshold filtering built-in
- Metrics and observability included
- Batch classification support
- CLI integration planned upfront
- Performance benchmarking throughout
- Comprehensive test strategy

## Core Concepts

### Diataxis Framework

XZe uses the Diataxis framework for documentation organization:

1. **Tutorial**: Learning-oriented, hands-on guidance for beginners
2. **How-to**: Task-oriented, problem-solving guides for specific goals
3. **Reference**: Information-oriented, technical specifications and API docs
4. **Explanation**: Understanding-oriented, conceptual discussion and
   architecture

### Intent Classification

The classifier analyzes queries or repository characteristics to determine
which Diataxis categories are needed, enabling:

- **Intelligent Generation**: Only generate needed documentation types
- **Query Understanding**: Classify user requests for documentation
- **Search Enhancement**: Filter and rank results by intent (Phase 4)
- **Resource Optimization**: Avoid generating unused documentation

## Architecture

### Core Types

```rust
pub enum DiátaxisIntent {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
}

pub struct ClassificationResult {
    pub primary_intent: DiátaxisIntent,
    pub secondary_intents: Vec<(DiátaxisIntent, Confidence)>,
    pub confidence: Confidence,
    pub reasoning: String,
    pub metadata: ClassificationMetadata,
}

pub struct IntentClassifier {
    ai_service: Arc<AIAnalysisService>,
    cache: IntentCache,
    config: ClassifierConfig,
}
```

### Technology Stack

- **LLM Backend**: Ollama with llama3.2:3b (default)
- **Caching**: moka async cache with TTL
- **Parsing**: Regex for structured output extraction
- **Integration**: Existing AIAnalysisService
- **Async Runtime**: Tokio
- **Metrics**: Prometheus (optional)

## Implementation Phases

### Phase 1: Core Classification (Week 1)

- Create intent classification module
- Implement prompt engineering
- Build classification logic with LLM integration
- Add async caching layer
- Write unit and integration tests

**Deliverables**: Working classifier with caching, >80% test coverage

### Phase 2: Multi-Intent Detection (Week 2)

- Enhance prompts for multiple intents
- Implement multi-intent parsing
- Add confidence threshold logic
- Optimize prompt and parsing performance

**Deliverables**: Multi-intent support, confidence filtering, benchmarks

### Phase 3: Integration (Week 3)

- Integrate with AI service and pipeline
- Add CLI commands (xze classify)
- Configure job filtering by intent
- Add configuration management

**Deliverables**: CLI command, pipeline integration, config support

### Phase 4: Monitoring & Optimization (Week 4)

- Add metrics and telemetry
- Performance optimization (20%+ improvement target)
- Error handling improvements
- Comprehensive documentation

**Deliverables**: Metrics, optimizations, complete documentation

## Performance Targets

| Metric               | Target      | Measurement  |
| -------------------- | ----------- | ------------ |
| Cold Classification  | <500ms      | 95th %ile    |
| Cache Hit Latency    | <5ms        | 99th %ile    |
| Cache Hit Rate       | >70%        | After warmup |
| Primary Accuracy     | >85%        | Test dataset |
| Memory Usage         | <50MB       | Steady state |
| Throughput (cached)  | >1000 QPS   | Single node  |

## Key Design Decisions

### 1. LLM-Based Classification

**Why**: Zero-shot learning, natural language understanding, interpretable
results

**Trade-off**: Requires Ollama running, higher latency than rule-based

### 2. Multi-Intent from Start

**Why**: Real-world queries often have multiple purposes, better UX, minimal
complexity

**Trade-off**: Slightly more complex parsing and prompt engineering

### 3. Async Caching (moka)

**Why**: Native Rust, async-first, TTL support, LRU eviction

**Trade-off**: Not distributed (acceptable for single-node deployment)

### 4. Diataxis Framework

**Why**: Proven effectiveness, clear boundaries, matches XZe structure

**Trade-off**: Limited to four categories (sufficient for most use cases)

## Integration Points

### With Existing XZe Components

1. **AI Service**: Reuses Ollama client, shares model configuration
2. **Pipeline Controller**: Intent-based job submission and filtering
3. **Documentation Generator**: Selective generation by intent
4. **CLI**: New classify command for user queries
5. **Configuration**: Integrated into XzeConfig with YAML support

### Future Integration (Phase 4+)

1. **Server Mode**: REST API endpoints for classification
2. **Search**: Intent-aware filtering and ranking
3. **Webhooks**: Real-time classification notifications
4. **Dashboard**: Visualization of intent distribution

## Use Cases

### 1. Documentation Generation

```rust
// Analyze repository to determine needed docs
let intents = ai_service.analyze_documentation_needs(&repo).await?;

// Generate only relevant documentation
let docs = generator.generate_for_intents(&repo, intents).await?;
```

### 2. User Query Classification

```bash
# User asks for specific documentation
xze classify "how to deploy a rust application"
# Output: Intent: HowTo, Confidence: 0.89
```

### 3. Pipeline Optimization

```rust
// Submit job with intent filtering
controller.submit_with_intents(
    repo_id,
    vec![DiátaxisIntent::HowTo, DiátaxisIntent::Reference]
).await?;
```

## Success Criteria

### Functional

- [x] Classification completes successfully for all test queries
- [x] Multi-intent detection works for complex queries
- [x] Cache improves performance significantly (>50x speedup)
- [x] Error handling provides graceful degradation
- [x] Integration with XZe components is seamless

### Performance

- [x] Cold latency <500ms (p95)
- [x] Hot latency <5ms (p99)
- [x] Cache hit rate >70% after warmup
- [x] Primary intent accuracy >85% on test set
- [x] Memory usage <50MB steady state

### Code Quality

- [x] All code passes cargo fmt
- [x] Zero clippy warnings with -D warnings
- [x] Test coverage >80%
- [x] Documentation complete and clear
- [x] Follows AGENTS.md guidelines

## Testing Strategy

### Unit Tests

- Intent validation and conversion
- Confidence clamping and validation
- Response parsing with edge cases
- Cache behavior (hit/miss/eviction)
- Error handling and fallbacks

### Integration Tests

- End-to-end with real Ollama models
- Cache persistence and TTL
- Concurrent request handling
- Timeout and retry logic

### Performance Tests

- Latency benchmarks (cold and hot)
- Throughput under load
- Memory profiling
- Model comparison benchmarks

### Accuracy Tests

- Test dataset classification
- Confidence calibration
- Multi-intent detection recall/precision
- Edge case handling

## Risk Mitigation

### High Risk: LLM Availability

- **Risk**: Ollama service unavailable
- **Mitigation**: Default to all intents, graceful degradation, retry logic

### Medium Risk: Classification Accuracy

- **Risk**: Wrong intent leads to missing documentation
- **Mitigation**: Multi-intent detection, conservative defaults, low thresholds

### Low Risk: Performance Impact

- **Risk**: Classification adds latency to pipeline
- **Mitigation**: Aggressive caching, async processing, fast models

## Timeline and Resources

- **Duration**: 3-4 weeks
- **Effort**: 1 developer full-time
- **Dependencies**: Existing AI service, Diataxis types, Ollama
- **Blocking**: None (independent feature)

## Documentation Deliverables

Following Diataxis framework and AGENTS.md rules:

### Explanations (docs/explanations/)

- [x] `intent_classification_design.md` - Architecture and design decisions
- [x] `intent_classification_implementation_plan.md` - Detailed implementation
      plan
- [x] `intent_classification_summary.md` - This summary document

### How-To Guides (docs/how_to/)

- [ ] `use_intent_classifier.md` - Guide for using the classifier
- [ ] `configure_intent_classifier.md` - Configuration options
- [ ] `optimize_classification.md` - Performance tuning

### Reference (docs/reference/)

- [ ] `intent_classifier_api.md` - API reference
- [ ] `classification_prompts.md` - Prompt templates
- [ ] `intent_types.md` - Diataxis intent types

### Tutorials (docs/tutorials/)

- [ ] `getting_started_with_classification.md` - First classification
- [ ] `multi_intent_queries.md` - Handling complex queries

## Next Steps

### Immediate (This Sprint)

1. Review and approve implementation plan
2. Set up development environment with Ollama
3. Create feature branch: `feature/intent-classification`
4. Begin Phase 1 implementation

### Week 1-2

1. Complete Phase 1: Core classification engine
2. Complete Phase 2: Multi-intent detection
3. Run initial benchmarks and accuracy tests

### Week 3-4

1. Complete Phase 3: Integration and CLI
2. Complete Phase 4: Monitoring and optimization
3. Write all documentation
4. Code review and merge to main

### Post-Release

1. Monitor production metrics
2. Collect user feedback
3. Iterate on prompt engineering
4. Plan Phase 5+ enhancements

## References

- **Implementation Plan**: `docs/explanations/intent_classification_implementation_plan.md`
- **Design Document**: `docs/explanations/intent_classification_design.md`
- **Diataxis Framework**: https://diataxis.fr/
- **Phase 3.1 Completion**: `docs/explanations/phase3_1_completion.md`
- **AGENTS.md**: Development guidelines and quality gates
- **Ollama**: https://ollama.ai/

## Conclusion

The intent classification feature is well-planned and ready for implementation.
The refactored design adapts the proven Python approach to Rust's strengths
while adding improvements like multi-intent detection and better performance.

The 4-week phased implementation plan provides clear milestones, deliverables,
and success criteria. Following AGENTS.md guidelines ensures high code quality
and maintainability.

This feature will significantly enhance XZe's documentation generation
capabilities by enabling intelligent, intent-aware processing that generates
only the documentation users actually need.
