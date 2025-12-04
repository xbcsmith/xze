// SPDX-License-Identifier: MIT OR Apache-2.0
//! Document ingestion and classification module

pub mod chunking;
pub mod classifier;
pub mod pipeline;

pub use chunking::{ChunkingConfig, ChunkingStrategy, ChunkingStrategyManager};
pub use classifier::{ClassificationResult, DiataxisClassifier};
pub use pipeline::{IngestionError, IngestionPipeline, IngestionResult};
