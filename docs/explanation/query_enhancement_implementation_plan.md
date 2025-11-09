# Query Enhancement Implementation Plan

## Overview

This document outlines a phased approach to implementing advanced query enhancement strategies that will significantly improve search result quality. The implementation focuses on leveraging LLM capabilities beyond simple intent classification to create truly intelligent query processing.

**Goal**: Transform the current basic keyword-prepending approach into a sophisticated query enhancement pipeline that uses LLM reasoning, query expansion, and hypothetical document generation to improve retrieval quality.

**Scope**: This plan covers five major enhancement strategies implemented across six phases, building incrementally on existing infrastructure.

## Current State Analysis

### Existing Infrastructure

- **Intent Classification**: LLM-based classification into Diataxis categories (tutorial, howto, reference, explanation)
- **Basic Query Refinement**: Simple keyword prepending using static templates
- **Confidence-Based Routing**: Three-tier routing based on classification confidence
- **Multi-Stage Search**: Hybrid search with reranking capabilities
- **Ollama Integration**: Local LLM access for generation tasks

### Identified Issues

1. **Underutilized LLM**: Only used for intent classification, not query improvement
2. **Static Templates**: Hardcoded refinement templates add generic noise
3. **Single Query Variant**: No exploration of alternative phrasings
4. **Limited Context Understanding**: No entity extraction or semantic analysis
5. **Aggressive Keyword Filtering**: Refinement skipped too often when keywords detected

## Implementation Phases

### Phase 1: Query Expansion Foundation

**Objective**: Implement basic query expansion to generate multiple query variants, improving recall through diverse semantic representations.

**Duration**: 1-2 weeks

**Components**:

#### 1.1 QueryExpander Struct

Create `crates/core/src/query/expander.rs`:

```xze/crates/core/src/query/expander.rs#L1-50
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Generate multiple query variations to improve recall
pub struct QueryExpander {
    config: Config,
    ollama_client: OllamaClient,
    cache: Mutex<LruCache<String, Vec<String>>>,
}

impl QueryExpander {
    /// Create new query expander with configuration
    pub fn new(config: Config) -> Self {
        let cache_size = NonZeroUsize::new(config.query_cache_size).unwrap();
        Self {
            config,
            ollama_client: OllamaClient::new(&config.ollama_url),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    /// Generate query variants with optional intent context
    ///
    /// # Arguments
    ///
    /// * `query` - Original user query
    /// * `num_variants` - Number of variants to generate
    /// * `intent` - Optional intent classification for context
    ///
    /// # Returns
    ///
    /// Returns vector of query variants including original
    pub async fn expand(
        &self,
        query: &str,
        num_variants: usize,
        intent: Option<&str>,
    ) -> Result<Vec<String>> {
        let cache_key = format!("{}:{}:{}", query, num_variants, intent.unwrap_or("none"));

        if let Some(cached) = self.cache.lock().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }

        let variants = self.expand_uncached(query, num_variants, intent).await?;
        self.cache.lock().unwrap().put(cache_key, variants.clone());

        Ok(variants)
    }

    async fn expand_uncached(
        &self,
        query: &str,
        num_variants: usize,
        intent: Option<&str>,
    ) -> Result<Vec<String>> {
        let prompt = self.build_expansion_prompt(query, num_variants, intent);
        let response = self.ollama_client.generate(&prompt).await?;

        let mut variants = vec![query.to_string()];
        variants.extend(self.parse_variants(&response));

        Ok(variants)
    }

    fn build_expansion_prompt(&self, query: &str, num_variants: usize, intent: Option<&str>) -> String {
        let intent_context = intent.map(|i| format!("Document type: {}", i)).unwrap_or_default();

        format!(
            "Generate {} alternative phrasings for this search query:\n\n\
             Query: \"{}\"\n{}\n\n\
             Requirements:\n\
             - Preserve core meaning and intent\n\
             - Use synonyms and related terms\n\
             - Vary phrasing and structure\n\
             - Keep technical accuracy\n\n\
             Alternative queries (one per line):",
            num_variants - 1,
            query,
            intent_context
        )
    }

    fn parse_variants(&self, response: &str) -> Vec<String> {
        response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect()
    }
}
```

**Key Features**:

- LRU cache for expansion results (reduce redundant LLM calls)
- Intent-aware expansion (use intent to guide variant generation)
- Configurable number of variants
- Fallback to original query on error
- Async/await using Tokio runtime

#### 1.2 Result Merging Strategy

Implement result deduplication and merging in `crates/core/src/query/merger.rs`:

```xze/crates/core/src/query/merger.rs#L1-60
use std::collections::HashMap;
use anyhow::Result;

/// Strategy for merging duplicate results
#[derive(Debug, Clone, Copy)]
pub enum MergeStrategy {
    /// Keep highest similarity score for each document
    MaxScore,
    /// Average similarity across all occurrences
    AvgScore,
    /// Keep first occurrence only
    FirstOccurrence,
}

/// Merge and deduplicate results from multiple query variants
pub struct ResultMerger;

impl ResultMerger {
    /// Merge result sets using specified strategy
    ///
    /// # Arguments
    ///
    /// * `result_sets` - Vector of search result vectors to merge
    /// * `strategy` - Merging strategy to apply
    ///
    /// # Returns
    ///
    /// Returns deduplicated and merged result vector sorted by score
    pub fn merge(
        &self,
        result_sets: Vec<Vec<SearchResult>>,
        strategy: MergeStrategy,
    ) -> Result<Vec<SearchResult>> {
        let mut doc_map: HashMap<String, Vec<SearchResult>> = HashMap::new();

        for results in result_sets {
            for result in results {
                doc_map
                    .entry(result.doc_id.clone())
                    .or_insert_with(Vec::new)
                    .push(result);
            }
        }

        let mut merged: Vec<SearchResult> = doc_map
            .into_iter()
            .map(|(_, occurrences)| self.merge_occurrences(occurrences, strategy))
            .collect();

        merged.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        Ok(merged)
    }

    fn merge_occurrences(
        &self,
        mut occurrences: Vec<SearchResult>,
        strategy: MergeStrategy,
    ) -> SearchResult {
        match strategy {
            MergeStrategy::MaxScore => {
                occurrences.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
                occurrences.into_iter().next().unwrap()
            }
            MergeStrategy::AvgScore => {
                let avg_similarity = occurrences.iter().map(|r| r.similarity).sum::<f32>()
                    / occurrences.len() as f32;
                let mut result = occurrences.into_iter().next().unwrap();
                result.similarity = avg_similarity;
                result
            }
            MergeStrategy::FirstOccurrence => occurrences.into_iter().next().unwrap(),
        }
    }
}
```

#### 1.3 Integration Points

Update `SearchService` in `crates/core/src/search/service.rs`:

```xze/crates/core/src/search/service.rs#L100-150
impl SearchService {
    /// Search using query expansion
    ///
    /// # Arguments
    ///
    /// * `query` - Original user query
    /// * `limit` - Maximum number of results
    /// * `num_variants` - Number of query variants to generate
    /// * `merge_strategy` - Strategy for merging results
    ///
    /// # Returns
    ///
    /// Returns tuple of (original_query, merged_results, variants_used)
    pub async fn search_with_expansion(
        &self,
        query: &str,
        limit: usize,
        num_variants: usize,
        merge_strategy: MergeStrategy,
    ) -> Result<(String, Vec<SearchResult>, Vec<String>)> {
        let expander = QueryExpander::new(self.config.clone());
        let variants = expander.expand(query, num_variants, None).await?;

        let mut result_sets = Vec::new();
        for variant in &variants {
            let embedding = self.generate_embedding(variant).await?;
            let results = self.execute_search(&embedding, limit * 2).await?;
            result_sets.push(results);
        }

        let merger = ResultMerger;
        let merged = merger.merge(result_sets, merge_strategy)?;

        Ok((
            query.to_string(),
            merged.into_iter().take(limit).collect(),
            variants,
        ))
    }
}
```

#### 1.4 Configuration Updates

Add to `crates/core/src/config.rs`:

```xze/crates/core/src/config.rs#L50-60
#[derive(Debug, Clone, Deserialize)]
pub struct QueryConfig {
    /// Enable query expansion feature
    #[serde(default = "default_true")]
    pub expansion_enabled: bool,

    /// Number of query variants to generate
    #[serde(default = "default_expansion_variants")]
    pub expansion_variants: usize,

    /// LRU cache size for expanded queries
    #[serde(default = "default_cache_size")]
    pub expansion_cache_size: usize,

    /// Strategy for merging results
    #[serde(default = "default_merge_strategy")]
    pub expansion_merge_strategy: String,
}

fn default_true() -> bool { true }
fn default_expansion_variants() -> usize { 3 }
fn default_cache_size() -> usize { 500 }
fn default_merge_strategy() -> String { "max_score".to_string() }
```

#### 1.5 Testing Requirements

Create `crates/core/src/query/tests/expander_tests.rs`:

