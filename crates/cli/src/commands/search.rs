//! Search command for semantic search of document chunks
//!
//! This module provides the CLI interface for searching document chunks
//! using semantic similarity. It generates embeddings for queries and
//! finds the most relevant chunks from the knowledge base.

use crate::commands::CliCommand;
use clap::Parser;
use std::sync::Arc;
use tracing::info;
use xze_core::ai::providers::{OllamaConfig, OllamaProvider, Provider};
use xze_core::search::{ExpandedDocument, SearchPipeline, SearchPipelineConfig};
use xze_core::storage::PostgresStorage;
use xze_core::Result;

/// Search for document chunks using semantic similarity
///
/// This command searches the knowledge base for chunks that are semantically
/// similar to the provided query. Results are ranked by similarity score.
///
/// # Examples
///
/// ```bash
/// # Basic search
/// xze search "how to configure logging"
///
/// # Limit results
/// xze search "installation steps" --max-results 5
///
/// # Enable reranking
/// xze search "deployment" --rerank
///
/// # Enable context expansion
/// xze search "error handling" --expand --context-window 2
///
/// # Output as JSON
/// xze search "deployment" --json
/// ```
#[derive(Parser, Debug, Clone)]
#[command(name = "search")]
#[command(about = "Search document chunks using semantic similarity")]
pub struct SearchArgs {
    /// Search query text
    ///
    /// The query will be embedded and compared against all chunks
    /// in the knowledge base using cosine similarity.
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Maximum number of results to return
    ///
    /// Limits the number of search results. Results are always
    /// sorted by similarity score (highest first).
    #[arg(short = 'n', long, default_value = "10")]
    pub max_results: i64,

    /// Enable LLM reranking
    ///
    /// Uses an LLM to re-score and re-order the initial search results
    /// for better relevance.
    #[arg(long)]
    pub rerank: bool,

    /// Enable context expansion
    ///
    /// Fetches surrounding chunks for each result to provide more context.
    #[arg(long)]
    pub expand: bool,

    /// Context window size for expansion
    ///
    /// Number of chunks to fetch before and after the matching chunk.
    #[arg(long, default_value = "1")]
    pub context_window: i32,

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

    /// Database connection URL
    ///
    /// PostgreSQL connection string. Can be provided via DATABASE_URL
    /// environment variable.
    #[arg(long)]
    pub database_url: String,

    /// Output results as JSON
    ///
    /// When enabled, outputs results in JSON format for easier
    /// programmatic processing.
    #[arg(long)]
    pub json: bool,

    /// Show full chunk content
    ///
    /// By default, content is truncated for display. This flag
    /// shows the complete chunk content.
    #[arg(long)]
    pub full_content: bool,

    /// Verbose output
    ///
    /// Show additional metadata including chunk indices, sentence
    /// ranges, and similarity scores within chunks.
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

impl CliCommand for SearchArgs {
    async fn execute(&self) -> Result<()> {
        info!("Executing search command");
        info!("Query: '{}'", self.query);
        info!("Max results: {}", self.max_results);

        // Validate configuration
        self.validate()?;

        // Connect to database
        info!(
            "Connecting to database: {}",
            mask_connection_string(&self.database_url)
        );
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
        let pipeline = SearchPipeline::new(storage, provider);

        // Build configuration
        let config = SearchPipelineConfig {
            limit: self.max_results,
            rerank: self.rerank,
            expand_context: self.expand,
            context_window: self.context_window,
        };

        // Execute search
        info!("Searching with config: {:?}", config);

        let results = pipeline
            .search(&self.query, &config)
            .await
            .map_err(|e| xze_core::XzeError::Generic(anyhow::anyhow!("Search failed: {}", e)))?;

        // Display results
        if self.json {
            self.display_json(&results)?;
        } else {
            self.display_human(&results);
        }

        info!(
            "Search completed successfully with {} results",
            results.len()
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "search"
    }

    fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "Query cannot be empty"
            )));
        }

        if self.max_results <= 0 {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "max_results must be greater than 0"
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

impl SearchArgs {
    /// Display results in human-readable format
    fn display_human(&self, results: &[ExpandedDocument]) {
        if results.is_empty() {
            println!("\nNo results found for query: '{}'", self.query);
            return;
        }

        println!(
            "\nFound {} result(s) for: '{}'\n",
            results.len(),
            self.query
        );
        println!("{}", "=".repeat(80));

        for (i, result) in results.iter().enumerate() {
            let doc = &result.original;
            println!(
                "\n{}. {} (Chunk {})",
                i + 1,
                doc.source_file,
                doc.chunk_index
            );

            if let Some(ref title) = doc.title {
                println!("   Title: {}", title);
            }

            if let Some(ref category) = doc.category {
                println!("   Category: {}", category);
            }

            if let Some(ref dt) = doc.diataxis_type {
                println!("   Type: {:?}", dt);
            }

            println!("\n   Content:");
            let content = if self.full_content {
                result.expanded_content.clone()
            } else {
                truncate_content(&result.expanded_content, 300)
            };

            for line in content.lines() {
                println!("   {}", line);
            }

            println!("\n{}", "-".repeat(80));
        }
    }

    /// Display results in JSON format
    fn display_json(&self, results: &[ExpandedDocument]) -> Result<()> {
        use serde_json::json;

        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                json!({
                    "original": r.original,
                    "expanded_content": r.expanded_content,
                    "context_chunks": r.context_chunks,
                })
            })
            .collect();

        let output = json!({
            "query": self.query,
            "result_count": results.len(),
            "config": {
                "max_results": self.max_results,
                "rerank": self.rerank,
                "expand": self.expand,
            },
            "results": json_results,
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }
}

/// Mask sensitive parts of connection string for logging
fn mask_connection_string(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(protocol_end) = url.find("://") {
            let protocol = &url[..protocol_end + 3];
            let host_part = &url[at_pos..];
            return format!("{}***{}", protocol, host_part);
        }
    }
    "***".to_string()
}

/// Truncate content to specified length, adding ellipsis if truncated
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    let truncated = &content[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_args_validation_empty_query() {
        let args = SearchArgs {
            query: "".to_string(),
            max_results: 10,
            rerank: false,
            expand: false,
            context_window: 1,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_search_args_validation_zero_results() {
        let args = SearchArgs {
            query: "test query".to_string(),
            max_results: 0,
            rerank: false,
            expand: false,
            context_window: 1,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_search_args_validation_valid() {
        let args = SearchArgs {
            query: "test query".to_string(),
            max_results: 10,
            rerank: true,
            expand: true,
            context_window: 2,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_mask_connection_string() {
        let url = "postgresql://user:password@localhost:5432/db";
        let masked = mask_connection_string(url);
        assert!(!masked.contains("password"));
        assert!(masked.contains("@localhost"));
    }

    #[test]
    fn test_cli_command_name() {
        let args = SearchArgs {
            query: "test".to_string(),
            max_results: 10,
            rerank: false,
            expand: false,
            context_window: 1,
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert_eq!(args.name(), "search");
    }
}
