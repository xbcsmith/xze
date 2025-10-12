//! AI analysis service using Ollama

use crate::{config::ModelConfig, error::Result, repository::CodeStructure, XzeError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod client;
pub mod prompts;

pub use client::OllamaClient;
pub use prompts::PromptTemplateLibrary;

/// AI analysis service
#[derive(Debug)]
pub struct AIAnalysisService {
    client: Arc<OllamaClient>,
    model_config: ModelConfig,
    prompt_templates: PromptTemplateLibrary,
}

impl AIAnalysisService {
    /// Create a new AI analysis service
    pub fn new(ollama_url: String, model_config: ModelConfig) -> Self {
        Self {
            client: Arc::new(OllamaClient::new(ollama_url)),
            model_config,
            prompt_templates: PromptTemplateLibrary::new(),
        }
    }

    /// Analyze code structure and generate summary
    pub async fn analyze_code_structure(
        &self,
        structure: &CodeStructure,
    ) -> Result<AnalysisResult> {
        let prompt = self.prompt_templates.code_analysis_prompt(structure);

        let response = self.generate(&prompt).await?;

        Ok(AnalysisResult {
            summary: response,
            confidence: 0.8, // TODO: Calculate confidence
        })
    }

    /// Generate API documentation
    pub async fn generate_api_documentation(&self, structure: &CodeStructure) -> Result<String> {
        let prompt = self.prompt_templates.api_documentation_prompt(structure);
        self.generate(&prompt).await
    }

    /// Generate tutorial content
    pub async fn generate_tutorial(
        &self,
        structure: &CodeStructure,
        topic: &str,
    ) -> Result<String> {
        let prompt = self.prompt_templates.tutorial_prompt(structure, topic);
        self.generate(&prompt).await
    }

    /// Generate how-to guide
    pub async fn generate_howto(&self, structure: &CodeStructure, task: &str) -> Result<String> {
        let prompt = self.prompt_templates.howto_prompt(structure, task);
        self.generate(&prompt).await
    }

    /// Generate explanation documentation
    pub async fn generate_explanation(
        &self,
        structure: &CodeStructure,
        concept: &str,
    ) -> Result<String> {
        let prompt = self.prompt_templates.explanation_prompt(structure, concept);
        self.generate(&prompt).await
    }

    /// Generate text using the configured model
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

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub summary: String,
    pub confidence: f32,
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
}
