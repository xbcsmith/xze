// SPDX-License-Identifier: MIT OR Apache-2.0
//! Search pipeline orchestrating hybrid search, reranking, and context expansion

use super::expansion::{ContextExpander, ExpandedDocument};
use super::hybrid::{HybridSearcher, SearchQuery};
use super::rerank::Reranker;
use crate::ai::providers::Provider;
use crate::storage::PostgresStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Pipeline errors
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Search error: {0}")]
    Search(#[from] super::hybrid::SearchError),

    #[error("Rerank error: {0}")]
    Rerank(#[from] super::rerank::RerankError),

    #[error("Expansion error: {0}")]
    Expansion(#[from] super::expansion::ExpansionError),
}

/// Configuration for the search pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPipelineConfig {
    /// Maximum number of results from hybrid search
    pub limit: i64,
    /// Whether to perform LLM reranking
    pub rerank: bool,
    /// Whether to expand context
    pub expand_context: bool,
    /// Number of chunks to expand (if enabled)
    pub context_window: i32,
}

impl Default for SearchPipelineConfig {
    fn default() -> Self {
        Self {
            limit: 10,
            rerank: true,
            expand_context: true,
            context_window: 1,
        }
    }
}

/// Orchestrates the search process
pub struct SearchPipeline {
    hybrid_searcher: HybridSearcher,
    reranker: Reranker,
    context_expander: ContextExpander,
}

impl SearchPipeline {
    /// Create a new search pipeline
    pub fn new(storage: PostgresStorage, provider: Arc<dyn Provider>) -> Self {
        Self {
            hybrid_searcher: HybridSearcher::new(storage.clone(), provider.clone()),
            reranker: Reranker::new(provider),
            context_expander: ContextExpander::new(storage),
        }
    }

    /// Execute the search pipeline
    ///
    /// # Arguments
    ///
    /// * `query_text` - The search query
    /// * `config` - Pipeline configuration
    ///
    /// # Returns
    ///
    /// Returns a list of `ExpandedDocument`s (even if expansion is disabled, for consistency)
    pub async fn search(
        &self,
        query_text: &str,
        config: &SearchPipelineConfig,
    ) -> Result<Vec<ExpandedDocument>, PipelineError> {
        // 1. Hybrid Search
        let search_query = SearchQuery {
            text: query_text.to_string(),
            limit: config.limit,
            include_code: true,
        };

        let mut documents = self.hybrid_searcher.search(search_query).await?;

        // 2. Reranking (optional)
        if config.rerank && !documents.is_empty() {
            documents = self.reranker.rerank(query_text, documents).await?;
        }

        // 3. Context Expansion (optional)
        let expanded_docs = if config.expand_context {
            self.context_expander
                .expand(documents, config.context_window)
                .await?
        } else {
            // If expansion is disabled, wrap original docs
            documents
                .into_iter()
                .map(|doc| ExpandedDocument {
                    expanded_content: doc.content.clone(),
                    context_chunks: vec![doc.clone()],
                    original: doc,
                })
                .collect()
        };

        Ok(expanded_docs)
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
            // Return valid JSON for reranking
            let content = r#"[
                {"index": 0, "score": 0.9, "reasoning": "Relevant"}
            ]"#;

            Ok(CompletionResponse {
                id: "test".to_string(),
                object: "chat.completion".to_string(),
                created: 0,
                model: "test".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant(content),
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
    fn test_pipeline_config_default() {
        let config = SearchPipelineConfig::default();
        assert_eq!(config.limit, 10);
        assert!(config.rerank);
        assert!(config.expand_context);
    }

    #[test]
    fn test_pipeline_creation() {
        // Just verify it compiles and constructs
        // We can't run full search without DB
        let provider = Arc::new(MockProvider);
        // Mock storage creation isn't possible here easily, so we skip full instantiation test
        // unless we mock PostgresStorage which we can't.
        // But we can test the config struct.
    }
}
