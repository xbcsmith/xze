//! Prompt template management for LLM keyword extraction
//!
//! This module provides optimized prompt templates for keyword extraction,
//! including A/B testing support and performance tracking for different
//! prompt variations.
//!
//! # Examples
//!
//! ```
//! use xze_core::prompt_templates::{PromptTemplate, PromptVariant};
//!
//! let template = PromptTemplate::new(PromptVariant::Technical);
//! let prompt = template.build("Sample documentation content", 8, 10);
//! assert!(prompt.contains("technical"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt template variants for A/B testing
///
/// Different prompt strategies optimized for various use cases and document types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PromptVariant {
    /// Original baseline prompt with balanced focus
    #[default]
    Baseline,

    /// Technical-focused prompt emphasizing tools, APIs, and commands
    Technical,

    /// Domain-aware prompt with prioritization of domain-specific terms
    DomainAware,

    /// Structured prompt with explicit examples
    Structured,

    /// Concise prompt optimized for speed
    Concise,

    /// Detailed prompt for complex documents
    Detailed,
}

impl PromptVariant {
    /// Get all available prompt variants
    pub fn all() -> Vec<PromptVariant> {
        vec![
            PromptVariant::Baseline,
            PromptVariant::Technical,
            PromptVariant::DomainAware,
            PromptVariant::Structured,
            PromptVariant::Concise,
            PromptVariant::Detailed,
        ]
    }

    /// Get variant name as string
    pub fn name(&self) -> &'static str {
        match self {
            PromptVariant::Baseline => "baseline",
            PromptVariant::Technical => "technical",
            PromptVariant::DomainAware => "domain_aware",
            PromptVariant::Structured => "structured",
            PromptVariant::Concise => "concise",
            PromptVariant::Detailed => "detailed",
        }
    }

    /// Parse variant from string name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "baseline" => Some(PromptVariant::Baseline),
            "technical" => Some(PromptVariant::Technical),
            "domain_aware" | "domain-aware" => Some(PromptVariant::DomainAware),
            "structured" => Some(PromptVariant::Structured),
            "concise" => Some(PromptVariant::Concise),
            "detailed" => Some(PromptVariant::Detailed),
            _ => None,
        }
    }
}

/// Prompt template builder for keyword extraction
///
/// Encapsulates prompt generation logic and supports multiple template variants.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    variant: PromptVariant,
}

impl PromptTemplate {
    /// Create a new prompt template with the specified variant
    pub fn new(variant: PromptVariant) -> Self {
        Self { variant }
    }

    /// Create template from environment variable or default
    pub fn from_env() -> Self {
        let variant = std::env::var("KEYWORD_EXTRACTION_PROMPT_VARIANT")
            .ok()
            .and_then(|v| PromptVariant::from_name(&v))
            .unwrap_or_default();

        Self { variant }
    }

    /// Get the current variant
    pub fn variant(&self) -> PromptVariant {
        self.variant
    }

    /// Build a prompt for the given content and limits
    ///
    /// # Arguments
    ///
    /// * `content` - Document content to extract keywords from
    /// * `max_descriptive` - Maximum number of descriptive keywords
    /// * `max_technical` - Maximum number of technical keywords
    ///
    /// # Returns
    ///
    /// Returns a formatted prompt string ready for LLM processing
    pub fn build(&self, content: &str, max_descriptive: usize, max_technical: usize) -> String {
        match self.variant {
            PromptVariant::Baseline => self.build_baseline(content, max_descriptive, max_technical),
            PromptVariant::Technical => {
                self.build_technical(content, max_descriptive, max_technical)
            }
            PromptVariant::DomainAware => {
                self.build_domain_aware(content, max_descriptive, max_technical)
            }
            PromptVariant::Structured => {
                self.build_structured(content, max_descriptive, max_technical)
            }
            PromptVariant::Concise => self.build_concise(content, max_descriptive, max_technical),
            PromptVariant::Detailed => self.build_detailed(content, max_descriptive, max_technical),
        }
    }

