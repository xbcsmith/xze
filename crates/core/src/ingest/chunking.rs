// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chunking strategies for different Diataxis document types

use crate::storage::DiataxisType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chunking errors
#[derive(Error, Debug)]
pub enum ChunkingError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Content too short: {0}")]
    ContentTooShort(String),
}

/// Configuration for a chunking strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Minimum chunk size in characters
    pub min_chunk_size: usize,
    /// Maximum chunk size in characters
    pub max_chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap_size: usize,
    /// Whether to preserve markdown structure (headers, code blocks)
    pub preserve_structure: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 600,
            max_chunk_size: 1000,
            overlap_size: 100,
            preserve_structure: true,
        }
    }
}

/// Chunking strategy trait
pub trait ChunkingStrategy: Send + Sync {
    /// Get the name of this strategy
    fn name(&self) -> &str;

    /// Get the configuration for this strategy
    fn config(&self) -> &ChunkingConfig;

    /// Chunk the content according to this strategy
    fn chunk(&self, content: &str) -> Result<Vec<String>, ChunkingError>;
}

/// Tutorial chunking strategy - Large chunks for learning flow
pub struct TutorialChunkingStrategy {
    config: ChunkingConfig,
}

impl TutorialChunkingStrategy {
    pub fn new() -> Self {
        Self {
            config: ChunkingConfig {
                min_chunk_size: 1200,
                max_chunk_size: 1800,
                overlap_size: 200,
                preserve_structure: true,
            },
        }
    }
}

impl Default for TutorialChunkingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkingStrategy for TutorialChunkingStrategy {
    fn name(&self) -> &str {
        "tutorial"
    }

    fn config(&self) -> &ChunkingConfig {
        &self.config
    }

    fn chunk(&self, content: &str) -> Result<Vec<String>, ChunkingError> {
        basic_chunk(content, &self.config)
    }
}

/// HowTo chunking strategy - Balanced chunks for task steps
pub struct HowToChunkingStrategy {
    config: ChunkingConfig,
}

impl HowToChunkingStrategy {
    pub fn new() -> Self {
        Self {
            config: ChunkingConfig {
                min_chunk_size: 800,
                max_chunk_size: 1200,
                overlap_size: 150,
                preserve_structure: true,
            },
        }
    }
}

impl Default for HowToChunkingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkingStrategy for HowToChunkingStrategy {
    fn name(&self) -> &str {
        "howto"
    }

    fn config(&self) -> &ChunkingConfig {
        &self.config
    }

    fn chunk(&self, content: &str) -> Result<Vec<String>, ChunkingError> {
        basic_chunk(content, &self.config)
    }
}

/// Reference chunking strategy - Small, precise chunks
pub struct ReferenceChunkingStrategy {
    config: ChunkingConfig,
}

impl ReferenceChunkingStrategy {
    pub fn new() -> Self {
        Self {
            config: ChunkingConfig {
                min_chunk_size: 600,
                max_chunk_size: 1000,
                overlap_size: 100,
                preserve_structure: true,
            },
        }
    }
}

impl Default for ReferenceChunkingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkingStrategy for ReferenceChunkingStrategy {
    fn name(&self) -> &str {
        "reference"
    }

    fn config(&self) -> &ChunkingConfig {
        &self.config
    }

    fn chunk(&self, content: &str) -> Result<Vec<String>, ChunkingError> {
        basic_chunk(content, &self.config)
    }
}

/// Explanation chunking strategy - Larger context chunks
pub struct ExplanationChunkingStrategy {
    config: ChunkingConfig,
}

impl ExplanationChunkingStrategy {
    pub fn new() -> Self {
        Self {
            config: ChunkingConfig {
                min_chunk_size: 1000,
                max_chunk_size: 1500,
                overlap_size: 200,
                preserve_structure: true,
            },
        }
    }
}

impl Default for ExplanationChunkingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkingStrategy for ExplanationChunkingStrategy {
    fn name(&self) -> &str {
        "explanation"
    }

    fn config(&self) -> &ChunkingConfig {
        &self.config
    }

    fn chunk(&self, content: &str) -> Result<Vec<String>, ChunkingError> {
        basic_chunk(content, &self.config)
    }
}

/// Manages chunking strategies for different document types
pub struct ChunkingStrategyManager {
    tutorial: Box<dyn ChunkingStrategy>,
    howto: Box<dyn ChunkingStrategy>,
    reference: Box<dyn ChunkingStrategy>,
    explanation: Box<dyn ChunkingStrategy>,
}

impl Default for ChunkingStrategyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkingStrategyManager {
    /// Create a new chunking strategy manager with default strategies
    pub fn new() -> Self {
        Self {
            tutorial: Box::new(TutorialChunkingStrategy::new()),
            howto: Box::new(HowToChunkingStrategy::new()),
            reference: Box::new(ReferenceChunkingStrategy::new()),
            explanation: Box::new(ExplanationChunkingStrategy::new()),
        }
    }

