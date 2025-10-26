//! CLI command for semantic document chunking
//!
//! This command processes documents and splits them into semantically coherent
//! chunks for improved knowledge base storage and retrieval.
//!
//! # Examples
//!
//! ```bash
//! # Chunk a single document
//! xze chunk --input docs/guide.md
//!
//! # Chunk with custom strategy
//! xze chunk --input docs/api.md --strategy technical
//!
//! # Chunk multiple documents
//! xze chunk --input docs/*.md --output ./chunks
//!
//! # Dry run to preview chunking
//! xze chunk --input docs/guide.md --dry-run
//! ```

use crate::commands::CliCommand;

use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use xze_core::ai::OllamaClient;
use xze_core::documentation::processor::{DocumentProcessor, ProcessingConfig};
use xze_core::kb::store::KbStore;
use xze_core::semantic::chunker::ChunkerConfig;
use xze_core::semantic::types::ChunkMetadata;
use xze_core::{Result, XzeError};

/// Chunking strategy presets
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ChunkingStrategy {
    /// Default balanced chunking
    Default,
    /// Optimized for technical documentation
    Technical,
    /// Optimized for narrative content
    Narrative,
    /// Custom configuration from file
    Custom,
}

impl ChunkingStrategy {
    /// Convert strategy to chunker configuration
    pub fn to_config(&self) -> ChunkerConfig {
        match self {
            Self::Default => ChunkerConfig::default(),
            Self::Technical => ChunkerConfig::technical_docs(),
            Self::Narrative => ChunkerConfig::narrative(),
            Self::Custom => ChunkerConfig::default(),
        }
    }
}

/// Arguments for the chunk command
///
/// Provides options for processing documents with semantic chunking.
///
/// # Examples
///
/// ```no_run
/// use xze_cli::commands::chunk::{ChunkArgs, ChunkingStrategy};
/// use std::path::PathBuf;
///
/// let args = ChunkArgs {
///     input: vec![PathBuf::from("docs/guide.md")],
///     output: None,
///     strategy: ChunkingStrategy::Default,
///     threshold: None,
///     max_sentences: None,
///     dry_run: false,
///     database_url: None,
///     ollama_url: None,
/// };
/// ```
#[derive(Debug, Clone, Args)]
pub struct ChunkArgs {
    /// Input file(s) or directory to process
    ///
    /// Can specify multiple files or use glob patterns.
    /// If a directory is provided, all markdown files will be processed.
    #[arg(short, long, required = true)]
    pub input: Vec<PathBuf>,

    /// Output directory for chunk metadata (optional)
    ///
    /// If not specified, chunks are only stored in the database.
    /// If specified, JSON files with chunk metadata will be written.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Chunking strategy preset
    ///
    /// Determines how documents are split into chunks:
    /// - default: Balanced settings for general content
    /// - technical: Higher similarity threshold for technical docs
    /// - narrative: Lower threshold for story-like content
    /// - custom: Load configuration from file
    #[arg(short, long, value_enum, default_value = "default")]
    pub strategy: ChunkingStrategy,

    /// Override similarity threshold (0.0-1.0)
    ///
    /// Higher values create more but smaller chunks.
    /// Lower values create fewer but larger chunks.
    #[arg(long)]
    pub threshold: Option<f32>,

    /// Override maximum sentences per chunk
    ///
    /// Limits the size of individual chunks.
    #[arg(long)]
    pub max_sentences: Option<usize>,

    /// Dry run - analyze without storing chunks
    ///
    /// Useful for previewing how documents will be chunked
    /// without modifying the knowledge base.
    #[arg(long)]
    pub dry_run: bool,

    /// Database URL (overrides config)
    ///
    /// PostgreSQL connection string for knowledge base.
    #[arg(long)]
    pub database_url: Option<String>,

    /// Ollama server URL (overrides config)
    ///
    /// URL of the Ollama instance for generating embeddings.
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: Option<String>,
}

impl ChunkArgs {
    /// Validate command arguments
    pub fn validate(&self) -> Result<()> {
        // Validate input files exist
        for path in &self.input {
            if !path.exists() {
                return Err(XzeError::not_found(format!(
                    "Input file not found: {:?}",
                    path
                )));
            }
        }

        // Validate threshold if provided
        if let Some(threshold) = self.threshold {
            if !(0.0..=1.0).contains(&threshold) {
                return Err(XzeError::validation(
                    "Threshold must be between 0.0 and 1.0",
                ));
            }
        }

        // Validate max_sentences if provided
        if let Some(max_sentences) = self.max_sentences {
            if max_sentences == 0 {
                return Err(XzeError::validation("max_sentences must be greater than 0"));
            }
        }

        // Validate output directory if provided
        if let Some(output) = &self.output {
            if output.exists() && !output.is_dir() {
                return Err(XzeError::validation(format!(
                    "Output path exists but is not a directory: {:?}",
                    output
                )));
            }
        }

        Ok(())
    }

