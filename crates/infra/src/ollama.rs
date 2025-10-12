//! Ollama client implementation for XZe infrastructure

use serde::{Deserialize, Serialize};
use std::time::Duration;
use xze_core::{Result, XzeError};

/// Ollama client configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            timeout: Duration::from_secs(300),
            model: "llama2".to_string(),
        }
    }
}

/// Ollama client for interacting with Ollama API
#[derive(Debug, Clone)]
pub struct OllamaClient {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(OllamaConfig::default())
    }
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap();

        Self { config, client }
    }

    /// Check if Ollama is available
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.config.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                tracing::warn!("Ollama health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Generate text using Ollama
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let request = GenerateRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let url = format!("{}/api/generate", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to send request to Ollama: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::network(format!(
                "Ollama API returned error: {}",
                response.status()
            )));
        }

        let generate_response: GenerateResponse = response
            .json()
            .await
            .map_err(|e| XzeError::network(format!("Failed to parse Ollama response: {}", e)))?;

        Ok(generate_response.response)
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to list models: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::network(format!(
                "Failed to list models: {}",
                response.status()
            )));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| XzeError::network(format!("Failed to parse models response: {}", e)))?;

        Ok(models_response.models.into_iter().map(|m| m.name).collect())
    }

    /// Check if a specific model is available
    pub async fn model_exists(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m == model_name))
    }
}

/// Generate request payload
#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Generate response payload
#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

/// Models response payload
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

/// Model information
#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert_eq!(config.model, "llama2");
    }

    #[test]
    fn test_ollama_client_creation() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(config.clone());
        assert_eq!(client.config.base_url, config.base_url);
    }

    #[test]
    fn test_generate_request_serialization() {
        let request = GenerateRequest {
            model: "test-model".to_string(),
            prompt: "test prompt".to_string(),
            stream: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-model"));
        assert!(json.contains("test prompt"));
    }
}
