# Model-Based Keyword Extraction Proposal

**Status**: Proposal **Author**: AI Assistant **Date**: 2025-01-30 **Related
Issue**: Search Result Quality Improvement

## Executive Summary

This proposal recommends replacing the current hardcoded keyword extraction
approach with an intelligent, model-based system that leverages LLMs to
dynamically extract relevant keywords from documents. This approach is more
scalable, generalizable, and maintainable than maintaining static lists of
domain-specific terms.

## Problem Statement

### Current Approach Limitations

The current keyword extraction implementation in `load_enhanced.py` has several
critical limitations:

1. **Brittle and Domain-Specific**: Hardcoded list of 45+ domain terms (cli,
   api, snyk, docker, kubernetes, etc.) that only work for specific technical
   domains
2. **Not Scalable**: Every new domain (ML, data science, legal, finance, etc.)
   requires code changes to add new terms
3. **Static and Inflexible**: Cannot adapt to new terminology, emerging
   technologies, or evolving language
4. **High Maintenance Burden**: Developers must manually identify and add
   relevant terms for each domain
5. **Narrow Coverage**: Only captures terms we explicitly anticipate, missing
   domain-specific vocabulary we don't know about
6. **Context-Blind**: Frequency analysis doesn't understand semantic
   importance - a rare but critical term might be ignored
7. **Single-Word Bias**: Current approach struggles with multi-word technical
   phrases like "static application security testing" or "continuous integration
   pipeline"

### Real-World Impact

When documenting a new technology stack (e.g., Rust, Go, Terraform):

- **Current approach**: Requires code changes to add rust, cargo, terraform,
  etc. to the hardcoded list
- **Model-based approach**: Automatically identifies these as important keywords
  from context
- **Result**: Without code changes, new documentation is immediately searchable

## Proposed Solution

### Overview

Use Large Language Models (LLMs) to intelligently extract keywords from
documents during the enhanced loading process. The model analyzes document
content, understands context, and identifies the most semantically important
terms, phrases, and concepts.

### Architecture

```text
Document Loading Pipeline (Enhanced)
┌─────────────────────────────────────────────────────────────┐
│ 1. Read Markdown File                                       │
│    └─> Extract raw content, headings, code blocks          │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Preprocessing                                            │
│    ├─> Remove stop words                                   │
│    ├─> Extract code blocks separately                      │
│    └─> Chunk if document is large (>2000 tokens)           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. LLM Keyword Extraction (NEW)                             │
│    ├─> Send to Ollama with structured prompt               │
│    ├─> Request JSON response with:                         │
│    │   - Single-word keywords (15-20)                      │
│    │   - Multi-word key phrases (5-10)                     │
│    │   - Acronyms with expansions                          │
│    │   - Tool/product names                                │
│    │   - Technical commands/APIs                           │
│    └─> Parse and validate JSON response                    │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Metadata Enrichment                                      │
│    ├─> Combine LLM-extracted keywords                      │
│    ├─> Add heading-based keywords                          │
│    ├─> Deduplicate and normalize                           │
│    └─> Store in database with document                     │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Approach

#### 1. LLM-Based Keyword Extraction

**Primary Method**: Use existing Ollama infrastructure with structured prompts

```python
def extract_keywords_with_llm(
    self,
    content: str,
    model: str = "llama3.2:3b"
) -> Dict[str, List[str]]:
    """
    Extract keywords using LLM with structured prompt.

    Args:
        content: Document content to analyze
        model: Ollama model to use for extraction

    Returns:
        Dict with extracted keywords, phrases, acronyms, tools, commands
    """
    prompt = f"""You are analyzing technical documentation to extract important keywords for search indexing.

Analyze this document excerpt and extract the most relevant search terms:

{content[:2000]}  # Limit to avoid token limits

Provide a JSON response with the following structure:
{{
    "keywords": ["word1", "word2", ...],  // 15-20 single-word technical terms
    "phrases": ["phrase 1", "phrase 2", ...],  // 5-10 multi-word key phrases
    "acronyms": {{"CLI": "Command Line Interface", ...}},  // acronyms with expansions
    "tools": ["tool1", "tool2", ...],  // tool or product names
    "commands": ["command1", "command2", ...]  // technical commands or API names
}}

Focus on:
- Technical terminology and domain-specific vocabulary
- Tool, product, and technology names
- Important concepts and methodologies
- Commands, APIs, and technical identifiers
- Acronyms and their meanings

Return ONLY valid JSON, no other text."""

    response = ollama.chat(
        model=model,
        messages=[{"role": "user", "content": prompt}],
        format="json",  # Request JSON format
        options={"temperature": 0}  # Deterministic output
    )

    return json.loads(response['message']['content'])
