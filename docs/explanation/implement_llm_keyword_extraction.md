# Implementing LLM-Based Keyword Extraction

This guide provides a phased implementation plan for migrating from hardcoded
keyword extraction to LLM-based extraction in production. This plan is designed
for safe, validated rollout with clear success criteria and rollback procedures.

## Overview

**Objective**: Replace frequency-based keyword extraction with intelligent
LLM-based extraction that automatically adapts to any documentation domain
without code changes.

**Timeline**: 6-8 weeks from research to full production rollout

**Success Criteria**: 15%+ improvement in search quality metrics with acceptable
performance

## Prerequisites

Before starting implementation, ensure:

- [ ] Ollama service is running and stable
- [ ] LLM model (llama3.2:3b or similar) is available
- [ ] Current system has >80% test coverage
- [ ] Baseline search quality metrics are established
- [ ] JIRA issue created for tracking (e.g., XZE-XXXX)

## Phase 0: Research and Validation (Week 1-2)

**Objective**: Prove that LLM-based extraction improves search quality before
investing in full implementation.

### Tasks

#### Task 0.1: Establish Baseline Metrics

**Effort**: 1 day

**Steps**:

1. Select 100 representative documents across different domains
2. Create test query set (50 queries covering different intents)
3. Measure current search quality:
   - Precision@5 (percentage of top 5 results that are relevant)
   - Recall@10 (percentage of relevant docs found in top 10)
   - Mean Reciprocal Rank (MRR)
   - Zero-result query percentage

**Deliverable**: `docs/reference/baseline_search_metrics.md`

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1-10
# Example test script
cargo run --bin measure-search-quality -- \
    --queries test_queries.json \
    --output baseline_metrics.json
```

**Success Criteria**: Clear baseline established for comparison

#### Task 0.2: Implement Prototype LLM Extractor

**Effort**: 2-3 days

**Steps**:

1. Create branch: `pr-llm-keyword-prototype-0001`
2. Create prototype module: `examples/prototype_llm_extractor.rs`
3. Implement basic LLM extraction with structured prompts
4. Test on 20 sample documents
5. Manually review keyword quality

**Prototype Code Structure**:

```xze/examples/prototype_llm_extractor.rs#L1-150
/// Prototype LLM-based keyword extractor for validation.
///
/// This is a standalone example for testing LLM extraction quality
/// before integrating into the main system.

use anyhow::{Context, Result};
use ollama_rs::Ollama;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    pub keywords: Vec<String>,
    pub phrases: Vec<String>,
    pub acronyms: std::collections::HashMap<String, String>,
    pub tools: Vec<String>,
    pub commands: Vec<String>,
}

/// Extract keywords using LLM with structured prompt.
///
/// # Arguments
///
/// * `content` - Document content to analyze
/// * `model` - Ollama model to use
/// * `max_content_length` - Maximum characters to send to LLM
///
/// # Returns
///
/// Returns extracted keywords, phrases, acronyms, tools, and commands
///
/// # Errors
///
/// Returns error if LLM extraction fails
pub async fn extract_keywords_llm(
    content: &str,
    model: &str,
    max_content_length: usize,
) -> Result<ExtractedKeywords> {
    // Truncate content to avoid token limits
    let truncated_content = if content.len() > max_content_length {
        &content[..max_content_length]
    } else {
        content
    };

    let prompt = format!(
        r#"You are analyzing technical documentation to extract important keywords for search indexing.

Analyze this document excerpt and extract the most relevant search terms:

{}

Provide a JSON response with the following structure:
{{
    "keywords": ["word1", "word2", ...],
    "phrases": ["phrase 1", "phrase 2", ...],
    "acronyms": {{"CLI": "Command Line Interface", ...}},
    "tools": ["tool1", "tool2", ...],
    "commands": ["command1", "command2", ...]
}}

Guidelines:
- Extract 15-20 single-word technical terms for "keywords"
- Extract 5-10 multi-word key phrases for "phrases"
- Map acronyms to their expansions in "acronyms"
- List tool or product names in "tools"
- List technical commands or API names in "commands"

Focus on technical terminology, domain-specific vocabulary, and important concepts.

Return ONLY valid JSON, no other text."#,
        truncated_content
    );

    let ollama = Ollama::default();

    let response = ollama
        .generate_json(model, &prompt, None)
        .await
        .context("Failed to generate LLM response")?;

    let result: ExtractedKeywords = serde_json::from_str(&response)
        .context("Failed to parse LLM response as JSON")?;

    Ok(result)
}

/// Test LLM extraction on sample documents.
///
/// # Arguments
///
/// * `sample_dir` - Directory containing sample markdown files
/// * `output_file` - JSON file to save results
///
/// # Errors
///
/// Returns error if file operations fail
pub async fn test_on_sample_documents(
    sample_dir: &Path,
    output_file: &Path,
) -> Result<()> {
    use tokio::fs;

    let mut results = Vec::new();

    let mut entries = fs::read_dir(sample_dir)
        .await
        .context("Failed to read sample directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        info!("Processing {:?}", path.file_name());

        let content = fs::read_to_string(&path)
            .await
            .context("Failed to read file")?;

        match extract_keywords_llm(&content, "llama3.2:3b", 2000).await {
            Ok(extracted) => {
                results.push(serde_json::json!({
                    "file": path.file_name(),
                    "success": true,
                    "keywords": extracted
                }));
            }
            Err(e) => {
                error!("Failed to extract keywords: {}", e);
                results.push(serde_json::json!({
                    "file": path.file_name(),
                    "success": false,
                    "error": e.to_string()
                }));
            }
        }
    }

    let json = serde_json::to_string_pretty(&results)?;
    fs::write(output_file, json)
        .await
        .context("Failed to write results")?;

    info!("Results saved to {:?}", output_file);
    Ok(())
}
```

**Deliverable**: Working prototype example with test results

**Success Criteria**: LLM successfully extracts structured keywords from sample
documents

#### Task 0.3: Quality Comparison

**Effort**: 2 days

**Steps**:

1. Run prototype on 100 test documents
2. Compare LLM-extracted keywords vs. frequency-based
3. Manually review 20 random samples for quality
4. Run search quality tests with both approaches
5. Calculate improvement metrics

**Comparison Commands**:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L200-210
# Extract with current method
cargo run --bin extract-keywords-current -- \
    --input test_docs/ \
    --output current_keywords.json

# Extract with LLM
cargo run --example prototype_llm_extractor -- \
    --input test_docs/ \
    --output llm_keywords.json

# Compare quality
cargo run --bin compare-keyword-quality -- \
    --current current_keywords.json \
    --llm llm_keywords.json \
    --output comparison_report.md
```

**Deliverable**: `docs/reference/keyword_extraction_comparison.md`

**Success Criteria**: LLM approach shows 15%+ improvement in search quality
metrics

#### Task 0.4: Decision Gate

**Effort**: 1 day

**Decision Criteria**:

