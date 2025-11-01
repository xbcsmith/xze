//! AI analysis service using Ollama

use crate::{config::ModelConfig, error::Result, repository::CodeStructure, XzeError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod client;
pub mod confidence;
pub mod context;
pub mod health;
pub mod intent_classifier;
pub mod intent_types;
pub mod metrics;
pub mod prompts;
pub mod validator;

pub use client::{GenerateOptions, GenerateRequest, OllamaClient};
pub use confidence::{ConfidenceScore, ConfidenceScorer, DocumentType, ScoringContext};
pub use context::{ContextManager, PromptComponent, TokenBudget};
pub use health::{CacheHealth, HealthCheck, HealthCheckResult, HealthStatus, ServiceHealth};
pub use intent_classifier::{ClassifierConfig, IntentClassifier};
pub use intent_types::{
    ClassificationError, ClassificationMetadata, ClassificationResult, Confidence, ConfidenceLevel,
    DiataxisIntent,
};
pub use metrics::ClassifierMetrics;
pub use prompts::PromptTemplateLibrary;
pub use validator::{ResponseValidator, ValidationResult};

/// AI analysis service with validation and confidence scoring
#[derive(Debug)]
pub struct AIAnalysisService {
    client: Arc<OllamaClient>,
    model_config: ModelConfig,
    prompt_templates: PromptTemplateLibrary,
    validator: ResponseValidator,
    confidence_scorer: ConfidenceScorer,
    context_manager: ContextManager,
    retry_attempts: u32,
    intent_classifier: Option<IntentClassifier>,
}

impl AIAnalysisService {
    /// Create a new AI analysis service
    pub fn new(ollama_url: String, model_config: ModelConfig) -> Self {
        let context_manager = ContextManager::new(model_config.context_window);

        Self {
            client: Arc::new(OllamaClient::new(ollama_url)),
            model_config,
            prompt_templates: PromptTemplateLibrary::new(),
            validator: ResponseValidator::new().with_min_length(100),
            confidence_scorer: ConfidenceScorer::new(),
            context_manager,
            retry_attempts: 3,
            intent_classifier: None,
        }
    }

    /// Create service with custom validator
    pub fn with_validator(mut self, validator: ResponseValidator) -> Self {
        self.validator = validator;
        self
    }

    /// Create service with custom confidence scorer
    pub fn with_confidence_scorer(mut self, scorer: ConfidenceScorer) -> Self {
        self.confidence_scorer = scorer;
        self
    }

    /// Set maximum retry attempts
    pub fn with_retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }

    /// Enable intent classification with the given configuration
    pub fn with_intent_classifier(mut self, config: ClassifierConfig) -> Self {
        self.intent_classifier = Some(IntentClassifier::new(config, Arc::clone(&self.client)));
        self
    }

    /// Classify a query to determine its documentation intent
    ///
    /// # Arguments
    ///
    /// * `query` - The query or code context to classify
    ///
    /// # Returns
    ///
    /// Returns a `ClassificationResult` containing the primary intent, confidence,
    /// and optionally secondary intents if multi-intent detection is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Intent classifier is not enabled
    /// - Classification fails
    /// - AI service is unavailable
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::ai::{AIAnalysisService, ClassifierConfig};
    /// use xze_core::config::ModelConfig;
    ///
    /// # async fn example() -> xze_core::Result<()> {
    /// let service = AIAnalysisService::new(
    ///     "http://localhost:11434".to_string(),
    ///     ModelConfig::default()
    /// ).with_intent_classifier(ClassifierConfig::default())?;
    ///
    /// let result = service.classify_query("How do I install this library?").await?;
    /// println!("Intent: {:?}", result.primary_intent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify_query(&self, query: &str) -> Result<ClassificationResult> {
        let classifier = self
            .intent_classifier
            .as_ref()
            .ok_or_else(|| XzeError::validation("Intent classifier not enabled"))?;

        classifier.classify(query).await
    }

    /// Analyze code structure and generate summary with validation
    pub async fn analyze_code_structure(
        &self,
        structure: &CodeStructure,
    ) -> Result<AnalysisResult> {
        let prompt = self.prompt_templates.code_analysis_prompt(structure);
        let context = ScoringContext::new(DocumentType::Summary);

        let response = self.generate_with_validation(&prompt, &context).await?;

        Ok(response)
    }

    /// Generate API documentation with validation
    pub async fn generate_api_documentation(&self, structure: &CodeStructure) -> Result<String> {
        let prompt = self.prompt_templates.api_documentation_prompt(structure);
        let context = ScoringContext::new(DocumentType::ApiDocs).requires_code();

        let result = self.generate_with_validation(&prompt, &context).await?;
        Ok(result.summary)
    }

    /// Generate tutorial content with validation
    pub async fn generate_tutorial(
        &self,
        structure: &CodeStructure,
        topic: &str,
    ) -> Result<String> {
        let prompt = self.prompt_templates.tutorial_prompt(structure, topic);
        let context = ScoringContext::new(DocumentType::Tutorial).requires_code();

        let result = self.generate_with_validation(&prompt, &context).await?;
        Ok(result.summary)
    }

    /// Generate how-to guide with validation
    pub async fn generate_howto(&self, structure: &CodeStructure, task: &str) -> Result<String> {
        let prompt = self.prompt_templates.howto_prompt(structure, task);
        let context = ScoringContext::new(DocumentType::HowTo).requires_code();

        let result = self.generate_with_validation(&prompt, &context).await?;
        Ok(result.summary)
    }

    /// Generate explanation documentation with validation
    pub async fn generate_explanation(
        &self,
        structure: &CodeStructure,
        concept: &str,
    ) -> Result<String> {
        let prompt = self.prompt_templates.explanation_prompt(structure, concept);
        let context = ScoringContext::new(DocumentType::Explanation);

        let result = self.generate_with_validation(&prompt, &context).await?;
        Ok(result.summary)
    }

    /// Generate text with validation, confidence scoring, and retry logic
    async fn generate_with_validation(
        &self,
        prompt: &str,
        context: &ScoringContext,
    ) -> Result<AnalysisResult> {
        // Check if prompt fits in context window
        let optimized_prompt = if !self.context_manager.fits_in_context(prompt) {
            tracing::warn!("Prompt exceeds context window, truncating");
            self.context_manager.truncate_to_fit(prompt)?
        } else {
            prompt.to_string()
        };

        let mut last_error = None;

        // Try with retries
        for attempt in 1..=self.retry_attempts {
            tracing::debug!("Generation attempt {} of {}", attempt, self.retry_attempts);

            match self.generate(&optimized_prompt).await {
                Ok(response) => {
                    // Validate response
                    let validation = self.validator.validate(&response)?;

                    if !validation.is_valid() {
                        tracing::warn!("Response validation failed: {:?}", validation.issues);
                        if attempt < self.retry_attempts {
                            last_error =
                                Some(format!("Validation failed: {}", validation.summary()));
                            continue;
                        }
                        return Err(XzeError::validation(format!(
                            "Response validation failed after {} attempts",
                            self.retry_attempts
                        )));
                    }

                    // Score confidence
                    let confidence = self.confidence_scorer.score(&response, context)?;

                    tracing::info!(
                        "Generated response with confidence: {:.2} ({})",
                        confidence.overall,
                        confidence.level()
                    );

                    // Check if confidence meets minimum threshold
                    if confidence.overall < 0.4 {
                        tracing::warn!("Low confidence score: {:.2}", confidence.overall);
                        if attempt < self.retry_attempts {
                            last_error = Some(format!("Low confidence: {:.2}", confidence.overall));
                            continue;
                        }
                    }

                    return Ok(AnalysisResult {
                        summary: response,
                        confidence: confidence.overall,
                        validation: Some(validation),
                        confidence_score: Some(confidence),
                    });
                }
                Err(e) => {
                    tracing::warn!("Generation attempt {} failed: {}", attempt, e);
                    last_error = Some(e.to_string());

                    if attempt < self.retry_attempts {
                        // Exponential backoff
                        let delay = std::time::Duration::from_secs(2_u64.pow(attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(XzeError::ai(format!(
            "Failed to generate valid response after {} attempts: {}",
            self.retry_attempts,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    /// Generate text using the configured model with fallback
    async fn generate(&self, prompt: &str) -> Result<String> {
        // Try primary model
        match self
            .generate_with_model(&self.model_config.primary, prompt)
            .await
        {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("Primary model failed: {}, trying fallback", e);

                // Try fallback models
                for fallback_model in &self.model_config.fallback {
                    match self.generate_with_model(fallback_model, prompt).await {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            tracing::warn!("Fallback model {} failed: {}", fallback_model, e);
                            continue;
                        }
                    }
                }

                Err(XzeError::ai("All models failed to generate response"))
            }
        }
    }

    /// Generate with a specific model
    async fn generate_with_model(&self, model: &str, prompt: &str) -> Result<String> {
        let request = client::GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options: Some(client::GenerateOptions {
                temperature: Some(self.model_config.temperature),
                num_predict: Some(self.model_config.context_window as i32),
                top_k: None,
                top_p: None,
            }),
        };

        self.client.generate(request).await
    }

    /// Check if a model is available
    pub async fn check_model_availability(&self, model: &str) -> Result<bool> {
        self.client
            .list_models()
            .await
            .map(|models| models.iter().any(|m| m.name == model))
    }

    /// Generate text with a simple prompt (utility method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String> {
        self.generate(prompt).await
    }
}

/// Analysis result with validation and confidence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Generated text
    pub summary: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Validation result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationResult>,
    /// Detailed confidence scoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_score: Option<ConfidenceScore>,
}

impl AnalysisResult {
    /// Check if result meets quality thresholds
    pub fn is_high_quality(&self) -> bool {
        self.confidence >= 0.7
            && self
                .validation
                .as_ref()
                .map(|v| v.is_valid())
                .unwrap_or(true)
    }

    /// Get a quality summary
    pub fn quality_summary(&self) -> String {
        format!(
            "Confidence: {:.1}%, Valid: {}",
            self.confidence * 100.0,
            self.validation
                .as_ref()
                .map(|v| v.is_valid())
                .unwrap_or(true)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_service_creation() {
        let config = ModelConfig {
            primary: "test-model".to_string(),
            fallback: vec![],
            context_window: 4096,
            temperature: 0.3,
        };

        let service = AIAnalysisService::new("http://localhost:11434".to_string(), config);
        assert_eq!(service.model_config.primary, "test-model");
    }

    #[test]
    fn test_generate_request_serialization() {
        let request = client::GenerateRequest {
            model: "test".to_string(),
            prompt: "Hello".to_string(),
            stream: false,
            options: Some(client::GenerateOptions {
                temperature: Some(0.7),
                num_predict: Some(100),
                top_k: None,
                top_p: None,
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_analysis_result_quality() {
        let high_quality = AnalysisResult {
            summary: "Good result".to_string(),
            confidence: 0.85,
            validation: Some(ValidationResult {
                valid: true,
                issues: vec![],
                warnings: vec![],
                length: 100,
                has_code_blocks: true,
                section_count: 5,
            }),
            confidence_score: None,
        };

        assert!(high_quality.is_high_quality());

        let low_quality = AnalysisResult {
            summary: "Poor result".to_string(),
            confidence: 0.3,
            validation: None,
            confidence_score: None,
        };

        assert!(!low_quality.is_high_quality());
    }
}
