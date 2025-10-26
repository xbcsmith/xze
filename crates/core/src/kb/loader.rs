//! Incremental loader for knowledge base operations
//!
//! This module orchestrates the incremental loading process, coordinating
//! file discovery, categorization, and processing based on configuration.

use crate::kb::categorizer::{CategorizedFiles, FileCategorizer};
use crate::kb::error::{KbError, Result};
use crate::kb::hash;
use crate::kb::store::{DocumentChunk, KbStore};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Configuration for incremental loader operations
///
/// Controls the behavior of the incremental loading process including
/// whether to resume interrupted loads, update modified files, or clean up
/// deleted files.
///
/// # Examples
///
/// ```
/// use xze_core::kb::loader::LoaderConfig;
///
/// // Default configuration (full load)
/// let config = LoaderConfig::default();
/// assert!(!config.resume);
///
/// // Resume mode configuration
/// let resume_config = LoaderConfig {
///     resume: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct LoaderConfig {
    /// Resume an interrupted load (skip unchanged files)
    pub resume: bool,
    /// Update modified files
    pub update: bool,
    /// Clean up deleted files from database
    pub cleanup: bool,
    /// Dry run mode (preview changes without applying)
    pub dry_run: bool,
    /// Force full reload (ignore existing state)
    pub force: bool,
}

impl LoaderConfig {
    /// Validate configuration for conflicts
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if configuration is valid
    ///
    /// # Errors
    ///
    /// Returns `KbError::ConfigError` if conflicting flags are set
    pub fn validate(&self) -> Result<()> {
        // Force and resume are mutually exclusive
        if self.force && self.resume {
            return Err(KbError::config("Cannot use --force and --resume together"));
        }

        // Force and update are mutually exclusive
        if self.force && self.update {
            return Err(KbError::config(
                "Cannot use --force and --update together (force implies full reload)",
            ));
        }

        Ok(())
    }

    /// Get human-readable mode description
    pub fn mode_description(&self) -> &'static str {
        if self.force {
            "Force Full Reload"
        } else if self.resume {
            "Resume (Skip Unchanged)"
        } else if self.update {
            "Incremental Update"
        } else {
            "Full Load"
        }
    }
}

/// Statistics from a load operation
///
/// Tracks the number of files processed in each category and the
/// total duration of the operation.
///
/// # Examples
///
/// ```
/// use xze_core::kb::loader::LoadStats;
///
/// let mut stats = LoadStats::new();
/// stats.files_added = 10;
/// stats.files_skipped = 5;
///
/// assert_eq!(stats.total_files(), 15);
/// assert_eq!(stats.files_to_process(), 10);
/// ```
#[derive(Debug, Default, Clone)]
pub struct LoadStats {
    /// Number of files skipped (unchanged)
    pub files_skipped: usize,
    /// Number of files added (new)
    pub files_added: usize,
    /// Number of files updated (modified)
    pub files_updated: usize,
    /// Number of files deleted (removed)
    pub files_deleted: usize,
    /// Number of chunks inserted
    pub chunks_inserted: usize,
    /// Number of chunks deleted
    pub chunks_deleted: usize,
    /// Duration of operation in seconds
    pub duration_secs: f64,
}

impl LoadStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of files discovered
    pub fn total_files(&self) -> usize {
        self.files_skipped + self.files_added + self.files_updated + self.files_deleted
    }

    /// Get number of files that need processing (excludes skipped)
    pub fn files_to_process(&self) -> usize {
        self.files_added + self.files_updated + self.files_deleted
    }

    /// Log summary of statistics
    pub fn log_summary(&self) {
        info!("Load operation completed in {:.2}s", self.duration_secs);
        info!("  Files discovered: {}", self.total_files());
        info!("  Files skipped:    {}", self.files_skipped);
        info!("  Files added:      {}", self.files_added);
        info!("  Files updated:    {}", self.files_updated);
        info!("  Files deleted:    {}", self.files_deleted);
        info!("  Chunks inserted:  {}", self.chunks_inserted);
        info!("  Chunks deleted:   {}", self.chunks_deleted);
    }
}

/// Incremental loader for knowledge base operations
///
/// Orchestrates the incremental loading process by discovering files,
/// categorizing them, and processing them according to configuration.
///
/// # Examples
///
/// ```no_run
/// use xze_core::kb::loader::{IncrementalLoader, LoaderConfig};
/// use sqlx::PgPool;
///
/// # async fn example(pool: PgPool) -> xze_core::kb::error::Result<()> {
/// let config = LoaderConfig {
///     resume: true,
///     ..Default::default()
/// };
///
/// let loader = IncrementalLoader::new(pool, config)?;
/// let paths = vec!["./docs".to_string()];
/// let stats = loader.load(&paths).await?;
///
/// println!("Loaded {} files", stats.files_to_process());
/// # Ok(())
/// # }
/// ```
pub struct IncrementalLoader {
    store: KbStore,
    config: LoaderConfig,
}

