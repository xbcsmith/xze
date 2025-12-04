// SPDX-License-Identifier: MIT OR Apache-2.0
//! Diataxis document classifier using AI providers

use crate::ai::providers::{Message, Provider};
use crate::storage::DiataxisType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Classification errors
#[derive(Error, Debug)]
pub enum ClassificationError {
    #[error("AI provider error: {0}")]
    ProviderError(String),

    #[error("Failed to parse classification response: {0}")]
    ParseError(String),

    #[error("Invalid content: {0}")]
    InvalidContent(String),
}

/// Classification result with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Classified Diataxis type
    pub diataxis_type: DiataxisType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Reasoning for the classification
    pub reasoning: Option<String>,
}

/// Diataxis classifier using AI providers
pub struct DiataxisClassifier {
    provider: Arc<dyn Provider>,
}

impl DiataxisClassifier {
    /// Create a new Diataxis classifier
    ///
    /// # Arguments
    ///
    /// * `provider` - AI provider for classification
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use xze_core::ingest::DiataxisClassifier;
    /// use xze_core::ai::providers::{Provider, ollama::OllamaProvider};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Assume we have a provider
    /// # let provider: Arc<dyn Provider> = todo!();
    /// let classifier = DiataxisClassifier::new(provider);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Classify document content into a Diataxis category
    ///
    /// # Arguments
    ///
    /// * `content` - The document content to classify
    ///
    /// # Returns
    ///
    /// Returns a `ClassificationResult` with the determined category and confidence
    ///
    /// # Errors
    ///
    /// Returns `ClassificationError::ProviderError` if AI provider fails
    /// Returns `ClassificationError::ParseError` if response cannot be parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use xze_core::ingest::DiataxisClassifier;
    /// # use xze_core::storage::DiataxisType;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider: Arc<dyn xze_core::ai::providers::Provider> = todo!();
    /// let classifier = DiataxisClassifier::new(provider);
    /// let content = "How to install XZe on your system...";
    /// let result = classifier.classify(content).await?;
    /// assert_eq!(result.diataxis_type, DiataxisType::HowTo);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn classify(
        &self,
        content: &str,
    ) -> Result<ClassificationResult, ClassificationError> {
        if content.trim().is_empty() {
            return Err(ClassificationError::InvalidContent(
                "Content cannot be empty".to_string(),
            ));
        }

        // Truncate content if too long (use first 2000 chars for classification)
        let content_sample = if content.len() > 2000 {
            &content[..2000]
        } else {
            content
        };

        let prompt = self.build_classification_prompt(content_sample);

        let messages = vec![Message::user(prompt)];

        // Call AI provider
        let response = self
            .provider
            .complete(&messages, &[], &Default::default())
            .await
            .map_err(|e| ClassificationError::ProviderError(e.to_string()))?;

        // Extract content from first choice
        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| ClassificationError::ParseError("No content in response".to_string()))?;

        self.parse_classification_response(content)
    }

    /// Build the classification prompt
    fn build_classification_prompt(&self, content: &str) -> String {
        format!(
            r#"You are a documentation classifier following the Diataxis framework.

Analyze the following documentation content and classify it into ONE of these categories:

1. **Tutorial** - Learning-oriented, step-by-step lessons to acquire basic competence
   - Helps beginners achieve small goals
   - Provides hands-on learning experience
   - Example: "Getting started with Rust" or "Your first web server"

2. **HowTo** - Task-oriented, problem-solving guides
   - Answers "How do I...?" questions
   - Provides specific recipes or solutions
   - Example: "How to configure logging" or "How to deploy to production"

3. **Reference** - Information-oriented, technical descriptions
   - Describes the machinery (APIs, configurations, commands)
   - Provides accurate technical information
   - Example: "API reference" or "Configuration options"

4. **Explanation** - Understanding-oriented, conceptual clarification
   - Explains the "why" behind things
   - Provides background and context
   - Example: "Why we chose this architecture" or "Understanding async/await"

Content to classify:
```
{}
```

Respond ONLY with a valid JSON object in this exact format:
{{
  "type": "tutorial" | "howto" | "reference" | "explanation",
  "confidence": 0.95,
  "reasoning": "Brief explanation of why this classification was chosen"
}}

JSON Response:"#,
            content
        )
    }

    /// Parse the AI response into a ClassificationResult
    fn parse_classification_response(
        &self,
        response: &str,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Try to extract JSON from the response (it might have extra text)
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        #[derive(Deserialize)]
        struct RawClassification {
            #[serde(rename = "type")]
            type_str: String,
            confidence: f32,
            reasoning: Option<String>,
        }

        let raw: RawClassification = serde_json::from_str(json_str).map_err(|e| {
            ClassificationError::ParseError(format!(
                "Failed to parse JSON: {}. Response: {}",
                e, response
            ))
        })?;

        let diataxis_type = raw.type_str.parse::<DiataxisType>().map_err(|e| {
            ClassificationError::ParseError(format!("Invalid Diataxis type: {}", e))
        })?;

        // Clamp confidence to [0, 1]
        let confidence = raw.confidence.clamp(0.0, 1.0);

        Ok(ClassificationResult {
            diataxis_type,
            confidence,
            reasoning: raw.reasoning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_classification_response() {
        let classifier = DiataxisClassifier {
            provider: Arc::new(MockProvider),
        };

        let response =
            r#"{"type": "tutorial", "confidence": 0.95, "reasoning": "Step-by-step guide"}"#;
        let result = classifier.parse_classification_response(response).unwrap();

        assert_eq!(result.diataxis_type, DiataxisType::Tutorial);
        assert!((result.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_parse_classification_with_extra_text() {
        let classifier = DiataxisClassifier {
            provider: Arc::new(MockProvider),
        };

        let response = r#"Here's my analysis: {"type": "howto", "confidence": 0.88, "reasoning": "Task-oriented"} and that's it."#;
        let result = classifier.parse_classification_response(response).unwrap();

        assert_eq!(result.diataxis_type, DiataxisType::HowTo);
    }

    // Mock provider for testing
    use crate::ai::providers::{
        Choice, CompletionResponse, Message, ModelConfig, ProviderError, ProviderMetadata,
    };
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[crate::ai::providers::Tool],
            _config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            let content =
                r#"{"type": "tutorial", "confidence": 0.9, "reasoning": "Mock"}"#.to_string();
            Ok(CompletionResponse {
                id: "test-id".to_string(),
                object: "chat.completion".to_string(),
                created: 0,
                model: "mock-model".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant(&content),
                    finish_reason: Some("stop".to_string()),
                }],
                usage: None,
            })
        }

        async fn complete_fast(
            &self,
            messages: &[Message],
            tools: &[crate::ai::providers::Tool],
            config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            self.complete(messages, tools, config).await
        }

        fn get_metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                name: "mock".to_string(),
                supports_streaming: false,
                supports_tools: false,
                supports_vision: false,
                max_tokens: 4096,
            }
        }
    }

    #[tokio::test]
    async fn test_classify_with_mock() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let classifier = DiataxisClassifier::new(provider);

        let content = "Step 1: Install Rust. Step 2: Create a new project.";
        let result = classifier.classify(content).await.unwrap();

        assert_eq!(result.diataxis_type, DiataxisType::Tutorial);
        assert!(result.confidence > 0.0);
    }
}
