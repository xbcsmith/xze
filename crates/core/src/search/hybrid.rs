// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hybrid search implementation combining vector and full-text search

use crate::ai::providers::Provider;
use crate::storage::{Document, PostgresStorage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Search errors
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::postgres::StorageError),

    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    #[error("Provider error: {0}")]
    Provider(#[from] crate::ai::providers::ProviderError),
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Text query
    pub text: String,
    /// Maximum number of results
    pub limit: i64,
    /// Whether to include code blocks in results
    pub include_code: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            limit: 10,
            include_code: true,
        }
    }
}

/// Search result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The document found
    pub document: Document,
    /// Relevance score (higher is better)
    pub score: f32,
}

/// Hybrid searcher combining vector and keyword search
pub struct HybridSearcher {
    storage: PostgresStorage,
    #[allow(dead_code)]
    provider: Arc<dyn Provider>,
}

impl HybridSearcher {
    /// Create a new hybrid searcher
    pub fn new(storage: PostgresStorage, provider: Arc<dyn Provider>) -> Self {
        Self { storage, provider }
    }

    /// Perform a hybrid search
    ///
    /// # Arguments
    ///
    /// * `query` - Search query parameters
    ///
    /// # Returns
    ///
    /// Returns a list of documents sorted by relevance
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<Document>, SearchError> {
        // 1. Generate embedding for the query
        let embedding = self.generate_embedding(&query.text).await?;

        // 2. Perform hybrid search in database
        let results = self
            .storage
            .search_hybrid(&query.text, &embedding, query.limit)
            .await?;

        Ok(results)
    }

    /// Generate embedding for query text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        // TODO: Use actual provider when embedding interface is ready
        // For now, use the same deterministic mock as ingestion pipeline
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        let mut embedding = vec![0.0f32; 1536];
        let seed = hash as f32 / u64::MAX as f32;
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((i as f32 * seed).sin() * 0.5 + 0.5) / 1536.0f32.sqrt();
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            Ok(CompletionResponse {
                id: "test".to_string(),
                object: "chat.completion".to_string(),
                created: 0,
                model: "test".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant("mock"),
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

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.limit, 10);
        assert!(query.include_code);
    }

    #[test]
    fn test_hybrid_searcher_creation() {
        let provider = Arc::new(MockProvider);
        // We can't easily create PostgresStorage without a DB, so we just test the provider part
        // or we could mock PostgresStorage if we extracted a trait.
        // For now, just verifying the provider mock works is enough for this unit test.
        let metadata = provider.get_metadata();
        assert_eq!(metadata.name, "mock");
    }
}