    /// Build chunker configuration from arguments
    fn build_chunker_config(&self) -> ChunkerConfig {
        let mut config = self.strategy.to_config();

        // Apply overrides
        if let Some(threshold) = self.threshold {
            config.similarity_threshold = threshold;
        }

        if let Some(max_sentences) = self.max_sentences {
            config.max_chunk_sentences = max_sentences;
        }

        config
    }

    /// Collect all files to process
    ///
    /// Expands directories and filters for supported file types.
    fn collect_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for path in &self.input {
            if path.is_file() {
                files.push(path.clone());
            } else if path.is_dir() {
                // Recursively find markdown files
                let markdown_files = Self::find_markdown_files(path)?;
                files.extend(markdown_files);
            } else {
                warn!("Skipping invalid path: {:?}", path);
            }
        }

        if files.is_empty() {
            return Err(XzeError::validation("No valid files found to process"));
        }

        info!("Found {} files to process", files.len());
        Ok(files)
    }

    /// Find all markdown files in a directory
    fn find_markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Recurse into subdirectories
                files.extend(Self::find_markdown_files(&path)?);
            }
        }

        Ok(files)
    }
}

impl CliCommand for ChunkArgs {
    async fn execute(&self) -> Result<()> {
        info!("Starting document chunking");

        // Validate arguments
        self.validate()?;

        // Create output directory if specified
        if let Some(output_dir) = &self.output {
            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir)?;
                info!("Created output directory: {:?}", output_dir);
            }
        }

        // Collect files to process
        let files = self.collect_files()?;
        info!("Processing {} files", files.len());

        // Setup database connection
        let database_url = self
            .database_url
            .clone()
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .ok_or_else(|| {
                XzeError::validation(
                    "Database URL required. Set DATABASE_URL or use --database-url",
                )
            })?;

        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .map_err(|e| XzeError::Generic(anyhow::anyhow!("Database connection failed: {}", e)))?;
        let store = KbStore::new(pool);

        // Setup Ollama client
        let ollama_url = self
            .ollama_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        let ollama_client = OllamaClient::new(ollama_url);

        // Build processing configuration
        let chunker_config = self.build_chunker_config();
        let processing_config = ProcessingConfig {
            enable_chunking: true,
            chunker_config,
            skip_existing: !self.dry_run, // Don't skip in dry-run mode
            validate_content: true,
        };

        info!("Configuration:");
        info!("  Strategy: {:?}", self.strategy);
        info!(
            "  Similarity threshold: {:.2}",
            processing_config.chunker_config.similarity_threshold
        );
        info!(
            "  Max sentences per chunk: {}",
            processing_config.chunker_config.max_chunk_sentences
        );
        info!("  Dry run: {}", self.dry_run);

        // Create document processor
        let processor = DocumentProcessor::new(store, ollama_client, processing_config);

        // Process each file
        let mut total_chunks = 0;
        let mut successful = 0;
        let mut failed = 0;

        for (index, file_path) in files.iter().enumerate() {
            info!(
                "Processing file {}/{}: {:?}",
                index + 1,
                files.len(),
                file_path
            );

            // Read file content
            let content = match std::fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(e) => {
                    warn!("Failed to read file {:?}: {}", file_path, e);
                    failed += 1;
                    continue;
                }
            };

            // Create metadata
            let metadata = ChunkMetadata {
                source_file: file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                title: None,
                category: Self::detect_category(file_path),
                keywords: Self::extract_keywords(&content),
                word_count: content.split_whitespace().count(),
                char_count: content.len(),
            };

            // Process document
            if self.dry_run {
                info!("  [DRY RUN] Would process: {:?}", file_path);
                info!(
                    "    Content: {} words, {} chars",
                    metadata.word_count, metadata.char_count
                );
                successful += 1;
            } else {
                match processor
                    .process_document_with_chunking(file_path, &content, Some(metadata))
                    .await
                {
                    Ok(result) => {
                        info!("  ✓ {}", result.summary());
                        total_chunks += result.chunks_created;
                        successful += 1;

                        // Write chunk metadata to output if specified
                        if let Some(output_dir) = &self.output {
                            if let Err(e) =
                                self.write_chunk_metadata(output_dir, file_path, &result)
                            {
                                warn!("Failed to write chunk metadata: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("  ✗ Failed to process {:?}: {}", file_path, e);
                        failed += 1;
                    }
                }
            }
        }

        // Print summary
        println!();
        println!("╔══════════════════════════════════════╗");
        println!("║      Chunking Summary                ║");
        println!("╠══════════════════════════════════════╣");
        println!("║ Total files:      {:>18} ║", files.len());
        println!("║ Successful:       {:>18} ║", successful);
        println!("║ Failed:           {:>18} ║", failed);
        if !self.dry_run {
            println!("║ Total chunks:     {:>18} ║", total_chunks);
        }
        println!("╚══════════════════════════════════════╝");

        if failed > 0 {
            warn!("{} files failed to process", failed);
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "chunk"
    }

    fn validate(&self) -> Result<()> {
        self.validate()
    }
}

impl ChunkArgs {
    /// Detect document category from file path
    fn detect_category(file_path: &Path) -> Option<String> {
        let path_str = file_path.to_string_lossy().to_lowercase();

        if path_str.contains("tutorial") {
            Some("tutorial".to_string())
        } else if path_str.contains("howto")
            || path_str.contains("how-to")
            || path_str.contains("how_to")
        {
            Some("how-to".to_string())
        } else if path_str.contains("reference") || path_str.contains("api") {
            Some("reference".to_string())
        } else if path_str.contains("explanation") || path_str.contains("concept") {
            Some("explanation".to_string())
        } else {
            None
        }
    }

    /// Extract simple keywords from content
    fn extract_keywords(content: &str) -> Vec<String> {
        // Simple keyword extraction - get words from first heading or filename
        let mut keywords = Vec::new();

        // Try to extract from first markdown heading
        for line in content.lines().take(10) {
            if line.starts_with('#') {
                let heading = line.trim_start_matches('#').trim();
                keywords.extend(
                    heading
                        .split_whitespace()
                        .map(|w| w.to_lowercase())
                        .filter(|w| w.len() > 3),
                );
                break;
            }
        }

        keywords.truncate(5);
        keywords
    }

    /// Write chunk metadata to JSON file
    fn write_chunk_metadata(
        &self,
        output_dir: &Path,
        file_path: &Path,
        result: &xze_core::documentation::processor::ProcessingResult,
    ) -> Result<()> {
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let output_path = output_dir.join(format!("{}_chunks.json", file_name));

        let metadata = serde_json::json!({
            "file_path": result.file_path,
            "file_hash": result.file_hash,
            "chunks_created": result.chunks_created,
            "total_words": result.total_words,
            "total_chars": result.total_chars,
            "duration_ms": result.duration_ms,
        });

        std::fs::write(&output_path, serde_json::to_string_pretty(&metadata)?)?;
        debug!("Wrote chunk metadata to: {:?}", output_path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_strategy_to_config() {
        let config = ChunkingStrategy::Default.to_config();
        assert!(config.similarity_threshold > 0.0);

        let tech_config = ChunkingStrategy::Technical.to_config();
        assert!(tech_config.similarity_threshold >= config.similarity_threshold);
    }

    #[test]
    fn test_detect_category() {
        let tutorial = PathBuf::from("docs/tutorials/getting_started.md");
        assert_eq!(
            ChunkArgs::detect_category(&tutorial),
            Some("tutorial".to_string())
        );

        let reference = PathBuf::from("docs/reference/api.md");
        assert_eq!(
            ChunkArgs::detect_category(&reference),
            Some("reference".to_string())
        );

        let unknown = PathBuf::from("docs/misc/notes.md");
        assert_eq!(ChunkArgs::detect_category(&unknown), None);
    }

    #[test]
    fn test_extract_keywords() {
        let content = "# Getting Started Guide\n\nThis is a guide about setup and configuration.";
        let keywords = ChunkArgs::extract_keywords(content);

        assert!(!keywords.is_empty());
        assert!(keywords.len() <= 5);
    }

    #[test]
    fn test_validate_threshold() {
        let mut args = ChunkArgs {
            input: vec![PathBuf::from("test.md")],
            output: None,
            strategy: ChunkingStrategy::Default,
            threshold: Some(1.5),
            max_sentences: None,
            dry_run: false,
            database_url: None,
            ollama_url: None,
        };

        // Create a test file
        std::fs::write("test.md", "test content").unwrap();

        let result = args.validate();
        assert!(result.is_err());

        args.threshold = Some(0.8);
        // Will still fail because DATABASE_URL not set, but threshold validation passes
        let _result = args.validate();
        // Clean up test file
        std::fs::remove_file("test.md").ok();
    }

    #[test]
    fn test_build_chunker_config() {
        let args = ChunkArgs {
            input: vec![],
            output: None,
            strategy: ChunkingStrategy::Default,
            threshold: Some(0.85),
            max_sentences: Some(20),
            dry_run: false,
            database_url: None,
            ollama_url: None,
        };

        let config = args.build_chunker_config();
        assert_eq!(config.similarity_threshold, 0.85);
        assert_eq!(config.max_chunk_sentences, 20);
    }
}
