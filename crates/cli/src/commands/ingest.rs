// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ingestion command for processing documentation

use crate::commands::CliCommand;
use clap::Parser;
use futures::stream::{self, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use walkdir::WalkDir;
use xze_core::ai::providers::{OllamaConfig, OllamaProvider, Provider};
use xze_core::ingest::IngestionPipeline;
use xze_core::storage::PostgresStorage;
use xze_core::Result;

/// Ingest documentation into the knowledge base
///
/// This command scans a directory for markdown files, processes them
/// (chunking, embedding), and stores them in the vector database.
///
/// # Examples
///
/// ```bash
/// # Basic ingestion
/// xze ingest ./docs
///
/// # Dry run
/// xze ingest ./docs --dry-run
///
/// # Custom database and model
/// xze ingest ./docs --database-url postgres://... --model llama3
/// ```
#[derive(Parser, Debug, Clone)]
#[command(name = "ingest")]
#[command(about = "Ingest documentation into the knowledge base")]
pub struct IngestArgs {
    /// Path to the documentation directory
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Database connection URL
    ///
    /// PostgreSQL connection string. Can be provided via DATABASE_URL
    /// environment variable.
    #[arg(long)]
    pub database_url: String,

    /// Ollama API URL
    ///
    /// URL of the Ollama server for generating embeddings.
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Ollama model name
    ///
    /// Model to use for embeddings and chat.
    #[arg(long, default_value = "llama3")]
    pub model: String,

    /// Dry run
    ///
    /// Process files but do not write to the database.
    #[arg(long)]
    pub dry_run: bool,

    /// Concurrency limit
    ///
    /// Maximum number of concurrent file processing tasks.
    #[arg(long, default_value = "4")]
    pub concurrency: usize,
}

impl CliCommand for IngestArgs {
    async fn execute(&self) -> Result<()> {
        info!("Executing ingest command");
        info!("Path: {}", self.path.display());
        info!("Dry run: {}", self.dry_run);

        // Validate configuration
        self.validate()?;

        // Connect to database
        let storage = PostgresStorage::new(&self.database_url)
            .await
            .map_err(|e| {
                xze_core::XzeError::Generic(anyhow::anyhow!("Database connection failed: {}", e))
            })?;

        // Initialize provider
        let provider_config = OllamaConfig {
            base_url: self.ollama_url.clone(),
            timeout: 30,
        };
        let ollama_provider =
            OllamaProvider::new(provider_config, self.model.clone()).map_err(|e| {
                xze_core::XzeError::Generic(anyhow::anyhow!("Failed to create provider: {}", e))
            })?;
        let provider: Arc<dyn Provider> = Arc::new(ollama_provider);

        // Initialize pipeline
        // Note: IngestionPipeline::new takes (provider, storage)
        let pipeline = Arc::new(IngestionPipeline::new(provider, storage));

        // Execute ingestion
        info!("Starting ingestion pipeline...");

        // Scan files
        let files: Vec<PathBuf> = WalkDir::new(&self.path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "md" || ext == "markdown")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        info!("Found {} markdown files", files.len());

        let concurrency = self.concurrency;

        let results = stream::iter(files)
            .map(|file| {
                let pipeline = pipeline.clone();
                async move {
                    info!("Processing {}", file.display());
                    pipeline.ingest_file(&file).await
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        let mut processed = 0;
        let mut failed = 0;
        let mut chunks = 0;

        for res in results {
            match res {
                Ok(r) => {
                    processed += 1;
                    chunks += r.chunk_count;
                }
                Err(e) => {
                    failed += 1;
                    tracing::error!("Failed to ingest file: {}", e);
                }
            }
        }

        info!("Ingestion completed");
        info!("Processed files: {}", processed);
        info!("Generated chunks: {}", chunks);
        info!("Failed files: {}", failed);

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ingest"
    }

    fn validate(&self) -> Result<()> {
        if !self.path.exists() {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "Path does not exist: {}",
                self.path.display()
            )));
        }

        if self.database_url.is_empty() {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "DATABASE_URL must be set"
            )));
        }

        Ok(())
    }
}