- [ ] LLM extraction succeeds on 95%+ of test documents
- [ ] Search quality improves by 15%+ (Precision@5 or Recall@10)
- [ ] LLM extraction time acceptable (<3s per document avg)
- [ ] Stakeholder approval to proceed

**Outcomes**:

- **GO**: Proceed to Phase 1 (Core Implementation)
- **NO-GO**: Document findings, consider alternative approaches

## Phase 1: Core Implementation (Week 3-4)

**Objective**: Build production-ready keyword extraction module with caching,
fallback, and testing.

**Branch**: `pr-llm-keyword-core-xxxx`

### Tasks

#### Task 1.1: Create Keyword Extractor Module

**Effort**: 3-4 days

**Steps**:

1. Create `crates/core/src/keyword_extractor.rs`
2. Implement `KeywordExtractor` struct with strategy pattern
3. Add LLM-based extraction method
4. Add embedding-based fallback
5. Add frequency-based fallback
6. Implement content-hash caching
7. Add comprehensive error handling
8. Write docstrings and type annotations

**Module Structure**:

```xze/crates/core/src/keyword_extractor.rs#L1-250
//! Keyword extraction with multiple strategies and caching.
//!
//! This module provides flexible keyword extraction using LLM, embedding-based,
//! or frequency-based approaches with automatic fallback and caching.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tracing::{info, warn};

/// Keyword extraction methods.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtractionMethod {
    /// LLM-based extraction
    Llm,
    /// Embedding-based extraction
    Embedding,
    /// Frequency-based extraction
    Frequency,
    /// Hybrid LLM and frequency approach
    Hybrid,
}

/// Result of keyword extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordExtractionResult {
    pub keywords: Vec<String>,
    pub phrases: Vec<String>,
    pub acronyms: HashMap<String, String>,
    pub tools: Vec<String>,
    pub commands: Vec<String>,
    pub method: ExtractionMethod,
    pub extraction_time_ms: f64,
    pub cache_hit: bool,
}

/// Extract keywords from documents using multiple strategies.
///
/// Supports LLM-based, embedding-based, and frequency-based extraction
/// with automatic fallback and content-hash caching.
pub struct KeywordExtractor {
    method: ExtractionMethod,
    model: String,
    cache_enabled: bool,
    cache_dir: PathBuf,
    fallback_enabled: bool,
}

impl KeywordExtractor {
    /// Initialize keyword extractor.
    ///
    /// # Arguments
    ///
    /// * `method` - Primary extraction method to use
    /// * `model` - LLM model name for Ollama
    /// * `cache_enabled` - Whether to cache extraction results
    /// * `cache_dir` - Directory for cache files
    /// * `fallback_enabled` - Whether to fallback on failure
    ///
    /// # Errors
    ///
    /// Returns error if cache directory cannot be created
    pub async fn new(
        method: ExtractionMethod,
        model: String,
        cache_enabled: bool,
        cache_dir: Option<PathBuf>,
        fallback_enabled: bool,
    ) -> Result<Self> {
        let cache_dir = cache_dir.unwrap_or_else(|| PathBuf::from(".cache/keywords"));

        if cache_enabled {
            fs::create_dir_all(&cache_dir)
                .await
                .context("Failed to create cache directory")?;
        }

        info!(
            "KeywordExtractor initialized: method={:?}, model={}, cache={}",
            method, model, cache_enabled
        );

        Ok(Self {
            method,
            model,
            cache_enabled,
            cache_dir,
            fallback_enabled,
        })
    }

    /// Extract keywords from content.
    ///
    /// # Arguments
    ///
    /// * `content` - Document content to analyze
    /// * `file_path` - Optional file path for logging
    ///
    /// # Returns
    ///
    /// Returns `KeywordExtractionResult` with extracted keywords
    ///
    /// # Errors
    ///
    /// Returns error if all extraction methods fail
    pub async fn extract(
        &self,
        content: &str,
        file_path: Option<&str>,
    ) -> Result<KeywordExtractionResult> {
        // Check cache first
        if self.cache_enabled {
            if let Some(cached) = self.get_cached_result(content).await? {
                return Ok(cached);
            }
        }

        // Try primary method
        match self.extract_with_method(content, self.method).await {
            Ok(result) => {
                self.cache_result(content, &result).await?;
                return Ok(result);
            }
            Err(e) => {
                if !self.fallback_enabled {
                    return Err(e).context("Extraction failed");
                }

                warn!(
                    "Primary method {:?} failed: {}, attempting fallback",
                    self.method, e
                );
            }
        }

        // Fallback chain
        let fallback_methods = self.get_fallback_methods();
        for method in fallback_methods {
            match self.extract_with_method(content, method).await {
                Ok(result) => {
                    self.cache_result(content, &result).await?;
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Fallback method {:?} failed: {}", method, e);
                }
            }
        }

        anyhow::bail!("All extraction methods failed")
    }

    /// Extract keywords using specified method.
    async fn extract_with_method(
        &self,
        content: &str,
        method: ExtractionMethod,
    ) -> Result<KeywordExtractionResult> {
        let start = Instant::now();

        let result = match method {
            ExtractionMethod::Llm => self.extract_llm(content).await?,
            ExtractionMethod::Embedding => self.extract_embedding(content).await?,
            ExtractionMethod::Frequency => self.extract_frequency(content).await?,
            ExtractionMethod::Hybrid => self.extract_hybrid(content).await?,
        };

        let extraction_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(KeywordExtractionResult {
            extraction_time_ms,
            cache_hit: false,
            ..result
        })
    }

    /// Extract keywords using LLM.
    async fn extract_llm(&self, content: &str) -> Result<KeywordExtractionResult> {
        // Implementation from prototype
        todo!("Implement LLM extraction")
    }

    /// Extract keywords using embedding-based approach.
    async fn extract_embedding(&self, content: &str) -> Result<KeywordExtractionResult> {
        // To be implemented
        todo!("Implement embedding-based extraction")
    }

    /// Extract keywords using frequency analysis.
    async fn extract_frequency(&self, content: &str) -> Result<KeywordExtractionResult> {
        // Use existing logic from document loader
        todo!("Implement frequency-based extraction")
    }

    /// Extract keywords using hybrid LLM and frequency approach.
    async fn extract_hybrid(&self, content: &str) -> Result<KeywordExtractionResult> {
        // To be implemented
        todo!("Implement hybrid extraction")
    }

    /// Get cached extraction result if available.
    async fn get_cached_result(
        &self,
        content: &str,
    ) -> Result<Option<KeywordExtractionResult>> {
        let content_hash = self.hash_content(content);
        let cache_file = self.cache_dir.join(format!("{}.json", content_hash));

        if !cache_file.exists() {
            return Ok(None);
        }

        match fs::read_to_string(&cache_file).await {
            Ok(data) => {
                let mut result: KeywordExtractionResult = serde_json::from_str(&data)
                    .context("Failed to parse cached result")?;
                result.cache_hit = true;
                Ok(Some(result))
            }
            Err(e) => {
                warn!("Cache read failed: {}", e);
                Ok(None)
            }
        }
    }

    /// Cache extraction result.
    async fn cache_result(
        &self,
        content: &str,
        result: &KeywordExtractionResult,
    ) -> Result<()> {
        if !self.cache_enabled {
            return Ok(());
        }

        let content_hash = self.hash_content(content);
        let cache_file = self.cache_dir.join(format!("{}.json", content_hash));

        let json = serde_json::to_string_pretty(result)
            .context("Failed to serialize result")?;

        if let Err(e) = fs::write(&cache_file, json).await {
            warn!("Cache write failed: {}", e);
        }

        Ok(())
    }

    /// Get ordered list of fallback methods.
    fn get_fallback_methods(&self) -> Vec<ExtractionMethod> {
        match self.method {
            ExtractionMethod::Llm => {
                vec![ExtractionMethod::Embedding, ExtractionMethod::Frequency]
            }
            ExtractionMethod::Embedding => vec![ExtractionMethod::Frequency],
            _ => vec![],
        }
    }

    /// Hash content for cache key.
    fn hash_content(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
```

