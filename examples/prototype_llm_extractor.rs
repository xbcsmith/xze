//! Prototype LLM-based keyword extractor for validation
//!
//! This is a standalone example for testing LLM extraction quality
//! before integrating into the main system.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example prototype_llm_extractor -- \
//!     --input docs/ \
//!     --output llm_keywords.json \
//!     --model llama3.2:3b
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};

/// Command-line arguments for the prototype extractor
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input directory containing markdown files
    #[arg(short, long)]
    input: PathBuf,

    /// Output JSON file for results
    #[arg(short, long)]
    output: PathBuf,

    /// Ollama model to use
    #[arg(short, long, default_value = "llama3.2:3b")]
    model: String,

    /// Maximum content length to send to LLM (characters)
    #[arg(long, default_value = "2000")]
    max_content_length: usize,

    /// Ollama server URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,
}

/// Extracted keywords with structured categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    /// Single-word technical terms
    pub keywords: Vec<String>,

    /// Multi-word key phrases
    pub phrases: Vec<String>,

    /// Acronyms mapped to their expansions
    pub acronyms: HashMap<String, String>,

    /// Tool or product names
    pub tools: Vec<String>,

    /// Technical commands or API names
    pub commands: Vec<String>,
}

/// Result of processing a single document
#[derive(Debug, Serialize, Deserialize)]
struct DocumentResult {
    file: String,
    success: bool,
    extraction_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    keywords: Option<ExtractedKeywords>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Summary statistics for the extraction run
#[derive(Debug, Serialize, Deserialize)]
struct ExtractionSummary {
    total_documents: usize,
    successful: usize,
    failed: usize,
    avg_extraction_time_ms: u128,
    total_time_ms: u128,
}

/// Extract keywords using LLM with structured prompt
///
/// # Arguments
///
/// * `content` - Document content to analyze
/// * `model` - Ollama model to use
/// * `max_content_length` - Maximum characters to send to LLM
/// * `ollama_url` - Ollama server URL
///
/// # Returns
///
/// Returns extracted keywords with structured categorization
///
/// # Errors
///
/// Returns error if LLM extraction fails or JSON parsing fails
async fn extract_keywords_llm(
    content: &str,
    model: &str,
    max_content_length: usize,
    ollama_url: &str,
) -> Result<ExtractedKeywords> {
    let truncated_content = if content.len() > max_content_length {
        &content[..max_content_length]
    } else {
        content
    };

    let prompt = format!(
        r#"You are analyzing technical documentation to extract important keywords for search indexing.

Analyze this document excerpt and extract the most relevant search terms:

{}

Provide a JSON response with the following structure:
{{
    "keywords": ["word1", "word2"],
    "phrases": ["phrase 1", "phrase 2"],
    "acronyms": {{"CLI": "Command Line Interface"}},
    "tools": ["tool1", "tool2"],
    "commands": ["command1", "command2"]
}}

Guidelines:
- Extract 15-20 single-word technical terms for "keywords"
- Extract 5-10 multi-word key phrases for "phrases"
- Map acronyms to their expansions in "acronyms"
- List tool or product names in "tools"
- List technical commands or API names in "commands"

Focus on technical terminology, domain-specific vocabulary, and important concepts.

Return ONLY valid JSON, no other text."#,
        truncated_content
    );

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        }))
        .send()
        .await
        .context("Failed to send request to Ollama")?;

    if !response.status().is_success() {
        anyhow::bail!("Ollama returned error: {}", response.status());
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse Ollama response")?;

    let response_text = response_json["response"]
        .as_str()
        .context("No response field in Ollama output")?;

    let result: ExtractedKeywords = serde_json::from_str(response_text)
        .context("Failed to parse LLM response as structured JSON")?;

    Ok(result)
}

/// Process a single markdown file
async fn process_file(
    path: &Path,
    model: &str,
    max_content_length: usize,
    ollama_url: &str,
) -> Result<DocumentResult> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Processing: {}", file_name);

    let content = fs::read_to_string(path)
        .await
        .context("Failed to read file")?;

    let start = std::time::Instant::now();

    match extract_keywords_llm(&content, model, max_content_length, ollama_url).await {
        Ok(keywords) => {
            let elapsed = start.elapsed().as_millis();
            info!("✓ {} ({} ms)", file_name, elapsed);
            Ok(DocumentResult {
                file: file_name,
                success: true,
                extraction_time_ms: elapsed,
                keywords: Some(keywords),
                error: None,
            })
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            error!("✗ {}: {}", file_name, e);
            Ok(DocumentResult {
                file: file_name,
                success: false,
                extraction_time_ms: elapsed,
                keywords: None,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Process all markdown files in a directory
async fn process_directory(
    input_dir: &Path,
    model: &str,
    max_content_length: usize,
    ollama_url: &str,
) -> Result<Vec<DocumentResult>> {
    let mut results = Vec::new();
    let mut entries = fs::read_dir(input_dir)
        .await
        .context("Failed to read input directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            match process_file(&path, model, max_content_length, ollama_url).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Skipping file due to error: {}", e);
                }
            }
        }
    }

    Ok(results)
}

