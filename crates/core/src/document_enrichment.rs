//! Document enrichment module for adding LLM-extracted keywords to chunks
//!
//! This module provides integration between the keyword extractor and the semantic
//! chunking system, enriching chunk metadata with high-quality LLM-extracted keywords.
//!
//! # Architecture
//!
//! The enrichment process:
//! 1. Takes semantic chunks from the chunker
//! 2. Extracts keywords using LLM (with caching and fallback)
//! 3. Enriches chunk metadata with extracted keywords
//! 4. Maintains backward compatibility with frequency-based keywords
//!
//! # Examples
//!
//! ```
//! use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
//! use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
//!
//! # async fn example() -> xze_core::Result<()> {
//! let config = EnrichmentConfig::default();
//! let enricher = DocumentEnricher::new(config).await?;
//!
//! let mut chunk = SemanticChunk::new(
//!     "Rust is a systems programming language.".to_string(),
//!     0,
//!     1,
//!     0,
//!     0,
//!     1.0,
//!     ChunkMetadata::new("test.md".to_string(), "content"),
//! );
//!
//! enricher.enrich_chunk(&mut chunk).await?;
//! assert!(!chunk.metadata.keywords.is_empty());
//! # Ok(())
//! # }
//! ```

use crate::{
    error::{Result, XzeError},
    keyword_extractor::{ExtractedKeywords, KeywordExtractor, KeywordExtractorConfig},
    semantic::types::SemanticChunk,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Configuration for document enrichment
///
/// Controls how chunks are enriched with keywords and provides
/// feature flags for enabling/disabling LLM extraction.
///
/// # Examples
///
/// ```
/// use xze_core::document_enrichment::EnrichmentConfig;
///
/// let config = EnrichmentConfig::default();
/// assert!(config.use_llm_keywords);
/// assert_eq!(config.max_keywords_per_chunk, 15);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentConfig {
    /// Whether to use LLM-based keyword extraction
    pub use_llm_keywords: bool,

    /// Maximum total keywords to add to each chunk
    pub max_keywords_per_chunk: usize,

    /// Whether to gracefully fallback to frequency-based extraction on LLM failure
    pub fallback_on_error: bool,

    /// Minimum content length to trigger LLM extraction (chars)
    pub min_content_length: usize,

    /// Keyword extractor configuration
    pub extractor_config: KeywordExtractorConfig,

    /// Number of parallel workers for batch enrichment
    pub batch_workers: usize,

    /// Batch size for processing multiple chunks
    pub batch_size: usize,

    /// Rollout percentage (0-100) for gradual LLM adoption
    pub rollout_percentage: u8,

    /// Whether to enable A/B testing mode
    pub ab_test_enabled: bool,

    /// Whether to collect and expose metrics
    pub metrics_enabled: bool,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            use_llm_keywords: true,
            max_keywords_per_chunk: 15,
            fallback_on_error: true,
            min_content_length: 50,
            extractor_config: KeywordExtractorConfig::default(),
            batch_workers: 4,
            batch_size: 10,
            rollout_percentage: Self::default_rollout_percentage(),
            ab_test_enabled: Self::default_ab_test_enabled(),
            metrics_enabled: Self::default_metrics_enabled(),
        }
    }
}