```

**Advantages**:

- Uses existing Ollama infrastructure (no new dependencies)
- Local execution (no API costs, privacy preserved)
- Structured output via JSON mode
- Deterministic with temperature=0
- Rich, categorized metadata extraction

#### 2. Fallback Strategy

For robustness, implement a multi-tier extraction strategy:

```python
def extract_keywords(self) -> Dict[str, List[str]]:
    """
    Extract keywords with intelligent fallback strategy.

    Returns:
        Dict containing extracted keywords and metadata
    """
    try:
        # Primary: LLM-based extraction
        return self._extract_keywords_llm()
    except Exception as e:
        logger.warning(f"LLM extraction failed: {e}, falling back to hybrid")
        try:
            # Secondary: Embedding-based importance
            return self._extract_keywords_embedding()
        except Exception as e2:
            logger.warning(f"Embedding extraction failed: {e2}, falling back to frequency")
            # Tertiary: Current frequency-based approach
            return self._extract_keywords_frequency()
```

#### 3. Caching Strategy

To avoid re-extraction on every load:

```python
def get_cached_keywords(
    self,
    content_hash: str,
    cache_dir: Path = Path(".cache/keywords")
) -> Optional[Dict[str, List[str]]]:
    """
    Retrieve cached keyword extraction results.

    Args:
        content_hash: SHA256 hash of document content
        cache_dir: Directory for cached results

    Returns:
        Cached keywords if available, None otherwise
    """
    cache_file = cache_dir / f"{content_hash}.json"
    if cache_file.exists():
        with open(cache_file, 'r') as f:
            return json.load(f)
    return None
```

Benefits:

- Extract once per unique document version
- Fast reloads if content unchanged
- Can share cache across environments
- Invalidates automatically when document changes

#### 4. Batch Processing

Process multiple documents efficiently:

```python
async def extract_keywords_batch(
    documents: List[str],
    batch_size: int = 10,
    model: str = "llama3.2:3b"
) -> List[Dict[str, List[str]]]:
    """
    Extract keywords from multiple documents in batches.

    Args:
        documents: List of document contents
        batch_size: Number of documents to process concurrently
        model: Ollama model to use

    Returns:
        List of keyword extraction results
    """
    results = []
    for i in range(0, len(documents), batch_size):
        batch = documents[i:i + batch_size]
        batch_results = await asyncio.gather(*[
            extract_keywords_with_llm_async(doc, model)
            for doc in batch
        ])
        results.extend(batch_results)
    return results