**Deliverable**: `crates/core/src/keyword_extractor.rs`

**Success Criteria**:

- [ ] Module implements all extraction methods
- [ ] Caching works correctly (verified by tests)
- [ ] Fallback chain executes properly
- [ ] All functions have doc comments
- [ ] Code passes `cargo clippy --all-targets --all-features -- -D warnings`

#### Task 1.2: Add Configuration Support

**Effort**: 1 day

**Steps**:

1. Update `crates/core/src/config.rs`
2. Add `KeywordExtractionConfig` struct
3. Add environment variable support
4. Add validation logic

**Configuration Code**:

```xze/crates/core/src/config.rs#L1-80
/// Configuration for keyword extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordExtractionConfig {
    pub enabled: bool,
    pub method: String,
    pub model: String,
    pub fallback_enabled: bool,
    pub cache_enabled: bool,
    pub cache_dir: PathBuf,
    pub batch_size: usize,
    pub max_workers: usize,
}

impl Default for KeywordExtractionConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("KEYWORD_EXTRACTION_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            method: std::env::var("KEYWORD_EXTRACTION_METHOD")
                .unwrap_or_else(|_| "llm".to_string()),
            model: std::env::var("KEYWORD_EXTRACTION_MODEL")
                .unwrap_or_else(|_| "llama3.2:3b".to_string()),
            fallback_enabled: std::env::var("KEYWORD_EXTRACTION_FALLBACK")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            cache_enabled: std::env::var("KEYWORD_EXTRACTION_CACHE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            cache_dir: std::env::var("KEYWORD_EXTRACTION_CACHE_DIR")
                .unwrap_or_else(|_| ".cache/keywords".to_string())
                .into(),
            batch_size: std::env::var("KEYWORD_EXTRACTION_BATCH_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            max_workers: std::env::var("KEYWORD_EXTRACTION_MAX_WORKERS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
        }
    }
}

/// Main configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ... existing fields ...
    pub keyword_extraction: KeywordExtractionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            keyword_extraction: KeywordExtractionConfig::default(),
        }
    }
}
```

**Deliverable**: Updated `crates/core/src/config.rs`

**Success Criteria**: Configuration loads correctly from environment variables

#### Task 1.3: Write Comprehensive Tests

**Effort**: 2-3 days

**Steps**:

1. Create `crates/core/src/keyword_extractor/tests.rs`
2. Write unit tests for each extraction method
3. Write tests for caching logic
4. Write tests for fallback chain
5. Write tests for error handling
6. Achieve 80%+ code coverage for new module

**Test Structure**:

```xze/crates/core/src/keyword_extractor/tests.rs#L1-200
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_initialization_default_settings() {
        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "llama3.2:3b".to_string(),
            true,
            None,
            true,
        )
        .await
        .unwrap();

        assert_eq!(extractor.method, ExtractionMethod::Llm);
        assert!(extractor.cache_enabled);
        assert!(extractor.fallback_enabled);
    }

    #[tokio::test]
    async fn test_initialization_custom_settings() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let extractor = KeywordExtractor::new(
            ExtractionMethod::Frequency,
            "test-model".to_string(),
            false,
            Some(cache_dir.clone()),
            false,
        )
        .await
        .unwrap();

        assert_eq!(extractor.method, ExtractionMethod::Frequency);
        assert!(!extractor.cache_enabled);
        assert_eq!(extractor.cache_dir, cache_dir);
    }

    #[tokio::test]
    async fn test_llm_extraction_success() {
        // Mock LLM response would go here
        // This test requires mocking the Ollama client

        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "llama3.2:3b".to_string(),
            false,
            None,
            false,
        )
        .await
        .unwrap();

        // Test with actual extraction once implementation complete
    }

    #[tokio::test]
    async fn test_caching_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let extractor = KeywordExtractor::new(
            ExtractionMethod::Frequency,
            "test-model".to_string(),
            true,
            Some(cache_dir),
            true,
        )
        .await
        .unwrap();

        let content = "Test content for caching";

        // First extraction - cache miss
        let result1 = extractor.extract(content, None).await.unwrap();
        assert!(!result1.cache_hit);

        // Second extraction - cache hit
        let result2 = extractor.extract(content, None).await.unwrap();
        assert!(result2.cache_hit);
        assert_eq!(result1.keywords, result2.keywords);
    }

    #[tokio::test]
    async fn test_fallback_chain() {
        // Test that fallback to alternative methods works on failure
        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "nonexistent-model".to_string(),
            false,
            None,
            true,
        )
        .await
        .unwrap();

        // Should fallback to frequency-based
        let result = extractor.extract("Test content", None).await.unwrap();
        assert_eq!(result.method, ExtractionMethod::Frequency);
    }

    #[tokio::test]
    async fn test_extraction_failure_without_fallback() {
        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "nonexistent-model".to_string(),
            false,
            None,
            false,
        )
        .await
        .unwrap();

        let result = extractor.extract("Test content", None).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_content() {
        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "test".to_string(),
            false,
            None,
            false,
        )
        .await
        .unwrap();

        let hash1 = extractor.hash_content("test content");
        let hash2 = extractor.hash_content("test content");
        let hash3 = extractor.hash_content("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_get_fallback_methods() {
        let extractor = KeywordExtractor::new(
            ExtractionMethod::Llm,
            "test".to_string(),
            false,
            None,
            true,
        )
        .await
        .unwrap();

        let fallbacks = extractor.get_fallback_methods();
        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[0], ExtractionMethod::Embedding);
        assert_eq!(fallbacks[1], ExtractionMethod::Frequency);
    }
}
```

**Deliverable**: `crates/core/src/keyword_extractor/tests.rs` with 80%+ coverage

**Success Criteria**:

- [ ] All tests pass
- [ ] Code coverage 80%+
- [ ] Tests cover success paths, error paths, edge cases

#### Task 1.4: Performance Benchmarking

**Effort**: 1-2 days

**Steps**:

1. Create benchmark module using criterion
2. Test extraction speed for different content sizes
3. Test caching performance
4. Test batch processing performance
5. Document performance characteristics

**Benchmark Structure**:

```xze/crates/core/benches/keyword_extraction.rs#L1-100
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xze_core::keyword_extractor::{ExtractionMethod, KeywordExtractor};

fn benchmark_extraction_methods(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_content = "Sample technical document with various keywords";

    let mut group = c.benchmark_group("extraction_methods");

    for method in [
        ExtractionMethod::Llm,
        ExtractionMethod::Embedding,
        ExtractionMethod::Frequency,
    ] {
        group.bench_function(format!("{:?}", method), |b| {
            b.to_async(&rt).iter(|| async {
                let extractor = KeywordExtractor::new(
                    method,
                    "llama3.2:3b".to_string(),
                    false,
                    None,
                    false,
                )
                .await
                .unwrap();

                extractor
                    .extract(black_box(test_content), None)
                    .await
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn benchmark_caching(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let test_content = "Sample document for cache testing";

    c.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let extractor = KeywordExtractor::new(
                ExtractionMethod::Frequency,
                "test".to_string(),
                true,
                None,
                false,
            )
            .await
            .unwrap();

            extractor
                .extract(black_box(test_content), None)
                .await
                .unwrap()
        });
    });

    c.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let extractor = KeywordExtractor::new(
                ExtractionMethod::Frequency,
                "test".to_string(),
                true,
                None,
                false,
            )
            .await
            .unwrap();

            // Prime cache
            let _ = extractor.extract(test_content, None).await;

            // Benchmark cache hit
            extractor
                .extract(black_box(test_content), None)
                .await
                .unwrap()
        });
    });
}

criterion_group!(benches, benchmark_extraction_methods, benchmark_caching);
criterion_main!(benches);
```

**Deliverable**: Performance benchmark results in
`docs/reference/keyword_extraction_performance.md`

**Success Criteria**: LLM extraction averages <3s per document

#### Task 1.5: Phase 1 Review and Testing

**Effort**: 1 day

**Steps**:

1. Run full test suite
2. Run performance benchmarks
3. Code review with team
4. Update documentation
5. Create pull request

**PR Checklist**:

- [ ] All tests pass
- [ ] Code coverage maintained (80%+)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Docstrings complete
- [ ] Performance benchmarks acceptable
- [ ] Code reviewed and approved

**Commit Message**:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L900-920
feat(keyword): implement llm-based keyword extraction with caching (XZE-XXXX)

- Add KeywordExtractor struct with multiple strategies
- Implement LLM, embedding, and frequency-based extraction
- Add content-hash caching for performance
- Implement automatic fallback chain
- Add comprehensive unit tests (80% coverage)
- Add performance benchmarking with criterion
- Add configuration support via environment variables

Performance:
- LLM extraction: ~1-2s per document
- Cache hit: <1ms
- Fallback to frequency: ~50ms

Breaking changes: None (new module, no existing code modified)
```

## Phase 2: Integration and Testing (Week 5-6)

**Objective**: Integrate keyword extractor with document loading pipeline and
validate end-to-end functionality.

**Branch**: `pr-llm-keyword-integration-xxxx`

### Tasks

#### Task 2.1: Integrate with Document Loader

**Effort**: 2-3 days

**Steps**:

1. Update `DocumentMetadata` struct in document loader
2. Add flag to enable/disable LLM extraction
3. Integrate `KeywordExtractor` into metadata extraction
4. Preserve backward compatibility
5. Add integration tests

**Integration Code**:

```xze/crates/core/src/document_loader.rs#L1-120
use crate::keyword_extractor::{KeywordExtractor, KeywordExtractionResult};

/// Document metadata with optional LLM keyword extraction.
pub struct DocumentMetadata {
    file_path: String,
    content: String,
    keywords: Vec<String>,
    use_llm_keywords: bool,
}

impl DocumentMetadata {
    /// Initialize document metadata.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the markdown file
    /// * `content` - Raw markdown content
    /// * `use_llm_keywords` - Whether to use LLM for keyword extraction
    /// * `keyword_extractor` - Optional KeywordExtractor instance
    ///
    /// # Errors
    ///
    /// Returns error if keyword extraction fails
    pub async fn new(
        file_path: String,
        content: String,
        use_llm_keywords: bool,
        keyword_extractor: Option<&KeywordExtractor>,
    ) -> Result<Self> {
        // Extract keywords (LLM or frequency-based)
        let keywords = if use_llm_keywords && keyword_extractor.is_some() {
            Self::extract_keywords_llm(&content, keyword_extractor.unwrap()).await?
        } else {
            Self::extract_keywords_frequency(&content)?
        };

        Ok(Self {
            file_path,
            content,
            keywords,
            use_llm_keywords,
        })
    }

    /// Extract keywords using LLM-based extractor.
    async fn extract_keywords_llm(
        content: &str,
        extractor: &KeywordExtractor,
    ) -> Result<Vec<String>> {
        match extractor.extract(content, None).await {
            Ok(result) => {
                // Combine all keyword types into flat list
                let mut all_keywords = Vec::new();
                all_keywords.extend(result.keywords);
                all_keywords.extend(result.phrases);
                all_keywords.extend(result.tools);
                all_keywords.extend(result.commands);

                // Add acronym expansions
                for (acronym, expansion) in result.acronyms {
                    all_keywords.push(acronym.to_lowercase());
                    all_keywords.extend(
                        expansion
                            .to_lowercase()
                            .split_whitespace()
                            .map(String::from),
                    );
                }

                // Deduplicate and limit
                all_keywords.sort();
                all_keywords.dedup();
                all_keywords.truncate(50);

                Ok(all_keywords)
            }
            Err(e) => {
                warn!(
                    "LLM keyword extraction failed, falling back to frequency-based: {}",
                    e
                );
                Self::extract_keywords_frequency(content)
            }
        }
    }

    /// Extract keywords using frequency analysis (existing method).
    fn extract_keywords_frequency(content: &str) -> Result<Vec<String>> {
        // Existing frequency-based implementation
        todo!("Use existing frequency-based logic")
    }
}
```

**Deliverable**: Updated document loader with LLM integration

**Success Criteria**:

- [ ] LLM extraction integrates smoothly
- [ ] Backward compatibility maintained (default to frequency)
- [ ] Graceful fallback on LLM failure
- [ ] No breaking changes to API

#### Task 2.2: Add Batch Processing Support

**Effort**: 2 days

**Steps**:

1. Implement async batch processing in `KeywordExtractor`
2. Update document loading to use batch processing
3. Add parallel processing configuration
4. Test with large document sets (1000+ docs)

**Batch Processing Code**:

```xze/crates/core/src/keyword_extractor.rs#L300-400
use futures::stream::{self, StreamExt};

