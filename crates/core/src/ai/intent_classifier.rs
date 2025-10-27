//! Intent classification service using AI models
//!
//! This module provides the core intent classification functionality for determining
//! the appropriate Diataxis documentation category for a given query or code context.
//!
//! # Examples
//!
//! ```no_run
//! use xze_core::ai::intent_classifier::{IntentClassifier, ClassifierConfig};
//! use xze_core::ai::client::OllamaClient;
//! use std::sync::Arc;
//!
//! # async fn example() -> xze_core::error::Result<()> {
//! let config = ClassifierConfig::default();
//! let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
//! let classifier = IntentClassifier::new(config, client);
//!
//! let result = classifier.classify("How do I install this library?").await?;
//! println!("Intent: {} (confidence: {})", result.primary_intent, result.confidence);
//! # Ok(())
//! # }
//! ```

use crate::ai::client::{GenerateOptions, GenerateRequest, OllamaClient};
use crate::ai::intent_types::{
    ClassificationError, ClassificationMetadata, ClassificationResult, Confidence, DiataxisIntent,
};
use crate::error::Result;
use regex::Regex;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Configuration for the intent classifier
///
/// # Examples
///
/// ```
/// use xze_core::ai::intent_classifier::ClassifierConfig;
///
/// let config = ClassifierConfig::default()
///     .with_model("llama2:latest")
///     .with_temperature(0.2)
///     .with_confidence_threshold(0.6);
///
/// assert_eq!(config.model, "llama2:latest");
/// assert_eq!(config.temperature, 0.2);
/// ```
#[derive(Debug, Clone)]
pub struct ClassifierConfig {
    /// Model to use for classification
    pub model: String,

    /// Temperature for generation (lower = more deterministic)
    pub temperature: f32,

    /// Minimum confidence threshold for accepting classifications
    pub confidence_threshold: f32,

    /// Enable detection of multiple intents
    pub enable_multi_intent: bool,

    /// Cache size (for Phase 2)
    pub cache_size: usize,

    /// Cache TTL in seconds (for Phase 2)
    pub cache_ttl_seconds: u64,

    /// Enable metrics collection (for Phase 4)
    pub enable_metrics: bool,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            model: "llama2:latest".to_string(),
            temperature: 0.3,
            confidence_threshold: 0.6,
            enable_multi_intent: false,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            enable_metrics: false,
        }
    }
}

impl ClassifierConfig {
    /// Set the model to use
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set the confidence threshold
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable multi-intent detection
    pub fn with_multi_intent(mut self, enabled: bool) -> Self {
        self.enable_multi_intent = enabled;
        self
    }

    /// Enable or disable metrics
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }
}

/// Intent classifier for determining documentation type
///
/// Uses AI models to classify queries according to the Diataxis framework.
#[derive(Debug)]
pub struct IntentClassifier {
    config: ClassifierConfig,
    client: Arc<OllamaClient>,
}

impl IntentClassifier {
    /// Create a new intent classifier
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the classifier
    /// * `client` - Ollama client for AI operations
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ai::intent_classifier::{IntentClassifier, ClassifierConfig};
    /// use xze_core::ai::client::OllamaClient;
    /// use std::sync::Arc;
    ///
    /// let config = ClassifierConfig::default();
    /// let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// let classifier = IntentClassifier::new(config, client);
    /// ```
    pub fn new(config: ClassifierConfig, client: Arc<OllamaClient>) -> Self {
        Self { config, client }
    }