    /// Baseline prompt (original implementation)
    fn build_baseline(
        &self,
        content: &str,
        max_descriptive: usize,
        max_technical: usize,
    ) -> String {
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
            max_descriptive, max_technical, content
        )
    }

    /// Technical-focused prompt emphasizing tools, APIs, and commands
    fn build_technical(
        &self,
        content: &str,
        max_descriptive: usize,
        max_technical: usize,
    ) -> String {
        format!(
            r#"Extract technical keywords from this documentation for search indexing.

Focus on identifying:
- Tools and technologies (docker, kubernetes, rust)
- APIs and interfaces (REST, GraphQL, OpenAPI)
- Commands and CLI syntax (cargo build, git commit)
- Programming concepts (async, concurrency, ORM)
- Methodologies (CI/CD, TDD, agile)

Extract up to {} descriptive keywords (general concepts, topics) and {} technical keywords (specific tools, APIs, commands).

Return ONLY valid JSON (no markdown):
{{
  "descriptive_keywords": ["concept1", "concept2"],
  "technical_keywords": ["tool1", "command1", "api1"],
  "confidence": 0.90
}}

Requirements:
- Lowercase only
- Prioritize technical specificity over general terms
- Include version numbers when mentioned (rust-1.70, node-18)
- Avoid filler words

Document:
{}

JSON:"#,
            max_descriptive, max_technical, content
        )
    }

    /// Domain-aware prompt with prioritization
    fn build_domain_aware(
        &self,
        content: &str,
        max_descriptive: usize,
        max_technical: usize,
    ) -> String {
        format!(
            r#"Analyze this technical document and extract keywords optimized for search.

Prioritize domain-specific terminology:
1. Product names and brands
2. Command syntax and flags
3. Technical concepts and patterns
4. Acronyms and abbreviations
5. File formats and protocols

Extract {} descriptive keywords and {} technical keywords.

Return JSON only:
{{
  "descriptive_keywords": ["domain_term1", "concept1"],
  "technical_keywords": ["product_name", "command_syntax", "protocol"],
  "confidence": 0.88
}}

Rules:
- Domain terms over common words
- Exact command syntax preserved
- Include both acronym and full form if present (HTTP, Hypertext Transfer Protocol)
- Lowercase except for proper names

Text:
{}

JSON:"#,
            max_descriptive, max_technical, content
        )
    }

    /// Structured prompt with explicit examples
    fn build_structured(
        &self,
        content: &str,
        max_descriptive: usize,
        max_technical: usize,
    ) -> String {
        format!(
            r#"Extract keywords for search indexing from the document below.

Example output structure:
{{
  "descriptive_keywords": ["configuration", "deployment", "monitoring"],
  "technical_keywords": ["docker-compose", "prometheus", "grafana", "yaml"],
  "confidence": 0.92
}}

Categories to extract:
- Descriptive (max {}): High-level concepts, processes, categories
- Technical (max {}): Specific tools, commands, file types, APIs

Extraction guidelines:
- Focus on searchable terms users would query
- Include multi-word phrases if significant (continuous integration)
- Lowercase formatting
- No generic terms (setup, guide, example)

Document text:
{}

Provide JSON response:"#,
            max_descriptive, max_technical, content
        )
    }

    /// Concise prompt optimized for speed
    fn build_concise(&self, content: &str, max_descriptive: usize, max_technical: usize) -> String {
        format!(
            r#"Extract {} descriptive and {} technical keywords from this text.

Return JSON:
{{
  "descriptive_keywords": [],
  "technical_keywords": [],
  "confidence": 0.0
}}

Text:
{}

JSON:"#,
            max_descriptive, max_technical, content
        )
    }

    /// Detailed prompt for complex documents
    fn build_detailed(
        &self,
        content: &str,
        max_descriptive: usize,
        max_technical: usize,
    ) -> String {
        format!(
            r#"You are an expert technical documentation analyst specializing in keyword extraction for search optimization.

Task: Extract keywords from the provided document to enhance search discoverability and relevance.

Keyword Categories:
1. Descriptive Keywords (max {}):
   - High-level concepts and themes
   - Subject matter domains
   - Conceptual topics
   - Process names
   - General categories

2. Technical Keywords (max {}):
   - Exact tool and product names
   - API endpoints and interfaces
   - Command-line commands with syntax
   - Programming language constructs
   - File formats and extensions
   - Protocol names
   - Technology stack components

Output Format (strict JSON, no markdown):
{{
  "descriptive_keywords": ["keyword1", "keyword2"],
  "technical_keywords": ["tool1", "api1", "command1"],
  "confidence": 0.95
}}

Extraction Rules:
1. All keywords must be lowercase unless a proper noun or command
2. Preserve exact technical syntax (e.g., cargo build, git-commit)
3. Include version numbers when explicitly mentioned
4. Prefer specific terms over generic ones
5. Multi-word phrases allowed if semantically significant
6. Avoid: "guide", "tutorial", "documentation", "example", "setup"
7. Include acronyms if commonly used (API, CLI, CI/CD)
8. Confidence score between 0.0 and 1.0 based on text clarity

Document Content:
{}

Respond with JSON only:"#,
            max_descriptive, max_technical, content
        )
    }
}