/// Calculate summary statistics from results
fn calculate_summary(results: &[DocumentResult], total_time_ms: u128) -> ExtractionSummary {
    let total_documents = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total_documents - successful;

    let avg_extraction_time_ms = if successful > 0 {
        results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.extraction_time_ms)
            .sum::<u128>()
            / successful as u128
    } else {
        0
    };

    ExtractionSummary {
        total_documents,
        successful,
        failed,
        avg_extraction_time_ms,
        total_time_ms,
    }
}

/// Save results to JSON file
async fn save_results(
    output_path: &Path,
    results: &[DocumentResult],
    summary: &ExtractionSummary,
) -> Result<()> {
    let output = serde_json::json!({
        "summary": summary,
        "results": results
    });

    let json = serde_json::to_string_pretty(&output)?;
    fs::write(output_path, json)
        .await
        .context("Failed to write output file")?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    let args = Args::parse();

    info!("LLM Keyword Extraction Prototype");
    info!("================================");
    info!("Input directory: {:?}", args.input);
    info!("Output file: {:?}", args.output);
    info!("Model: {}", args.model);
    info!("Ollama URL: {}", args.ollama_url);
    info!("");

    if !args.input.exists() {
        anyhow::bail!("Input directory does not exist: {:?}", args.input);
    }

    let total_start = std::time::Instant::now();

    let results = process_directory(
        &args.input,
        &args.model,
        args.max_content_length,
        &args.ollama_url,
    )
    .await?;

    let total_elapsed = total_start.elapsed().as_millis();

    if results.is_empty() {
        warn!("No markdown files found in input directory");
        return Ok(());
    }

    let summary = calculate_summary(&results, total_elapsed);

    info!("");
    info!("Extraction Summary");
    info!("==================");
    info!("Total documents: {}", summary.total_documents);
    info!("Successful: {}", summary.successful);
    info!("Failed: {}", summary.failed);
    info!(
        "Success rate: {:.1}%",
        (summary.successful as f64 / summary.total_documents as f64) * 100.0
    );
    info!(
        "Average extraction time: {} ms",
        summary.avg_extraction_time_ms
    );
    info!("Total time: {} ms", summary.total_time_ms);

    save_results(&args.output, &results, &summary).await?;

    info!("");
    info!("Results saved to: {:?}", args.output);

    if summary.failed > 0 {
        warn!("Some documents failed to process. Check the output file for details.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_keywords_serialization() {
        let keywords = ExtractedKeywords {
            keywords: vec!["rust".to_string(), "async".to_string()],
            phrases: vec!["error handling".to_string()],
            acronyms: HashMap::from([(
                "API".to_string(),
                "Application Programming Interface".to_string(),
            )]),
            tools: vec!["cargo".to_string()],
            commands: vec!["cargo build".to_string()],
        };

        let json = serde_json::to_string(&keywords).unwrap();
        let deserialized: ExtractedKeywords = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.keywords.len(), 2);
        assert_eq!(deserialized.phrases.len(), 1);
        assert_eq!(deserialized.acronyms.len(), 1);
    }

    #[test]
    fn test_summary_calculation() {
        let results = vec![
            DocumentResult {
                file: "test1.md".to_string(),
                success: true,
                extraction_time_ms: 100,
                keywords: None,
                error: None,
            },
            DocumentResult {
                file: "test2.md".to_string(),
                success: true,
                extraction_time_ms: 200,
                keywords: None,
                error: None,
            },
            DocumentResult {
                file: "test3.md".to_string(),
                success: false,
                extraction_time_ms: 50,
                keywords: None,
                error: Some("test error".to_string()),
            },
        ];

        let summary = calculate_summary(&results, 500);

        assert_eq!(summary.total_documents, 3);
        assert_eq!(summary.successful, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.avg_extraction_time_ms, 150);
        assert_eq!(summary.total_time_ms, 500);
    }
}