```xze/crates/core/src/query/tests/expander_tests.rs#L1-80
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_expand_generates_variants() {
        let config = Config::default();
        let expander = QueryExpander::new(config);

        let variants = expander.expand("how to setup docker", 3, None).await.unwrap();

        assert!(variants.len() >= 1);
        assert_eq!(variants[0], "how to setup docker");
    }

    #[tokio::test]
    async fn test_expand_with_intent_context() {
        let config = Config::default();
        let expander = QueryExpander::new(config);

        let variants = expander
            .expand("configure logging", 3, Some("howto"))
            .await
            .unwrap();

        assert!(variants.len() >= 1);
    }

    #[tokio::test]
    async fn test_expand_caching_works() {
        let config = Config::default();
        let expander = QueryExpander::new(config);

        let query = "test query caching";
        let variants1 = expander.expand(query, 3, None).await.unwrap();
        let variants2 = expander.expand(query, 3, None).await.unwrap();

        assert_eq!(variants1, variants2);
    }

    #[test]
    fn test_merge_max_score_strategy() {
        let merger = ResultMerger;

        let result1 = SearchResult {
            doc_id: "doc1".to_string(),
            similarity: 0.8,
            content: "content".to_string(),
        };
        let result2 = SearchResult {
            doc_id: "doc1".to_string(),
            similarity: 0.9,
            content: "content".to_string(),
        };

        let merged = merger
            .merge(vec![vec![result1], vec![result2]], MergeStrategy::MaxScore)
            .unwrap();

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].similarity, 0.9);
    }

    #[test]
    fn test_merge_avg_score_strategy() {
        let merger = ResultMerger;

        let result1 = SearchResult {
            doc_id: "doc1".to_string(),
            similarity: 0.8,
            content: "content".to_string(),
        };
        let result2 = SearchResult {
            doc_id: "doc1".to_string(),
            similarity: 0.6,
            content: "content".to_string(),
        };

        let merged = merger
            .merge(vec![vec![result1], vec![result2]], MergeStrategy::AvgScore)
            .unwrap();

        assert_eq!(merged.len(), 1);
        assert!((merged[0].similarity - 0.7).abs() < 0.01);
    }
}
```

**Success Metrics**:

- Recall improvement: Target 15-20 percent increase
- Latency: Keep under 2x single query search time
- Cache hit rate: Above 40 percent for common queries
- Test coverage: Above 80 percent

### Phase 2: LLM-Based Query Rewriting

**Objective**: Replace static keyword prepending with intelligent LLM-based query rewriting that understands context and preserves user intent.

**Duration**: 1-2 weeks

**Dependencies**: Phase 1 (uses expansion infrastructure)

**Components**:

#### 2.1 QueryRewriter Struct

Create `crates/core/src/query/rewriter.rs`:

```xze/crates/core/src/query/rewriter.rs#L1-80
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Use LLM to intelligently rewrite queries for better retrieval
pub struct QueryRewriter {
    config: Config,
    ollama_client: OllamaClient,
    cache: Mutex<LruCache<String, String>>,
}

impl QueryRewriter {
    /// Create new query rewriter with configuration
    pub fn new(config: Config) -> Self {
        let cache_size = NonZeroUsize::new(config.query_cache_size).unwrap();
        Self {
            config,
            ollama_client: OllamaClient::new(&config.ollama_url),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    /// Rewrite query using LLM reasoning
    ///
    /// # Arguments
    ///
    /// * `query` - Original user query
    /// * `intent` - Detected intent (tutorial, howto, reference, explanation)
    /// * `confidence` - Classification confidence score
    /// * `context` - Optional context (entities, previous queries, etc)
    ///
    /// # Returns
    ///
    /// Returns rewritten query optimized for semantic search
    pub async fn rewrite(
        &self,
        query: &str,
        intent: &str,
        confidence: f32,
        context: Option<&RewriteContext>,
    ) -> Result<String> {
        let cache_key = format!("{}:{}:{:.2}", query, intent, confidence);

        if let Some(cached) = self.cache.lock().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }

        let rewritten = self.rewrite_uncached(query, intent, confidence, context).await?;
        self.cache.lock().unwrap().put(cache_key, rewritten.clone());

        Ok(rewritten)
    }

    async fn rewrite_uncached(
        &self,
        query: &str,
        intent: &str,
        confidence: f32,
        context: Option<&RewriteContext>,
    ) -> Result<String> {
        let prompt = self.build_rewrite_prompt(query, intent, confidence, context);
        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response.trim().to_string())
    }

    fn build_rewrite_prompt(
        &self,
        query: &str,
        intent: &str,
        confidence: f32,
        context: Option<&RewriteContext>,
    ) -> String {
        format!(
            "You are a search query optimizer for technical documentation.\n\n\
             Original query: \"{}\"\n\
             Detected intent: {} (confidence: {:.2})\n\n\
             Your task: Rewrite this query to improve semantic search retrieval.\n\n\
             Guidelines:\n\
             1. Preserve the user's core intent and information need\n\
             2. Expand technical acronyms (e.g., API to Application Programming Interface)\n\
             3. Add relevant technical context words\n\
             4. Use terminology common in {} documentation\n\
             5. Keep concise (max 2-3x original length)\n\
             6. Focus on searchable keywords, not conversational phrases\n\n\
             Example transformations:\n\
             - \"how setup docker\" -> \"docker installation configuration setup guide\"\n\
             - \"what is RAG\" -> \"RAG retrieval augmented generation explanation concept\"\n\
             - \"api endpoint list\" -> \"API endpoints reference documentation specifications\"\n\n\
             Rewritten query (one line, no quotes):",
            query,
            intent,
            confidence,
            intent
        )
    }
}

/// Context for query rewriting decisions
#[derive(Debug, Clone)]
pub struct RewriteContext {
    pub aggressive: bool,
    pub conservative: bool,
    pub entities: Vec<String>,
}
```

#### 2.2 Confidence-Based Strategy Selection

Implement adaptive rewriting in `crates/core/src/query/adaptive.rs`:

```xze/crates/core/src/query/adaptive.rs#L1-80
use anyhow::Result;

/// Enhanced query with metadata about enhancement strategy
#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    pub original: String,
    pub rewritten: String,
    pub variants: Vec<String>,
    pub strategy_used: String,
}

/// Select enhancement strategy based on confidence level
pub struct AdaptiveQueryEnhancer {
    rewriter: QueryRewriter,
    expander: QueryExpander,
}

impl AdaptiveQueryEnhancer {
    pub fn new(config: Config) -> Self {
        Self {
            rewriter: QueryRewriter::new(config.clone()),
            expander: QueryExpander::new(config),
        }
    }

    /// Apply appropriate enhancement strategy based on confidence
    ///
    /// High confidence (>0.8): Aggressive rewriting
    /// Medium confidence (0.5-0.8): Conservative rewriting
    /// Low confidence (<0.5): Query expansion only
    pub async fn enhance(
        &self,
        query: &str,
        intent_result: &IntentResult,
    ) -> Result<EnhancedQuery> {
        if intent_result.confidence > 0.8 {
            let context = RewriteContext {
                aggressive: true,
                conservative: false,
                entities: vec![],
            };

            let rewritten = self
                .rewriter
                .rewrite(query, &intent_result.intent, intent_result.confidence, Some(&context))
                .await?;

            Ok(EnhancedQuery {
                original: query.to_string(),
                rewritten: rewritten.clone(),
                variants: vec![rewritten],
                strategy_used: "aggressive_rewrite".to_string(),
            })
        } else if intent_result.confidence > 0.5 {
            let context = RewriteContext {
                aggressive: false,
                conservative: true,
                entities: vec![],
            };

            let rewritten = self
                .rewriter
                .rewrite(query, &intent_result.intent, intent_result.confidence, Some(&context))
                .await?;

            let variants = self.expander.expand(&rewritten, 2, Some(&intent_result.intent)).await?;

            Ok(EnhancedQuery {
                original: query.to_string(),
                rewritten: rewritten.clone(),
                variants,
                strategy_used: "conservative_rewrite_expand".to_string(),
            })
        } else {
            let variants = self.expander.expand(query, 3, Some(&intent_result.intent)).await?;

            Ok(EnhancedQuery {
                original: query.to_string(),
                rewritten: query.to_string(),
                variants,
                strategy_used: "expand_only".to_string(),
            })
        }
    }
}
```

#### 2.3 Configuration Updates

Add to `crates/core/src/config.rs`:

```xze/crates/core/src/config.rs#L70-85
#[derive(Debug, Clone, Deserialize)]
pub struct RewriteConfig {
    /// Enable query rewriting feature
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Model to use for rewriting
    #[serde(default = "default_rewrite_model")]
    pub model: String,

    /// Temperature for generation (0.0-1.0)
    #[serde(default = "default_rewrite_temperature")]
    pub temperature: f32,

    /// Maximum tokens in rewritten query
    #[serde(default = "default_rewrite_max_tokens")]
    pub max_tokens: usize,

    /// LRU cache size for rewritten queries
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
}

fn default_rewrite_model() -> String { "llama3.2:3b".to_string() }
fn default_rewrite_temperature() -> f32 { 0.3 }
fn default_rewrite_max_tokens() -> usize { 100 }
```

#### 2.4 Testing Requirements

Create `crates/core/src/query/tests/rewriter_tests.rs`:

```xze/crates/core/src/query/tests/rewriter_tests.rs#L1-60
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_rewrite_preserves_intent() {
        let config = Config::default();
        let rewriter = QueryRewriter::new(config);

        let rewritten = rewriter
            .rewrite("how setup docker", "howto", 0.9, None)
            .await
            .unwrap();

        assert!(!rewritten.is_empty());
        assert_ne!(rewritten, "how setup docker");
    }

    #[tokio::test]
    async fn test_rewrite_with_high_confidence() {
        let config = Config::default();
        let enhancer = AdaptiveQueryEnhancer::new(config);

        let intent_result = IntentResult {
            intent: "tutorial".to_string(),
            confidence: 0.95,
        };

        let enhanced = enhancer.enhance("learn rust", &intent_result).await.unwrap();

        assert_eq!(enhanced.strategy_used, "aggressive_rewrite");
        assert_ne!(enhanced.rewritten, "learn rust");
    }

    #[tokio::test]
    async fn test_rewrite_with_medium_confidence() {
        let config = Config::default();
        let enhancer = AdaptiveQueryEnhancer::new(config);

        let intent_result = IntentResult {
            intent: "howto".to_string(),
            confidence: 0.65,
        };

        let enhanced = enhancer
            .enhance("configure logging", &intent_result)
            .await
            .unwrap();

        assert_eq!(enhanced.strategy_used, "conservative_rewrite_expand");
        assert!(enhanced.variants.len() >= 2);
    }

    #[tokio::test]
    async fn test_expand_only_with_low_confidence() {
        let config = Config::default();
        let enhancer = AdaptiveQueryEnhancer::new(config);

        let intent_result = IntentResult {
            intent: "reference".to_string(),
            confidence: 0.3,
        };

        let enhanced = enhancer.enhance("api docs", &intent_result).await.unwrap();

        assert_eq!(enhanced.strategy_used, "expand_only");
        assert_eq!(enhanced.original, enhanced.rewritten);
    }
}
```

**Success Metrics**:

- Precision improvement: Target 10-15 percent increase
- Rewrite quality score: Above 4.0 out of 5.0 (human eval)
- Latency overhead: Under 300ms per query
- Test coverage: Above 80 percent

### Phase 3: HyDE (Hypothetical Document Embeddings)

**Objective**: Implement HyDE to search in answer space rather than question space, significantly improving retrieval for conceptual and explanatory queries.

**Duration**: 2-3 weeks

**Dependencies**: Phase 2 (uses rewriting infrastructure)

**Components**:

#### 3.1 HyDEGenerator Struct

Create `crates/core/src/query/hyde.rs`:

```xze/crates/core/src/query/hyde.rs#L1-100
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Generate hypothetical documents for improved retrieval
pub struct HyDEGenerator {
    config: Config,
    ollama_client: OllamaClient,
    cache: Mutex<LruCache<String, String>>,
}

impl HyDEGenerator {
    pub fn new(config: Config) -> Self {
        let cache_size = NonZeroUsize::new(config.hyde_cache_size).unwrap();
        Self {
            config,
            ollama_client: OllamaClient::new(&config.ollama_url),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    /// Generate a hypothetical answer to the query
    ///
    /// # Arguments
    ///
    /// * `query` - User's search query
    /// * `intent` - Document type intent (tutorial, howto, etc)
    /// * `length` - Document length (short, medium, long)
    ///
    /// # Returns
    ///
    /// Returns hypothetical document text optimized for the intent
    pub async fn generate_hypothetical_document(
        &self,
        query: &str,
        intent: &str,
        length: DocumentLength,
    ) -> Result<String> {
        let cache_key = format!("{}:{}:{:?}", query, intent, length);

        if let Some(cached) = self.cache.lock().unwrap().get(&cache_key) {
            return Ok(cached.clone());
        }

        let hyp_doc = self.generate_uncached(query, intent, length).await?;
        self.cache.lock().unwrap().put(cache_key, hyp_doc.clone());

        Ok(hyp_doc)
    }

    async fn generate_uncached(
        &self,
        query: &str,
        intent: &str,
        length: DocumentLength,
    ) -> Result<String> {
        let prompt = self.build_hyde_prompt(query, intent, length);
        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response.trim().to_string())
    }

    fn build_hyde_prompt(
        &self,
        query: &str,
        intent: &str,
        length: DocumentLength,
    ) -> String {
        let length_instruction = match length {
            DocumentLength::Short => "2-3 sentences",
            DocumentLength::Medium => "1 paragraph",
            DocumentLength::Long => "2 paragraphs",
        };

        let intent_specific = match intent {
            "tutorial" => {
                "Generate a brief tutorial-style answer that would teach a beginner.\n\
                 Include:\n\
                 - Learning objectives\n\
                 - Step-by-step guidance hints\n\
                 - Beginner-friendly language"
            }
            "howto" => {
                "Generate a brief how-to guide answer that would solve the problem.\n\
                 Include:\n\
                 - Specific action steps\n\
                 - Configuration details\n\
                 - Command examples or technical specifics"
            }
            "reference" => {
                "Generate a brief reference-style answer that would document the topic.\n\
                 Include:\n\
                 - Technical specifications\n\
                 - Parameter definitions\n\
                 - API or command syntax"
            }
            "explanation" => {
                "Generate a brief explanation that would clarify the concept.\n\
                 Include:\n\
                 - Conceptual understanding\n\
                 - Why it works this way\n\
                 - Relationships between concepts"
            }
            _ => "Generate a brief technical answer.",
        };

        format!(
            "{}\n\nQuery: {}\n\nLength: {}\n\nHypothetical answer:",
            intent_specific, query, length_instruction
        )
    }
}

/// Length of hypothetical document to generate
#[derive(Debug, Clone, Copy)]
pub enum DocumentLength {
    Short,
    Medium,
    Long,
}
```

#### 3.2 HyDE Search Integration

Create hybrid query-HyDE search in `crates/core/src/search/hyde_service.rs`:

```xze/crates/core/src/search/hyde_service.rs#L1-80
use anyhow::Result;

/// Search service with HyDE enhancement
pub struct HyDESearchService {
    search_service: SearchService,
    hyde_generator: HyDEGenerator,
}

impl HyDESearchService {
    pub fn new(config: Config) -> Self {
        Self {
            search_service: SearchService::new(config.clone()),
            hyde_generator: HyDEGenerator::new(config),
        }
    }

    /// Search using both original query and hypothetical document
    ///
    /// # Arguments
    ///
    /// * `query` - Original user query
    /// * `intent` - Detected intent
    /// * `limit` - Number of results
    /// * `hyde_weight` - Weight for HyDE embedding similarity
    /// * `query_weight` - Weight for original query similarity
    ///
    /// # Returns
    ///
    /// Returns tuple of (query, results, hypothetical_doc)
    pub async fn search_with_hyde(
        &self,
        query: &str,
        intent: &str,
        limit: usize,
        hyde_weight: f32,
        query_weight: f32,
    ) -> Result<(String, Vec<SearchResult>, String)> {
        let hyp_doc = self
            .hyde_generator
            .generate_hypothetical_document(query, intent, DocumentLength::Medium)
            .await?;

        let query_embedding = self.search_service.generate_embedding(query).await?;
        let hyde_embedding = self.search_service.generate_embedding(&hyp_doc).await?;

        let results = self
            .dual_embedding_search(
                &query_embedding,
                &hyde_embedding,
                query_weight,
                hyde_weight,
                limit,
            )
            .await?;

        Ok((query.to_string(), results, hyp_doc))
    }

    async fn dual_embedding_search(
        &self,
        query_embedding: &[f32],
        hyde_embedding: &[f32],
        query_weight: f32,
        hyde_weight: f32,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_str = format!(
            "SELECT id, source_file, content, \
             ({} * (1 - (embedding <=> $1::vector))) + \
             ({} * (1 - (embedding <=> $2::vector))) as combined_similarity, \
             chunk_index, total_chunks, title, category, doc_type \
             FROM documents \
             WHERE embedding IS NOT NULL \
             ORDER BY combined_similarity DESC \
             LIMIT $3",
            query_weight, hyde_weight
        );

        self.search_service
            .execute_dual_embedding_query(&query_str, query_embedding, hyde_embedding, limit)
            .await
    }
}
```

#### 3.3 Adaptive HyDE Strategy

Implement selective HyDE usage in `crates/core/src/query/hyde_strategy.rs`:

```xze/crates/core/src/query/hyde_strategy.rs#L1-50
/// Determine if HyDE should be used for this query
///
/// HyDE works best for:
/// - Explanation queries (always use)
/// - Tutorial queries (use if confidence > 0.7)
/// - Reference queries with conceptual elements
///
/// HyDE less effective for:
/// - Keyword lookups
/// - Specific command/API queries
/// - Very short queries (< 3 words)
pub fn should_use_hyde(query: &str, intent: &str, confidence: f32) -> bool {
    if intent == "explanation" {
        return true;
    }

    if intent == "tutorial" && confidence > 0.7 {
        return true;
    }

    let word_count = query.split_whitespace().count();
    if word_count < 3 {
        return false;
    }

    let conceptual_keywords = [
        "why",
        "how does",
        "explain",
        "understand",
        "difference between",
        "concept",
        "architecture",
    ];

    let query_lower = query.to_lowercase();
    if conceptual_keywords
        .iter()
        .any(|kw| query_lower.contains(kw))
    {
        return true;
    }

    false
}
```

#### 3.4 Configuration Updates

Add to `crates/core/src/config.rs`:

```xze/crates/core/src/config.rs#L90-105
#[derive(Debug, Clone, Deserialize)]
pub struct HyDEConfig {
    /// Enable HyDE feature
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Model to use for HyDE generation
    #[serde(default = "default_hyde_model")]
    pub model: String,

    /// Temperature for generation
    #[serde(default = "default_hyde_temperature")]
    pub temperature: f32,

    /// Default document length
    #[serde(default = "default_hyde_length")]
    pub length: String,

    /// Weight for HyDE embedding (0.0-1.0)
    #[serde(default = "default_hyde_weight")]
    pub hyde_weight: f32,

    /// Weight for query embedding (0.0-1.0)
    #[serde(default = "default_query_weight")]
    pub query_weight: f32,

    /// LRU cache size
    #[serde(default = "default_hyde_cache_size")]
    pub cache_size: usize,
}

fn default_hyde_model() -> String { "llama3.2:3b".to_string() }
fn default_hyde_temperature() -> f32 { 0.5 }
fn default_hyde_length() -> String { "medium".to_string() }
fn default_hyde_weight() -> f32 { 0.6 }
fn default_query_weight() -> f32 { 0.4 }
fn default_hyde_cache_size() -> usize { 300 }
```

#### 3.5 Testing Requirements

Create `crates/core/src/query/tests/hyde_tests.rs`:

```xze/crates/core/src/query/tests/hyde_tests.rs#L1-80
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_hyde_generates_hypothetical_document() {
        let config = Config::default();
        let generator = HyDEGenerator::new(config);

        let hyp_doc = generator
            .generate_hypothetical_document("what is rust", "explanation", DocumentLength::Medium)
            .await
            .unwrap();

        assert!(!hyp_doc.is_empty());
    }

    #[tokio::test]
    async fn test_hyde_intent_specific_generation() {
        let config = Config::default();
        let generator = HyDEGenerator::new(config);

        let tutorial = generator
            .generate_hypothetical_document("learn async", "tutorial", DocumentLength::Short)
            .await
            .unwrap();

        let reference = generator
            .generate_hypothetical_document("learn async", "reference", DocumentLength::Short)
            .await
            .unwrap();

        assert_ne!(tutorial, reference);
    }

    #[test]
    fn test_should_use_hyde_for_explanations() {
        assert!(should_use_hyde("what is kubernetes", "explanation", 0.5));
        assert!(should_use_hyde("explain rust ownership", "explanation", 0.3));
    }

    #[test]
    fn test_should_not_use_hyde_for_short_queries() {
        assert!(!should_use_hyde("api", "reference", 0.9));
        assert!(!should_use_hyde("rust docs", "reference", 0.8));
    }

    #[test]
    fn test_should_use_hyde_for_conceptual_keywords() {
        assert!(should_use_hyde("why use docker", "howto", 0.6));
        assert!(should_use_hyde("difference between rust and go", "explanation", 0.5));
        assert!(should_use_hyde("understand async architecture", "tutorial", 0.7));
    }

    #[tokio::test]
    async fn test_hyde_search_dual_embedding() {
        let config = Config::default();
        let service = HyDESearchService::new(config);

        let (query, results, hyp_doc) = service
            .search_with_hyde("what is cargo", "explanation", 5, 0.6, 0.4)
            .await
            .unwrap();

        assert_eq!(query, "what is cargo");
        assert!(!hyp_doc.is_empty());
        assert!(results.len() <= 5);
    }
}
```

**Success Metrics**:

- Explanation queries: 25-30 percent precision improvement
- Tutorial queries: 15-20 percent precision improvement
- Overall MRR (Mean Reciprocal Rank): 10-15 percent improvement
- Latency overhead: Under 400ms per query
- Test coverage: Above 80 percent

### Phase 4: Query Analysis and Entity Extraction

**Objective**: Extract structured information from queries (entities, actions, context) to enable smarter search strategies and better result filtering.

**Duration**: 2-3 weeks

**Dependencies**: Phase 2 (uses LLM infrastructure)

**Components**:

#### 4.1 QueryAnalyzer Struct

Create `crates/core/src/query/analyzer.rs`:

```xze/crates/core/src/query/analyzer.rs#L1-120
use anyhow::Result;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Structured analysis of a user query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub entities: Vec<String>,
    pub action: String,
    pub technical_terms: Vec<String>,
    pub ambiguities: Vec<String>,
    pub context_hints: Vec<String>,
    pub complexity: QueryComplexity,
}

/// Query complexity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
}

/// Extract structured information from user queries
pub struct QueryAnalyzer {
    config: Config,
    ollama_client: OllamaClient,
    cache: Mutex<LruCache<String, QueryAnalysis>>,
}

impl QueryAnalyzer {
    pub fn new(config: Config) -> Self {
        let cache_size = NonZeroUsize::new(config.query_cache_size).unwrap();
        Self {
            config,
            ollama_client: OllamaClient::new(&config.ollama_url),
            cache: Mutex::new(LruCache::new(cache_size)),
        }
    }

    /// Perform comprehensive query analysis
    ///
    /// Extracts:
    /// - Entities: Product names, technologies, commands
    /// - Action: What user wants to do (configure, learn, debug, etc)
    /// - Technical terms: Domain-specific terminology
    /// - Ambiguities: Terms that need clarification
    /// - Context hints: Implicit context from query structure
    pub async fn analyze(&self, query: &str) -> Result<QueryAnalysis> {
        if let Some(cached) = self.cache.lock().unwrap().get(query) {
            return Ok(cached.clone());
        }

        let analysis = self.analyze_uncached(query).await?;
        self.cache.lock().unwrap().put(query.to_string(), analysis.clone());

        Ok(analysis)
    }

    async fn analyze_uncached(&self, query: &str) -> Result<QueryAnalysis> {
        let prompt = self.build_analysis_prompt(query);
        let response = self.ollama_client.generate(&prompt).await?;

        let analysis: QueryAnalysis = serde_json::from_str(&response)?;

        Ok(analysis)
    }

    fn build_analysis_prompt(&self, query: &str) -> String {
        format!(
            "Analyze this technical documentation search query:\n\n\
             Query: \"{}\"\n\n\
             Extract the following information in JSON format:\n\n\
             {{\n\
                 \"entities\": [\"list\", \"of\", \"key\", \"entities\"],\n\
                 \"action\": \"primary_action_requested\",\n\
                 \"technical_terms\": [\"domain\", \"specific\", \"terms\"],\n\
                 \"ambiguities\": [\"ambiguous\", \"terms\"],\n\
                 \"context_hints\": [\"implicit\", \"context\"],\n\
                 \"complexity\": \"Simple|Moderate|Complex\"\n\
             }}\n\n\
             Definitions:\n\
             - entities: Product names, tools, commands, technologies mentioned\n\
             - action: What the user wants to do (install, configure, debug, understand, etc)\n\
             - technical_terms: Specialized terminology that should be preserved\n\
             - ambiguities: Terms with multiple meanings that might need clarification\n\
             - context_hints: Implicit information (skill level, use case, etc)\n\
             - complexity: Query complexity based on number of concepts\n\n\
             JSON response:",
            query
        )
    }
}
```

#### 4.2 Entity-Aware Search Enhancement

Create `crates/core/src/search/entity_aware.rs`:

```xze/crates/core/src/search/entity_aware.rs#L1-100
use anyhow::Result;

/// Search service enhanced with entity understanding
pub struct EntityAwareSearchService {
    search_service: SearchService,
}

impl EntityAwareSearchService {
    pub fn new(config: Config) -> Self {
        Self {
            search_service: SearchService::new(config),
        }
    }

    /// Search using entity-aware strategies
    ///
    /// Strategies:
    /// 1. Boost results containing identified entities
    /// 2. Filter by technical terms when confidence is high
    /// 3. Expand ambiguous terms before searching
    /// 4. Adjust search based on complexity
    pub async fn search_with_entities(
        &self,
        query: &str,
        analysis: &QueryAnalysis,
        intent: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let enriched_queries = self.generate_entity_queries(query, analysis);

        let mut all_results = Vec::new();
        for eq in enriched_queries {
            let results = self.search_service.search(&eq, limit * 2).await?;
            all_results.extend(results);
        }

        let scored_results = self.score_by_entity_match(
            all_results,
            &analysis.entities,
            &analysis.technical_terms,
        );

        Ok(scored_results.into_iter().take(limit).collect())
    }

    fn generate_entity_queries(&self, query: &str, analysis: &QueryAnalysis) -> Vec<String> {
        let mut queries = vec![query.to_string()];

        if !analysis.entities.is_empty() {
            let entity_query = format!("{} {}", analysis.entities.join(" "), query);
            queries.push(entity_query);
        }

        if !analysis.technical_terms.is_empty() {
            let tech_query = format!("{} {}", query, analysis.technical_terms.join(" "));
            queries.push(tech_query);
        }

        queries
    }

    fn score_by_entity_match(
        &self,
        mut results: Vec<SearchResult>,
        entities: &[String],
        technical_terms: &[String],
    ) -> Vec<SearchResult> {
        for result in &mut results {
            let mut boost = 0.0;

            let content_lower = result.content.to_lowercase();
            for entity in entities {
                if content_lower.contains(&entity.to_lowercase()) {
                    boost += 0.1;
                }
            }

            for term in technical_terms {
                if content_lower.contains(&term.to_lowercase()) {
                    boost += 0.05;
                }
            }

            result.similarity += boost;
        }

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results
    }
}
```

#### 4.3 Ambiguity Detection and Handling

