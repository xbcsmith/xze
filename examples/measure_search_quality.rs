//! Measure baseline search quality metrics
//!
//! This tool evaluates search quality using standard metrics:
//! - Precision@K: Percentage of top K results that are relevant
//! - Recall@K: Percentage of relevant docs found in top K results
//! - Mean Reciprocal Rank (MRR): Average of reciprocal ranks of first relevant result
//! - Zero-result rate: Percentage of queries returning no results
//!
//! # Usage
//!
//! ```bash
//! cargo run --example measure_search_quality -- \
//!     --queries test_queries.json \
//!     --output baseline_metrics.json
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use tokio::fs;
use tracing::{info, warn};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input JSON file with test queries and expected results
    #[arg(short, long)]
    queries: PathBuf,

    /// Output JSON file for metrics
    #[arg(short, long)]
    output: PathBuf,

    /// Number of top results to evaluate (K)
    #[arg(short = 'k', long, default_value = "5")]
    top_k: usize,

    /// Search service URL (if testing against running service)
    #[arg(long)]
    search_url: Option<String>,
}

/// Test query with expected relevant documents
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestQuery {
    /// Query text
    query: String,

    /// Intent/category of the query
    intent: String,

    /// List of document IDs that are relevant to this query
    relevant_docs: Vec<String>,

    /// Optional: Minimum expected similarity score
    #[serde(skip_serializing_if = "Option::is_none")]
    min_similarity: Option<f32>,
}

/// Search result from the system
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    doc_id: String,
    similarity: f32,
    title: Option<String>,
}

/// Metrics for a single query
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryMetrics {
    query: String,
    intent: String,
    num_results: usize,
    relevant_in_results: usize,
    precision_at_k: f32,
    recall_at_k: f32,
    reciprocal_rank: f32,
    has_zero_results: bool,
}

/// Overall metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsSummary {
    total_queries: usize,
    avg_precision_at_k: f32,
    avg_recall_at_k: f32,
    mean_reciprocal_rank: f32,
    zero_result_rate: f32,
    queries_with_results: usize,
    avg_results_per_query: f32,
}

/// Load test queries from JSON file
async fn load_test_queries(path: &PathBuf) -> Result<Vec<TestQuery>> {
    let content = fs::read_to_string(path)
        .await
        .context("Failed to read queries file")?;

    let queries: Vec<TestQuery> =
        serde_json::from_str(&content).context("Failed to parse queries JSON")?;

    Ok(queries)
}

/// Execute search query (mock implementation for prototype)
///
/// In production, this would call the actual search service
async fn execute_search_query(
    query: &str,
    top_k: usize,
    _search_url: Option<&str>,
) -> Result<Vec<SearchResult>> {
    // MOCK IMPLEMENTATION for Phase 0 prototype
    // Replace with actual search API call in production

    info!("Executing search: {}", query);

    // For Phase 0, return mock results
    // In production, call actual search API:
    // let client = reqwest::Client::new();
    // let response = client.get(search_url).query(&[("q", query)]).send().await?;

    let mock_results = vec![
        SearchResult {
            doc_id: format!("doc_{}", 1),
            similarity: 0.85,
            title: Some("Mock Document 1".to_string()),
        },
        SearchResult {
            doc_id: format!("doc_{}", 2),
            similarity: 0.72,
            title: Some("Mock Document 2".to_string()),
        },
        SearchResult {
            doc_id: format!("doc_{}", 3),
            similarity: 0.68,
            title: Some("Mock Document 3".to_string()),
        },
    ];

    Ok(mock_results.into_iter().take(top_k).collect())
}

/// Calculate metrics for a single query
fn calculate_query_metrics(
    test_query: &TestQuery,
    results: &[SearchResult],
    k: usize,
) -> QueryMetrics {
    let num_results = results.len();
    let has_zero_results = num_results == 0;

    let result_doc_ids: Vec<&str> = results.iter().map(|r| r.doc_id.as_str()).collect();

    let relevant_in_results = result_doc_ids
        .iter()
        .filter(|&&doc_id| test_query.relevant_docs.iter().any(|rd| rd == doc_id))
        .count();

    let precision_at_k = if num_results > 0 {
        relevant_in_results as f32 / k.min(num_results) as f32
    } else {
        0.0
    };

    let recall_at_k = if !test_query.relevant_docs.is_empty() {
        relevant_in_results as f32 / test_query.relevant_docs.len() as f32
    } else {
        0.0
    };

    let reciprocal_rank = result_doc_ids
        .iter()
        .position(|&doc_id| test_query.relevant_docs.iter().any(|rd| rd == doc_id))
        .map(|pos| 1.0 / (pos + 1) as f32)
        .unwrap_or(0.0);

    QueryMetrics {
        query: test_query.query.clone(),
        intent: test_query.intent.clone(),
        num_results,
        relevant_in_results,
        precision_at_k,
        recall_at_k,
        reciprocal_rank,
        has_zero_results,
    }
}

