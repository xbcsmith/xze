// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reorganization command for Diataxis structure

use crate::commands::CliCommand;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use xze_core::ai::providers::{OllamaConfig, OllamaProvider, Provider};
use xze_core::reorg::{ReorgExecutor, ReorgPlanner, RepositoryScanner};
use xze_core::Result;

/// Reorganize documentation according to Diataxis
///
/// This command scans the documentation directory, uses an LLM to classify
/// files into Diataxis categories (Tutorials, How-To, Reference, Explanation),
/// and optionally moves them to the correct structure.
///
/// # Examples
///
/// ```bash
/// # Generate a reorganization plan
/// xze reorg ./docs
///
/// # Execute the reorganization (dry run)
/// xze reorg ./docs --execute --dry-run
///
/// # Execute the reorganization (real)
/// xze reorg ./docs --execute
/// ```
#[derive(Parser, Debug, Clone)]
#[command(name = "reorg")]
#[command(about = "Reorganize documentation according to Diataxis")]
pub struct ReorgArgs {
    /// Path to the documentation directory
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

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
    /// When executing, simulate moves without modifying the filesystem.
    #[arg(long)]
    pub dry_run: bool,

    /// Execute the plan
    ///
    /// If not specified, only the plan is generated and printed.
    /// If specified, the plan is executed (subject to --dry-run).
    #[arg(long)]
    pub execute: bool,
}

impl CliCommand for ReorgArgs {
    async fn execute(&self) -> Result<()> {
        info!("Executing reorg command");
        info!("Path: {}", self.path.display());
        info!("Execute: {}", self.execute);
        info!("Dry run: {}", self.dry_run);

        // Validate configuration
        self.validate()?;

        // 1. Scan repository
        info!("Scanning repository...");
        let scanner = RepositoryScanner::new(&self.path);
        let files = scanner
            .scan()
            .await
            .map_err(|e| xze_core::XzeError::Generic(anyhow::anyhow!("Scan failed: {}", e)))?;
        info!("Found {} markdown files", files.len());

        if files.is_empty() {
            info!("No markdown files found. Exiting.");
            return Ok(());
        }

        // 2. Generate plan
        info!("Generating reorganization plan...");
        let provider_config = OllamaConfig {
            base_url: self.ollama_url.clone(),
            timeout: 60, // Longer timeout for planning
        };
        let ollama_provider =
            OllamaProvider::new(provider_config, self.model.clone()).map_err(|e| {
                xze_core::XzeError::Generic(anyhow::anyhow!("Failed to create provider: {}", e))
            })?;
        let provider: Arc<dyn Provider> = Arc::new(ollama_provider);

        let planner = ReorgPlanner::new(provider);
        let plan = planner.generate_plan(files).await.map_err(|e| {
            xze_core::XzeError::Generic(anyhow::anyhow!("Plan generation failed: {}", e))
        })?;

        // Print plan
        println!("\nProposed Reorganization Plan:");
        println!("{}", "=".repeat(80));
        for mv in &plan.moves {
            if mv.original_path != mv.new_path {
                println!(
                    "MOVE: {} -> {}\n      Reason: {}",
                    mv.original_path.display(),
                    mv.new_path.display(),
                    mv.reason
                );
            } else {
                println!(
                    "KEEP: {}\n      Reason: {}",
                    mv.original_path.display(),
                    mv.reason
                );
            }
            println!("{}", "-".repeat(80));
        }

        // 3. Execute plan (if requested)
        if self.execute {
            info!("Executing plan...");
            let executor = ReorgExecutor::new(self.dry_run);
            let results = executor.execute(plan).await;

            println!("\nExecution Results:");
            println!("{}", "=".repeat(80));
            let mut success_count = 0;
            let mut fail_count = 0;

            for res in results {
                if res.success {
                    success_count += 1;
                    println!(
                        "SUCCESS: {} -> {}",
                        res.original_path.display(),
                        res.new_path.display()
                    );
                } else {
                    fail_count += 1;
                    println!(
                        "FAILED:  {} -> {}\n         Error: {}",
                        res.original_path.display(),
                        res.new_path.display(),
                        res.error.as_deref().unwrap_or("Unknown error")
                    );
                }
            }
            println!("{}", "-".repeat(80));
            println!(
                "Total: {}, Success: {}, Failed: {}",
                success_count + fail_count,
                success_count,
                fail_count
            );
        } else {
            println!("\nRun with --execute to apply changes.");
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "reorg"
    }

    fn validate(&self) -> Result<()> {
        if !self.path.exists() {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "Path does not exist: {}",
                self.path.display()
            )));
        }

        Ok(())
    }
}