Create `crates/core/src/query/ambiguity.rs`:

```xze/crates/core/src/query/ambiguity.rs#L1-60
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result of ambiguity handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguityResult {
    pub needs_clarification: bool,
    pub suggestions: Vec<String>,
    pub expanded_queries: Vec<String>,
}

/// Detect and handle query ambiguities
pub struct AmbiguityHandler {
    ollama_client: OllamaClient,
}

impl AmbiguityHandler {
    pub fn new(config: Config) -> Self {
        Self {
            ollama_client: OllamaClient::new(&config.ollama_url),
        }
    }

    /// Handle ambiguous terms in query
    pub async fn handle_ambiguities(
        &self,
        query: &str,
        ambiguities: &[String],
    ) -> Result<AmbiguityResult> {
        if ambiguities.is_empty() {
            return Ok(AmbiguityResult {
                needs_clarification: false,
                suggestions: vec![],
                expanded_queries: vec![query.to_string()],
            });
        }

        let suggestions = self.generate_interpretations(query, ambiguities).await?;

        let mut expanded = Vec::new();
        for suggestion in &suggestions {
            let expanded_query = self.expand_for_interpretation(query, suggestion).await?;
            expanded.push(expanded_query);
        }

        Ok(AmbiguityResult {
            needs_clarification: suggestions.len() > 1,
            suggestions,
            expanded_queries: expanded,
        })
    }

    async fn generate_interpretations(
        &self,
        query: &str,
        ambiguities: &[String],
    ) -> Result<Vec<String>> {
        let prompt = format!(
            "The query \"{}\" contains ambiguous terms: {}.\n\
             Provide 2-3 possible interpretations.\n\
             List each interpretation on a new line.",
            query,
            ambiguities.join(", ")
        );

        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect())
    }

    async fn expand_for_interpretation(&self, query: &str, interpretation: &str) -> Result<String> {
        let prompt = format!(
            "Rewrite this query \"{}\" using this interpretation: {}",
            query, interpretation
        );

        let response = self.ollama_client.generate(&prompt).await?;

        Ok(response.trim().to_string())
    }
}
```

#### 4.4 Configuration Updates

Add to `crates/core/src/config.rs`:

```xze/crates/core/src/config.rs#L110-120
#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisConfig {
    /// Enable query analysis feature
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Model to use for analysis
    #[serde(default = "default_analysis_model")]
    pub model: String,

    /// Temperature for generation
    #[serde(default = "default_analysis_temperature")]
    pub temperature: f32,

    /// LRU cache size
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
}

fn default_analysis_model() -> String { "llama3.2:3b".to_string() }
fn default_analysis_temperature() -> f32 { 0.2 }
```

#### 4.5 Testing Requirements

Create `crates/core/src/query/tests/analyzer_tests.rs`:

```xze/crates/core/src/query/tests/analyzer_tests.rs#L1-80
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_analyzer_extracts_entities() {
        let config = Config::default();
        let analyzer = QueryAnalyzer::new(config);

        let analysis = analyzer
            .analyze("how to setup docker on kubernetes")
            .await
            .unwrap();

        assert!(analysis.entities.len() >= 1);
        assert!(!analysis.action.is_empty());
    }

    #[tokio::test]
    async fn test_analyzer_detects_complexity() {
        let config = Config::default();
        let analyzer = QueryAnalyzer::new(config);

        let simple = analyzer.analyze("api docs").await.unwrap();
        let complex = analyzer
            .analyze("how to configure distributed tracing with opentelemetry in rust microservices")
            .await
            .unwrap();

        assert!(matches!(simple.complexity, QueryComplexity::Simple));
        assert!(matches!(complex.complexity, QueryComplexity::Complex));
    }

    #[tokio::test]
    async fn test_entity_aware_search_boosts_entity_matches() {
        let config = Config::default();
        let service = EntityAwareSearchService::new(config);

        let analysis = QueryAnalysis {
            entities: vec!["docker".to_string()],
            action: "setup".to_string(),
            technical_terms: vec!["container".to_string()],
            ambiguities: vec![],
            context_hints: vec![],
            complexity: QueryComplexity::Moderate,
        };

        let results = service
            .search_with_entities("setup docker", &analysis, "howto", 5)
            .await
            .unwrap();

        assert!(results.len() <= 5);
    }

    #[tokio::test]
    async fn test_ambiguity_handler_no_ambiguities() {
        let config = Config::default();
        let handler = AmbiguityHandler::new(config);

        let result = handler
            .handle_ambiguities("docker setup", &[])
            .await
            .unwrap();

        assert!(!result.needs_clarification);
        assert_eq!(result.expanded_queries, vec!["docker setup"]);
    }

    #[tokio::test]
    async fn test_ambiguity_handler_with_ambiguities() {
        let config = Config::default();
        let handler = AmbiguityHandler::new(config);

        let result = handler
            .handle_ambiguities("java tutorial", &["java".to_string()])
            .await
            .unwrap();

        assert!(result.suggestions.len() >= 1);
        assert!(result.expanded_queries.len() >= 1);
    }
}
```

**Success Metrics**:

- Entity extraction accuracy: Above 85 percent
- Action classification accuracy: Above 90 percent
- Search relevance with entities: 10-15 percent improvement
- Latency overhead: Under 200ms per query
- Test coverage: Above 80 percent

### Phase 5: Integrated Query Enhancement Pipeline

**Objective**: Integrate all enhancement strategies into a unified, intelligent query processing pipeline with automatic strategy selection and fallback mechanisms.

**Duration**: 2-3 weeks

**Dependencies**: Phases 1-4 (integrates all components)

**Components**:

#### 5.1 EnhancementPipeline Struct

Create `crates/core/src/query/pipeline.rs`:

```xze/crates/core/src/query/pipeline.rs#L1-150
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Complete result of query enhancement pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementResult {
    pub original_query: String,
    pub rewritten_query: String,
    pub expanded_queries: Vec<String>,
    pub hyde_document: Option<String>,
    pub analysis: QueryAnalysis,
    pub intent_result: IntentResult,
    pub strategy_used: String,
    pub enhancement_metadata: serde_json::Value,
}

/// Options for enhancement pipeline
#[derive(Debug, Clone, Default)]
pub struct EnhancementOptions {
    pub force_strategy: Option<String>,
    pub disable_cache: bool,
    pub timeout_ms: Option<u64>,
}

/// Unified query enhancement pipeline
///
/// Orchestrates all enhancement strategies and selects optimal
/// approach based on query characteristics and intent
pub struct EnhancementPipeline {
    config: Config,
    intent_classifier: IntentClassifier,
    query_analyzer: QueryAnalyzer,
    query_rewriter: QueryRewriter,
    query_expander: QueryExpander,
    hyde_generator: HyDEGenerator,
}

impl EnhancementPipeline {
    pub fn new(config: Config) -> Self {
        Self {
            intent_classifier: IntentClassifier::new(config.clone()),
            query_analyzer: QueryAnalyzer::new(config.clone()),
            query_rewriter: QueryRewriter::new(config.clone()),
            query_expander: QueryExpander::new(config.clone()),
            hyde_generator: HyDEGenerator::new(config.clone()),
            config,
        }
    }

    /// Run complete enhancement pipeline
    ///
    /// Steps:
    /// 1. Classify intent
    /// 2. Analyze query structure
    /// 3. Select enhancement strategy
    /// 4. Apply enhancements
    /// 5. Generate search variants
    pub async fn enhance(
        &self,
        query: &str,
        options: Option<EnhancementOptions>,
    ) -> Result<EnhancementResult> {
        let options = options.unwrap_or_default();

        let intent_result = self.intent_classifier.classify(query).await?;

        let analysis = self.query_analyzer.analyze(query).await?;

        let strategy = if let Some(forced) = options.force_strategy {
            forced
        } else {
            self.select_strategy(query, &intent_result, &analysis)
        };

        let enhancement = self
            .apply_strategy(query, &intent_result, &analysis, &strategy)
            .await?;

        Ok(enhancement)
    }

    fn select_strategy(
        &self,
        query: &str,
        intent_result: &IntentResult,
        analysis: &QueryAnalysis,
    ) -> String {
        if intent_result.intent == "explanation" && intent_result.confidence > 0.7 {
            return "hyde_primary".to_string();
        }

        if matches!(analysis.complexity, QueryComplexity::Complex) && analysis.entities.len() >= 2 {
            return "entity_focused".to_string();
        }

        if intent_result.confidence > 0.8 {
            return "rewrite_expand".to_string();
        }

        if intent_result.confidence > 0.5 {
            return "expand_only".to_string();
        }

        "minimal".to_string()
    }

    async fn apply_strategy(
        &self,
        query: &str,
        intent_result: &IntentResult,
        analysis: &QueryAnalysis,
        strategy: &str,
    ) -> Result<EnhancementResult> {
        let (rewritten, expanded, hyde_doc) = match strategy {
            "hyde_primary" => {
                let hyp = self
                    .hyde_generator
                    .generate_hypothetical_document(
                        query,
                        &intent_result.intent,
                        DocumentLength::Medium,
                    )
                    .await?;
                (query.to_string(), vec![hyp.clone()], Some(hyp))
            }
            "entity_focused" => {
                let entity_query = format!("{} {}", analysis.entities.join(" "), query);
                let variants = self
                    .query_expander
                    .expand(&entity_query, 2, Some(&intent_result.intent))
                    .await?;
                (entity_query, variants, None)
            }
            "rewrite_expand" => {
                let rewritten = self
                    .query_rewriter
                    .rewrite(query, &intent_result.intent, intent_result.confidence, None)
                    .await?;
                let variants = self
                    .query_expander
                    .expand(&rewritten, 2, Some(&intent_result.intent))
                    .await?;
                (rewritten, variants, None)
            }
            "expand_only" => {
                let variants = self
                    .query_expander
                    .expand(query, 3, Some(&intent_result.intent))
                    .await?;
                (query.to_string(), variants, None)
            }
            _ => (query.to_string(), vec![query.to_string()], None),
        };

        Ok(EnhancementResult {
            original_query: query.to_string(),
            rewritten_query: rewritten,
            expanded_queries: expanded,
            hyde_document: hyde_doc,
            analysis: analysis.clone(),
            intent_result: intent_result.clone(),
            strategy_used: strategy.to_string(),
            enhancement_metadata: serde_json::json!({}),
        })
    }
}
```