    /// Classify a query to determine its documentation intent
    ///
    /// # Arguments
    ///
    /// * `query` - The query or context to classify
    ///
    /// # Returns
    ///
    /// Returns a `ClassificationResult` with the primary intent, confidence, and reasoning.
    ///
    /// # Errors
    ///
    /// Returns `ClassificationError` if:
    /// - Query is empty
    /// - AI service is unavailable
    /// - Response cannot be parsed
    /// - Confidence is below threshold
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ai::intent_classifier::{IntentClassifier, ClassifierConfig};
    /// # use xze_core::ai::client::OllamaClient;
    /// # use std::sync::Arc;
    /// # async fn example() -> xze_core::error::Result<()> {
    /// # let config = ClassifierConfig::default();
    /// # let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// # let classifier = IntentClassifier::new(config, client);
    /// let result = classifier.classify("How do I configure logging?").await?;
    /// println!("Intent: {}", result.primary_intent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify(&self, query: &str) -> Result<ClassificationResult> {
        // Validate input
        if query.trim().is_empty() {
            return Err(ClassificationError::EmptyQuery.into());
        }

        let start = Instant::now();

        debug!("Classifying query: {}", query);

        // Build prompt
        let prompt = self.build_classification_prompt(query);

        // Call AI service
        let response = self.generate_classification(&prompt).await?;

        // Parse response
        let mut result = self.parse_classification_response(&response)?;

        // Check confidence threshold
        if result.confidence.value() < self.config.confidence_threshold {
            warn!(
                "Classification confidence {:.2} below threshold {:.2}",
                result.confidence.value(),
                self.config.confidence_threshold
            );
            return Err(ClassificationError::LowConfidence {
                actual: result.confidence.value(),
                threshold: self.config.confidence_threshold,
            }
            .into());
        }

        // Add metadata
        let duration = start.elapsed().as_millis() as u64;
        result.metadata = ClassificationMetadata::new(self.config.model.clone())
            .set_duration(duration)
            .set_cached(false);

        info!(
            "Classified as {} with {:.1}% confidence in {}ms",
            result.primary_intent,
            result.confidence.percentage(),
            duration
        );

