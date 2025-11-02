//! Keyword extraction module for AI-powered documentation analysis
//!
//! This module provides advanced keyword extraction using Large Language Models (LLMs)
//! with intelligent caching and fallback mechanisms to ensure reliable operation.
//!
//! # Architecture
//!
//! The keyword extractor implements a hybrid strategy:
//!
//! 1. **Primary Strategy**: LLM-based extraction using structured JSON output
//!    - Provides high-quality, context-aware keywords
//!    - Understands semantic relationships and technical concepts
//!    - Extracts descriptive and technical keywords separately
//!
//! 2. **Caching Layer**: LRU cache for extracted keywords
//!    - Reduces redundant LLM calls for unchanged documents
//!    - Content-hash based cache keys
//!    - Configurable cache size
//!
//! 3. **Fallback Strategy**: Frequency-based extraction
//!    - Activates when LLM is unavailable or fails
//!    - Uses TF-IDF style scoring
//!    - Filters common stop words
//!
//! # Examples
//!
//! ```
//! use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
//!
//! # async fn example() -> xze_core::Result<()> {
//! let config = KeywordExtractorConfig::default();
//! let extractor = KeywordExtractor::new(config)?;
//!
//! let content = "This guide explains how to configure Rust projects using Cargo.toml.";
//! let keywords = extractor.extract(content).await?;
//!
//! println!("Descriptive: {:?}", keywords.descriptive);
//! println!("Technical: {:?}", keywords.technical);
//! # Ok(())
//! # }
//! ```

use crate::{
    ai::OllamaClient,
    error::{Result, XzeError},
};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    sync::Arc,
};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Configuration for keyword extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordExtractorConfig {
    /// Ollama server base URL
    pub ollama_base_url: String,

    /// Model to use for extraction
    pub model: String,

    /// Maximum number of descriptive keywords to extract
    pub max_descriptive_keywords: usize,

    /// Maximum number of technical keywords to extract
    pub max_technical_keywords: usize,

    /// Cache size (number of documents)
    pub cache_size: usize,

    /// Timeout for LLM requests in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for LLM requests
    pub max_retries: usize,

    /// Enable fallback to frequency-based extraction
    pub enable_fallback: bool,

    /// Minimum document length for LLM extraction (chars)
    pub min_document_length: usize,

    /// Maximum document length for LLM extraction (chars)
    pub max_document_length: usize,

    /// Temperature for LLM generation (0.0 to 1.0)
    pub temperature: f32,

    /// Rollout percentage for gradual deployment (0-100)
    pub rollout_percentage: u8,

    /// Enable A/B testing mode
    pub ab_test_enabled: bool,

    /// Enable metrics collection
    pub metrics_enabled: bool,
}