    /// Get the appropriate strategy for a Diataxis type
    pub fn get_strategy(&self, diataxis_type: DiataxisType) -> &dyn ChunkingStrategy {
        match diataxis_type {
            DiataxisType::Tutorial => self.tutorial.as_ref(),
            DiataxisType::HowTo => self.howto.as_ref(),
            DiataxisType::Reference => self.reference.as_ref(),
            DiataxisType::Explanation => self.explanation.as_ref(),
        }
    }

    /// Chunk content using the appropriate strategy for the given type
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ingest::ChunkingStrategyManager;
    /// use xze_core::storage::DiataxisType;
    ///
    /// let manager = ChunkingStrategyManager::new();
    /// let content = "Step 1: Install. Step 2: Configure. Step 3: Run.".repeat(50);
    /// let chunks = manager.chunk(&content, DiataxisType::Tutorial).unwrap();
    /// assert!(!chunks.is_empty());
    /// ```
    pub fn chunk(
        &self,
        content: &str,
        diataxis_type: DiataxisType,
    ) -> Result<Vec<String>, ChunkingError> {
        let strategy = self.get_strategy(diataxis_type);
        strategy.chunk(content)
    }
}

/// Basic chunking implementation with overlap
fn basic_chunk(content: &str, config: &ChunkingConfig) -> Result<Vec<String>, ChunkingError> {
    if content.trim().is_empty() {
        return Err(ChunkingError::ContentTooShort(
            "Content cannot be empty".to_string(),
        ));
    }

    let content_len = content.len();

    // If content is smaller than max chunk size, return as single chunk
    if content_len <= config.max_chunk_size {
        return Ok(vec![content.to_string()]);
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content_len {
        // Calculate end position for this chunk
        let mut end = (start + config.max_chunk_size).min(content_len);

        // If this isn't the last chunk, try to break at a sentence or word boundary
        if end < content_len && config.preserve_structure {
            // Look for good break points: newlines, periods, or spaces
            let break_window = 100.min(config.max_chunk_size / 10);
            let search_start = end.saturating_sub(break_window);

            if let Some(last_newline) = content[search_start..end].rfind('\n') {
                end = search_start + last_newline + 1;
            } else if let Some(last_period) = content[search_start..end].rfind(". ") {
                end = search_start + last_period + 2;
            } else if let Some(last_space) = content[search_start..end].rfind(' ') {
                end = search_start + last_space + 1;
            }
        }

        // Extract the chunk
        let chunk = content[start..end].trim().to_string();

        if !chunk.is_empty() {
            chunks.push(chunk);
        }

        // Move start position with overlap
        if end >= content_len {
            break;
        }

        start = end.saturating_sub(config.overlap_size);

        // Ensure we make progress
        if start <= chunks.len() * config.min_chunk_size / 2 {
            start = end;
        }
    }

    if chunks.is_empty() {
        return Err(ChunkingError::ContentTooShort(
            "Failed to create any chunks".to_string(),
        ));
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tutorial_strategy_config() {
        let strategy = TutorialChunkingStrategy::new();
        let config = strategy.config();
        assert_eq!(config.min_chunk_size, 1200);
        assert_eq!(config.max_chunk_size, 1800);
        assert_eq!(strategy.name(), "tutorial");
    }

    #[test]
    fn test_chunking_strategy_manager() {
        let manager = ChunkingStrategyManager::new();

        let tutorial_strategy = manager.get_strategy(DiataxisType::Tutorial);
        assert_eq!(tutorial_strategy.name(), "tutorial");

        let howto_strategy = manager.get_strategy(DiataxisType::HowTo);
        assert_eq!(howto_strategy.name(), "howto");
    }

    #[test]
    fn test_basic_chunking() {
        let content = "Lorem ipsum dolor sit amet. ".repeat(100); // ~2800 chars
        let config = ChunkingConfig {
            min_chunk_size: 500,
            max_chunk_size: 1000,
            overlap_size: 100,
            preserve_structure: true,
        };

        let chunks = basic_chunk(&content, &config).unwrap();
        assert!(chunks.len() >= 2);

        // Check that chunks respect max size (with some tolerance for boundary adjustments)
        for chunk in &chunks {
            assert!(chunk.len() <= config.max_chunk_size + 200);
        }
    }

    #[test]
    fn test_small_content() {
        let content = "Short content";
        let config = ChunkingConfig::default();

        let chunks = basic_chunk(content, &config).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], content);
    }

    #[test]
    fn test_manager_chunk() {
        let manager = ChunkingStrategyManager::new();
        let content = "Step 1: First step.\n\nStep 2: Second step.\n\n".repeat(50);

        let chunks = manager.chunk(&content, DiataxisType::Tutorial).unwrap();
        assert!(!chunks.is_empty());
    }
}