/// Calculate overall summary from individual query metrics
fn calculate_summary(metrics: &[QueryMetrics]) -> MetricsSummary {
    let total_queries = metrics.len();

    if total_queries == 0 {
        return MetricsSummary {
            total_queries: 0,
            avg_precision_at_k: 0.0,
            avg_recall_at_k: 0.0,
            mean_reciprocal_rank: 0.0,
            zero_result_rate: 0.0,
            queries_with_results: 0,
            avg_results_per_query: 0.0,
        };
    }

    let avg_precision_at_k =
        metrics.iter().map(|m| m.precision_at_k).sum::<f32>() / total_queries as f32;
    let avg_recall_at_k = metrics.iter().map(|m| m.recall_at_k).sum::<f32>() / total_queries as f32;
    let mean_reciprocal_rank =
        metrics.iter().map(|m| m.reciprocal_rank).sum::<f32>() / total_queries as f32;

    let queries_with_zero_results = metrics.iter().filter(|m| m.has_zero_results).count();
    let zero_result_rate = queries_with_zero_results as f32 / total_queries as f32;

    let queries_with_results = total_queries - queries_with_zero_results;
    let avg_results_per_query =
        metrics.iter().map(|m| m.num_results).sum::<usize>() as f32 / total_queries as f32;

    MetricsSummary {
        total_queries,
        avg_precision_at_k,
        avg_recall_at_k,
        mean_reciprocal_rank,
        zero_result_rate,
        queries_with_results,
        avg_results_per_query,
    }
}

/// Save metrics to JSON file
async fn save_metrics(
    output_path: &PathBuf,
    summary: &MetricsSummary,
    query_metrics: &[QueryMetrics],
) -> Result<()> {
    let output = serde_json::json!({
        "summary": summary,
        "query_metrics": query_metrics,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let json = serde_json::to_string_pretty(&output)?;
    fs::write(output_path, json)
        .await
        .context("Failed to write output file")?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let args = Args::parse();

    info!("Search Quality Measurement Tool");
    info!("================================");
    info!("Queries file: {:?}", args.queries);
    info!("Output file: {:?}", args.output);
    info!("Top K: {}", args.top_k);
    info!("");

    if !args.queries.exists() {
        anyhow::bail!("Queries file does not exist: {:?}", args.queries);
    }

    let test_queries = load_test_queries(&args.queries).await?;
    info!("Loaded {} test queries", test_queries.len());

    let mut query_metrics = Vec::new();

    for test_query in &test_queries {
        let results =
            execute_search_query(&test_query.query, args.top_k, args.search_url.as_deref()).await?;

        let metrics = calculate_query_metrics(test_query, &results, args.top_k);
        query_metrics.push(metrics);
    }

    let summary = calculate_summary(&query_metrics);

    info!("");
    info!("Metrics Summary");
    info!("===============");
    info!("Total queries: {}", summary.total_queries);
    info!(
        "Avg Precision@{}: {:.3}",
        args.top_k, summary.avg_precision_at_k
    );
    info!("Avg Recall@{}: {:.3}", args.top_k, summary.avg_recall_at_k);
    info!("Mean Reciprocal Rank: {:.3}", summary.mean_reciprocal_rank);
    info!("Zero-result rate: {:.1}%", summary.zero_result_rate * 100.0);
    info!("Queries with results: {}", summary.queries_with_results);
    info!(
        "Avg results per query: {:.1}",
        summary.avg_results_per_query
    );

    save_metrics(&args.output, &summary, &query_metrics).await?;

    info!("");
    info!("Metrics saved to: {:?}", args.output);

    if summary.zero_result_rate > 0.1 {
        warn!(
            "High zero-result rate detected: {:.1}%",
            summary.zero_result_rate * 100.0
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_calculation() {
        let test_query = TestQuery {
            query: "rust async".to_string(),
            intent: "tutorial".to_string(),
            relevant_docs: vec!["doc1".to_string(), "doc2".to_string(), "doc5".to_string()],
            min_similarity: None,
        };

        let results = vec![
            SearchResult {
                doc_id: "doc1".to_string(),
                similarity: 0.9,
                title: None,
            },
            SearchResult {
                doc_id: "doc3".to_string(),
                similarity: 0.8,
                title: None,
            },
            SearchResult {
                doc_id: "doc5".to_string(),
                similarity: 0.7,
                title: None,
            },
        ];

        let metrics = calculate_query_metrics(&test_query, &results, 5);

        assert_eq!(metrics.relevant_in_results, 2);
        assert_eq!(metrics.precision_at_k, 2.0 / 3.0);
        assert_eq!(metrics.recall_at_k, 2.0 / 3.0);
        assert_eq!(metrics.reciprocal_rank, 1.0);
    }

    #[test]
    fn test_zero_results() {
        let test_query = TestQuery {
            query: "test query".to_string(),
            intent: "howto".to_string(),
            relevant_docs: vec!["doc1".to_string()],
            min_similarity: None,
        };

        let results = vec![];

        let metrics = calculate_query_metrics(&test_query, &results, 5);

        assert!(metrics.has_zero_results);
        assert_eq!(metrics.precision_at_k, 0.0);
        assert_eq!(metrics.recall_at_k, 0.0);
        assert_eq!(metrics.reciprocal_rank, 0.0);
    }

    #[test]
    fn test_summary_calculation() {
        let metrics = vec![
            QueryMetrics {
                query: "query1".to_string(),
                intent: "tutorial".to_string(),
                num_results: 3,
                relevant_in_results: 2,
                precision_at_k: 0.67,
                recall_at_k: 0.67,
                reciprocal_rank: 1.0,
                has_zero_results: false,
            },
            QueryMetrics {
                query: "query2".to_string(),
                intent: "howto".to_string(),
                num_results: 0,
                relevant_in_results: 0,
                precision_at_k: 0.0,
                recall_at_k: 0.0,
                reciprocal_rank: 0.0,
                has_zero_results: true,
            },
        ];

        let summary = calculate_summary(&metrics);

        assert_eq!(summary.total_queries, 2);
        assert_eq!(summary.queries_with_results, 1);
        assert_eq!(summary.zero_result_rate, 0.5);
        assert!((summary.avg_precision_at_k - 0.335).abs() < 0.01);
    }
}