impl KeywordExtractor {
    /// Extract keywords from multiple documents in parallel batches.
    ///
    /// # Arguments
    ///
    /// * `contents` - List of document contents
    /// * `batch_size` - Number of documents per batch
    /// * `max_workers` - Maximum parallel workers
    ///
    /// # Returns
    ///
    /// Returns list of KeywordExtractionResult objects
    ///
    /// # Errors
    ///
    /// Returns error if extraction fails for any document
    pub async fn extract_batch(
        &self,
        contents: Vec<String>,
        batch_size: usize,
        max_workers: usize,
    ) -> Result<Vec<KeywordExtractionResult>> {
        let total = contents.len();
        let results = stream::iter(contents)
            .enumerate()
            .map(|(i, content)| async move {
                let result = self.extract(&content, None).await?;

                if i % batch_size == 0 {
                    info!("Processed batch {}, {}/{} documents complete",
                          i / batch_size + 1, i, total);
                }

                Ok::<_, anyhow::Error>(result)
            })
            .buffer_unordered(max_workers)
            .collect::<Vec<_>>()
            .await;

        results.into_iter().collect()
    }
}
```

**Deliverable**: Batch processing support in `KeywordExtractor`

**Success Criteria**:

- [ ] Batch processing 5-10x faster than serial
- [ ] Handles 1000+ documents without issues
- [ ] Resource usage stays within acceptable limits

#### Task 2.3: Database Schema Updates (if needed)

**Effort**: 1-2 days

**Steps**:

1. Assess if schema changes needed (likely not)
2. If needed, create migration script
3. Test migration on development database
4. Document migration procedure

**Note**: Current `keywords` column should accommodate LLM-extracted keywords
without schema changes. Verify column type supports array of strings.

**Deliverable**: Migration script (if needed) or confirmation that no migration
required

**Success Criteria**: Database can store LLM-extracted keywords

#### Task 2.4: Integration Testing

**Effort**: 2-3 days

**Steps**:

1. Create `crates/core/tests/integration_keyword_extraction.rs`
2. Test end-to-end document loading with LLM extraction
3. Test with different document types (tutorials, how_to, reference)
4. Test fallback scenarios
5. Test batch processing at scale

**Integration Test Structure**:

xze/crates/core/tests/integration_keyword_extraction.rs#L1-200

````rust
use xze_core::config::Config;
use xze_core::document_loader::DocumentMetadata;
use xze_core::keyword_extractor::{ExtractionMethod, KeywordExtractor};

#[tokio::test]
async fn test_document_loading_with_llm_keywords() {
    let extractor = KeywordExtractor::new(
        ExtractionMethod::Llm,
        "llama3.2:3b".to_string(),
        true,
        None,
        true,
    )
    .await
    .unwrap();

    let sample_content = r#"
# Snyk CLI Guide

Learn how to use the Snyk CLI for security scanning.

## Installation

Install Snyk CLI:

```bash
npm install -g snyk
```

## Running Scans

Run a SAST scan:

```bash
snyk code test
```

"#;

    let doc_metadata = DocumentMetadata::new(
        "test_doc.md".to_string(),
        sample_content.to_string(),
        true,
        Some(&extractor),
    )
    .await
    .unwrap();

    // Verify LLM extracted relevant keywords
    let keywords_lower: Vec<String> = doc_metadata
        .keywords
        .iter()
        .map(|k| k.to_lowercase())
        .collect();

    assert!(keywords_lower.contains(&"snyk".to_string()));
    assert!(keywords_lower.contains(&"cli".to_string()));
    assert!(keywords_lower.iter().any(|k| k.contains("scan")));

}

#[tokio::test]
async fn test_fallback_to_frequency_on_llm_failure() {
let extractor = KeywordExtractor::new(
ExtractionMethod::Llm,
"nonexistent-model".to_string(),
false,
None,
true,
)
.await
.unwrap();

    let content = "Sample content with some keywords";

    let doc_metadata = DocumentMetadata::new(
        "test_doc.md".to_string(),
        content.to_string(),
        true,
        Some(&extractor),
    )
    .await
    .unwrap();

    // Should still have keywords from frequency fallback
    assert!(!doc_metadata.keywords.is_empty());

}

#[tokio::test]
async fn test_batch_document_loading() {
let extractor = KeywordExtractor::new(
ExtractionMethod::Llm,
"llama3.2:3b".to_string(),
true,
None,
true,
)
.await
.unwrap();

    // Create 50 sample documents
    let documents: Vec<String> = (0..50)
        .map(|i| {
            format!(
                "# Document {}\n\nThis is test document {} about topic_{}.\nIt contains some technical content.",
                i, i, i
            )
        })
        .collect();

    // Batch extract keywords
    let results = extractor
        .extract_batch(documents, 10, 4)
        .await
        .unwrap();

    assert_eq!(results.len(), 50);
    assert!(results.iter().all(|r| !r.keywords.is_empty()));

}

#[tokio::test] #[ignore] // Mark as slow test
async fn test_large_scale_document_loading() {
use tempfile::TempDir;
use tokio::fs;

    let temp_dir = TempDir::new().unwrap();
    let docs_dir = temp_dir.path().join("test_docs");
    fs::create_dir(&docs_dir).await.unwrap();

    // Create 1000 test documents
    for i in 0..1000 {
        let doc_file = docs_dir.join(format!("doc_{}.md", i));
        let content = format!("# Document {}\n\nContent for doc {}", i, i);
        fs::write(&doc_file, content).await.unwrap();
    }

    // Load with LLM extraction
    // This test validates performance at scale
    // Should complete in reasonable time (< 30 min with caching)

}

````

**Deliverable**: Integration test suite with 95%+ pass rate

**Success Criteria**:

- [ ] All integration tests pass
- [ ] End-to-end workflow functions correctly
- [ ] Fallback mechanisms work as expected

#### Task 2.5: Phase 2 Review

**Effort**: 1 day

**Steps**:

1. Run full test suite (unit + integration)
2. Performance testing with 1000 documents
3. Code review
4. Update documentation
5. Create pull request

**PR Checklist**:

- [ ] All tests pass
- [ ] Integration with document loader complete
- [ ] Batch processing works correctly
- [ ] Documentation updated
- [ ] Code reviewed and approved

**Commit Message**:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1300-1320
feat(keyword): integrate llm extraction with document loading (XZE-XXXX)

- Integrate KeywordExtractor with DocumentMetadata
- Add batch processing for parallel extraction
- Maintain backward compatibility with feature flag
- Add comprehensive integration tests
- Test at scale with 1000+ documents

Performance at scale:
- 1000 docs: ~20 min initial load (with parallelization)
- 1000 docs: ~8 min reload (cache hits)
- Batch processing: 10x faster than serial

Breaking changes: None (feature flag defaults to disabled)
````

## Phase 3: Production Rollout (Week 7)

**Objective**: Deploy to production with gradual rollout, A/B testing, and
monitoring.

**Branch**: `pr-llm-keyword-production-xxxx`

### Tasks

#### Task 3.1: Add Feature Flag and Configuration

**Effort**: 1 day

**Steps**:

1. Add feature flag for gradual rollout
2. Add configuration for A/B testing
3. Add monitoring and metrics collection
4. Document configuration options

**Feature Flag Implementation**:

```xze/crates/core/src/config.rs#L80-120
/// Configuration for keyword extraction with rollout support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordExtractionConfig {
    // Existing fields...
    pub rollout_percentage: u8,
    pub ab_test_enabled: bool,
    pub metrics_enabled: bool,
}

