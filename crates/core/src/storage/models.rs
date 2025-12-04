// SPDX-License-Identifier: MIT OR Apache-2.0
//! Data models for storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Diataxis documentation classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum DiataxisType {
    /// Learning-oriented tutorials
    Tutorial,
    /// Task-oriented how-to guides
    #[sqlx(rename = "howto")]
    HowTo,
    /// Information-oriented reference material
    Reference,
    /// Understanding-oriented explanations
    Explanation,
}

impl std::fmt::Display for DiataxisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiataxisType::Tutorial => write!(f, "tutorial"),
            DiataxisType::HowTo => write!(f, "howto"),
            DiataxisType::Reference => write!(f, "reference"),
            DiataxisType::Explanation => write!(f, "explanation"),
        }
    }
}

impl std::str::FromStr for DiataxisType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tutorial" => Ok(DiataxisType::Tutorial),
            "howto" | "how-to" | "how_to" => Ok(DiataxisType::HowTo),
            "reference" => Ok(DiataxisType::Reference),
            "explanation" => Ok(DiataxisType::Explanation),
            _ => Err(format!("Invalid Diataxis type: {}", s)),
        }
    }
}

/// Document chunk stored in the database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier
    pub id: Uuid,
    /// Source file path
    pub source_file: String,
    /// Text content
    pub content: String,
    /// Vector embedding (1536 dimensions)
    #[sqlx(skip)]
    pub embedding: Option<Vec<f32>>,
    /// Chunk index in the source document
    pub chunk_index: i32,
    /// Total number of chunks in the source document
    pub total_chunks: i32,
    /// Diataxis classification
    pub diataxis_type: Option<DiataxisType>,
    /// Chunking strategy used
    pub chunk_strategy: Option<String>,
    /// Document title
    pub title: Option<String>,
    /// Document category
    pub category: Option<String>,
    /// Keywords
    #[sqlx(skip)]
    pub keywords: Vec<String>,
    /// Number of code blocks
    pub code_blocks: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document chunk
    pub fn new(source_file: String, content: String, chunk_index: i32, total_chunks: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_file,
            content,
            embedding: None,
            chunk_index,
            total_chunks,
            diataxis_type: None,
            chunk_strategy: None,
            title: None,
            category: None,
            keywords: Vec::new(),
            code_blocks: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Set the embedding vector
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set the Diataxis classification
    pub fn with_diataxis_type(mut self, diataxis_type: DiataxisType) -> Self {
        self.diataxis_type = Some(diataxis_type);
        self
    }

    /// Set the chunking strategy
    pub fn with_chunk_strategy(mut self, strategy: String) -> Self {
        self.chunk_strategy = Some(strategy);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: DocumentMetadata) -> Self {
        self.title = metadata.title;
        self.category = metadata.category;
        self.keywords = metadata.keywords;
        self.code_blocks = metadata.code_blocks;
        self
    }
}

/// Document metadata for enrichment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document category
    pub category: Option<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Number of code blocks
    pub code_blocks: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diataxis_type_from_str() {
        assert_eq!(
            "tutorial".parse::<DiataxisType>().unwrap(),
            DiataxisType::Tutorial
        );
        assert_eq!(
            "howto".parse::<DiataxisType>().unwrap(),
            DiataxisType::HowTo
        );
        assert_eq!(
            "how-to".parse::<DiataxisType>().unwrap(),
            DiataxisType::HowTo
        );
        assert_eq!(
            "reference".parse::<DiataxisType>().unwrap(),
            DiataxisType::Reference
        );
        assert_eq!(
            "explanation".parse::<DiataxisType>().unwrap(),
            DiataxisType::Explanation
        );
    }

    #[test]
    fn test_diataxis_type_display() {
        assert_eq!(DiataxisType::Tutorial.to_string(), "tutorial");
        assert_eq!(DiataxisType::HowTo.to_string(), "howto");
        assert_eq!(DiataxisType::Reference.to_string(), "reference");
        assert_eq!(DiataxisType::Explanation.to_string(), "explanation");
    }

    #[test]
    fn test_document_builder() {
        let doc = Document::new("test.md".to_string(), "content".to_string(), 0, 1)
            .with_embedding(vec![0.1; 1536])
            .with_diataxis_type(DiataxisType::Tutorial)
            .with_chunk_strategy("tutorial".to_string());

        assert_eq!(doc.source_file, "test.md");
        assert_eq!(doc.chunk_index, 0);
        assert_eq!(doc.diataxis_type, Some(DiataxisType::Tutorial));
        assert!(doc.embedding.is_some());
    }
}