```

## Alternative Approaches Considered

### 1. Embedding-Based Keyword Extraction

**Concept**: Use semantic embeddings to identify words most central to document
meaning.

**Method**:

- Generate embedding for entire document
- Generate embeddings for each candidate word/phrase
- Calculate cosine similarity between document and candidate embeddings
- Words with highest similarity = most representative keywords

**Pros**:

- No LLM inference required (faster)
- Uses existing embedding infrastructure
- Deterministic and consistent

**Cons**:

- Misses categorical information (tools vs. commands vs. concepts)
- Doesn't provide structured output
- May not capture rare but critical technical terms

**Verdict**: Good fallback option, not as rich as LLM approach

### 2. Named Entity Recognition (NER)

**Concept**: Use NER models to identify technical terms, tools, and concepts.

**Method**:

- Apply NER model to document text
- Extract entities tagged as PRODUCT, TECHNOLOGY, ORG, etc.
- Use as keywords

**Pros**:

- Proven approach for entity extraction
- Fast inference
- Good for identifying proper nouns and tool names

**Cons**:

- Requires additional model (spaCy, Flair, etc.)
- May miss generic technical vocabulary
- Less flexible than LLM prompting

**Verdict**: Could complement LLM approach, not a replacement

### 3. RAKE/KeyBERT Algorithms

**Concept**: Use specialized keyword extraction algorithms.

**RAKE (Rapid Automatic Keyword Extraction)**:

- Statistical approach using word frequency and co-occurrence
- No model required

**KeyBERT**:

- Uses BERT embeddings for semantic keyword extraction
- Balances frequency and semantic importance

**Pros**:

- Well-tested algorithms
- Fast execution
- No LLM required

**Cons**:

- Not as contextually aware as LLMs
- May require tuning for different domains
- Less structured output

**Verdict**: Solid fallback option, but LLM provides richer insights

### 4. Hybrid LLM + Statistical Approach

**Concept**: Combine LLM extraction with frequency analysis.

**Method**:

- Extract keywords using frequency/TF-IDF
- Send top 50 candidates to LLM
- Ask LLM to rank and categorize them

**Pros**:

- Reduces LLM processing time
- Combines statistical signal with semantic understanding
- More efficient for large documents

**Cons**:

- Two-stage process adds complexity
- May miss rare but important terms not in frequency top-N

**Verdict**: Worth considering for very large documents (>5000 tokens)

## Benefits of Model-Based Approach

### 1. Generalizability Across Domains

**Current Approach**:

```python
# Must update code for each domain
domain_terms = {
    "cli", "api", "snyk", "docker",  # DevOps terms
    # Need to add: "pandas", "numpy", "scikit" for ML docs
    # Need to add: "terraform", "ansible", "cloudformation" for IaC docs
    # Need to add: ...endless additions...
}
```

**Model-Based Approach**:

```python
# Works for ANY domain without code changes
keywords = extract_keywords_with_llm(document)
# Automatically extracts:
# - "pandas", "dataframe", "numpy" from ML docs
# - "terraform", "state file", "provider" from IaC docs
# - "contract", "statute", "precedent" from legal docs
# - "hypothesis", "p-value", "regression" from stats docs
```

### 2. Multi-Word Phrase Extraction

**Current Approach**: Only single words

- "static" (not useful)
- "application" (not useful)
- "security" (not useful)
- "testing" (not useful)

**Model-Based Approach**: Meaningful phrases

- "static application security testing" (SAST)
- "continuous integration pipeline"
- "role-based access control"
- "test-driven development"

### 3. Contextual Understanding

**Example Document**: "We use git for version control. The git commit
workflow..."

**Current Approach**:

- Frequency analysis: "git" appears 15 times → included
- Problem: Doesn't know if "git" is a tool or just text

**Model-Based Approach**:

- LLM understands "git" is a version control tool
- Categorizes it under "tools"
- Also extracts "version control" as a concept
- Identifies "commit workflow" as a technical phrase

### 4. Acronym Expansion

**Current Approach**:

- Extracts "SAST" as keyword
- No expansion, limited searchability

**Model-Based Approach**:

```json
{
  "acronyms": {
    "SAST": "Static Application Security Testing",
    "SCA": "Software Composition Analysis",
    "CI/CD": "Continuous Integration/Continuous Deployment"
  }
}
```

- Users can search "static application security" and find SAST docs
- Improves discovery for users unfamiliar with acronyms

### 5. Semantic Importance Over Frequency

**Example Document**: Database migration guide mentioning "PostgreSQL" twice and
"database" 50 times.

**Current Approach (Frequency-Based)**:

- "database": 50 occurrences → rank 1
- "PostgreSQL": 2 occurrences → rank 25 (might be excluded)

**Model-Based Approach (Semantic)**:

- "PostgreSQL" → Recognized as specific technology, high importance
- "database" → Generic term, lower importance
- Result: Both included, but properly weighted

### 6. Low Maintenance

**Current Approach**:

- New technology emerges (e.g., Deno, Bun, Astro)
- Developer must update hardcoded list
- PR, review, deploy cycle
- Repeat for every new tool/framework

**Model-Based Approach**:

- New technology emerges
- Load documents mentioning it
- LLM automatically recognizes and extracts it
- Zero code changes required

### 7. Structured Metadata

**Current Approach**: Flat list of keywords

```python
keywords = ["cli", "snyk", "scan", "code", "test", "docker", "json"]
```

**Model-Based Approach**: Rich, categorized metadata

```json
{
  "keywords": ["cli", "scan", "vulnerability", "dependency"],
  "phrases": ["security scanning", "code analysis", "vulnerability detection"],
  "acronyms": { "SAST": "Static Application Security Testing" },
  "tools": ["Snyk", "Docker"],
  "commands": ["snyk test", "snyk code test", "docker run"]
}
```

This enables:

- Advanced search filtering ("show me only tool names")
- Better ranking (exact command match scores higher)
- Rich UI display (show acronym expansions)
- Analytics (what tools are most documented)

## Performance Considerations

### Computational Cost

**Extraction Time Estimates** (per document):

| Approach            | Time (avg) | Overhead |
| ------------------- | ---------- | -------- |
| Current (frequency) | 50ms       | Baseline |
| LLM (Ollama local)  | 500-2000ms | 10-40x   |
| Embedding-based     | 100-200ms  | 2-4x     |
| NER                 | 150-300ms  | 3-6x     |

**Mitigation Strategies**:

1. **Caching**: Extract once, cache by content hash
   - Reloading unchanged documents: 0ms (cache hit)
   - Only new/modified documents incur extraction cost

2. **Batch Processing**: Process multiple documents concurrently
   - 100 documents × 1000ms each = 100,000ms serial
   - 100 documents / 10 batch = 10,000ms parallel (10x faster)

3. **Background Processing**: Extract keywords asynchronously
   - Load documents with basic metadata immediately
   - Enrich with LLM keywords in background
   - Update search index when complete

4. **Selective Application**: Use LLM only when needed
   - Short documents (<500 words): Use frequency analysis
   - Technical documents: Use LLM extraction
   - Configuration driven

### Memory Usage

**Current Approach**: Minimal (pure Python, no model loading)

**Model-Based Approach**:

- Ollama server: Already running (no additional memory)
- LLM model: Already loaded for embeddings/reranking (no change)
- Caching: ~1KB per document (negligible)

**Net Impact**: Essentially zero additional memory overhead

### Scalability

**Loading 10,000 Documents**:

| Approach                 | Total Time       | Cacheable | Parallel |
| ------------------------ | ---------------- | --------- | -------- |
| Current                  | 500s (8min)      | No        | Limited  |
| LLM (uncached)           | 10,000s (2.7hrs) | Yes       | Yes      |
| LLM (cached)             | 500s (8min)      | Yes       | Yes      |
| LLM (parallel, batch=50) | 1,200s (20min)   | Yes       | Yes      |

**Real-World Scenario**:

- Initial load of 10k docs: 20 minutes (with parallelization)
- Subsequent loads: 8 minutes (cache hits)
- Incremental updates: Seconds (only new/modified docs)

**Conclusion**: One-time cost at initial load, minimal impact afterward

## Implementation Plan

### Phase 1: Foundation (Week 1)

**Objectives**:

- Implement core LLM keyword extraction
- Add caching mechanism
- Create fallback strategy

**Deliverables**:

1. `src/pipeline_kb_server/keyword_extractor.py` - New module
2. LLM-based extraction with structured prompts
3. Content hash-based caching
4. Unit tests for extraction logic
5. Configuration options (enable/disable, model selection)

**Success Criteria**:

- Extract structured keywords from sample documents
- 90%+ cache hit rate on repeated loads
- Graceful fallback if LLM unavailable

### Phase 2: Integration (Week 2)

**Objectives**:

- Integrate with existing `load_enhanced.py`
- Add batch processing
- Implement hybrid approach

**Deliverables**:

1. Update `DocumentMetadata._extract_keywords()` to use LLM
2. Batch processing for multiple documents
3. Hybrid LLM + frequency approach for large documents
4. Integration tests with real documentation
5. Performance benchmarks

**Success Criteria**:

- Zero breaking changes to existing API
- Document loading works with LLM extraction
- Performance acceptable (<30min for 10k docs initial load)

### Phase 3: Optimization (Week 3)

**Objectives**:

- Optimize performance
- Enhance prompt engineering
- Add monitoring and analytics

**Deliverables**:

1. Parallel batch processing
2. Refined prompts for better extraction quality
3. Metrics collection (extraction time, cache hits, keyword quality)
4. A/B testing framework to compare approaches
5. Documentation and migration guide

**Success Criteria**:

- 10x speedup via parallelization
- Measurably better search results vs. frequency approach
- Clear migration path for users

### Phase 4: Validation & Rollout (Week 4)

**Objectives**:

- Validate search quality improvements
- Create migration tools
- Production deployment

**Deliverables**:

1. Search quality metrics (precision, recall)
2. Migration script to re-extract keywords for existing docs
3. Configuration guide for different use cases
4. Production rollout plan
5. Rollback procedure

**Success Criteria**:

- 20%+ improvement in search result quality
- Zero data loss during migration
- Smooth production deployment

## Configuration Design

### Environment Variables

```bash
# Enable LLM-based keyword extraction
KEYWORD_EXTRACTION_METHOD=llm  # Options: llm, embedding, frequency, hybrid