impl Default for KeywordExtractionConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            rollout_percentage: std::env::var("KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            ab_test_enabled: std::env::var("KEYWORD_EXTRACTION_AB_TEST")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            metrics_enabled: std::env::var("KEYWORD_EXTRACTION_METRICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        }
    }
}
```

**Rollout Logic**:

```xze/crates/core/src/document_loader.rs#L150-180
use rand::Rng;

/// Determine if LLM extraction should be used based on rollout percentage.
///
/// # Arguments
///
/// * `config` - Keyword extraction configuration
///
/// # Returns
///
/// Returns true if LLM extraction should be used, false otherwise
pub fn should_use_llm_extraction(config: &KeywordExtractionConfig) -> bool {
    if !config.enabled {
        return false;
    }

    if config.rollout_percentage >= 100 {
        return true;
    }

    if config.rollout_percentage == 0 {
        return false;
    }

    // Random selection based on percentage
    let mut rng = rand::thread_rng();
    rng.gen_range(0..100) < config.rollout_percentage
}
```

**Deliverable**: Feature flag and rollout configuration

**Success Criteria**: Can control LLM usage via environment variable

#### Task 3.2: Add Monitoring and Metrics

**Effort**: 2 days

**Steps**:

1. Add extraction time metrics
2. Add cache hit rate metrics
3. Add fallback rate metrics
4. Add error rate metrics
5. Create monitoring dashboard (if applicable)

**Metrics Collection**:

```xze/crates/core/src/keyword_extractor.rs#L400-550
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics for keyword extraction.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExtractionMetrics {
    pub total_extractions: usize,
    pub llm_extractions: usize,
    pub embedding_extractions: usize,
    pub frequency_extractions: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub fallback_count: usize,
    pub error_count: usize,
    pub total_time_ms: f64,
}

impl ExtractionMetrics {
    /// Calculate cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average extraction time.
    pub fn avg_extraction_time_ms(&self) -> f64 {
        if self.total_extractions > 0 {
            self.total_time_ms / self.total_extractions as f64
        } else {
            0.0
        }
    }

    /// Convert metrics to JSON-serializable format.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_extractions": self.total_extractions,
            "method_breakdown": {
                "llm": self.llm_extractions,
                "embedding": self.embedding_extractions,
                "frequency": self.frequency_extractions,
            },
            "cache_hit_rate": format!("{:.1}%", self.cache_hit_rate()),
            "cache_hits": self.cache_hits,
            "cache_misses": self.cache_misses,
            "fallback_count": self.fallback_count,
            "error_count": self.error_count,
            "avg_extraction_time_ms": format!("{:.1}", self.avg_extraction_time_ms()),
            "total_time_ms": format!("{:.1}", self.total_time_ms),
        })
    }
}

impl KeywordExtractor {
    pub fn new_with_metrics(
        method: ExtractionMethod,
        model: String,
        cache_enabled: bool,
        cache_dir: Option<PathBuf>,
        fallback_enabled: bool,
    ) -> Result<Self> {
        // ... existing initialization ...
        Ok(Self {
            // ... existing fields ...
            metrics: Arc::new(RwLock::new(ExtractionMetrics::default())),
        })
    }

    /// Get current metrics.
    pub async fn get_metrics(&self) -> ExtractionMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset metrics counters.
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ExtractionMetrics::default();
    }
}
```

**Deliverable**: Metrics collection and reporting

**Success Criteria**: Can monitor extraction performance in real-time

#### Task 3.3: Create A/B Testing Framework

**Effort**: 2 days

**Steps**:

1. Implement document-level A/B testing
2. Track which documents use LLM vs. frequency
3. Collect search quality metrics per group
4. Create analysis scripts

**A/B Testing Implementation**:

```xze/examples/ab_test_keyword_extraction.rs#L1-150
//! A/B testing framework for keyword extraction methods.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use xze_core::keyword_extractor::ExtractionMethod;

/// A/B test for keyword extraction methods.
#[derive(Debug)]
pub struct KeywordExtractionABTest {
    control_method: ExtractionMethod,
    treatment_method: ExtractionMethod,
    split_ratio: f64,
    assignments: HashMap<String, ExtractionMethod>,
}

impl KeywordExtractionABTest {
    /// Initialize A/B test.
    ///
    /// # Arguments
    ///
    /// * `control_method` - Control group extraction method
    /// * `treatment_method` - Treatment group extraction method
    /// * `split_ratio` - Percentage of documents in treatment group (0.0-1.0)
    pub fn new(
        control_method: ExtractionMethod,
        treatment_method: ExtractionMethod,
        split_ratio: f64,
    ) -> Self {
        Self {
            control_method,
            treatment_method,
            split_ratio,
            assignments: HashMap::new(),
        }
    }

    /// Assign document to control or treatment group.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to document
    ///
    /// # Returns
    ///
    /// Returns ExtractionMethod for this document
    pub fn assign_group(&mut self, file_path: &str) -> ExtractionMethod {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        if let Some(&method) = self.assignments.get(file_path) {
            return method;
        }

        // Use consistent assignment based on file path hash
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        let hash = hasher.finish();

        let in_treatment = (hash % 100) < (self.split_ratio * 100.0) as u64;
        let method = if in_treatment {
            self.treatment_method
        } else {
            self.control_method
        };

        self.assignments.insert(file_path.to_string(), method);
        method
    }

    /// Save A/B test assignments to file.
    ///
    /// # Errors
    ///
    /// Returns error if file write fails
    pub async fn save_assignments(&self, output_file: &Path) -> Result<()> {
        #[derive(Serialize)]
        struct AssignmentData {
            control_method: String,
            treatment_method: String,
            split_ratio: f64,
            assignments: HashMap<String, String>,
        }

        let data = AssignmentData {
            control_method: format!("{:?}", self.control_method),
            treatment_method: format!("{:?}", self.treatment_method),
            split_ratio: self.split_ratio,
            assignments: self
                .assignments
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
        };

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(output_file, json).await?;

        Ok(())
    }
}
```

**Deliverable**: A/B testing framework

**Success Criteria**: Can split documents 50/50 and track results

#### Task 3.4: Staged Rollout Plan

**Effort**: Execution over 2-3 weeks

**Rollout Stages**:

**Stage 1: Internal Testing (Week 7, Days 1-2)**

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1600-1610
# Enable for 0% (manual testing only)
export KEYWORD_EXTRACTION_ENABLED=true
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0
export KEYWORD_EXTRACTION_METHOD=llm
export KEYWORD_EXTRACTION_CACHE_ENABLED=true
```