impl Default for KeywordExtractorConfig {
    fn default() -> Self {
        Self {
            ollama_base_url: "http://localhost:11434".to_string(),
            model: "llama3.2:3b".to_string(),
            max_descriptive_keywords: 8,
            max_technical_keywords: 10,
            cache_size: 1000,
            timeout_seconds: 30,
            max_retries: 3,
            enable_fallback: true,
            min_document_length: 50,
            max_document_length: 8000,
            temperature: 0.1,
            rollout_percentage: std::env::var("KEYWORD_EXTRACTION_ROLLOUT_PERCENTAGE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            ab_test_enabled: std::env::var("KEYWORD_EXTRACTION_AB_TEST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            metrics_enabled: std::env::var("KEYWORD_EXTRACTION_METRICS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
        }
    }
}

impl KeywordExtractorConfig {
    /// Determine if LLM extraction should be used based on rollout percentage
    ///
    /// Uses deterministic random selection based on document content hash
    /// to ensure consistent behavior for the same document.
    ///
    /// # Arguments
    ///
    /// * `content` - Document content to hash for rollout decision
    ///
    /// # Returns
    ///
    /// Returns true if LLM extraction should be used, false otherwise
    pub fn should_use_llm_extraction(&self, content: &str) -> bool {
        if self.rollout_percentage >= 100 {
            return true;
        }

        if self.rollout_percentage == 0 {
            return false;
        }

        // Use consistent hash-based selection
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        (hash % 100) < self.rollout_percentage as u64
    }
}

/// Extracted keywords with categorization
///
/// Keywords are separated into descriptive (general concepts, topics)
/// and technical (APIs, tools, specific terms) categories for better
/// organization and search relevance.
///
/// # Examples
///
/// ```
/// use xze_core::keyword_extractor::ExtractedKeywords;
///
/// let keywords = ExtractedKeywords {
///     descriptive: vec!["configuration".to_string(), "setup".to_string()],
///     technical: vec!["Cargo.toml".to_string(), "dependencies".to_string()],
///     extraction_method: "llm".to_string(),
///     confidence: Some(0.92),
/// };
///
/// assert_eq!(keywords.all_keywords().len(), 4);
/// assert!(keywords.is_llm_extracted());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedKeywords {
    /// General descriptive keywords (topics, concepts)
    pub descriptive: Vec<String>,

    /// Technical keywords (APIs, tools, specific terms)
    pub technical: Vec<String>,

    /// Method used for extraction (llm, frequency, cached)
    pub extraction_method: String,

    /// Confidence score (0.0 to 1.0) if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

impl ExtractedKeywords {
    /// Create new extracted keywords
    pub fn new(
        descriptive: Vec<String>,
        technical: Vec<String>,
        extraction_method: String,
        confidence: Option<f32>,
    ) -> Self {
        Self {
            descriptive,
            technical,
            extraction_method,
            confidence,
        }
    }

    /// Get all keywords combined
    pub fn all_keywords(&self) -> Vec<String> {
        let mut all = self.descriptive.clone();
        all.extend(self.technical.clone());
        all
    }

    /// Check if keywords were extracted using LLM
    pub fn is_llm_extracted(&self) -> bool {
        self.extraction_method == "llm"
    }

    /// Check if keywords came from cache
    pub fn is_cached(&self) -> bool {
        self.extraction_method == "cached"
    }

    /// Total keyword count
    pub fn total_count(&self) -> usize {
        self.descriptive.len() + self.technical.len()
    }
}

/// Response format expected from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmKeywordResponse {
    descriptive_keywords: Vec<String>,
    technical_keywords: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

/// Keyword extractor with LLM and caching support
///
/// The extractor maintains an LRU cache of previously extracted keywords
/// and provides automatic fallback to frequency-based extraction when
/// the LLM is unavailable.
///
/// # Thread Safety
///
/// The extractor is thread-safe and can be shared across async tasks
/// using `Arc<KeywordExtractor>`.
/// Metrics for keyword extraction operations
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExtractionMetrics {
    /// Total number of extractions performed
    pub total_extractions: usize,
    /// Number of LLM-based extractions
    pub llm_extractions: usize,
    /// Number of frequency-based extractions
    pub frequency_extractions: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Number of fallback operations
    pub fallback_count: usize,
    /// Number of errors encountered
    pub error_count: usize,
    /// Total extraction time in milliseconds
    pub total_time_ms: f64,
}

impl ExtractionMetrics {
    /// Calculate cache hit rate as percentage
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average extraction time in milliseconds
    pub fn avg_extraction_time_ms(&self) -> f64 {
        if self.total_extractions > 0 {
            self.total_time_ms / self.total_extractions as f64
        } else {
            0.0
        }
    }

    /// Calculate fallback rate as percentage
    pub fn fallback_rate(&self) -> f64 {
        if self.total_extractions > 0 {
            (self.fallback_count as f64 / self.total_extractions as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Convert metrics to JSON-serializable format
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total_extractions": self.total_extractions,
            "method_breakdown": {
                "llm": self.llm_extractions,
                "frequency": self.frequency_extractions,
            },
            "cache": {
                "hit_rate": format!("{:.1}%", self.cache_hit_rate()),
                "hits": self.cache_hits,
                "misses": self.cache_misses,
            },
            "fallback": {
                "count": self.fallback_count,
                "rate": format!("{:.1}%", self.fallback_rate()),
            },
            "errors": self.error_count,
            "performance": {
                "avg_extraction_time_ms": format!("{:.2}", self.avg_extraction_time_ms()),
                "total_time_ms": format!("{:.2}", self.total_time_ms),
            }
        })
    }
}

pub struct KeywordExtractor {
    config: KeywordExtractorConfig,
    client: OllamaClient,
    cache: Arc<Mutex<LruCache<String, ExtractedKeywords>>>,
    metrics: Arc<Mutex<ExtractionMetrics>>,
}

impl KeywordExtractor {
    /// Create a new keyword extractor with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the cache size is invalid or the Ollama client
    /// cannot be initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # fn example() -> xze_core::Result<()> {
    /// let config = KeywordExtractorConfig::default();
    /// let extractor = KeywordExtractor::new(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: KeywordExtractorConfig) -> Result<Self> {
        let cache_size = NonZeroUsize::new(config.cache_size).ok_or_else(|| {
            XzeError::validation("Cache size must be greater than zero".to_string())
        })?;

        let client = OllamaClient::with_timeout(
            config.ollama_base_url.clone(),
            std::time::Duration::from_secs(config.timeout_seconds),
        );

        Ok(Self {
            config,
            client,
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            metrics: Arc::new(Mutex::new(ExtractionMetrics::default())),
        })
    }

    /// Extract keywords from the given content
    ///
    /// This method attempts to use the LLM for extraction, falling back to
    /// frequency-based extraction if configured and necessary. Results are
    /// cached based on content hash.
    ///
    /// # Arguments
    ///
    /// * `content` - The document content to extract keywords from
    ///
    /// # Returns
    ///
    /// Returns `ExtractedKeywords` containing both descriptive and technical keywords.
    ///
    /// # Errors
    ///
    /// Returns an error if both LLM extraction and fallback fail, or if the
    /// content is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let extractor = KeywordExtractor::new(KeywordExtractorConfig::default())?;
    /// let keywords = extractor.extract("Documentation about Rust async programming").await?;
    /// assert!(keywords.total_count() > 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn extract(&self, content: &str) -> Result<ExtractedKeywords> {
        let start_time = std::time::Instant::now();

        // Validate content
        if content.trim().is_empty() {
            if self.config.metrics_enabled {
                let mut metrics = self.metrics.lock().await;
                metrics.error_count += 1;
            }
            return Err(XzeError::validation("Content cannot be empty".to_string()));
        }

        // Generate cache key
        let cache_key = self.generate_cache_key(content);

        // Check cache first
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                debug!("Cache hit for content (hash: {})", &cache_key[..8]);

                if self.config.metrics_enabled {
                    let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                    let mut metrics = self.metrics.lock().await;
                    metrics.total_extractions += 1;
                    metrics.cache_hits += 1;
                    metrics.total_time_ms += elapsed;
                }

                let mut result = cached.clone();
                result.extraction_method = "cached".to_string();
                return Ok(result);
            } else if self.config.metrics_enabled {
                let mut metrics = self.metrics.lock().await;
                metrics.cache_misses += 1;
            }
        }

        // Check if LLM extraction should be used based on rollout
        let use_llm = self.config.should_use_llm_extraction(content);

        // Try LLM extraction if enabled by rollout
        let result = if use_llm {
            match self.extract_with_llm(content).await {
                Ok(keywords) => {
                    info!(
                        "LLM extraction successful: {} total keywords",
                        keywords.total_count()
                    );

                    if self.config.metrics_enabled {
                        let mut metrics = self.metrics.lock().await;
                        metrics.llm_extractions += 1;
                    }

                    keywords
                }
                Err(e) => {
                    warn!("LLM extraction failed: {}", e);

                    if self.config.metrics_enabled {
                        let mut metrics = self.metrics.lock().await;
                        metrics.error_count += 1;
                    }

                    if self.config.enable_fallback {
                        info!("Falling back to frequency-based extraction");

                        if self.config.metrics_enabled {
                            let mut metrics = self.metrics.lock().await;
                            metrics.fallback_count += 1;
                        }

                        self.extract_with_frequency(content)?
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            // Use frequency-based extraction directly
            if self.config.metrics_enabled {
                let mut metrics = self.metrics.lock().await;
                metrics.frequency_extractions += 1;
            }
            self.extract_with_frequency(content)?
        };

        // Update metrics
        if self.config.metrics_enabled {
            let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
            let mut metrics = self.metrics.lock().await;
            metrics.total_extractions += 1;
            metrics.total_time_ms += elapsed;
        }

        // Cache the result
        {
            let mut cache = self.cache.lock().await;
            cache.put(cache_key, result.clone());
        }

        Ok(result)
    }

    /// Extract keywords using the LLM
    async fn extract_with_llm(&self, content: &str) -> Result<ExtractedKeywords> {
        // Truncate content if too long
        let content = if content.len() > self.config.max_document_length {
            debug!(
                "Truncating content from {} to {} chars",
                content.len(),
                self.config.max_document_length
            );
            &content[..self.config.max_document_length]
        } else {
            content
        };

        // Skip LLM for very short content
        if content.len() < self.config.min_document_length {
            debug!("Content too short for LLM extraction, using fallback");
            return self.extract_with_frequency(content);
        }

        let prompt = self.build_extraction_prompt(content);

        // Retry logic
        let mut last_error = None;
        for attempt in 1..=self.config.max_retries {
            match self.call_llm(&prompt).await {
                Ok(response) => {
                    return self.parse_llm_response(&response);
                }
                Err(e) => {
                    warn!("LLM extraction attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| XzeError::ai("LLM extraction failed".to_string())))
    }

    /// Build the prompt for LLM keyword extraction
    fn build_extraction_prompt(&self, content: &str) -> String {
        format!(
            r#"You are a technical documentation analyst. Extract keywords from the following text.

Extract TWO types of keywords:
1. Descriptive keywords: General concepts, topics, and themes (max {})
2. Technical keywords: Specific APIs, tools, commands, file names, or technical terms (max {})

Return ONLY valid JSON in this exact format (no markdown, no explanation):
{{
  "descriptive_keywords": ["keyword1", "keyword2"],
  "technical_keywords": ["api_name", "tool_name"],
  "confidence": 0.95
}}

Guidelines:
- Use lowercase for all keywords
- Be specific and relevant
- Avoid generic words like "guide", "documentation"
- For technical keywords, include version numbers if mentioned
- Confidence should be 0.0 to 1.0

Text to analyze:
{}

JSON response:"#,
            self.config.max_descriptive_keywords, self.config.max_technical_keywords, content
        )
    }

    /// Call the LLM with the given prompt
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        use crate::ai::{GenerateOptions, GenerateRequest};

        let request = GenerateRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(GenerateOptions {
                temperature: Some(self.config.temperature),
                num_predict: Some(500),
                top_k: None,
                top_p: None,
            }),
        };

        self.client.generate(request).await
    }

    /// Parse the LLM response into structured keywords
    fn parse_llm_response(&self, response: &str) -> Result<ExtractedKeywords> {
        // Try to find JSON in the response (handle markdown code blocks)
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let parsed: LlmKeywordResponse = serde_json::from_str(json_str).map_err(|e| {
            XzeError::ai(format!(
                "Failed to parse LLM response as JSON: {}. Response: {}",
                e,
                &json_str[..json_str.len().min(200)]
            ))
        })?;

        // Validate and clean keywords
        let descriptive = self.clean_keywords(
            parsed.descriptive_keywords,
            self.config.max_descriptive_keywords,
        );
        let technical = self.clean_keywords(
            parsed.technical_keywords,
            self.config.max_technical_keywords,
        );

        if descriptive.is_empty() && technical.is_empty() {
            return Err(XzeError::ai("LLM returned no valid keywords".to_string()));
        }

        Ok(ExtractedKeywords::new(
            descriptive,
            technical,
            "llm".to_string(),
            parsed.confidence,
        ))
    }

    /// Extract keywords using frequency-based analysis (fallback)
    #[doc(hidden)]
    pub fn extract_with_frequency(&self, content: &str) -> Result<ExtractedKeywords> {
        let words = self.tokenize(content);
        let frequencies = self.calculate_frequencies(&words);

        // Filter stop words and score
        let mut scored_words: Vec<(String, f32)> = frequencies
            .into_iter()
            .filter(|(word, _)| !self.is_stop_word(word) && word.len() > 2)
            .map(|(word, count)| {
                let score = count as f32 * (word.len() as f32).ln();
                (word, score)
            })
            .collect();

        scored_words.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Separate into descriptive and technical based on heuristics
        let mut descriptive = Vec::new();
        let mut technical = Vec::new();

        for (word, _score) in scored_words
            .iter()
            .take(self.config.max_descriptive_keywords + self.config.max_technical_keywords)
        {
            if self.is_technical_term(word) {
                if technical.len() < self.config.max_technical_keywords {
                    technical.push(word.clone());
                }
            } else if descriptive.len() < self.config.max_descriptive_keywords {
                descriptive.push(word.clone());
            }
        }

        Ok(ExtractedKeywords::new(
            descriptive,
            technical,
            "frequency".to_string(),
            Some(0.6), // Lower confidence for fallback method
        ))
    }

    /// Tokenize content into words
    #[doc(hidden)]
    pub fn tokenize(&self, content: &str) -> Vec<String> {
        content
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Calculate word frequencies
    #[doc(hidden)]
    pub fn calculate_frequencies(&self, words: &[String]) -> HashMap<String, usize> {
        let mut frequencies = HashMap::new();
        for word in words {
            *frequencies.entry(word.clone()).or_insert(0) += 1;
        }
        frequencies
    }

    /// Check if a word is a stop word
    fn is_stop_word(&self, word: &str) -> bool {
        const STOP_WORDS: &[&str] = &[
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "can", "this", "that", "these", "those", "it", "its", "they", "them", "their", "we",
            "our", "you", "your", "he", "she", "him", "her", "his",
        ];
        STOP_WORDS.contains(&word)
    }

    /// Check if a word appears to be a technical term
    fn is_technical_term(&self, word: &str) -> bool {
        // Heuristics: contains underscore, dash, mixed case, file extension, or common tech patterns
        word.contains('_')
            || word.contains('-')
            || word.contains('.')
            || word.ends_with("rs")
            || word.ends_with("toml")
            || word.ends_with("yaml")
            || word.ends_with("json")
            || word.starts_with("cargo")
            || word.starts_with("rust")
            || word.starts_with("async")
            || word.len() > 12
    }

    /// Clean and deduplicate keywords
    #[doc(hidden)]
    pub fn clean_keywords(&self, keywords: Vec<String>, max_count: usize) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut cleaned = Vec::new();

        for keyword in keywords {
            let normalized = keyword.trim().to_lowercase();
            if !normalized.is_empty() && normalized.len() > 1 && seen.insert(normalized.clone()) {
                cleaned.push(normalized);
                if cleaned.len() >= max_count {
                    break;
                }
            }
        }

        cleaned
    }

    /// Generate a cache key from content
    #[doc(hidden)]
    pub fn generate_cache_key(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().await;
        (cache.len(), cache.cap().get())
    }

    /// Clear the keyword cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
        info!("Keyword extraction cache cleared");
    }

    /// Extract keywords from multiple documents in parallel
    ///
    /// # Arguments
    ///
    /// * `documents` - Slice of document contents to process
    ///
    /// # Returns
    ///
    /// Returns a vector of results, one for each input document
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let extractor = KeywordExtractor::new(KeywordExtractorConfig::default())?;
    /// let docs = vec!["Document 1 content", "Document 2 content"];
    /// let results = extractor.extract_batch(&docs).await;
    /// assert_eq!(results.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn extract_batch(&self, documents: &[&str]) -> Vec<Result<ExtractedKeywords>> {
        use futures::future::join_all;

        let tasks: Vec<_> = documents.iter().map(|doc| self.extract(doc)).collect();

        join_all(tasks).await
    }

    /// Check if the LLM service is available
    pub async fn is_llm_available(&self) -> bool {
        self.client.health_check().await.unwrap_or(false)
    }

    /// Get current extraction metrics
    ///
    /// # Returns
    ///
    /// Returns a clone of the current metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let extractor = KeywordExtractor::new(KeywordExtractorConfig::default())?;
    /// let metrics = extractor.get_metrics().await;
    /// println!("Total extractions: {}", metrics.total_extractions);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_metrics(&self) -> ExtractionMetrics {
        self.metrics.lock().await.clone()
    }

    /// Reset all metrics counters to zero
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let extractor = KeywordExtractor::new(KeywordExtractorConfig::default())?;
    /// extractor.reset_metrics().await;
    /// let metrics = extractor.get_metrics().await;
    /// assert_eq!(metrics.total_extractions, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock().await;
        *metrics = ExtractionMetrics::default();
        info!("Keyword extraction metrics reset");
    }

    /// Export metrics as JSON
    ///
    /// # Returns
    ///
    /// Returns metrics formatted as JSON value
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::keyword_extractor::{KeywordExtractor, KeywordExtractorConfig};
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let extractor = KeywordExtractor::new(KeywordExtractorConfig::default())?;
    /// let json = extractor.export_metrics().await;
    /// println!("{}", serde_json::to_string_pretty(&json)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export_metrics(&self) -> serde_json::Value {
        let metrics = self.metrics.lock().await;
        metrics.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_keywords_creation() {
        let keywords = ExtractedKeywords::new(
            vec!["async".to_string(), "programming".to_string()],
            vec!["tokio".to_string(), "futures".to_string()],
            "llm".to_string(),
            Some(0.95),
        );

        assert_eq!(keywords.descriptive.len(), 2);
        assert_eq!(keywords.technical.len(), 2);
        assert_eq!(keywords.total_count(), 4);
        assert!(keywords.is_llm_extracted());
        assert!(!keywords.is_cached());
    }

    #[test]
    fn test_extracted_keywords_all_keywords() {
        let keywords = ExtractedKeywords::new(
            vec!["testing".to_string()],
            vec!["cargo".to_string()],
            "frequency".to_string(),
            None,
        );

        let all = keywords.all_keywords();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&"testing".to_string()));
        assert!(all.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = KeywordExtractorConfig::default();
        assert_eq!(config.model, "llama3.2:3b");
        assert_eq!(config.max_descriptive_keywords, 8);
        assert_eq!(config.max_technical_keywords, 10);
        assert!(config.enable_fallback);
    }

    #[tokio::test]
    async fn test_extractor_creation() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config);
        assert!(extractor.is_ok());
    }

    #[tokio::test]
    async fn test_extractor_invalid_config() {
        let mut config = KeywordExtractorConfig::default();
        config.cache_size = 0;
        let extractor = KeywordExtractor::new(config);
        assert!(extractor.is_err());
    }

    #[tokio::test]
    async fn test_extract_empty_content() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        let result = extractor.extract("").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_tokenize() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        let words = extractor.tokenize("Hello, world! This is a test.");
        assert_eq!(words, vec!["hello", "world", "this", "is", "a", "test"]);
    }

    #[test]
    fn test_is_stop_word() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        assert!(extractor.is_stop_word("the"));
        assert!(extractor.is_stop_word("and"));
        assert!(!extractor.is_stop_word("rust"));
    }

    #[test]
    fn test_is_technical_term() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        assert!(extractor.is_technical_term("cargo_toml"));
        assert!(extractor.is_technical_term("async-await"));
        assert!(extractor.is_technical_term("main.rs"));
        assert!(!extractor.is_technical_term("simple"));
    }

    #[test]
    fn test_clean_keywords() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        let keywords = vec![
            "Rust".to_string(),
            "rust".to_string(),
            "  cargo  ".to_string(),
            "a".to_string(),
            "testing".to_string(),
        ];
        let cleaned = extractor.clean_keywords(keywords, 5);
        assert_eq!(cleaned.len(), 3); // Deduplicated and filtered
        assert!(cleaned.contains(&"rust".to_string()));
        assert!(cleaned.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_calculate_frequencies() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();
        let words = vec!["rust".to_string(), "cargo".to_string(), "rust".to_string()];
        let frequencies = extractor.calculate_frequencies(&words);
        assert_eq!(frequencies.get("rust"), Some(&2));
        assert_eq!(frequencies.get("cargo"), Some(&1));
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let (used, capacity) = extractor.cache_stats().await;
        assert_eq!(used, 0);
        assert_eq!(capacity, 1000);

        extractor.clear_cache().await;
        let (used, _) = extractor.cache_stats().await;
        assert_eq!(used, 0);
    }

    #[test]
    fn test_generate_cache_key() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let key1 = extractor.generate_cache_key("test content");
        let key2 = extractor.generate_cache_key("test content");
        let key3 = extractor.generate_cache_key("different content");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_frequency_extraction() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let content = "Rust programming with Cargo and async-await. \
                       Rust is great for systems programming. \
                       Use cargo build to compile your Rust project.";

        let keywords = extractor.extract_with_frequency(content).unwrap();

        assert!(keywords.total_count() > 0);
        assert_eq!(keywords.extraction_method, "frequency");
        assert!(!keywords.is_llm_extracted());
    }

    #[test]
    fn test_build_extraction_prompt() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let prompt = extractor.build_extraction_prompt("Sample documentation text");

        assert!(prompt.contains("descriptive_keywords"));
        assert!(prompt.contains("technical_keywords"));
        assert!(prompt.contains("Sample documentation text"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_parse_llm_response_valid() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let response = r#"{
            "descriptive_keywords": ["async", "programming"],
            "technical_keywords": ["tokio", "futures"],
            "confidence": 0.95
        }"#;

        let result = extractor.parse_llm_response(response);
        assert!(result.is_ok());

        let keywords = result.unwrap();
        assert_eq!(keywords.descriptive.len(), 2);
        assert_eq!(keywords.technical.len(), 2);
        assert_eq!(keywords.confidence, Some(0.95));
    }

    #[test]
    fn test_parse_llm_response_with_markdown() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let response = r#"Here is the JSON:
```json
{
    "descriptive_keywords": ["testing"],
    "technical_keywords": ["cargo"],
    "confidence": 0.8
}
```"#;

        let result = extractor.parse_llm_response(response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_llm_response_invalid() {
        let config = KeywordExtractorConfig::default();
        let extractor = KeywordExtractor::new(config).unwrap();

        let response = "This is not valid JSON";
        let result = extractor.parse_llm_response(response);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_batch() {
        let config = KeywordExtractorConfig {
            enable_fallback: true,
            ..Default::default()
        };
        let extractor = KeywordExtractor::new(config).unwrap();

        let documents = vec![
            "Rust async programming",
            "Cargo build system",
            "Error handling in Rust",
        ];

        let results = extractor.extract_batch(&documents).await;
        assert_eq!(results.len(), 3);
    }
}