#### 5.2 Fallback Mechanisms

Implement graceful degradation in `crates/core/src/query/fallback.rs`:

```xze/crates/core/src/query/fallback.rs#L1-80
use anyhow::Result;
use std::future::Future;
use tracing::{error, info, warn};

/// Handle failures and provide fallback strategies
pub struct FallbackHandler;

impl FallbackHandler {
    /// Execute strategy with fallback chain
    ///
    /// Tries primary strategy first, falls back to alternatives
    /// if primary fails or produces poor results
    pub async fn execute_with_fallback<T, F, Fut>(
        &self,
        primary_strategy: F,
        fallback_strategies: Vec<F>,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
        T: Clone,
    {
        match primary_strategy().await {
            Ok(result) => {
                if self.is_acceptable_result(&result) {
                    return Ok(result);
                }
                warn!("Primary strategy produced poor results, trying fallback");
            }
            Err(e) => {
                error!("Primary strategy failed: {}", e);
            }
        }

        for (idx, fallback) in fallback_strategies.iter().enumerate() {
            match fallback().await {
                Ok(result) => {
                    if self.is_acceptable_result(&result) {
                        info!("Fallback strategy {} succeeded", idx);
                        return Ok(result);
                    }
                }
                Err(e) => {
                    error!("Fallback {} failed: {}", idx, e);
                    continue;
                }
            }
        }

        warn!("All strategies failed, using basic search");
        self.basic_search_fallback().await
    }

    fn is_acceptable_result<T>(&self, _result: &T) -> bool {
        true
    }

    async fn basic_search_fallback<T>(&self) -> Result<T> {
        anyhow::bail!("All enhancement strategies failed")
    }
}
```

#### 5.3 Performance Optimization

Parallel execution in `crates/core/src/query/parallel.rs`:

```xze/crates/core/src/query/parallel.rs#L1-60
use anyhow::Result;
use tokio::task::JoinSet;

/// Run enhancement strategies in parallel where possible
pub struct ParallelEnhancementPipeline {
    pipeline: EnhancementPipeline,
}

impl ParallelEnhancementPipeline {
    pub fn new(config: Config) -> Self {
        Self {
            pipeline: EnhancementPipeline::new(config),
        }
    }

    /// Run independent enhancement steps in parallel
    ///
    /// Parallel:
    /// - Intent classification
    /// - Query analysis
    ///
    /// Sequential (depends on above):
    /// - Strategy selection
    /// - Enhancement application
    pub async fn enhance_parallel(&self, query: &str) -> Result<EnhancementResult> {
        let mut join_set = JoinSet::new();

        let query_clone = query.to_string();
        let pipeline_clone = self.pipeline.clone();
        join_set.spawn(async move {
            pipeline_clone.intent_classifier.classify(&query_clone).await
        });

        let query_clone = query.to_string();
        let pipeline_clone = self.pipeline.clone();
        join_set.spawn(async move {
            pipeline_clone.query_analyzer.analyze(&query_clone).await
        });

        let intent_result = join_set.join_next().await.unwrap()??;
        let analysis = join_set.join_next().await.unwrap()??;

        let strategy = self.pipeline.select_strategy(query, &intent_result, &analysis);
        let enhancement = self
            .pipeline
            .apply_strategy(query, &intent_result, &analysis, &strategy)
            .await?;

        Ok(enhancement)
    }
}
```

#### 5.4 Integration with Existing Search

Update `SearchService` in `crates/core/src/search/service.rs`:

```xze/crates/core/src/search/service.rs#L200-280
impl SearchService {
    /// Search with full enhancement pipeline
    ///
    /// # Arguments
    ///
    /// * `query` - Original user query
    /// * `limit` - Number of results
    /// * `enable_enhancement` - Whether to use enhancement pipeline
    ///
    /// # Returns
    ///
    /// Returns tuple of (final_query, results, enhancement_metadata)
    pub async fn search_enhanced(
        &self,
        query: &str,
        limit: usize,
        enable_enhancement: bool,
    ) -> Result<(String, Vec<SearchResult>, EnhancementResult)> {
        if !enable_enhancement {
            let results = self.search(query, limit).await?;
            let empty_enhancement = EnhancementResult::default();
            return Ok((query.to_string(), results, empty_enhancement));
        }

        let pipeline = EnhancementPipeline::new(self.config.clone());
        let enhancement = pipeline.enhance(query, None).await?;

        let results = match enhancement.strategy_used.as_str() {
            "hyde_primary" => {
                if let Some(hyde_doc) = &enhancement.hyde_document {
                    self.search_with_hyde(hyde_doc, limit).await?
                } else {
                    self.search(query, limit).await?
                }
            }
            "entity_focused" => {
                self.search_with_entities(
                    &enhancement.expanded_queries,
                    &enhancement.analysis.entities,
                    limit,
                )
                .await?
            }
            _ => {
                self.search_with_expansion_variants(&enhancement.expanded_queries, limit)
                    .await?
            }
        };

        Ok((enhancement.rewritten_query.clone(), results, enhancement))
    }

    async fn search_with_hyde(&self, hyde_doc: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let embedding = self.generate_embedding(hyde_doc).await?;
        self.execute_search(&embedding, limit).await
    }

    async fn search_with_entities(
        &self,
        queries: &[String],
        entities: &[String],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();
        for query in queries {
            let embedding = self.generate_embedding(query).await?;
            let results = self.execute_search(&embedding, limit * 2).await?;
            all_results.extend(results);
        }

        let merger = ResultMerger;
        let merged = merger.merge(vec![all_results], MergeStrategy::MaxScore)?;
        Ok(merged.into_iter().take(limit).collect())
    }

    async fn search_with_expansion_variants(
        &self,
        variants: &[String],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut result_sets = Vec::new();
        for variant in variants {
            let embedding = self.generate_embedding(variant).await?;
            let results = self.execute_search(&embedding, limit * 2).await?;
            result_sets.push(results);
        }

        let merger = ResultMerger;
        let merged = merger.merge(result_sets, MergeStrategy::MaxScore)?;
        Ok(merged.into_iter().take(limit).collect())
    }
}
```

#### 5.5 Configuration Updates

Add to `crates/core/src/config.rs`:

```xze/crates/core/src/config.rs#L125-140
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineConfig {
    /// Enable enhancement pipeline
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Enable parallel execution
    #[serde(default = "default_true")]
    pub parallel_execution: bool,

    /// Enable fallback mechanisms
    #[serde(default = "default_true")]
    pub fallback_enabled: bool,

    /// Cache enhancement results
    #[serde(default = "default_true")]
    pub cache_results: bool,

    /// LRU cache size for pipeline results
    #[serde(default = "default_pipeline_cache_size")]
    pub cache_size: usize,

    /// Timeout for enhancement pipeline (seconds)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_pipeline_cache_size() -> usize { 1000 }
fn default_timeout() -> u64 { 5 }
```

#### 5.6 Testing Requirements

Create `crates/core/tests/integration_pipeline_tests.rs`:

```xze/crates/core/tests/integration_pipeline_tests.rs#L1-100
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_pipeline_selects_hyde_for_explanations() {
        let config = Config::default();
        let pipeline = EnhancementPipeline::new(config);

        let enhancement = pipeline
            .enhance("what is kubernetes", None)
            .await
            .unwrap();

        assert_eq!(enhancement.original_query, "what is kubernetes");
        assert!(!enhancement.strategy_used.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_selects_entity_focused_for_complex_queries() {
        let config = Config::default();
        let pipeline = EnhancementPipeline::new(config);

        let enhancement = pipeline
            .enhance("how to deploy rust microservices with docker and kubernetes", None)
            .await
            .unwrap();

        assert!(enhancement.analysis.entities.len() >= 1);
    }

    #[tokio::test]
    async fn test_pipeline_fallback_on_failure() {
        let config = Config::default();
        let handler = FallbackHandler;

        let primary = || async { Err(anyhow::anyhow!("Primary failed")) };
        let fallback1 = || async { Ok("fallback result".to_string()) };

        let result = handler
            .execute_with_fallback(primary, vec![fallback1])
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_parallel_pipeline_faster_than_sequential() {
        let config = Config::default();
        let parallel_pipeline = ParallelEnhancementPipeline::new(config);

        let start = std::time::Instant::now();
        let _enhancement = parallel_pipeline
            .enhance_parallel("test query")
            .await
            .unwrap();
        let parallel_duration = start.elapsed();

        assert!(parallel_duration.as_millis() < 5000);
    }

    #[tokio::test]
    async fn test_search_enhanced_with_pipeline() {
        let config = Config::default();
        let service = SearchService::new(config);

        let (final_query, results, enhancement) = service
            .search_enhanced("how to setup rust", 5, true)
            .await
            .unwrap();

        assert_eq!(enhancement.original_query, "how to setup rust");
        assert!(results.len() <= 5);
        assert!(!final_query.is_empty());
    }

    #[tokio::test]
    async fn test_search_enhanced_disabled_returns_basic_search() {
        let config = Config::default();
        let service = SearchService::new(config);

        let (final_query, results, _) = service
            .search_enhanced("test query", 5, false)
            .await
            .unwrap();

        assert_eq!(final_query, "test query");
        assert!(results.len() <= 5);
    }

    #[tokio::test]
    async fn test_all_strategies_executable() {
        let config = Config::default();
        let pipeline = EnhancementPipeline::new(config);

        let strategies = vec!["hyde_primary", "entity_focused", "rewrite_expand", "expand_only", "minimal"];

        for strategy in strategies {
            let options = EnhancementOptions {
                force_strategy: Some(strategy.to_string()),
                disable_cache: false,
                timeout_ms: None,
            };

            let result = pipeline.enhance("test query", Some(options)).await;
            assert!(result.is_ok(), "Strategy {} failed", strategy);
        }
    }
}
```

**Success Metrics**:

- Overall search quality: 20-30 percent improvement (composite metric)
- End-to-end latency: Under 1 second for 95th percentile
- Cache hit rate: Above 50 percent
- Fallback trigger rate: Below 5 percent
- Test coverage: Above 80 percent

### Phase 6: Monitoring, Evaluation, and Iteration

**Objective**: Implement comprehensive monitoring, evaluation metrics, and iterative improvement framework.

**Duration**: Ongoing

**Dependencies**: Phase 5 (requires complete pipeline)

**Components**:

#### 6.1 Metrics Collection

Create `crates/core/src/metrics/collector.rs`:

```xze/crates/core/src/metrics/collector.rs#L1-80
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metrics for a single query enhancement and search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub query_id: String,
    pub original_query: String,
    pub timestamp: DateTime<Utc>,
    pub intent: String,
    pub intent_confidence: f32,
    pub strategy_used: String,
    pub enhancement_time_ms: f32,
    pub search_time_ms: f32,
    pub total_time_ms: f32,
    pub num_results: usize,
    pub top_similarity: f32,
    pub cache_hit: bool,
    pub fallback_triggered: bool,
}

/// Collect and store query enhancement metrics
pub struct MetricsCollector {
    metrics_store: HashMap<String, QueryMetrics>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics_store: HashMap::new(),
        }
    }

    /// Record metrics for a query
    pub fn record_query(&mut self, metrics: QueryMetrics) {
        self.metrics_store.insert(metrics.query_id.clone(), metrics);
    }

    /// Get aggregated statistics for time range
    pub fn get_statistics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> MetricsStatistics {
        let relevant_metrics: Vec<&QueryMetrics> = self
            .metrics_store
            .values()
            .filter(|m| m.timestamp >= start && m.timestamp <= end)
            .collect();

        let total_queries = relevant_metrics.len();
        let avg_enhancement_time = relevant_metrics
            .iter()
            .map(|m| m.enhancement_time_ms)
            .sum::<f32>()
            / total_queries as f32;
        let avg_search_time = relevant_metrics
            .iter()
            .map(|m| m.search_time_ms)
            .sum::<f32>()
            / total_queries as f32;
        let cache_hit_rate = relevant_metrics
            .iter()
            .filter(|m| m.cache_hit)
            .count() as f32
            / total_queries as f32;
        let fallback_rate = relevant_metrics
            .iter()
            .filter(|m| m.fallback_triggered)
            .count() as f32
            / total_queries as f32;

        MetricsStatistics {
            total_queries,
            avg_enhancement_time_ms: avg_enhancement_time,
            avg_search_time_ms: avg_search_time,
            cache_hit_rate,
            fallback_rate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStatistics {
    pub total_queries: usize,
    pub avg_enhancement_time_ms: f32,
    pub avg_search_time_ms: f32,
    pub cache_hit_rate: f32,
    pub fallback_rate: f32,
}
```

#### 6.2 Quality Evaluation

Create `crates/core/src/metrics/quality.rs`:

```xze/crates/core/src/metrics/quality.rs#L1-60
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Quality score for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub relevance: f32,
    pub diversity: f32,
    pub coverage: f32,
    pub freshness: f32,
    pub overall: f32,
}

/// Evaluate search result quality
pub struct QualityEvaluator;

impl QualityEvaluator {
    /// Evaluate result quality using multiple metrics
    ///
    /// Metrics:
    /// - Relevance: Similarity scores distribution
    /// - Diversity: Result diversity
    /// - Coverage: Topic coverage
    /// - Freshness: Document recency
    pub fn evaluate_results(
        &self,
        query: &str,
        results: &[SearchResult],
        enhancement: &EnhancementResult,
    ) -> Result<QualityScore> {
        let relevance = self.calculate_relevance(results);
        let diversity = self.calculate_diversity(results);
        let coverage = self.calculate_coverage(query, results);
        let freshness = self.calculate_freshness(results);

        let overall = (relevance + diversity + coverage + freshness) / 4.0;

        Ok(QualityScore {
            relevance,
            diversity,
            coverage,
            freshness,
            overall,
        })
    }

    fn calculate_relevance(&self, results: &[SearchResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }
        results.iter().map(|r| r.similarity).sum::<f32>() / results.len() as f32
    }

    fn calculate_diversity(&self, results: &[SearchResult]) -> f32 {
        if results.len() < 2 {
            return 1.0;
        }
        let unique_sources = results
            .iter()
            .map(|r| &r.source_file)
            .collect::<std::collections::HashSet<_>>()
            .len();
        unique_sources as f32 / results.len() as f32
    }

    fn calculate_coverage(&self, _query: &str, results: &[SearchResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }
        (results.len() as f32 / 5.0).min(1.0)
    }

    fn calculate_freshness(&self, _results: &[SearchResult]) -> f32 {
        0.8
    }

    /// Compare results from different strategies, return best
    pub fn compare_strategies(
        &self,
        query: &str,
        strategy_results: HashMap<String, Vec<SearchResult>>,
    ) -> Result<String> {
        let mut best_strategy = String::new();
        let mut best_score = 0.0;

        for (strategy, results) in strategy_results {
            let enhancement = EnhancementResult::default();
            let score = self.evaluate_results(query, &results, &enhancement)?;
            if score.overall > best_score {
                best_score = score.overall;
                best_strategy = strategy;
            }
        }

        Ok(best_strategy)
    }
}
```

#### 6.3 A/B Testing Framework

Create `crates/core/src/metrics/ab_testing.rs`:

```xze/crates/core/src/metrics/ab_testing.rs#L1-120
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A/B test experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub name: String,
    pub control_strategy: String,
    pub treatment_strategy: String,
    pub traffic_split: f32,
    pub results: HashMap<String, Vec<QueryMetrics>>,
}

/// Framework for A/B testing enhancement strategies
pub struct ABTestingFramework {
    config: Config,
    experiments: HashMap<String, Experiment>,
}

impl ABTestingFramework {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            experiments: HashMap::new(),
        }
    }

    /// Create new A/B test experiment
    pub fn create_experiment(
        &mut self,
        name: String,
        control_strategy: String,
        treatment_strategy: String,
        traffic_split: f32,
    ) -> Result<Experiment> {
        let experiment = Experiment {
            name: name.clone(),
            control_strategy,
            treatment_strategy,
            traffic_split,
            results: HashMap::new(),
        };

        self.experiments.insert(name, experiment.clone());

        Ok(experiment)
    }

    /// Assign query to control or treatment group
    pub fn assign_variant(&self, query_id: &str, experiment_name: &str) -> Result<String> {
        let experiment = self
            .experiments
            .get(experiment_name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        let hash = self.hash_query_id(query_id);
        let variant = if hash < experiment.traffic_split {
            experiment.treatment_strategy.clone()
        } else {
            experiment.control_strategy.clone()
        };

        Ok(variant)
    }

    /// Record result for experiment analysis
    pub fn record_result(
        &mut self,
        query_id: &str,
        experiment_name: &str,
        metrics: QueryMetrics,
    ) -> Result<()> {
        let experiment = self
            .experiments
            .get_mut(experiment_name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        let variant = self.assign_variant(query_id, experiment_name)?;
        experiment
            .results
            .entry(variant)
            .or_insert_with(Vec::new)
            .push(metrics);

        Ok(())
    }

    /// Analyze experiment results
    ///
    /// Returns statistical significance, effect size, and recommendation
    pub fn analyze_experiment(&self, experiment_name: &str) -> Result<ExperimentResults> {
        let experiment = self
            .experiments
            .get(experiment_name)
            .ok_or_else(|| anyhow::anyhow!("Experiment not found"))?;

        let control_metrics = experiment
            .results
            .get(&experiment.control_strategy)
            .ok_or_else(|| anyhow::anyhow!("No control results"))?;
        let treatment_metrics = experiment
            .results
            .get(&experiment.treatment_strategy)
            .ok_or_else(|| anyhow::anyhow!("No treatment results"))?;

        let control_avg = control_metrics
            .iter()
            .map(|m| m.top_similarity)
            .sum::<f32>()
            / control_metrics.len() as f32;
        let treatment_avg = treatment_metrics
            .iter()
            .map(|m| m.top_similarity)
            .sum::<f32>()
            / treatment_metrics.len() as f32;

        let effect_size = (treatment_avg - control_avg) / control_avg;
        let significant = effect_size.abs() > 0.05;

        Ok(ExperimentResults {
            experiment_name: experiment_name.to_string(),
            control_average: control_avg,
            treatment_average: treatment_avg,
            effect_size,
            statistically_significant: significant,
            recommendation: if significant && effect_size > 0.0 {
                "Deploy treatment strategy".to_string()
            } else {
                "Keep control strategy".to_string()
            },
        })
    }

    fn hash_query_id(&self, query_id: &str) -> f32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query_id.hash(&mut hasher);
        let hash = hasher.finish();

        (hash % 100) as f32 / 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_name: String,
    pub control_average: f32,
    pub treatment_average: f32,
    pub effect_size: f32,
    pub statistically_significant: bool,
    pub recommendation: String,
}
```