- Manually test with sample documents
- Verify monitoring and metrics
- Ensure no errors in logs
- **Decision gate**: All systems working correctly

**Stage 2: Canary (10%) (Week 7, Days 3-4)**

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1620-1625
# Enable for 10% of documents
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=10
```

- Monitor extraction times
- Monitor cache hit rates
- Monitor error rates
- Check search quality metrics
- **Decision gate**: Error rate <1%, performance acceptable

**Stage 3: Limited Rollout (25%) (Week 7, Days 5-7)**

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1635-1640
# Enable for 25% of documents
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=25
```

- Continue monitoring
- Collect user feedback (if applicable)
- Compare search quality: 25% LLM vs. 75% frequency
- **Decision gate**: Search quality improved by 10%+

**Stage 4: Majority Rollout (50%) (Week 8)**

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1650-1655
# Enable for 50% of documents (A/B test)
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=50
export KEYWORD_EXTRACTION_AB_TEST=true
```

- Run formal A/B test
- Collect comprehensive metrics
- Statistical analysis of results
- **Decision gate**: Search quality improved by 15%+, p-value <0.05

**Stage 5: Full Rollout (100%) (Week 8+)**

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1665-1670
# Enable for 100% of documents
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=100
```

- Complete transition to LLM extraction
- Continue monitoring for 1 week
- Declare production stable
- **Decision gate**: No regressions, metrics stable

**Deliverable**: Staged rollout execution

**Success Criteria**: Successful progression through all stages without rollback

#### Task 3.5: Documentation and Training

**Effort**: 2 days

**Steps**:

1. Update user documentation
2. Create operator guide
3. Create troubleshooting guide
4. Document configuration options
5. Create migration guide (if users need to reload documents)

**Documentation Files**:

- `docs/reference/keyword_extraction_configuration.md` - All config options
- `docs/how_to/configure_llm_keyword_extraction.md` - Setup guide
- `docs/how_to/troubleshoot_keyword_extraction.md` - Troubleshooting
- `docs/explanation/keyword_extraction_architecture.md` - How it works

**Deliverable**: Complete documentation suite

**Success Criteria**: Operators can configure and troubleshoot without
assistance

#### Task 3.6: Rollback Procedure

**Effort**: 1 day (documentation and testing)

**Rollback Steps**:

If LLM extraction causes issues, rollback is simple:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1720-1730
# Immediate rollback - disable LLM extraction
export KEYWORD_EXTRACTION_ENABLED=false
# OR
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Restart services to pick up new config
# Documents will use frequency-based extraction
systemctl restart xze-server
```

**Complete Rollback**:

If code needs to be reverted:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1740-1755
# Revert to previous release
git revert <commit-hash>

# OR checkout previous tag
git checkout tags/v1.x.x

# Rebuild and redeploy
cargo build --release
make deploy
```

**Data Rollback**:

If documents were loaded with LLM keywords and need to be reverted:

```xze/docs/explanation/implement_llm_keyword_extraction.md#L1765-1775
# Reload documents with frequency-based extraction
export KEYWORD_EXTRACTION_ENABLED=false
cargo run --bin xze -- load --enhanced data/
```

**Deliverable**: Documented rollback procedure in
`docs/how_to/rollback_llm_keyword_extraction.md`

**Success Criteria**: Can rollback in <5 minutes without data loss

## Phase 4: Optimization and Monitoring (Week 8-9)

**Objective**: Optimize performance, refine prompts, and establish long-term
monitoring.

### Tasks

#### Task 4.1: Prompt Engineering Optimization

**Effort**: 3-4 days

**Steps**:

1. Analyze keyword extraction quality across document types
2. Refine LLM prompts for better results
3. Test different prompt variations
4. Measure impact on search quality
5. Document best prompts

**Prompt Variations to Test**:

```xze/crates/core/src/keyword_extractor.rs#L600-650
// Variation 1: More specific technical focus
const PROMPT_V1: &str = r#"Extract technical keywords from this documentation.
Focus on: tools, APIs, commands, technologies, methodologies.
Return JSON with keywords, phrases, acronyms, tools, commands."#;

// Variation 2: Domain-aware
const PROMPT_V2: &str = r#"Analyze this technical document and extract search keywords.
Identify: product names, command syntax, technical concepts, acronyms.
Prioritize domain-specific terminology over common words."#;

// Variation 3: Structured with examples
const PROMPT_V3: &str = r#"Extract keywords for search indexing.
Examples:
- Keywords: cli, api, docker, kubernetes
- Phrases: continuous integration, version control
- Tools: Git, Jenkins, Snyk
Return JSON matching this structure."#;
```

**Deliverable**: Optimized prompts documented in
`docs/reference/llm_prompt_templates.md`

**Success Criteria**: 5%+ improvement in keyword relevance

#### Task 4.2: Performance Optimization

**Effort**: 2-3 days

**Steps**:

1. Profile extraction performance
2. Optimize slow operations
3. Tune batch sizes and parallelization
4. Optimize cache strategy
5. Benchmark improvements

**Optimization Areas**:

- Cache warming strategy
- Optimal batch size for different document sizes
- Connection pooling for Ollama
- Memory optimization for large document sets

**Deliverable**: Performance optimization report

**Success Criteria**: 20%+ reduction in average extraction time

#### Task 4.3: Monitoring Dashboard

**Effort**: 2 days

**Steps**:

1. Create monitoring dashboard (if infrastructure supports)
2. Set up alerts for key metrics
3. Document monitoring procedures
4. Create runbook for common issues

**Key Metrics to Monitor**:

- Extraction time (p50, p95, p99)
- Cache hit rate
- Error rate
- Fallback rate
- Search quality metrics (Precision@5, Recall@10)

**Alert Thresholds**:

- Error rate >5%: Warning
- Error rate >10%: Critical
- Average extraction time >5s: Warning
- Cache hit rate <80%: Warning

**Deliverable**: Monitoring dashboard and alerts

**Success Criteria**: Can detect and respond to issues within 5 minutes

#### Task 4.4: Long-term Validation

**Effort**: Ongoing (2+ weeks)

**Steps**:

1. Monitor production metrics daily
2. Collect user feedback
3. Analyze search quality trends
4. Identify and address edge cases
5. Document lessons learned

**Validation Criteria**:

- [ ] Error rate consistently <1%
- [ ] Search quality improved by 15%+
- [ ] Cache hit rate >90%
- [ ] Average extraction time <2s
- [ ] No user complaints about search quality degradation

**Deliverable**: Production validation report

**Success Criteria**: All validation criteria met for 2+ weeks

## Success Metrics and KPIs

### Technical Metrics

| Metric                | Baseline | Target  | Measurement |
| --------------------- | -------- | ------- | ----------- |
| Precision@5           | 60%      | 80%+    | Weekly      |
| Recall@10             | 70%      | 90%+    | Weekly      |
| MRR                   | 0.65     | 0.85+   | Weekly      |
| Zero-result queries   | 15%      | <5%     | Daily       |
| Extraction time (avg) | 50ms     | <2000ms | Real-time   |
| Cache hit rate        | N/A      | >90%    | Real-time   |
| Error rate            | N/A      | <1%     | Real-time   |