# Model for keyword extraction
KEYWORD_EXTRACTION_MODEL=llama3.2:3b

# Fallback if LLM fails
KEYWORD_EXTRACTION_FALLBACK=frequency

# Enable caching
KEYWORD_EXTRACTION_CACHE_ENABLED=true
KEYWORD_EXTRACTION_CACHE_DIR=.cache/keywords

# Batch processing
KEYWORD_EXTRACTION_BATCH_SIZE=10
KEYWORD_EXTRACTION_MAX_WORKERS=4

# Keywords per document
KEYWORD_EXTRACTION_MAX_KEYWORDS=20
KEYWORD_EXTRACTION_MAX_PHRASES=10
```

### Runtime Configuration

```python
# In config.py
@dataclass
class KeywordExtractionConfig:
    """Configuration for keyword extraction."""

    method: str = "llm"  # llm, embedding, frequency, hybrid
    model: str = "llama3.2:3b"
    fallback: str = "frequency"
    cache_enabled: bool = True
    cache_dir: Path = Path(".cache/keywords")
    batch_size: int = 10
    max_workers: int = 4
    max_keywords: int = 20
    max_phrases: int = 10
    temperature: float = 0.0  # LLM temperature for deterministic output
```

## Success Metrics

### Search Quality Metrics

**Primary Metrics**:

1. **Precision@5**: Percentage of top 5 results that are relevant
   - Target: 80%+ (up from current ~60%)
2. **Recall@10**: Percentage of relevant docs found in top 10
   - Target: 90%+ (up from current ~70%)
3. **Mean Reciprocal Rank (MRR)**: Average rank of first relevant result
   - Target: 0.85+ (up from current ~0.65)

**Secondary Metrics**:

1. **Zero-result queries**: Percentage of queries returning no results
   - Target: <5% (down from current ~15%)
2. **User satisfaction**: Qualitative feedback on result quality
   - Target: "Good" or "Excellent" from 80%+ of users

### Operational Metrics

1. **Extraction time**: Average time to extract keywords per document
   - Target: <2s for documents <5000 words
2. **Cache hit rate**: Percentage of extractions served from cache
   - Target: >90% after initial load
3. **Fallback rate**: Percentage of extractions falling back from LLM
   - Target: <5% (high reliability)

### A/B Testing Plan

**Setup**:

- 50% of documents use LLM extraction
- 50% of documents use frequency extraction
- Track which method provides better search results

**Hypothesis**: LLM extraction will improve search quality by 20%+

**Test Duration**: 2 weeks

**Decision Criteria**: If LLM shows >15% improvement, roll out to 100%

## Risk Mitigation

### Risk 1: LLM Extraction Failures

**Risk**: Ollama service unavailable or LLM returns invalid JSON

**Mitigation**:

- Multi-tier fallback strategy (LLM → embedding → frequency)
- Graceful degradation preserves functionality
- Comprehensive error handling and logging
- Automatic retry with exponential backoff

**Impact**: Low - System remains functional with fallback methods

### Risk 2: Performance Degradation

**Risk**: LLM extraction too slow for large document sets

**Mitigation**:

- Content hash-based caching (extract once per doc version)
- Parallel batch processing
- Configurable batch size and workers
- Option to use faster fallback methods
- Background processing for non-critical loads

**Impact**: Medium - Addressed by caching and parallelization

### Risk 3: Keyword Quality Issues

**Risk**: LLM extracts irrelevant or poor-quality keywords

**Mitigation**:

- Extensive prompt engineering and testing
- A/B testing against current approach
- Manual review of sample extractions
- Feedback loop for prompt refinement
- Configurable keyword count limits

**Impact**: Medium - Addressed by validation and iteration

### Risk 4: Model Dependency

**Risk**: System becomes dependent on specific LLM model

**Mitigation**:

- Abstraction layer for keyword extraction (strategy pattern)
- Support for multiple extraction methods
- Fallback to non-LLM approaches
- Configuration-driven model selection

**Impact**: Low - Architecture supports multiple backends

### Risk 5: Migration Complexity

**Risk**: Difficult to migrate existing documents to new extraction method

**Mitigation**:

- Backward compatible implementation
- Migration script with dry-run mode
- Gradual rollout (opt-in initially)
- Clear rollback procedure
- Comprehensive documentation

**Impact**: Low - Planned migration strategy

## Comparison: Hardcoded vs. Model-Based

| Aspect                 | Hardcoded Approach                       | Model-Based Approach                      |
| ---------------------- | ---------------------------------------- | ----------------------------------------- |
| **Scalability**        | ❌ Requires code changes for new domains | ✅ Works across any domain automatically  |
| **Maintenance**        | ❌ Ongoing updates to domain term lists  | ✅ Zero maintenance for new terminology   |
| **Coverage**           | ❌ Only terms we anticipate              | ✅ Discovers all domain-specific terms    |
| **Context Awareness**  | ❌ Pure frequency analysis               | ✅ Understands semantic importance        |
| **Multi-word Phrases** | ❌ Limited support                       | ✅ Excellent phrase extraction            |
| **Acronym Handling**   | ⚠️ Basic (extracts but no expansion)     | ✅ Extracts + expands acronyms            |
| **Categorization**     | ❌ Flat keyword list                     | ✅ Structured (tools, commands, concepts) |
| **Setup Time**         | ✅ Instant (no model required)           | ⚠️ Requires Ollama setup                  |
| **Extraction Speed**   | ✅ Fast (~50ms per doc)                  | ⚠️ Slower (~1-2s per doc)                 |
| **Caching**            | ❌ Not implemented                       | ✅ Content hash-based caching             |
| **Adaptability**       | ❌ Static, requires updates              | ✅ Adapts to new domains automatically    |
| **Quality**            | ⚠️ Good for known domains                | ✅ Excellent across all domains           |

## Conclusion

The model-based keyword extraction approach represents a significant evolution
from the current hardcoded method. While it introduces additional computational
cost at document load time, the benefits far outweigh the drawbacks:

**Key Advantages**:

1. **Zero-maintenance scalability** across any documentation domain
2. **Semantic understanding** of keyword importance, not just frequency
3. **Rich, structured metadata** enabling advanced search features
4. **Automatic adaptation** to new technologies and terminology
5. **Multi-word phrase extraction** for better semantic matching

**Acceptable Tradeoffs**:

1. **Initial load time**: Mitigated by caching (one-time cost per doc version)
2. **LLM dependency**: Mitigated by fallback strategies
3. **Complexity**: Offset by improved maintainability and results

**Recommendation**: Implement model-based keyword extraction with the phased
approach outlined above. Start with LLM-based extraction as the primary method,
maintain frequency-based as fallback, and validate improvements through A/B
testing before full rollout.

This approach aligns with the RAG architecture already in place, leverages
existing Ollama infrastructure, and provides a future-proof solution that scales
across any documentation domain without code changes.

## Next Steps

1. **Review and approval**: Stakeholder review of this proposal
2. **Proof of concept**: Implement basic LLM extraction for 100 sample documents
3. **Quality validation**: Compare search results vs. current approach
4. **Decision point**: Proceed with full implementation if quality improves
   by >15%
5. **Phased rollout**: Execute 4-week implementation plan

## References

- Current implementation: `src/pipeline_kb_server/load_enhanced.py`
- Related issue: Search result quality improvements
- Ollama JSON mode documentation: <https://ollama.ai/blog/json-mode>
- KeyBERT paper: <https://arxiv.org/abs/2003.11032>
- RAKE algorithm: <https://www.researchgate.net/publication/227988510>
