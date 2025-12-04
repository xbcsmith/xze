// SPDX-License-Identifier: MIT OR Apache-2.0
//! LLM-based reranking of search results

use crate::ai::providers::{Message, Provider};
use crate::storage::Document;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Reranking errors
#[derive(Error, Debug)]
pub enum RerankError {
    #[error("Provider error: {0}")]
    Provider(#[from] crate::ai::providers::ProviderError),

    #[error("Failed to parse reranking response: {0}")]
    ParseError(String),
}

/// Reranking result for a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Original index in the input list
    pub index: usize,
    /// Relevance score (0.0 to 1.0)
    pub score: f32,
    /// Reasoning for the score
    pub reasoning: Option<String>,
}

/// LLM-based reranker
pub struct Reranker {
    provider: Arc<dyn Provider>,
}

impl Reranker {
    /// Create a new reranker
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Rerank a list of documents based on a query
    ///
    /// # Arguments
    ///
    /// * `query` - The search query
    /// * `documents` - List of documents to rerank
    ///
    /// # Returns
    ///
    /// Returns a new list of documents sorted by relevance score
    pub async fn rerank(
        &self,
        query: &str,
        documents: Vec<Document>,
    ) -> Result<Vec<Document>, RerankError> {
        if documents.is_empty() {
            return Ok(documents);
        }

        // 1. Build prompt with document previews
        let prompt = self.build_rerank_prompt(query, &documents);
        let messages = vec![Message::user(prompt)];

        // 2. Call AI provider
        let response = self
            .provider
            .complete(&messages, &[], &Default::default())
            .await?;

        // 3. Parse response
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| RerankError::ParseError("No content in response".to_string()))?;

        let scores = self.parse_rerank_response(content, documents.len())?;

        // 4. Reorder documents
        let mut doc_scores: Vec<(Document, f32)> = documents
            .into_iter()
            .enumerate()
            .map(|(i, doc)| {
                // Find score for this index, default to 0.0 if missing
                let score = scores
                    .iter()
                    .find(|s| s.index == i)
                    .map(|s| s.score)
                    .unwrap_or(0.0);
                (doc, score)
            })
            .collect();

        // Sort by score descending
        doc_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(doc_scores.into_iter().map(|(doc, _)| doc).collect())
    }

    fn build_rerank_prompt(&self, query: &str, documents: &[Document]) -> String {
        let mut prompt = format!(
            r#"You are a search relevance evaluator.
Query: "{}"

Analyze the relevance of the following documents to the query.
Rate each document on a scale of 0.0 (irrelevant) to 1.0 (highly relevant).

Documents:
"#,
            query
        );

        for (i, doc) in documents.iter().enumerate() {
            // Truncate content for prompt efficiency
            let preview = if doc.content.len() > 300 {
                &doc.content[..300]
            } else {
                &doc.content
            };

            prompt.push_str(&format!(
                "\n[Document {}]\nTitle: {}\nContent: {}...\n",
                i,
                doc.title.as_deref().unwrap_or("Untitled"),
                preview
            ));
        }

        prompt.push_str(
            r#"
Respond ONLY with a valid JSON array of objects in this format:
[
  {"index": 0, "score": 0.95, "reasoning": "Exact match for query"},
  {"index": 1, "score": 0.1, "reasoning": "Unrelated topic"}
]
JSON Response:"#,
        );

        prompt
    }

    fn parse_rerank_response(
        &self,
        response: &str,
        expected_count: usize,
    ) -> Result<Vec<RerankResult>, RerankError> {
        // Extract JSON
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let results: Vec<RerankResult> = serde_json::from_str(json_str).map_err(|e| {
            RerankError::ParseError(format!(
                "Failed to parse JSON: {}. Response: {}",
                e, response
            ))
        })?;

        // Validate indices
        for result in &results {
            if result.index >= expected_count {
                return Err(RerankError::ParseError(format!(
                    "Invalid index {} in response (max {})",
                    result.index,
                    expected_count - 1
                )));
            }
        }

        Ok(results)
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
            let content = r#"[
                {"index": 1, "score": 0.9, "reasoning": "Relevant"},
                {"index": 0, "score": 0.1, "reasoning": "Irrelevant"}
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

    #[tokio::test]
    async fn test_rerank() {
        let provider = Arc::new(MockProvider);
        let reranker = Reranker::new(provider);

        let doc1 = Document::new("doc1.md".into(), "content1".into(), 0, 1);
        let doc2 = Document::new("doc2.md".into(), "content2".into(), 0, 1);

        let documents = vec![doc1.clone(), doc2.clone()];

        let reranked = reranker.rerank("query", documents).await.unwrap();

        assert_eq!(reranked.len(), 2);
        // doc2 should be first because mock returns score 0.9 for index 1
        assert_eq!(reranked[0].source_file, "doc2.md");
        assert_eq!(reranked[1].source_file, "doc1.md");
    }
}