impl EnrichmentConfig {
    /// Gets rollout percentage from environment or defaults to 0
    fn default_rollout_percentage() -> u8 {
        std::env::var("XZE_KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Gets A/B test flag from environment or defaults to false
    fn default_ab_test_enabled() -> bool {
        std::env::var("XZE_KEYWORD_EXTRACTION_AB_TEST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false)
    }

    /// Gets metrics flag from environment or defaults to true
    fn default_metrics_enabled() -> bool {
        std::env::var("XZE_KEYWORD_EXTRACTION_METRICS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true)
    }

    /// Determines if LLM extraction should be used for this invocation
    ///
    /// Takes into account the rollout percentage using deterministic selection.
    ///
    /// # Arguments
    ///
    /// * `seed` - A seed value for deterministic selection (e.g., document hash)
    ///
    /// # Returns
    ///
    /// Returns true if LLM extraction should be used, false otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentConfig;
    ///
    /// let mut config = EnrichmentConfig::default();
    /// config.rollout_percentage = 50;
    ///
    /// // Deterministic selection based on seed
    /// let use_llm = config.should_use_llm_extraction(12345);
    /// ```
    pub fn should_use_llm_extraction(&self, seed: u64) -> bool {
        if !self.use_llm_keywords {
            return false;
        }

        if self.rollout_percentage >= 100 {
            return true;
        }

        if self.rollout_percentage == 0 {
            return false;
        }

        // Deterministic selection based on seed
        (seed % 100) < self.rollout_percentage as u64
    }
}

impl EnrichmentConfig {
    /// Creates configuration with LLM extraction disabled
    ///
    /// Useful for backward compatibility or when LLM is unavailable.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentConfig;
    ///
    /// let config = EnrichmentConfig::frequency_only();
    /// assert!(!config.use_llm_keywords);
    /// ```
    pub fn frequency_only() -> Self {
        Self {
            use_llm_keywords: false,
            ..Default::default()
        }
    }

    /// Creates configuration optimized for large document sets
    ///
    /// Uses larger batch sizes and more workers for parallel processing.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentConfig;
    ///
    /// let config = EnrichmentConfig::high_throughput();
    /// assert_eq!(config.batch_workers, 8);
    /// assert_eq!(config.batch_size, 20);
    /// ```
    pub fn high_throughput() -> Self {
        Self {
            batch_workers: 8,
            batch_size: 20,
            ..Default::default()
        }
    }

    /// Validates the configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration values are invalid
    pub fn validate(&self) -> Result<()> {
        if self.max_keywords_per_chunk == 0 {
            return Err(XzeError::validation(
                "max_keywords_per_chunk must be greater than 0",
            ));
        }
        if self.batch_workers == 0 {
            return Err(XzeError::validation("batch_workers must be greater than 0"));
        }
        if self.batch_size == 0 {
            return Err(XzeError::validation("batch_size must be greater than 0"));
        }
        Ok(())
    }
}

/// Enrichment statistics for monitoring and debugging
///
/// Tracks how many chunks were enriched, cache hits, and method breakdown.
///
/// # Examples
///
/// ```
/// use xze_core::document_enrichment::EnrichmentStats;
///
/// let stats = EnrichmentStats::default();
/// assert_eq!(stats.total_enriched, 0);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnrichmentStats {
    /// Total chunks enriched
    pub total_enriched: usize,

    /// Chunks enriched using LLM
    pub llm_enriched: usize,

    /// Chunks enriched using frequency fallback
    pub frequency_enriched: usize,

    /// Chunks enriched from cache
    pub cached_enriched: usize,

    /// Number of enrichment failures
    pub failures: usize,

    /// Average keywords per chunk
    pub avg_keywords_per_chunk: f64,

    /// Total processing time in milliseconds
    pub total_processing_ms: u64,

    /// Cache hits (for metrics)
    pub cache_hits: usize,

    /// Cache misses (for metrics)
    pub cache_misses: usize,

    /// Total errors encountered
    pub error_count: usize,
}

impl EnrichmentStats {
    /// Updates statistics after enriching a chunk
    fn record_enrichment(&mut self, keywords: &ExtractedKeywords, processing_ms: u64) {
        self.total_enriched += 1;
        self.total_processing_ms += processing_ms;

        match keywords.extraction_method.as_str() {
            "llm" => {
                self.llm_enriched += 1;
                self.cache_misses += 1;
            }
            "frequency" => {
                self.frequency_enriched += 1;
                self.cache_misses += 1;
            }
            "cached" => {
                self.cached_enriched += 1;
                self.cache_hits += 1;
            }
            _ => {}
        }

        let total_keywords = keywords.descriptive.len() + keywords.technical.len();
        let new_avg = (self.avg_keywords_per_chunk * (self.total_enriched - 1) as f64
            + total_keywords as f64)
            / self.total_enriched as f64;
        self.avg_keywords_per_chunk = new_avg;
    }

    /// Records a failed enrichment
    fn record_failure(&mut self) {
        self.failures += 1;
        self.error_count += 1;
    }

    /// Resets all statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Calculate cache hit rate as percentage
    ///
    /// # Returns
    ///
    /// Returns cache hit rate as percentage (0.0-100.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentStats;
    ///
    /// let mut stats = EnrichmentStats::default();
    /// assert_eq!(stats.cache_hit_rate(), 0.0);
    /// ```
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average extraction time in milliseconds
    ///
    /// # Returns
    ///
    /// Returns average extraction time in ms
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentStats;
    ///
    /// let mut stats = EnrichmentStats::default();
    /// assert_eq!(stats.avg_extraction_time_ms(), 0.0);
    /// ```
    pub fn avg_extraction_time_ms(&self) -> f64 {
        if self.total_enriched > 0 {
            self.total_processing_ms as f64 / self.total_enriched as f64
        } else {
            0.0
        }
    }

    /// Calculate error rate as percentage
    ///
    /// # Returns
    ///
    /// Returns error rate as percentage (0.0-100.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentStats;
    ///
    /// let mut stats = EnrichmentStats::default();
    /// assert_eq!(stats.error_rate(), 0.0);
    /// ```
    pub fn error_rate(&self) -> f64 {
        let total_attempts = self.total_enriched + self.error_count;
        if total_attempts > 0 {
            (self.error_count as f64 / total_attempts as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Convert metrics to JSON-serializable format
    ///
    /// # Returns
    ///
    /// Returns JSON representation of metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::EnrichmentStats;
    ///
    /// let stats = EnrichmentStats::default();
    /// let json = stats.to_json();
    /// assert!(json.is_object());
    /// ```
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_enriched": self.total_enriched,
            "method_breakdown": {
                "llm": self.llm_enriched,
                "frequency": self.frequency_enriched,
                "cached": self.cached_enriched,
            },
            "cache_hit_rate": format!("{:.1}%", self.cache_hit_rate()),
            "cache_hits": self.cache_hits,
            "cache_misses": self.cache_misses,
            "error_count": self.error_count,
            "error_rate": format!("{:.2}%", self.error_rate()),
            "failures": self.failures,
            "avg_keywords_per_chunk": format!("{:.1}", self.avg_keywords_per_chunk),
            "avg_extraction_time_ms": format!("{:.1}", self.avg_extraction_time_ms()),
            "total_processing_ms": self.total_processing_ms,
        })
    }
}

/// Document enricher for adding keywords to semantic chunks
///
/// Integrates with KeywordExtractor to add high-quality keywords
/// to chunk metadata, with caching and fallback support.
///
/// # Examples
///
/// ```
/// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
///
/// # async fn example() -> xze_core::Result<()> {
/// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
/// let stats = enricher.stats().await;
/// println!("Enriched {} chunks", stats.total_enriched);
/// # Ok(())
/// # }
/// ```
pub struct DocumentEnricher {
    config: EnrichmentConfig,
    extractor: Option<KeywordExtractor>,
    stats: Arc<RwLock<EnrichmentStats>>,
}

impl DocumentEnricher {
    /// Creates a new document enricher
    ///
    /// # Arguments
    ///
    /// * `config` - Enrichment configuration
    ///
    /// # Returns
    ///
    /// Returns a new DocumentEnricher instance
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid or keyword extractor cannot be created
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: EnrichmentConfig) -> Result<Self> {
        config.validate()?;

        let extractor = if config.use_llm_keywords {
            Some(KeywordExtractor::new(config.extractor_config.clone())?)
        } else {
            None
        };

        Ok(Self {
            config,
            extractor,
            stats: Arc::new(RwLock::new(EnrichmentStats::default())),
        })
    }

    /// Enriches a single chunk with keywords
    ///
    /// Updates the chunk's metadata in-place with extracted keywords.
    ///
    /// # Arguments
    ///
    /// * `chunk` - Mutable reference to the chunk to enrich
    ///
    /// # Returns
    ///
    /// Returns the extraction method used (llm, frequency, or cached)
    ///
    /// # Errors
    ///
    /// Returns error if extraction fails and fallback is disabled
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    /// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    /// let mut chunk = SemanticChunk::new(
    ///     "Content about Rust programming.".to_string(),
    ///     0, 1, 0, 0, 1.0,
    ///     ChunkMetadata::new("test.md".to_string(), "content"),
    /// );
    ///
    /// let method = enricher.enrich_chunk(&mut chunk).await?;
    /// assert!(!chunk.metadata.keywords.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enrich_chunk(&self, chunk: &mut SemanticChunk) -> Result<String> {
        let start = std::time::Instant::now();

        // Skip enrichment if content is too short
        if chunk.content.len() < self.config.min_content_length {
            debug!(
                "Skipping enrichment for chunk {} (content too short: {} chars)",
                chunk.chunk_index,
                chunk.content.len()
            );
            return Ok("skipped".to_string());
        }

        let keywords = if let Some(extractor) = &self.extractor {
            // Try LLM extraction
            match extractor.extract(&chunk.content).await {
                Ok(kw) => kw,
                Err(e) => {
                    warn!(
                        "LLM extraction failed for chunk {}: {}",
                        chunk.chunk_index, e
                    );
                    if self.config.fallback_on_error {
                        // Fallback to frequency-based extraction
                        debug!("Using frequency fallback for chunk {}", chunk.chunk_index);
                        extractor.extract_with_frequency(&chunk.content)?
                    } else {
                        self.stats.write().await.record_failure();
                        return Err(e);
                    }
                }
            }
        } else {
            // Use frequency-based extraction (LLM disabled)
            let extractor = KeywordExtractor::new(self.config.extractor_config.clone())?;
            extractor.extract_with_frequency(&chunk.content)?
        };

        // Combine all keywords and deduplicate
        let mut all_keywords = keywords.all_keywords();
        all_keywords.sort();
        all_keywords.dedup();

        // Limit to max keywords
        if all_keywords.len() > self.config.max_keywords_per_chunk {
            all_keywords.truncate(self.config.max_keywords_per_chunk);
        }

        // Update chunk metadata
        chunk.metadata.keywords = all_keywords;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        self.stats
            .write()
            .await
            .record_enrichment(&keywords, elapsed_ms);

        debug!(
            "Enriched chunk {} with {} keywords using {} ({}ms)",
            chunk.chunk_index,
            chunk.metadata.keywords.len(),
            keywords.extraction_method,
            elapsed_ms
        );

        Ok(keywords.extraction_method.clone())
    }

    /// Enriches multiple chunks in parallel batches
    ///
    /// Processes chunks in batches for efficient parallel extraction.
    ///
    /// # Arguments
    ///
    /// * `chunks` - Mutable slice of chunks to enrich
    ///
    /// # Returns
    ///
    /// Returns vector of extraction methods, one per chunk
    ///
    /// # Errors
    ///
    /// Returns error if batch processing fails critically
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    /// use xze_core::semantic::types::{SemanticChunk, ChunkMetadata};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    ///
    /// let mut chunks: Vec<SemanticChunk> = (0..5)
    ///     .map(|i| {
    ///         SemanticChunk::new(
    ///             format!("Chunk {} content about programming.", i),
    ///             i, 5, i, i, 0.9,
    ///             ChunkMetadata::new("test.md".to_string(), "content"),
    ///         )
    ///     })
    ///     .collect();
    ///
    /// let methods = enricher.enrich_chunks(&mut chunks).await?;
    /// assert_eq!(methods.len(), 5);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enrich_chunks(&self, chunks: &mut [SemanticChunk]) -> Result<Vec<String>> {
        use futures::stream::{self, StreamExt};

        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let total = chunks.len();
        let batch_size = self.config.batch_size;

        info!(
            "Starting batch enrichment of {} chunks (batch_size={}, workers={})",
            total, batch_size, self.config.batch_workers
        );

        let start = std::time::Instant::now();

        // Process chunks in parallel
        let methods: Vec<String> = stream::iter(chunks.iter_mut().enumerate())
            .map(|(i, chunk)| async move {
                if i > 0 && i % batch_size == 0 {
                    info!("Processed {}/{} chunks", i, total);
                }
                self.enrich_chunk(chunk).await.unwrap_or_else(|e| {
                    warn!("Failed to enrich chunk {}: {}", i, e);
                    "error".to_string()
                })
            })
            .buffer_unordered(self.config.batch_workers)
            .collect()
            .await;

        let elapsed = start.elapsed();
        info!(
            "Batch enrichment complete: {} chunks in {:.2}s ({:.2} chunks/sec)",
            total,
            elapsed.as_secs_f64(),
            total as f64 / elapsed.as_secs_f64()
        );

        Ok(methods)
    }