/// Performance statistics for prompt variants
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptPerformanceStats {
    /// Number of times this prompt was used
    pub usage_count: usize,

    /// Total extraction time in milliseconds
    pub total_time_ms: f64,

    /// Number of successful extractions
    pub success_count: usize,

    /// Number of failed extractions
    pub failure_count: usize,

    /// Total keywords extracted
    pub total_keywords: usize,
}

impl PromptPerformanceStats {
    /// Calculate average extraction time
    pub fn avg_time_ms(&self) -> f64 {
        if self.usage_count > 0 {
            self.total_time_ms / self.usage_count as f64
        } else {
            0.0
        }
    }

    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.usage_count > 0 {
            (self.success_count as f64 / self.usage_count as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average keywords per extraction
    pub fn avg_keywords(&self) -> f64 {
        if self.success_count > 0 {
            self.total_keywords as f64 / self.success_count as f64
        } else {
            0.0
        }
    }
}

/// Tracker for prompt variant performance
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PromptPerformanceTracker {
    stats: HashMap<String, PromptPerformanceStats>,
}

impl PromptPerformanceTracker {
    /// Create a new performance tracker
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    /// Record a successful extraction
    pub fn record_success(&mut self, variant: PromptVariant, time_ms: f64, keyword_count: usize) {
        let stats = self.stats.entry(variant.name().to_string()).or_default();
        stats.usage_count += 1;
        stats.success_count += 1;
        stats.total_time_ms += time_ms;
        stats.total_keywords += keyword_count;
    }

    /// Record a failed extraction
    pub fn record_failure(&mut self, variant: PromptVariant, time_ms: f64) {
        let stats = self.stats.entry(variant.name().to_string()).or_default();
        stats.usage_count += 1;
        stats.failure_count += 1;
        stats.total_time_ms += time_ms;
    }

    /// Get statistics for a specific variant
    pub fn get_stats(&self, variant: PromptVariant) -> Option<&PromptPerformanceStats> {
        self.stats.get(variant.name())
    }

    /// Get all statistics
    pub fn all_stats(&self) -> &HashMap<String, PromptPerformanceStats> {
        &self.stats
    }

    /// Export statistics as JSON
    pub fn to_json(&self) -> serde_json::Value {
        let mut variants = serde_json::Map::new();

        for (name, stats) in &self.stats {
            variants.insert(
                name.clone(),
                serde_json::json!({
                    "usage_count": stats.usage_count,
                    "success_rate": format!("{:.1}%", stats.success_rate()),
                    "avg_time_ms": format!("{:.2}", stats.avg_time_ms()),
                    "avg_keywords": format!("{:.1}", stats.avg_keywords()),
                    "total_keywords": stats.total_keywords,
                }),
            );
        }

        serde_json::json!({
            "variants": variants
        })
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.stats.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_variant_all() {
        let variants = PromptVariant::all();
        assert_eq!(variants.len(), 6);
        assert!(variants.contains(&PromptVariant::Baseline));
        assert!(variants.contains(&PromptVariant::Technical));
    }

    #[test]
    fn test_prompt_variant_name() {
        assert_eq!(PromptVariant::Baseline.name(), "baseline");
        assert_eq!(PromptVariant::Technical.name(), "technical");
        assert_eq!(PromptVariant::DomainAware.name(), "domain_aware");
    }

    #[test]
    fn test_prompt_variant_from_name() {
        assert_eq!(
            PromptVariant::from_name("baseline"),
            Some(PromptVariant::Baseline)
        );
        assert_eq!(
            PromptVariant::from_name("technical"),
            Some(PromptVariant::Technical)
        );
        assert_eq!(
            PromptVariant::from_name("domain_aware"),
            Some(PromptVariant::DomainAware)
        );
        assert_eq!(
            PromptVariant::from_name("domain-aware"),
            Some(PromptVariant::DomainAware)
        );
        assert_eq!(PromptVariant::from_name("invalid"), None);
    }

    #[test]
    fn test_prompt_variant_default() {
        assert_eq!(PromptVariant::default(), PromptVariant::Baseline);
    }

    #[test]
    fn test_prompt_template_new() {
        let template = PromptTemplate::new(PromptVariant::Technical);
        assert_eq!(template.variant(), PromptVariant::Technical);
    }

    #[test]
    fn test_prompt_template_build_baseline() {
        let template = PromptTemplate::new(PromptVariant::Baseline);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("test content"));
        assert!(prompt.contains("max 5"));
        assert!(prompt.contains("max 10"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_prompt_template_build_technical() {
        let template = PromptTemplate::new(PromptVariant::Technical);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("technical"));
        assert!(prompt.contains("tools"));
        assert!(prompt.contains("APIs"));
    }

    #[test]
    fn test_prompt_template_build_domain_aware() {
        let template = PromptTemplate::new(PromptVariant::DomainAware);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("domain-specific"));
        assert!(prompt.contains("Product names"));
    }

    #[test]
    fn test_prompt_template_build_structured() {
        let template = PromptTemplate::new(PromptVariant::Structured);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("Example output"));
        assert!(prompt.contains("Categories"));
    }

    #[test]
    fn test_prompt_template_build_concise() {
        let template = PromptTemplate::new(PromptVariant::Concise);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("test content"));
        assert!(prompt.len() < 300);
    }

    #[test]
    fn test_prompt_template_build_detailed() {
        let template = PromptTemplate::new(PromptVariant::Detailed);
        let prompt = template.build("test content", 5, 10);
        assert!(prompt.contains("expert"));
        assert!(prompt.contains("Extraction Rules"));
        assert!(prompt.len() > 500);
    }

    #[test]
    fn test_prompt_performance_stats_avg_time() {
        let mut stats = PromptPerformanceStats::default();
        stats.usage_count = 5;
        stats.total_time_ms = 500.0;
        assert_eq!(stats.avg_time_ms(), 100.0);
    }

    #[test]
    fn test_prompt_performance_stats_success_rate() {
        let mut stats = PromptPerformanceStats::default();
        stats.usage_count = 10;
        stats.success_count = 8;
        assert_eq!(stats.success_rate(), 80.0);
    }

    #[test]
    fn test_prompt_performance_stats_avg_keywords() {
        let mut stats = PromptPerformanceStats::default();
        stats.success_count = 5;
        stats.total_keywords = 50;
        assert_eq!(stats.avg_keywords(), 10.0);
    }

    #[test]
    fn test_prompt_performance_tracker_record_success() {
        let mut tracker = PromptPerformanceTracker::new();
        tracker.record_success(PromptVariant::Baseline, 100.0, 10);

        let stats = tracker.get_stats(PromptVariant::Baseline).unwrap();
        assert_eq!(stats.usage_count, 1);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.total_time_ms, 100.0);
        assert_eq!(stats.total_keywords, 10);
    }

