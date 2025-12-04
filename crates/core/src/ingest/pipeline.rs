// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ingestion pipeline that orchestrates classification, chunking, and storage

use super::chunking::ChunkingStrategyManager;
use super::classifier::{ClassificationError, ClassificationResult, DiataxisClassifier};
use crate::ai::providers::Provider;
use crate::storage::postgres::StorageError;
use crate::storage::{Document, PostgresStorage};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Ingestion pipeline errors
#[derive(Error, Debug)]
pub enum IngestionError {
    #[error("Classification failed: {0}")]
    Classification(#[from] ClassificationError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),
}

/// Result of ingesting a document
#[derive(Debug, Clone)]
pub struct IngestionResult {
    /// Source file path
    pub source_file: String,
    /// Classification result
    pub classification: ClassificationResult,
    /// Number of chunks created
    pub chunk_count: usize,
    /// Document IDs created
    pub document_ids: Vec<uuid::Uuid>,
}

/// Ingestion pipeline for processing documents
pub struct IngestionPipeline {
    classifier: DiataxisClassifier,
    chunking_manager: ChunkingStrategyManager,
    storage: PostgresStorage,
    #[allow(dead_code)]
    provider: Arc<dyn Provider>,
}

impl IngestionPipeline {
    /// Create a new ingestion pipeline
    ///
    /// # Arguments
    ///
    /// * `provider` - AI provider for classification and embeddings
    /// * `storage` - PostgreSQL storage
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use xze_core::ingest::IngestionPipeline;
    /// use xze_core::storage::PostgresStorage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let provider: Arc<dyn xze_core::ai::providers::Provider> = todo!();
    /// let storage = PostgresStorage::new("postgresql://localhost/xze").await?;
    /// let pipeline = IngestionPipeline::new(provider, storage);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(provider: Arc<dyn Provider>, storage: PostgresStorage) -> Self {
        Self {
            classifier: DiataxisClassifier::new(provider.clone()),
            chunking_manager: ChunkingStrategyManager::new(),
            storage,
            provider,
        }
    }

    /// Ingest a document from a file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the document file
    ///
    /// # Returns
    ///
    /// Returns an `IngestionResult` with classification and chunk information
    ///
    /// # Errors
    ///
    /// Returns `IngestionError::Io` if file cannot be read
    /// Returns `IngestionError::Classification` if classification fails
    /// Returns `IngestionError::Storage` if database operations fail
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ingest::IngestionPipeline;
    /// # async fn example(pipeline: IngestionPipeline) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = pipeline.ingest_file("docs/tutorial.md").await?;
    /// println!("Ingested {} chunks", result.chunk_count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ingest_file<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<IngestionResult, IngestionError> {
        let path = file_path.as_ref();
        let content = tokio::fs::read_to_string(path).await?;

        self.ingest_content(&content, path.to_string_lossy().to_string())
            .await
    }

    /// Ingest document content directly
    ///
    /// # Arguments
    ///
    /// * `content` - Document content
    /// * `source_file` - Source file path (for tracking)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ingest::IngestionPipeline;
    /// # async fn example(pipeline: IngestionPipeline) -> Result<(), Box<dyn std::error::Error>> {
    /// let content = "# Tutorial\n\nStep 1: Install...";
    /// let result = pipeline.ingest_content(content, "tutorial.md").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ingest_content(
        &self,
        content: &str,
        source_file: String,
    ) -> Result<IngestionResult, IngestionError> {
        // Step 1: Classify the content
        let classification = self.classifier.classify(content).await?;

        // Step 2: Chunk the content based on classification
        let chunks = self
            .chunking_manager
            .chunk(content, classification.diataxis_type)
            .map_err(|e| IngestionError::EmbeddingFailed(e.to_string()))?;

        let chunk_count = chunks.len();
        let mut document_ids = Vec::new();

        // Step 3: Process each chunk
        for (index, chunk_content) in chunks.iter().enumerate() {
            // Generate embedding for this chunk
            let embedding = self.generate_embedding(chunk_content).await?;

            // Create document
            let document = Document::new(
                source_file.clone(),
                chunk_content.clone(),
                index as i32,
                chunk_count as i32,
            )
            .with_embedding(embedding)
            .with_diataxis_type(classification.diataxis_type)
            .with_chunk_strategy(
                self.chunking_manager
                    .get_strategy(classification.diataxis_type)
                    .name()
                    .to_string(),
            );

            // Store in database
            let doc_id = self.storage.insert(document).await?;
            document_ids.push(doc_id);
        }

        Ok(IngestionResult {
            source_file,
            classification,
            chunk_count,
            document_ids,
        })
    }

    /// Generate an embedding for text content
    async fn generate_embedding(&self, content: &str) -> Result<Vec<f32>, IngestionError> {
        // For now, generate a simple mock embedding
        // In a real implementation, this would call the embedding API
        // TODO: Implement actual embedding generation using provider

        // Generate a deterministic but unique embedding based on content
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        // Create a 1536-dimensional vector (nomic-embed-text dimension)
        let mut embedding = vec![0.0f32; 1536];

        // Use hash to seed pseudo-random values
        let seed = hash as f32 / u64::MAX as f32;
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((i as f32 * seed).sin() * 0.5 + 0.5) / 1536.0f32.sqrt();
        }

        Ok(embedding)
    }

    /// Delete all documents from a source file
    ///
    /// # Arguments
    ///
    /// * `source_file` - Source file path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xze_core::ingest::IngestionPipeline;
    /// # async fn example(pipeline: IngestionPipeline) -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = pipeline.delete_source("tutorial.md").await?;
    /// println!("Deleted {} documents", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_source(&self, source_file: &str) -> Result<u64, IngestionError> {
        Ok(self.storage.delete_by_source_file(source_file).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::providers::{
        Choice, CompletionResponse, Message, ModelConfig, ProviderError, ProviderMetadata,
    };
    use async_trait::async_trait;

    struct MockEmbeddingProvider;

    #[async_trait]
    impl Provider for MockEmbeddingProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[crate::ai::providers::Tool],
            _config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            // Return a tutorial classification
            let content =
                r#"{"type": "tutorial", "confidence": 0.95, "reasoning": "Step by step guide"}"#;
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
    fn test_ingestion_result_creation() {
        use crate::storage::DiataxisType;
        use uuid::Uuid;

        let result = IngestionResult {
            source_file: "test.md".to_string(),
            classification: ClassificationResult {
                diataxis_type: DiataxisType::Tutorial,
                confidence: 0.95,
                reasoning: Some("Test".to_string()),
            },
            chunk_count: 3,
            document_ids: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
        };

        assert_eq!(result.source_file, "test.md");
        assert_eq!(result.chunk_count, 3);
        assert_eq!(result.document_ids.len(), 3);
    }

    #[test]
    fn test_mock_provider() {
        let provider = MockEmbeddingProvider;
        let metadata = provider.get_metadata();
        assert_eq!(metadata.name, "mock");
    }
}