    /// Returns current enrichment statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    /// let stats = enricher.stats().await;
    /// println!("Total enriched: {}", stats.total_enriched);
    /// println!("Cache hits: {}", stats.cached_enriched);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> EnrichmentStats {
        self.stats.read().await.clone()
    }

    /// Resets enrichment statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    /// enricher.reset_stats().await;
    /// let stats = enricher.stats().await;
    /// assert_eq!(stats.total_enriched, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset_stats(&self) {
        self.stats.write().await.reset();
    }

    /// Checks if LLM extraction is available
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::document_enrichment::{DocumentEnricher, EnrichmentConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let enricher = DocumentEnricher::new(EnrichmentConfig::default()).await?;
    /// if enricher.is_llm_enabled().await {
    ///     println!("LLM extraction is enabled");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_llm_enabled(&self) -> bool {
        if let Some(extractor) = &self.extractor {
            extractor.is_llm_available().await
        } else {
            false
        }
    }

    /// Returns a reference to the configuration
    pub fn config(&self) -> &EnrichmentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::ChunkMetadata;

    fn create_test_chunk(index: usize, content: &str) -> SemanticChunk {
        SemanticChunk::new(
            content.to_string(),
            index,
            1,
            0,
            0,
            1.0,
            ChunkMetadata::new("test.md".to_string(), content),
        )
    }

    #[test]
    fn test_enrichment_config_default() {
        let config = EnrichmentConfig::default();
        assert!(config.use_llm_keywords);
        assert_eq!(config.max_keywords_per_chunk, 15);
        assert!(config.fallback_on_error);
        assert_eq!(config.batch_workers, 4);
    }

    #[test]
    fn test_enrichment_config_frequency_only() {
        let config = EnrichmentConfig::frequency_only();
        assert!(!config.use_llm_keywords);
    }

    #[test]
    fn test_enrichment_config_high_throughput() {
        let config = EnrichmentConfig::high_throughput();
        assert_eq!(config.batch_workers, 8);
        assert_eq!(config.batch_size, 20);
    }

    #[test]
    fn test_enrichment_config_validation() {
        let mut config = EnrichmentConfig::default();
        assert!(config.validate().is_ok());

        config.max_keywords_per_chunk = 0;
        assert!(config.validate().is_err());

        config.max_keywords_per_chunk = 15;
        config.batch_workers = 0;
        assert!(config.validate().is_err());

        config.batch_workers = 4;
        config.batch_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_enrichment_stats_default() {
        let stats = EnrichmentStats::default();
        assert_eq!(stats.total_enriched, 0);
        assert_eq!(stats.llm_enriched, 0);
        assert_eq!(stats.frequency_enriched, 0);
        assert_eq!(stats.failures, 0);
    }

    #[test]
    fn test_enrichment_stats_record() {
        let mut stats = EnrichmentStats::default();
        let keywords = ExtractedKeywords::new(
            vec!["rust".to_string()],
            vec!["cargo".to_string()],
            "llm".to_string(),
            Some(0.95),
        );

        stats.record_enrichment(&keywords, 100);
        assert_eq!(stats.total_enriched, 1);
        assert_eq!(stats.llm_enriched, 1);
        assert_eq!(stats.total_processing_ms, 100);
        assert_eq!(stats.avg_keywords_per_chunk, 2.0);
    }

    #[test]
    fn test_enrichment_stats_reset() {
        let mut stats = EnrichmentStats {
            total_enriched: 10,
            llm_enriched: 8,
            frequency_enriched: 2,
            cached_enriched: 5,
            failures: 1,
            avg_keywords_per_chunk: 12.5,
            total_processing_ms: 5000,
            cache_hits: 3,
            cache_misses: 7,
            error_count: 2,
        };

        stats.reset();
        assert_eq!(stats.total_enriched, 0);
        assert_eq!(stats.failures, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.error_count, 0);
    }

    #[tokio::test]
    async fn test_enricher_creation() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await;
        assert!(enricher.is_ok());
    }

    #[tokio::test]
    async fn test_enricher_invalid_config() {
        let mut config = EnrichmentConfig::default();
        config.max_keywords_per_chunk = 0;

        let enricher = DocumentEnricher::new(config).await;
        assert!(enricher.is_err());
    }

    #[tokio::test]
    async fn test_enrich_chunk_short_content() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let mut chunk = create_test_chunk(0, "Short");
        let result = enricher.enrich_chunk(&mut chunk).await.unwrap();

        assert_eq!(result, "skipped");
        assert!(chunk.metadata.keywords.is_empty());
    }

    #[tokio::test]
    async fn test_enrich_chunk_with_frequency() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let content =
            "Rust programming language provides memory safety without garbage collection. \
                       Rust compiler ensures type safety and prevents data races.";
        let mut chunk = create_test_chunk(0, content);

        let result = enricher.enrich_chunk(&mut chunk).await.unwrap();

        assert_eq!(result, "frequency");
        assert!(!chunk.metadata.keywords.is_empty());
        assert!(chunk.metadata.keywords.contains(&"rust".to_string()));
    }

    #[tokio::test]
    async fn test_enrich_chunks_empty() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let mut chunks: Vec<SemanticChunk> = vec![];
        let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();

        assert!(methods.is_empty());
    }

    #[tokio::test]
    async fn test_enrich_chunks_batch() {
        let config = EnrichmentConfig {
            batch_size: 2,
            batch_workers: 2,
            ..EnrichmentConfig::frequency_only()
        };
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let mut chunks: Vec<SemanticChunk> = (0..5)
            .map(|i| {
                create_test_chunk(
                    i,
                    &format!(
                        "This is test document {} about Rust programming and software development.",
                        i
                    ),
                )
            })
            .collect();

        let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();

        assert_eq!(methods.len(), 5);
        for chunk in &chunks {
            assert!(!chunk.metadata.keywords.is_empty());
        }
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let content = "Rust provides memory safety and concurrent programming features.";
        let mut chunk = create_test_chunk(0, content);

        enricher.enrich_chunk(&mut chunk).await.unwrap();

        let stats = enricher.stats().await;
        assert_eq!(stats.total_enriched, 1);
        assert_eq!(stats.frequency_enriched, 1);
        assert!(stats.avg_keywords_per_chunk > 0.0);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();

        let content = "Test content about programming.";
        let mut chunk = create_test_chunk(0, content);
        enricher.enrich_chunk(&mut chunk).await.unwrap();

        enricher.reset_stats().await;
        let stats = enricher.stats().await;
        assert_eq!(stats.total_enriched, 0);
    }

    #[tokio::test]
    async fn test_is_llm_enabled() {
        let config = EnrichmentConfig::frequency_only();
        let enricher = DocumentEnricher::new(config).await.unwrap();
        assert!(!enricher.is_llm_enabled().await);
    }

    #[test]
    fn test_config_access() {
        let config = EnrichmentConfig::default();
        let enricher_config = config.clone();
        assert_eq!(enricher_config.max_keywords_per_chunk, 15);
    }
}