impl IncrementalLoader {
    /// Create a new incremental loader
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `config` - Loader configuration
    ///
    /// # Returns
    ///
    /// Returns a new IncrementalLoader instance
    ///
    /// # Errors
    ///
    /// Returns `KbError::ConfigError` if configuration is invalid
    pub fn new(pool: PgPool, config: LoaderConfig) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            store: KbStore::new(pool),
            config,
        })
    }

    /// Load files from specified paths
    ///
    /// Discovers files, categorizes them based on hash comparison,
    /// and processes according to configuration settings.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to load files from
    ///
    /// # Returns
    ///
    /// Returns statistics about the load operation
    ///
    /// # Errors
    ///
    /// Returns `KbError` if discovery, categorization, or processing fails
    pub async fn load(&self, paths: &[String]) -> Result<LoadStats> {
        let start = Instant::now();
        let mut stats = LoadStats::new();

        info!("Starting incremental load");
        info!("  Mode: {}", self.config.mode_description());
        info!("  Paths: {:?}", paths);
        info!("  Dry run: {}", self.config.dry_run);

        // Phase 1: Discover files with hashes
        debug!("Discovering files and calculating hashes...");
        let current_files = FileCategorizer::discover_files_with_hashes(paths).await?;
        info!("Discovered {} files", current_files.len());

        // Store hashes for later use
        let file_hashes: HashMap<String, String> = current_files.clone();

        // Phase 2: Query existing files from database
        let existing_files = if self.config.force {
            // In force mode, treat all files as new
            debug!("Force mode: treating all files as new");
            std::collections::HashMap::new()
        } else {
            debug!("Querying existing files from database...");
            self.store.query_existing_files().await?
        };
        info!("Found {} existing files in database", existing_files.len());

        // Phase 3: Categorize files
        debug!("Categorizing files...");
        let categorizer = FileCategorizer::new(current_files, existing_files);
        let categorized = categorizer.categorize();
        FileCategorizer::log_summary(&categorized);

        // Update statistics from categorization
        stats.files_skipped = categorized.skip.len();
        stats.files_added = categorized.add.len();
        stats.files_updated = categorized.update.len();
        stats.files_deleted = categorized.delete.len();

        // Phase 4: Process files based on configuration
        if self.config.dry_run {
            info!("Dry run mode: skipping actual processing");
            self.log_dry_run_summary(&categorized);
        } else {
            // Process add files (new files)
            if !categorized.add.is_empty() {
                info!("Processing {} new files...", categorized.add.len());
                let chunks = self
                    .process_add_files(&categorized.add, &file_hashes)
                    .await?;
                stats.chunks_inserted += chunks;
                info!("Inserted {} chunks for new files", chunks);
            }

            // Process update files (only if update flag is set)
            if self.config.update && !categorized.update.is_empty() {
                info!("Processing {} modified files...", categorized.update.len());
                let chunks = self
                    .process_update_files(&categorized.update, &file_hashes)
                    .await?;
                stats.chunks_inserted += chunks;
                info!("Updated {} chunks for modified files", chunks);
            } else if !categorized.update.is_empty() && !self.config.update {
                warn!(
                    "Skipping {} modified files (use --update to process them)",
                    categorized.update.len()
                );
            }

            // Process delete files (only if cleanup flag is set)
            if self.config.cleanup && !categorized.delete.is_empty() {
                info!("Processing {} deleted files...", categorized.delete.len());
                let chunks = self.process_delete_files(&categorized.delete).await?;
                stats.chunks_deleted += chunks;
                info!("Removed {} chunks for deleted files", chunks);
            } else if !categorized.delete.is_empty() && !self.config.cleanup {
                warn!(
                    "Skipping {} deleted files (use --cleanup to remove them)",
                    categorized.delete.len()
                );
            }
        }

        stats.duration_secs = start.elapsed().as_secs_f64();
        stats.log_summary();

        Ok(stats)
    }

    /// Process files to be added
    ///
    /// Generates chunks for new files and inserts them into the database.
    ///
    /// # Arguments
    ///
    /// * `files` - List of file paths to add
    /// * `file_hashes` - Map of file paths to their SHA-256 hashes
    ///
    /// # Returns
    ///
    /// Returns the total number of chunks inserted
    ///
    /// # Errors
    ///
    /// Returns `KbError` if chunk generation or insertion fails
    async fn process_add_files(
        &self,
        files: &[String],
        file_hashes: &HashMap<String, String>,
    ) -> Result<usize> {
        debug!("Processing {} files to add", files.len());

        let mut total_chunks = 0;

        for file_str in files {
            let file_path = PathBuf::from(file_str);
            let hash = file_hashes
                .get(file_str)
                .ok_or_else(|| KbError::loader(format!("Hash not found for file: {}", file_str)))?;

            info!("Adding file: {}", file_str);

            // Generate chunks for the file
            let chunks = self.generate_chunks(&file_path).await?;

            if chunks.is_empty() {
                warn!("No chunks generated for file: {}", file_str);
                continue;
            }

            // Insert chunks into database
            self.store
                .insert_file_chunks(&file_path, hash, &chunks)
                .await?;

            total_chunks += chunks.len();
            info!("Inserted {} chunks for file: {}", chunks.len(), file_str);
        }

        Ok(total_chunks)
    }

    /// Process files to be updated
    ///
    /// Generates chunks for modified files and updates them in the database
    /// atomically within a transaction.
    ///
    /// # Arguments
    ///
    /// * `files` - List of file paths to update
    /// * `file_hashes` - Map of file paths to their new SHA-256 hashes
    ///
    /// # Returns
    ///
    /// Returns the total number of chunks inserted
    ///
    /// # Errors
    ///
    /// Returns `KbError` if chunk generation or update fails
    async fn process_update_files(
        &self,
        files: &[String],
        file_hashes: &HashMap<String, String>,
    ) -> Result<usize> {
        debug!("Processing {} files to update", files.len());

        let mut total_chunks = 0;

        for file_str in files {
            let file_path = PathBuf::from(file_str);
            let hash = file_hashes
                .get(file_str)
                .ok_or_else(|| KbError::loader(format!("Hash not found for file: {}", file_str)))?;

            info!("Updating file: {}", file_str);

            // Generate new chunks for the file
            let chunks = self.generate_chunks(&file_path).await?;

            if chunks.is_empty() {
                warn!("No chunks generated for file: {}", file_str);
                continue;
            }

            // Update chunks atomically in transaction
            self.store
                .update_file_chunks(&file_path, hash, &chunks)
                .await?;

            total_chunks += chunks.len();
            info!("Updated {} chunks for file: {}", chunks.len(), file_str);
        }

        Ok(total_chunks)
    }

    /// Process files to be deleted
    ///
    /// Stub implementation for Phase 3 - actual implementation
    /// will be completed in Phase 5.
    ///
    /// # Arguments
    ///
    /// * `files` - List of file paths to delete
    ///
    /// # Returns
    ///
    /// Returns the number of chunks deleted
    ///
    /// # Errors
    ///
    /// Returns `KbError` if processing fails
    async fn process_delete_files(&self, files: &[String]) -> Result<usize> {
        debug!("Processing {} files to delete", files.len());

        // TODO Phase 5: Implement actual cleanup from database
        // For now, just log that we would process these files
        for file in files {
            debug!("  Would delete: {}", file);
        }

        // Return 0 chunks for now (Phase 5 will implement actual cleanup)
        Ok(0)
    }

    /// Generate document chunks from a file
    ///
    /// This is a placeholder implementation that creates basic chunks.
    /// Future phases will integrate with the existing AIDocumentationGenerator
    /// or more sophisticated chunking logic.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to chunk
    ///
    /// # Returns
    ///
    /// Returns a vector of DocumentChunk instances
    ///
    /// # Errors
    ///
    /// Returns `KbError` if file cannot be read or chunked
    async fn generate_chunks(&self, file_path: &Path) -> Result<Vec<DocumentChunk>> {
        debug!("Generating chunks for file: {}", file_path.display());

        // Read file content
        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            KbError::loader(format!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        // Skip empty files
        if content.trim().is_empty() {
            debug!("Skipping empty file: {}", file_path.display());
            return Ok(Vec::new());
        }

        // Simple chunking strategy: split by paragraphs (double newline)
        // This is a placeholder - future implementation will use semantic chunking
        let paragraphs: Vec<&str> = content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();

        if paragraphs.is_empty() {
            // Fall back to treating entire content as one chunk
            return Ok(vec![self.create_chunk(0, &content).await?]);
        }

        let mut chunks = Vec::new();
        for (idx, paragraph) in paragraphs.iter().enumerate() {
            let chunk = self.create_chunk(idx, paragraph).await?;
            chunks.push(chunk);
        }

        debug!(
            "Generated {} chunks for file: {}",
            chunks.len(),
            file_path.display()
        );

        Ok(chunks)
    }

    /// Create a single document chunk with embedding
    ///
    /// # Arguments
    ///
    /// * `index` - Chunk index within the document
    /// * `content` - Text content for the chunk
    ///
    /// # Returns
    ///
    /// Returns a DocumentChunk instance
    ///
    /// # Errors
    ///
    /// Returns `KbError` if embedding generation fails
    async fn create_chunk(&self, index: usize, content: &str) -> Result<DocumentChunk> {
        let chunk_id = format!("chunk_{}", index);

        // Placeholder embedding - in production, this would call an AI service
        // to generate semantic embeddings. For now, use a simple hash-based approach.
        let embedding = self.generate_placeholder_embedding(content);

        // Create metadata
        let metadata = serde_json::json!({
            "chunk_index": index,
            "content_length": content.len(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        Ok(DocumentChunk::new(
            chunk_id,
            content.to_string(),
            embedding,
            metadata,
        ))
    }

    /// Generate a placeholder embedding vector
    ///
    /// This creates a deterministic embedding based on content hash.
    /// In production, this should be replaced with actual AI-generated embeddings.
    ///
    /// # Arguments
    ///
    /// * `content` - Text content to generate embedding for
    ///
    /// # Returns
    ///
    /// Returns a 384-dimensional vector (common embedding size)
    fn generate_placeholder_embedding(&self, content: &str) -> Vec<f32> {
        // Use content hash to generate deterministic embedding
        let content_hash = hash::calculate_content_hash(content);

        // Convert hash bytes to f32 values
        // This is a placeholder - real embeddings would come from an AI model
        let hash_bytes = content_hash.as_bytes();
        let mut embedding = Vec::with_capacity(384);

        for i in 0..384 {
            let byte_index = i % hash_bytes.len();
            let value = hash_bytes[byte_index] as f32 / 255.0;
            embedding.push(value);
        }

        embedding
    }

    /// Log dry run summary
    fn log_dry_run_summary(&self, categorized: &CategorizedFiles) {
        info!("Dry run summary:");
        info!("  Would skip:   {} files", categorized.skip.len());
        info!("  Would add:    {} files", categorized.add.len());

        if self.config.update {
            info!("  Would update: {} files", categorized.update.len());
        } else if !categorized.update.is_empty() {
            info!(
                "  Would skip update: {} files (use --update to process)",
                categorized.update.len()
            );
        }

        if self.config.cleanup {
            info!("  Would delete: {} files", categorized.delete.len());
        } else if !categorized.delete.is_empty() {
            info!(
                "  Would skip cleanup: {} files (use --cleanup to remove)",
                categorized.delete.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert!(!config.resume);
        assert!(!config.update);
        assert!(!config.cleanup);
        assert!(!config.dry_run);
        assert!(!config.force);
    }

    #[test]
    fn test_loader_config_validate_success() {
        let config = LoaderConfig {
            resume: true,
            update: false,
            cleanup: true,
            dry_run: true,
            force: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_loader_config_validate_force_resume_conflict() {
        let config = LoaderConfig {
            resume: true,
            force: true,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_loader_config_validate_force_update_conflict() {
        let config = LoaderConfig {
            update: true,
            force: true,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_loader_config_mode_description() {
        let config = LoaderConfig::default();
        assert_eq!(config.mode_description(), "Full Load");

        let resume_config = LoaderConfig {
            resume: true,
            ..Default::default()
        };
        assert_eq!(resume_config.mode_description(), "Resume (Skip Unchanged)");

        let update_config = LoaderConfig {
            update: true,
            ..Default::default()
        };
        assert_eq!(update_config.mode_description(), "Incremental Update");

        let force_config = LoaderConfig {
            force: true,
            ..Default::default()
        };
        assert_eq!(force_config.mode_description(), "Force Full Reload");
    }

    #[test]
    fn test_load_stats_default() {
        let stats = LoadStats::default();
        assert_eq!(stats.total_files(), 0);
        assert_eq!(stats.files_to_process(), 0);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.duration_secs, 0.0);
    }

    #[test]
    fn test_load_stats_new() {
        let stats = LoadStats::new();
        assert_eq!(stats.total_files(), 0);
        assert_eq!(stats.files_to_process(), 0);
    }

    #[test]
    fn test_load_stats_counts() {
        let mut stats = LoadStats::new();
        stats.files_skipped = 5;
        stats.files_added = 3;
        stats.files_updated = 2;
        stats.files_deleted = 1;
        stats.chunks_inserted = 10;
        stats.chunks_deleted = 5;

        assert_eq!(stats.total_files(), 11);
        assert_eq!(stats.files_to_process(), 6);
        assert_eq!(stats.chunks_inserted, 10);
        assert_eq!(stats.chunks_deleted, 5);
    }

    #[test]
    fn test_load_stats_with_duration() {
        let mut stats = LoadStats::new();
        stats.duration_secs = 12.5;
        stats.files_added = 100;

        assert_eq!(stats.total_files(), 100);
        assert_eq!(stats.duration_secs, 12.5);
    }
}