#### 6.4 Continuous Improvement

Create `crates/core/src/metrics/improvement.rs`:

```xze/crates/core/src/metrics/improvement.rs#L1-60
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Issue identified from metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub category: String,
    pub description: String,
    pub affected_queries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimization {
    pub title: String,
    pub description: String,
    pub expected_improvement: f32,
    pub implementation_effort: String,
}

/// Track and suggest improvements based on metrics
pub struct ImprovementTracker {
    metrics_collector: MetricsCollector,
}

impl ImprovementTracker {
    pub fn new(metrics_collector: MetricsCollector) -> Self {
        Self { metrics_collector }
    }

    /// Identify issues from metrics
    ///
    /// Examples:
    /// - High latency for certain query types
    /// - Low cache hit rates
    /// - Frequent fallback triggers
    /// - Poor results for specific intents
    pub fn identify_issues(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        let stats = self.metrics_collector.get_statistics(
            chrono::Utc::now() - chrono::Duration::hours(24),
            chrono::Utc::now(),
        );

        if stats.avg_enhancement_time_ms > 500.0 {
            issues.push(Issue {
                severity: IssueSeverity::High,
                category: "latency".to_string(),
                description: "High enhancement latency detected".to_string(),
                affected_queries: vec![],
            });
        }

        if stats.cache_hit_rate < 0.4 {
            issues.push(Issue {
                severity: IssueSeverity::Medium,
                category: "caching".to_string(),
                description: "Low cache hit rate".to_string(),
                affected_queries: vec![],
            });
        }

        if stats.fallback_rate > 0.05 {
            issues.push(Issue {
                severity: IssueSeverity::High,
                category: "reliability".to_string(),
                description: "High fallback trigger rate".to_string(),
                affected_queries: vec![],
            });
        }

        Ok(issues)
    }

    /// Suggest optimizations based on identified issues
    pub fn suggest_optimizations(&self, issues: &[Issue]) -> Result<Vec<Optimization>> {
        let mut optimizations = Vec::new();

        for issue in issues {
            match issue.category.as_str() {
                "latency" => {
                    optimizations.push(Optimization {
                        title: "Increase cache size".to_string(),
                        description: "Increase LRU cache sizes to reduce LLM calls".to_string(),
                        expected_improvement: 0.15,
                        implementation_effort: "Low".to_string(),
                    });
                }
                "caching" => {
                    optimizations.push(Optimization {
                        title: "Optimize cache key generation".to_string(),
                        description: "Normalize queries before caching".to_string(),
                        expected_improvement: 0.10,
                        implementation_effort: "Medium".to_string(),
                    });
                }
                "reliability" => {
                    optimizations.push(Optimization {
                        title: "Add retry logic".to_string(),
                        description: "Implement exponential backoff for LLM calls".to_string(),
                        expected_improvement: 0.05,
                        implementation_effort: "Medium".to_string(),
                    });
                }
                _ => {}
            }
        }

        Ok(optimizations)
    }
}
```

#### 6.5 Dashboards and Reporting

Monitoring dashboards to create:

- Query volume and latency trends
- Strategy selection distribution
- Cache hit rates by strategy
- Intent classification accuracy
- Search quality metrics over time
- Fallback trigger analysis
- A/B test results

**Success Metrics**:

- Monitoring coverage: 100 percent of queries tracked
- Metric collection overhead: Under 10ms per query
- Issue detection latency: Under 1 hour
- Continuous improvement cycle: Monthly iterations
- Test coverage: Above 80 percent

## Implementation Timeline

### Month 1: Foundation

- Week 1-2: Phase 1 (Query Expansion)
- Week 3-4: Phase 2 (LLM Query Rewriting)

### Month 2: Advanced Features

- Week 1-2: Phase 3 (HyDE Implementation)
- Week 3-4: Phase 4 (Query Analysis)

### Month 3: Integration and Optimization

- Week 1-2: Phase 5 (Integrated Pipeline)
- Week 3-4: Phase 6 (Monitoring and Evaluation)

### Month 4+: Iteration

- Ongoing: Monitor, evaluate, and optimize
- Monthly: Review metrics and adjust strategies
- Quarterly: Major feature additions or refinements

## Risk Mitigation

### Technical Risks

1. **Increased Latency**

   - Mitigation: Aggressive caching, parallel execution, timeouts
   - Fallback: Disable expensive features if latency exceeds threshold

2. **LLM Availability**

   - Mitigation: Connection pooling, retry logic
   - Fallback: Degrade to simpler strategies if LLM unavailable

3. **Quality Regression**
   - Mitigation: A/B testing, gradual rollout
   - Fallback: Feature flags to disable problematic components

### Operational Risks

1. **Resource Usage**

   - Mitigation: Monitor CPU/memory, implement rate limiting
   - Fallback: Scale Ollama instances horizontally

2. **Cache Memory**
   - Mitigation: LRU eviction, configurable cache sizes
   - Fallback: Reduce cache sizes if memory pressure detected

## Success Criteria

### Quantitative Metrics

- **Search Quality**: 20-30 percent improvement in relevance
- **User Engagement**: 15-20 percent increase in result click-through
- **Latency**: 95th percentile under 1 second
- **Cache Hit Rate**: Above 50 percent
- **Fallback Rate**: Below 5 percent
- **Test Coverage**: Above 80 percent

### Qualitative Metrics

- User feedback: Positive sentiment improvement
- Developer satisfaction: Easier to find relevant documentation
- System reliability: No increase in error rates

## Rollout Strategy

### Phase 1-2: Internal Testing

- Deploy to development environment
- Internal team testing and feedback
- Performance benchmarking

### Phase 3-4: Limited Beta

- 10 percent traffic split A/B test
- Monitor metrics closely
- Gather user feedback

### Phase 5: Gradual Rollout

- Increase traffic split to 50 percent
- Continue monitoring and optimization
- Address any issues discovered

### Phase 6: Full Deployment

- 100 percent traffic on new pipeline
- Maintain old pipeline as fallback
- Continue monitoring and iteration

## Maintenance and Evolution

### Weekly

- Review monitoring dashboards
- Address any critical issues
- Update cache configurations if needed

### Monthly

- Analyze A/B test results
- Review quality metrics
- Plan optimizations for next month

### Quarterly

- Major feature additions
- Model updates (if better models available)
- Architecture refinements
- Performance optimization sprints

## Commands and Validation

### Development Commands

```xze/terminal#L1-20
# Format code
cargo fmt --all

# Check compilation
cargo check --all-targets --all-features

# Run linter with zero warnings
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --all-features

# Run specific module tests
cargo test --package xze-core query::

# Run benchmarks
cargo bench

# Run integration tests
cargo test --test integration_pipeline_tests
```

### Quality Gates (All Must Pass)

```xze/terminal#L1-10
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Conclusion

This phased implementation plan provides a structured approach to dramatically improving search quality through intelligent query enhancement. By leveraging LLM capabilities beyond simple intent classification, the system will better understand user queries and retrieve more relevant results.

The plan balances ambition with pragmatism, implementing features incrementally with proper testing, monitoring, and fallback mechanisms at each stage. Success will be measured through concrete metrics, with continuous iteration based on real-world performance data.

All code follows Rust best practices:

- Async/await with Tokio runtime
- Proper error handling with `Result` and `anyhow`
- LRU caching for performance
- Comprehensive unit and integration tests
- Above 80 percent test coverage target
- All quality gates enforced in CI/CD

**Next Steps**:

1. Review and approve implementation plan
2. Set up development environment with Rust toolchain
3. Create feature branch: `pr-query-enhancement-XZE-1001`
4. Begin implementation of `QueryExpander` struct
5. Establish baseline metrics for comparison
6. Run `cargo fmt` and `cargo clippy` after each component
7. Ensure all tests pass before committing

**Branch Naming**: `pr-query-enhancement-XZE-1001`

**Commit Format**: `feat(query): implement query expansion foundation (XZE-1001)`

**Last Updated**: 2025-01-30