### Business Metrics

| Metric                     | Target                        |
| -------------------------- | ----------------------------- |
| Time to support new domain | <1 hour (from days)           |
| Code maintenance burden    | Zero (from quarterly updates) |
| User satisfaction          | 80%+ "Good" or better         |

## Risk Management

### Critical Risks

**Risk 1: Performance Unacceptable**

- **Probability**: Low
- **Impact**: High
- **Mitigation**: Caching, batch processing, fallback to frequency
- **Contingency**: Adjust rollout percentage, optimize prompts

**Risk 2: Search Quality Not Improved**

- **Probability**: Low (validated in Phase 0)
- **Impact**: High
- **Mitigation**: A/B testing, prompt optimization, hybrid approach
- **Contingency**: Rollback to frequency-based, iterate on prompts

**Risk 3: LLM Service Reliability**

- **Probability**: Medium
- **Impact**: Medium
- **Mitigation**: Automatic fallback, retry logic, monitoring
- **Contingency**: Disable LLM extraction, use frequency fallback

**Risk 4: Unexpected Edge Cases**

- **Probability**: Medium
- **Impact**: Low
- **Mitigation**: Comprehensive testing, error handling, monitoring
- **Contingency**: Fix issues incrementally, maintain fallback

## Timeline and Resource Allocation

### Overall Timeline: 8-9 Weeks

| Phase                          | Duration      | Effort (person-days) |
| ------------------------------ | ------------- | -------------------- |
| Phase 0: Research & Validation | 2 weeks       | 7 days               |
| Phase 1: Core Implementation   | 2 weeks       | 12 days              |
| Phase 2: Integration & Testing | 2 weeks       | 12 days              |
| Phase 3: Production Rollout    | 1 week        | 8 days               |
| Phase 4: Optimization          | 1-2 weeks     | 10 days              |
| **Total**                      | **8-9 weeks** | **49 days**          |

### Resource Requirements

- **1 Senior Rust Developer**: Core implementation, integration
- **1 ML Engineer**: Prompt optimization, LLM tuning
- **1 QA Engineer**: Testing, validation (part-time)
- **1 DevOps Engineer**: Deployment, monitoring (part-time)

## Decision Gates and Go/No-Go Criteria

### Phase 0 to Phase 1

- [ ] LLM extraction success rate >95%
- [ ] Search quality improvement >15%
- [ ] Extraction time <3s average
- [ ] Stakeholder approval

### Phase 1 to Phase 2

- [ ] All unit tests pass
- [ ] Code coverage >80%
- [ ] Performance benchmarks acceptable
- [ ] Code review approved

### Phase 2 to Phase 3

- [ ] Integration tests pass
- [ ] End-to-end workflow validated
- [ ] Batch processing verified at scale
- [ ] Documentation complete

### Phase 3 to Phase 4

- [ ] Canary rollout successful (error rate <1%)
- [ ] A/B test shows >15% improvement (p<0.05)
- [ ] No critical issues in production
- [ ] User feedback positive

### Phase 4 to Production Stable

- [ ] All validation criteria met for 2+ weeks
- [ ] Error rate consistently <1%
- [ ] Cache hit rate >90%
- [ ] Search quality sustained
- [ ] Operations team trained

## Rollback Procedures

### Immediate Rollback (Environment Variable)

```xze/docs/explanation/implement_llm_keyword_extraction.md#L2000-2010
# Stop LLM extraction immediately
export KEYWORD_EXTRACTION_ENABLED=false

# Or reduce rollout percentage
export KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE=0

# Restart services
systemctl restart xze-server
```

**Time to rollback**: <1 minute

### Code Rollback (Git Revert)

```xze/docs/explanation/implement_llm_keyword_extraction.md#L2015-2025
# Identify commit to revert
git log --oneline | grep "keyword"

# Revert specific commits
git revert <commit-hash>

# Push and deploy
git push origin main
cargo build --release
make deploy
```

**Time to rollback**: <10 minutes

### Data Rollback (Reload Documents)

```xze/docs/explanation/implement_llm_keyword_extraction.md#L2030-2040
# Disable LLM extraction
export KEYWORD_EXTRACTION_ENABLED=false

# Reload all documents with frequency-based extraction
cargo run --bin xze -- load --enhanced data/

# Verify
cargo run --bin xze -- verify data/
```

**Time to rollback**: Depends on document count (20-30 min for 10k docs)

## Communication Plan

### Internal Team

- **Daily standups**: Progress updates during implementation
- **Weekly demos**: Show progress to stakeholders
- **Slack updates**: Real-time issues and resolutions
- **Retrospectives**: After each phase completion

### Stakeholders

- **Phase 0 results**: Email summary with decision recommendation
- **Phase 3 rollout**: Weekly status updates
- **Phase 4 completion**: Final results presentation

### Users (if applicable)

- **Pre-rollout**: Announcement of upcoming improvements
- **During rollout**: Feedback collection mechanism
- **Post-rollout**: Results summary and thank you

## Post-Implementation

### Maintenance

- **Weekly**: Review metrics, address issues
- **Monthly**: Analyze search quality trends
- **Quarterly**: Optimize prompts based on new document types
- **Annually**: Evaluate new LLM models for improvement

### Continuous Improvement

1. Collect feedback on keyword quality
2. Identify document types needing better extraction
3. Refine prompts for specific domains
4. Consider custom fine-tuned models (future)
5. Explore multi-lingual support (future)

## Lessons Learned Documentation

After completion, document:

- What went well
- What could be improved
- Unexpected challenges
- Best practices discovered
- Recommendations for future similar projects

## Conclusion

This phased implementation plan provides a safe, validated path to production
for LLM-based keyword extraction. The approach emphasizes:

1. **Validation before investment**: Phase 0 proves value before full
   implementation
2. **Incremental rollout**: Gradual deployment minimizes risk
3. **Comprehensive testing**: Multiple test layers ensure quality
4. **Monitoring and metrics**: Data-driven decision making
5. **Rollback capability**: Quick recovery if issues arise

**Expected Outcomes**:

- 15-30% improvement in search quality
- Zero ongoing maintenance for new domains
- Automatic support for any documentation type
- Production-stable system within 8-9 weeks

**Next Steps**:

1. Review and approve this plan
2. Create JIRA epic and tasks
3. Assign resources
4. Begin Phase 0: Research and Validation

---

**Document History**:

- 2025-01-30: Initial implementation plan created
- 2025-01-31: Refactored for Rust implementation in XZe project
- Status: Ready for review and approval

**Related Documents**:

- `docs/explanation/model_based_keyword_extraction_proposal.md` - Model-Based Keyword Extraction Proposal
- `docs/explanation/keyword_extraction_analysis.md` - Keyword Extraction Analysis
- `docs/explanation/improve_search_results.md` - Search Result Improvements
