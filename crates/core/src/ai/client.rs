//! Ollama client implementation for AI operations

use crate::{
    config::ModelConfig,
    error::{Result, XzeError},
};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Ollama client for interacting with the Ollama API
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: String) -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Create a client with custom timeout
    pub fn with_timeout(base_url: String, timeout: Duration) -> Self {
        let client = ClientBuilder::new()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Get the base URL of the Ollama server
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check if Ollama server is accessible
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                warn!("Ollama health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);

        debug!("Fetching models from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai(format!(
                "Failed to list models: HTTP {}",
                response.status()
            )));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse models response: {}", e)))?;

        Ok(models_response.models)
    }

    /// Generate text using a model
    pub async fn generate(&self, request: GenerateRequest) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        debug!("Generating with model: {}", request.model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to send generate request: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai(format!(
                "Generate request failed: HTTP {}",
                response.status()
            )));
        }

        // Handle streaming response (Ollama returns JSONL)
        let response_text = response
            .text()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to read response: {}", e)))?;

        // Parse the last line of the JSONL response
        let mut generated_text = String::new();
        for line in response_text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let generate_response: GenerateResponse = serde_json::from_str(line)
                .map_err(|e| XzeError::ai(format!("Failed to parse response line: {}", e)))?;

            generated_text.push_str(&generate_response.response);

            if generate_response.done {
                break;
            }
        }

        if generated_text.is_empty() {
            return Err(XzeError::ai("No response generated"));
        }

        info!("Generated {} characters of text", generated_text.len());
        Ok(generated_text)
    }

    /// Pull a model if not available
    pub async fn pull_model(&self, model_name: &str) -> Result<()> {
        let url = format!("{}/api/pull", self.base_url);

        info!("Pulling model: {}", model_name);

        let request = PullRequest {
            name: model_name.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to pull model: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai(format!(
                "Model pull failed: HTTP {}",
                response.status()
            )));
        }

        info!("Successfully pulled model: {}", model_name);
        Ok(())
    }

    /// Check if a specific model is available
    pub async fn has_model(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model_name))
    }

    /// Ensure a model is available, pulling if necessary
    pub async fn ensure_model(&self, model_name: &str) -> Result<()> {
        if !self.has_model(model_name).await? {
            self.pull_model(model_name).await?;
        }
        Ok(())
    }

    /// Generate embeddings for text
    pub async fn embed(&self, request: EmbedRequest) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);

        debug!("Generating embeddings with model: {}", request.model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to send embed request: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai(format!(
                "Embed request failed: HTTP {}",
                response.status()
            )));
        }

        let embed_response: EmbedResponse = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse embed response: {}", e)))?;

        Ok(embed_response.embedding)
    }
}

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

/// Response from the models list endpoint
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

/// Request for text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerateOptions>,
}

/// Generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Response from text generation
#[derive(Debug, Deserialize)]
struct GenerateResponse {
    pub response: String,
    pub done: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub context: Vec<i32>,
}

/// Request for model pulling
#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
    stream: bool,
}

/// Request for embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub model: String,
    pub prompt: String,
}

/// Response from embeddings
#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            num_predict: Some(2048),
            top_k: None,
            top_p: None,
        }
    }
}

impl From<&ModelConfig> for GenerateOptions {
    fn from(config: &ModelConfig) -> Self {
        Self {
            temperature: Some(config.temperature),
            num_predict: Some(config.context_window as i32),
            top_k: None,
            top_p: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new("http://localhost:11434".to_string());
        assert_eq!(client.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_generate_options_default() {
        let options = GenerateOptions::default();
        assert_eq!(options.temperature, Some(0.7));
        assert_eq!(options.num_predict, Some(2048));
    }

    #[test]
    fn test_generate_request_serialization() {
        let request = GenerateRequest {
            model: "llama2".to_string(),
            prompt: "Hello world".to_string(),
            stream: false,
            options: Some(GenerateOptions::default()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("llama2"));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn test_model_config_to_options() {
        let config = ModelConfig {
            primary: "test".to_string(),
            fallback: vec![],
            context_window: 4096,
            temperature: 0.3,
        };

        let options = GenerateOptions::from(&config);
        assert_eq!(options.temperature, Some(0.3));
        assert_eq!(options.num_predict, Some(4096));
    }
}
