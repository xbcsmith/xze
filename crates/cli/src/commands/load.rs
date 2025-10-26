//! Load command for knowledge base operations
//!
//! This module provides the CLI command for loading documents into the
//! knowledge base with support for incremental loading and resume functionality.

use crate::commands::CliCommand;
use clap::Args;
use std::path::PathBuf;
use tracing::{error, info};
use xze_core::kb::loader::{IncrementalLoader, LoaderConfig};
use xze_core::Result;

/// Load documents into the knowledge base
///
/// This command discovers files, calculates their hashes, and loads them
/// into the database. It supports incremental loading to skip unchanged files.
///
/// # Examples
///
/// ```bash
/// # Full load (load all files)
/// xze load --paths ./docs ./src
///
/// # Resume mode (skip unchanged files)
/// xze load --paths ./docs --resume
///
/// # Dry run (preview what would be loaded)
/// xze load --paths ./docs --resume --dry-run
/// ```
#[derive(Debug, Clone, Args)]
pub struct LoadArgs {
    /// Paths to load documents from
    ///
    /// Can specify multiple paths separated by spaces.
    #[arg(short, long, required = true)]
    pub paths: Vec<PathBuf>,

    /// Resume mode: skip files that are already loaded and unchanged
    ///
    /// This mode calculates hashes for all files and compares them with
    /// the database to determine which files need to be processed.
    #[arg(short, long)]
    pub resume: bool,

    /// Update mode: update modified files and add new files
    ///
    /// This mode detects files that have changed (different hash) and
    /// regenerates their chunks, replacing the old chunks atomically.
    #[arg(short, long)]
    pub update: bool,

    /// Database connection URL
    ///
    /// If not provided, uses the DATABASE_URL environment variable.
    #[arg(long, default_value_t = std::env::var("DATABASE_URL").unwrap_or_default())]
    pub database_url: String,

    /// Dry run mode: show what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,
}

impl LoadArgs {
    /// Validate command arguments
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if arguments are valid
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No paths are provided
    /// - Database URL is not set
    /// - Paths don't exist
    pub fn validate(&self) -> Result<()> {
        if self.paths.is_empty() {
            return Err(xze_core::XzeError::validation(
                "At least one path must be provided",
            ));
        }

        if self.database_url.is_empty() {
            return Err(xze_core::XzeError::validation(
                "Database URL must be provided via --database-url or DATABASE_URL environment variable",
            ));
        }

        // Verify paths exist
        for path in &self.paths {
            if !path.exists() {
                return Err(xze_core::XzeError::not_found(format!(
                    "Path does not exist: {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }

    /// Convert paths to strings
    fn paths_as_strings(&self) -> Vec<String> {
        self.paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
}

impl CliCommand for LoadArgs {
    async fn execute(&self) -> Result<()> {
        info!("Starting knowledge base load operation");

        // Create loader configuration
        let config = LoaderConfig {
            resume: self.resume,
            dry_run: self.dry_run,
            update: self.update,
            cleanup: false, // Phase 5
            force: false,
        };

        info!("Configuration:");
        info!("  Mode: {}", config.mode_description());
        info!("  Paths: {} path(s)", self.paths.len());
        for path in &self.paths {
            info!("    - {}", path.display());
        }
        info!("  Update: {}", self.update);
        info!("  Dry run: {}", self.dry_run);

        // Create database connection pool
        info!("Connecting to database...");
        let pool = sqlx::PgPool::connect(&self.database_url)
            .await
            .map_err(|e| {
                xze_core::XzeError::pipeline(format!("Failed to connect to database: {}", e))
            })?;

        // Create incremental loader
        let loader = IncrementalLoader::new(pool, config)
            .map_err(|e| xze_core::XzeError::pipeline(format!("Failed to create loader: {}", e)))?;

        // Execute load operation
        let paths = self.paths_as_strings();
        let stats = loader.load(&paths).await.map_err(|e| {
            error!("Load operation failed: {}", e);
            xze_core::XzeError::pipeline(format!("Load operation failed: {}", e))
        })?;

        // Log results
        info!("Load operation completed successfully");
        info!("Summary:");
        info!("  Total files discovered: {}", stats.total_files());
        info!("  Files skipped:          {}", stats.files_skipped);
        info!("  Files to process:       {}", stats.files_to_process());
        info!("  Duration:               {:.2}s", stats.duration_secs);

        if self.dry_run {
            info!("Dry run complete - no changes were made");
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "load"
    }

    fn validate(&self) -> Result<()> {
        self.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_load_args_validate_no_paths() {
        let args = LoadArgs {
            paths: vec![],
            resume: false,
            update: false,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        let result = args.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_args_validate_no_database_url() {
        let args = LoadArgs {
            paths: vec![PathBuf::from("/tmp")],
            resume: false,
            update: false,
            database_url: String::new(),
            dry_run: false,
        };

        let result = args.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_args_validate_nonexistent_path() {
        let args = LoadArgs {
            paths: vec![PathBuf::from("/nonexistent/path/that/does/not/exist")],
            resume: false,
            update: false,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        let result = args.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_load_args_validate_success() {
        // Use /tmp which should exist on Unix systems
        if env::consts::OS != "windows" {
            let args = LoadArgs {
                paths: vec![PathBuf::from("/tmp")],
                resume: false,
                update: false,
                database_url: "postgres://localhost/test".to_string(),
                dry_run: false,
            };

            let result = args.validate();
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_load_args_paths_as_strings() {
        let args = LoadArgs {
            paths: vec![PathBuf::from("/tmp"), PathBuf::from("/var")],
            resume: false,
            update: false,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        let paths = args.paths_as_strings();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_load_args_name() {
        let args = LoadArgs {
            paths: vec![PathBuf::from("/tmp")],
            resume: false,
            update: false,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        assert_eq!(args.name(), "load");
    }

    #[test]
    fn test_load_args_cli_command_validate() {
        let args = LoadArgs {
            paths: vec![],
            resume: false,
            update: false,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        let result = CliCommand::validate(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_args_with_update_flag() {
        let args = LoadArgs {
            paths: vec![PathBuf::from("/tmp")],
            resume: false,
            update: true,
            database_url: "postgres://localhost/test".to_string(),
            dry_run: false,
        };

        assert!(args.update);
    }
}
