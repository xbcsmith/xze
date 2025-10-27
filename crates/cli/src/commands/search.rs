//! Search command for semantic search of document chunks
//!
//! This module provides the CLI interface for searching document chunks
//! using semantic similarity. It generates embeddings for queries and
//! finds the most relevant chunks from the knowledge base.

use crate::commands::CliCommand;
use clap::Parser;
use sqlx::PgPool;
use tracing::info;
use xze_core::semantic::search::{search_with_chunks, SearchConfig};
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
/// # Filter by category
/// xze search "API usage" --category tutorial
///
/// # Set minimum similarity threshold
/// xze search "error handling" --min-similarity 0.5
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
    pub max_results: usize,

    /// Minimum similarity threshold (0.0 to 1.0)
    ///
    /// Only return results with similarity scores above this threshold.
    /// A value of 0.0 returns all results, 1.0 requires exact matches.
    #[arg(short = 's', long, default_value = "0.0")]
    pub min_similarity: f32,

    /// Filter by document category
    ///
    /// Only search within documents of the specified category.
    /// Categories follow the Diataxis framework: tutorial, how_to,
    /// explanation, reference.
    #[arg(short = 'c', long)]
    pub category: Option<String>,

    /// Ollama API URL
    ///
    /// URL of the Ollama server for generating embeddings.
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

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
        info!("Min similarity: {}", self.min_similarity);

        // Validate configuration
        self.validate()?;

        // Connect to database
        info!(
            "Connecting to database: {}",
            mask_connection_string(&self.database_url)
        );
        let pool = PgPool::connect(&self.database_url).await.map_err(|e| {
            xze_core::XzeError::Generic(anyhow::anyhow!("Database connection failed: {}", e))
        })?;

        // Build search configuration
        let config = SearchConfig {
            max_results: self.max_results,
            min_similarity: self.min_similarity,
            category_filter: self.category.clone(),
        };

        // Execute search
        info!(
            "Searching with config: max={}, min_sim={}, category={:?}",
            config.max_results, config.min_similarity, config.category_filter
        );

        let results = search_with_chunks(&pool, &self.query, &self.ollama_url, &config)
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

        if self.max_results == 0 {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "max_results must be greater than 0"
            )));
        }

        if self.min_similarity < 0.0 || self.min_similarity > 1.0 {
            return Err(xze_core::XzeError::Generic(anyhow::anyhow!(
                "min_similarity must be between 0.0 and 1.0, got {}",
                self.min_similarity
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
    fn display_human(&self, results: &[xze_core::semantic::search::ChunkSearchResult]) {
        if results.is_empty() {
            println!("\nNo results found for query: '{}'", self.query);
            println!("Try:");
            println!("  - Using different keywords");
            println!("  - Lowering the minimum similarity threshold");
            println!("  - Removing category filters");
            return;
        }

        println!(
            "\nFound {} result(s) for: '{}'\n",
            results.len(),
            self.query
        );
        println!("{}", "=".repeat(80));

        for (i, result) in results.iter().enumerate() {
            println!(
                "\n{}. {} (Similarity: {:.2}%)",
                i + 1,
                result.source_file,
                result.similarity * 100.0
            );

            if let Some(ref title) = result.title {
                println!("   Title: {}", title);
            }

            if let Some(ref category) = result.category {
                println!("   Category: {}", category);
            }

            if self.verbose {
                println!(
                    "   Chunk: {}/{}",
                    result.chunk_index + 1,
                    result.total_chunks
                );
                println!(
                    "   Sentences: {} to {}",
                    result.sentence_range.0, result.sentence_range.1
                );
                println!(
                    "   Avg chunk similarity: {:.2}%",
                    result.avg_chunk_similarity * 100.0
                );
            }

            println!("\n   Content:");
            let content = if self.full_content {
                result.content.clone()
            } else {
                truncate_content(&result.content, 300)
            };

            for line in content.lines() {
                println!("   {}", line);
            }

            println!("\n{}", "-".repeat(80));
        }

        println!(
            "\nShowing {} of {} results",
            results.len().min(self.max_results),
            results.len()
        );
    }

    /// Display results in JSON format
    fn display_json(
        &self,
        results: &[xze_core::semantic::search::ChunkSearchResult],
    ) -> Result<()> {
        use serde_json::json;

        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "source_file": r.source_file,
                    "content": r.content,
                    "similarity": r.similarity,
                    "chunk_index": r.chunk_index,
                    "total_chunks": r.total_chunks,
                    "title": r.title,
                    "category": r.category,
                    "sentence_range": {
                        "start": r.sentence_range.0,
                        "end": r.sentence_range.1,
                    },
                    "avg_chunk_similarity": r.avg_chunk_similarity,
                })
            })
            .collect();

        let output = json!({
            "query": self.query,
            "result_count": results.len(),
            "max_results": self.max_results,
            "min_similarity": self.min_similarity,
            "category_filter": self.category,
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
            min_similarity: 0.0,
            category: None,
            ollama_url: "http://localhost:11434".to_string(),
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
            min_similarity: 0.0,
            category: None,
            ollama_url: "http://localhost:11434".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_search_args_validation_invalid_similarity() {
        let args = SearchArgs {
            query: "test query".to_string(),
            max_results: 10,
            min_similarity: 1.5,
            category: None,
            ollama_url: "http://localhost:11434".to_string(),
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
            min_similarity: 0.5,
            category: Some("tutorial".to_string()),
            ollama_url: "http://localhost:11434".to_string(),
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
    fn test_mask_connection_string_no_credentials() {
        let url = "postgresql://localhost:5432/db";
        let masked = mask_connection_string(url);
        assert_eq!(masked, "***");
    }

    #[test]
    fn test_truncate_content_short() {
        let content = "Short content";
        let truncated = truncate_content(content, 100);
        assert_eq!(truncated, "Short content");
    }

    #[test]
    fn test_truncate_content_long() {
        let content = "This is a very long piece of content that should be truncated to a reasonable length for display purposes.";
        let truncated = truncate_content(content, 50);
        assert!(truncated.len() <= 53); // 50 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_content_at_word_boundary() {
        let content = "This is a test sentence with multiple words";
        let truncated = truncate_content(content, 20);
        assert!(truncated.ends_with("..."));
        // Should truncate at last space before limit
        assert!(!truncated.contains("sentence"));
    }

    #[test]
    fn test_cli_command_name() {
        let args = SearchArgs {
            query: "test".to_string(),
            max_results: 10,
            min_similarity: 0.0,
            category: None,
            ollama_url: "http://localhost:11434".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            json: false,
            full_content: false,
            verbose: false,
        };

        assert_eq!(args.name(), "search");
    }
}