        Ok(result)
    }

    /// Classify multiple queries in batch
    ///
    /// # Arguments
    ///
    /// * `queries` - Slice of queries to classify
    ///
    /// # Returns
    ///
    /// Returns a vector of classification results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ai::intent_classifier::{IntentClassifier, ClassifierConfig};
    /// # use xze_core::ai::client::OllamaClient;
    /// # use std::sync::Arc;
    /// # async fn example() -> xze_core::error::Result<()> {
    /// # let config = ClassifierConfig::default();
    /// # let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
    /// # let classifier = IntentClassifier::new(config, client);
    /// let queries = vec!["How to install?", "What is the architecture?"];
    /// let results = classifier.classify_batch(&queries).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify_batch(&self, queries: &[&str]) -> Result<Vec<ClassificationResult>> {
        let mut results = Vec::with_capacity(queries.len());

        for query in queries {
            match self.classify(query).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Failed to classify query '{}': {}", query, e);
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Build the classification prompt using Diataxis framework
    fn build_classification_prompt(&self, query: &str) -> String {
        let multi_intent_instruction = if self.config.enable_multi_intent {
            "If multiple intents are present, list them in order of relevance."
        } else {
            ""
        };

        format!(
            r#"You are an expert in technical documentation classification using the Diataxis framework.

The Diataxis framework categorizes documentation into four types:

1. **Tutorial**: Learning-oriented documentation that teaches through hands-on lessons
   - Purpose: Help beginners learn by doing
   - Characteristics: Step-by-step, prescriptive, beginner-friendly
   - Example queries: "Getting started with...", "First steps...", "Learn..."

2. **HowTo**: Task-oriented documentation that solves specific problems
   - Purpose: Guide users to accomplish a specific goal
   - Characteristics: Goal-oriented, practical, assumes some knowledge
   - Example queries: "How to...", "How do I...", "Configure...", "Setup..."

3. **Reference**: Information-oriented documentation with technical specifications
   - Purpose: Provide accurate technical information
   - Characteristics: Structured, detailed, factual, comprehensive
   - Example queries: "API documentation", "Configuration options", "What parameters..."

4. **Explanation**: Understanding-oriented documentation that clarifies concepts
   - Purpose: Help users understand why and how things work
   - Characteristics: Conceptual, theoretical, big-picture oriented
   - Example queries: "Why does...", "What is the architecture...", "Explain..."

Classify the following query into one of these categories:

Query: "{}"

{}

Provide your classification in the following format:

Intent: <tutorial|howto|reference|explanation>
Confidence: <0.0-1.0>
Reasoning: <brief explanation>

Be precise and only respond with the format above."#,
            query, multi_intent_instruction
        )
    }

    /// Generate classification using AI model
    async fn generate_classification(&self, prompt: &str) -> Result<String> {
        let request = GenerateRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(GenerateOptions {
                temperature: Some(self.config.temperature),
                num_predict: Some(512),
                top_k: None,
                top_p: None,
            }),
        };

        self.client
            .generate(request)
            .await
            .map_err(|e| ClassificationError::ServiceUnavailable(e.to_string()).into())
    }

    /// Parse the AI response into a classification result
    fn parse_classification_response(&self, response: &str) -> Result<ClassificationResult> {
        debug!("Parsing classification response: {}", response);

        // Extract intent
        let intent = self.extract_intent(response)?;

        // Extract confidence
        let confidence = self.extract_confidence(response)?;

        // Extract reasoning
        let reasoning = self.extract_reasoning(response)?;

        Ok(ClassificationResult::new(intent, confidence, reasoning))
    }

    /// Extract intent from response
    fn extract_intent(&self, response: &str) -> Result<DiataxisIntent> {
        let intent_re = Regex::new(r"(?i)Intent:\s*(\w+)").unwrap();

        if let Some(captures) = intent_re.captures(response) {
            if let Some(intent_str) = captures.get(1) {
                if let Some(intent) = DiataxisIntent::parse(intent_str.as_str()) {
                    return Ok(intent);
                }
                return Err(
                    ClassificationError::InvalidIntent(intent_str.as_str().to_string()).into(),
                );
            }
        }

        // Fallback: search for intent keywords in the response
        let lower = response.to_lowercase();
        if lower.contains("tutorial") {
            Ok(DiataxisIntent::Tutorial)
        } else if lower.contains("howto") || lower.contains("how-to") || lower.contains("how to") {
            Ok(DiataxisIntent::HowTo)
        } else if lower.contains("reference") {
            Ok(DiataxisIntent::Reference)
        } else if lower.contains("explanation") {
            Ok(DiataxisIntent::Explanation)
        } else {
            Err(ClassificationError::ParseError("Could not extract intent".to_string()).into())
        }
    }

    /// Extract confidence score from response
    fn extract_confidence(&self, response: &str) -> Result<Confidence> {
        let conf_re = Regex::new(r"(?i)Confidence:\s*([0-9.]+)").unwrap();

        if let Some(captures) = conf_re.captures(response) {
            if let Some(conf_str) = captures.get(1) {
                if let Ok(value) = conf_str.as_str().parse::<f32>() {
                    return Ok(Confidence::new(value));
                }
            }
        }

        // Default to medium confidence if not specified
        warn!("Could not extract confidence, defaulting to 0.7");
        Ok(Confidence::new(0.7))
    }

    /// Extract reasoning from response
    fn extract_reasoning(&self, response: &str) -> Result<String> {
        let reasoning_re = Regex::new(r"(?i)Reasoning:\s*(.+?)(?:\n|$)").unwrap();

        if let Some(captures) = reasoning_re.captures(response) {
            if let Some(reasoning) = captures.get(1) {
                return Ok(reasoning.as_str().trim().to_string());
            }
        }

        // Return full response as reasoning if pattern not found
        Ok(response.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classifier_config_default() {
        let config = ClassifierConfig::default();
        assert_eq!(config.model, "llama2:latest");
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.confidence_threshold, 0.6);
        assert!(!config.enable_multi_intent);
    }

    #[test]
    fn test_classifier_config_builder() {
        let config = ClassifierConfig::default()
            .with_model("mistral")
            .with_temperature(0.2)
            .with_confidence_threshold(0.8)
            .with_multi_intent(true)
            .with_metrics(true);

        assert_eq!(config.model, "mistral");
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.confidence_threshold, 0.8);
        assert!(config.enable_multi_intent);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_temperature_clamping() {
        let config = ClassifierConfig::default().with_temperature(5.0);
        assert_eq!(config.temperature, 2.0);

        let config = ClassifierConfig::default().with_temperature(-1.0);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_confidence_threshold_clamping() {
        let config = ClassifierConfig::default().with_confidence_threshold(1.5);
        assert_eq!(config.confidence_threshold, 1.0);

        let config = ClassifierConfig::default().with_confidence_threshold(-0.5);
        assert_eq!(config.confidence_threshold, 0.0);
    }

    #[test]
    fn test_build_classification_prompt() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        let prompt = classifier.build_classification_prompt("How do I install this?");

        assert!(prompt.contains("Diataxis"));
        assert!(prompt.contains("Tutorial"));
        assert!(prompt.contains("HowTo"));
        assert!(prompt.contains("Reference"));
        assert!(prompt.contains("Explanation"));
        assert!(prompt.contains("How do I install this?"));
    }

    #[test]
    fn test_extract_intent_from_response() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        let response = "Intent: tutorial\nConfidence: 0.85\nReasoning: Step-by-step guide";
        let intent = classifier.extract_intent(response).unwrap();
        assert_eq!(intent, DiataxisIntent::Tutorial);

        let response = "Intent: HowTo\nConfidence: 0.9\nReasoning: Task-oriented";
        let intent = classifier.extract_intent(response).unwrap();
        assert_eq!(intent, DiataxisIntent::HowTo);
    }

    #[test]
    fn test_extract_confidence_from_response() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        let response = "Intent: tutorial\nConfidence: 0.85\nReasoning: Test";
        let confidence = classifier.extract_confidence(response).unwrap();
        assert_eq!(confidence.value(), 0.85);

        let response = "Intent: howto\nConfidence: 0.95\nReasoning: Test";
        let confidence = classifier.extract_confidence(response).unwrap();
        assert_eq!(confidence.value(), 0.95);
    }

    #[test]
    fn test_extract_reasoning_from_response() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        let response =
            "Intent: tutorial\nConfidence: 0.85\nReasoning: This is a step-by-step guide";
        let reasoning = classifier.extract_reasoning(response).unwrap();
        assert_eq!(reasoning, "This is a step-by-step guide");
    }

    #[test]
    fn test_parse_classification_response() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        let response = "Intent: tutorial\nConfidence: 0.85\nReasoning: Step-by-step instructions for beginners";
        let result = classifier.parse_classification_response(response).unwrap();

        assert_eq!(result.primary_intent, DiataxisIntent::Tutorial);
        assert_eq!(result.confidence.value(), 0.85);
        assert!(result
            .reasoning
            .contains("Step-by-step instructions for beginners"));
    }

    #[test]
    fn test_extract_intent_fallback() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        // Test fallback when format is not perfect
        let response = "This query is clearly a tutorial about learning";
        let intent = classifier.extract_intent(response).unwrap();
        assert_eq!(intent, DiataxisIntent::Tutorial);

        let response = "This is a how-to guide for solving problems";
        let intent = classifier.extract_intent(response).unwrap();
        assert_eq!(intent, DiataxisIntent::HowTo);
    }

    #[test]
    fn test_extract_confidence_default() {
        let config = ClassifierConfig::default();
        let client = Arc::new(OllamaClient::new("http://localhost:11434".to_string()));
        let classifier = IntentClassifier::new(config, client);

        // Should default to 0.7 when confidence not found
        let response = "Intent: tutorial\nReasoning: Test";
        let confidence = classifier.extract_confidence(response).unwrap();
        assert_eq!(confidence.value(), 0.7);
    }
}