    #[test]
    fn test_prompt_performance_tracker_record_failure() {
        let mut tracker = PromptPerformanceTracker::new();
        tracker.record_failure(PromptVariant::Technical, 50.0);

        let stats = tracker.get_stats(PromptVariant::Technical).unwrap();
        assert_eq!(stats.usage_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.total_time_ms, 50.0);
    }

    #[test]
    fn test_prompt_performance_tracker_multiple_variants() {
        let mut tracker = PromptPerformanceTracker::new();
        tracker.record_success(PromptVariant::Baseline, 100.0, 10);
        tracker.record_success(PromptVariant::Technical, 150.0, 12);

        assert!(tracker.get_stats(PromptVariant::Baseline).is_some());
        assert!(tracker.get_stats(PromptVariant::Technical).is_some());
        assert_eq!(tracker.all_stats().len(), 2);
    }

    #[test]
    fn test_prompt_performance_tracker_to_json() {
        let mut tracker = PromptPerformanceTracker::new();
        tracker.record_success(PromptVariant::Baseline, 100.0, 10);

        let json = tracker.to_json();
        assert!(json["variants"]["baseline"].is_object());
        assert!(json["variants"]["baseline"]["usage_count"].is_number());
    }

    #[test]
    fn test_prompt_performance_tracker_reset() {
        let mut tracker = PromptPerformanceTracker::new();
        tracker.record_success(PromptVariant::Baseline, 100.0, 10);
        tracker.reset();

        assert!(tracker.get_stats(PromptVariant::Baseline).is_none());
        assert_eq!(tracker.all_stats().len(), 0);
    }
}
