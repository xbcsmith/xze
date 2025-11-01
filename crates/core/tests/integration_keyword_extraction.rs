//! Integration tests for keyword extraction and document enrichment
//!
//! Tests the end-to-end integration of LLM-based keyword extraction
//! with the semantic chunking and document loading pipeline.

use xze_core::{
    document_enrichment::{DocumentEnricher, EnrichmentConfig, EnrichmentStats},
    keyword_extractor::{KeywordExtractor, KeywordExtractorConfig},
    semantic::types::SemanticChunk,
    ChunkMetadata, Result,
};

/// Helper function to create a test chunk
fn create_test_chunk(index: usize, content: &str, source_file: &str) -> SemanticChunk {
    SemanticChunk::new(
        content.to_string(),
        index,
        1,
        0,
        0,
        1.0,
        ChunkMetadata::new(source_file.to_string(), content),
    )
}

#[tokio::test]
async fn test_enricher_creation_with_default_config() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await;
    assert!(enricher.is_ok());
}

#[tokio::test]
async fn test_enricher_creation_with_invalid_config() {
    let mut config = EnrichmentConfig::default();
    config.max_keywords_per_chunk = 0;

    let enricher = DocumentEnricher::new(config).await;
    assert!(enricher.is_err());
}

#[tokio::test]
async fn test_enrich_single_chunk_with_frequency() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = r#"
# Rust Programming Guide

Rust is a systems programming language that runs blazingly fast, prevents segfaults,
and guarantees thread safety. Rust achieves memory safety without garbage collection.
The Rust compiler uses ownership rules to manage memory automatically.
"#;

    let mut chunk = create_test_chunk(0, content, "docs/rust_guide.md");

    let method = enricher.enrich_chunk(&mut chunk).await.unwrap();

    assert_eq!(method, "frequency");
    assert!(!chunk.metadata.keywords.is_empty());
    assert!(chunk.metadata.keywords.contains(&"rust".to_string()));
    assert!(chunk
        .metadata
        .keywords
        .iter()
        .any(|k| k.contains("programming") || k.contains("memory")));
}

#[tokio::test]
async fn test_enrich_chunk_skips_short_content() {
    let config = EnrichmentConfig {
        min_content_length: 100,
        ..EnrichmentConfig::frequency_only()
    };
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let mut chunk = create_test_chunk(0, "Short text", "test.md");

    let method = enricher.enrich_chunk(&mut chunk).await.unwrap();

    assert_eq!(method, "skipped");
    assert!(chunk.metadata.keywords.is_empty());
}

#[tokio::test]
async fn test_enrich_multiple_chunks_batch() {
    let config = EnrichmentConfig {
        batch_size: 3,
        batch_workers: 2,
        ..EnrichmentConfig::frequency_only()
    };
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let test_contents = vec![
        "Kubernetes orchestrates containerized applications across clusters.",
        "Docker provides container runtime for packaging applications.",
        "Terraform manages infrastructure as code using declarative configuration.",
        "Ansible automates configuration management and application deployment.",
        "Jenkins enables continuous integration and continuous delivery pipelines.",
    ];

    let mut chunks: Vec<SemanticChunk> = test_contents
        .iter()
        .enumerate()
        .map(|(i, content)| create_test_chunk(i, content, &format!("doc_{}.md", i)))
        .collect();

    let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();

    assert_eq!(methods.len(), 5);
    assert!(methods.iter().all(|m| m == "frequency"));

    for (i, chunk) in chunks.iter().enumerate() {
        assert!(
            !chunk.metadata.keywords.is_empty(),
            "Chunk {} should have keywords",
            i
        );
    }

    // Verify specific keywords for first chunk
    assert!(chunks[0]
        .metadata
        .keywords
        .contains(&"kubernetes".to_string()));
}

#[tokio::test]
async fn test_enrich_chunks_respects_max_keywords() {
    let config = EnrichmentConfig {
        max_keywords_per_chunk: 5,
        ..EnrichmentConfig::frequency_only()
    };
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = "Rust programming language provides memory safety concurrency performance \
                   ownership borrowing lifetimes traits generics macros async await testing \
                   documentation cargo clippy rustfmt compiler type system error handling";

    let mut chunk = create_test_chunk(0, content, "test.md");

    enricher.enrich_chunk(&mut chunk).await.unwrap();

    assert!(chunk.metadata.keywords.len() <= 5);
}

#[tokio::test]
async fn test_enrichment_statistics_tracking() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let stats_before = enricher.stats().await;
    assert_eq!(stats_before.total_enriched, 0);

    let content =
        "Testing enrichment statistics with meaningful content about software development.";
    let mut chunk = create_test_chunk(0, content, "test.md");

    enricher.enrich_chunk(&mut chunk).await.unwrap();

    let stats_after = enricher.stats().await;
    assert_eq!(stats_after.total_enriched, 1);
    assert_eq!(stats_after.frequency_enriched, 1);
    assert!(stats_after.avg_keywords_per_chunk > 0.0);
    assert!(stats_after.total_processing_ms > 0);
}

#[tokio::test]
async fn test_enrichment_stats_reset() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = "Content for testing statistics reset functionality.";
    let mut chunk = create_test_chunk(0, content, "test.md");

    enricher.enrich_chunk(&mut chunk).await.unwrap();
    let stats = enricher.stats().await;
    assert!(stats.total_enriched > 0);

    enricher.reset_stats().await;
    let stats_after_reset = enricher.stats().await;
    assert_eq!(stats_after_reset.total_enriched, 0);
    assert_eq!(stats_after_reset.total_processing_ms, 0);
}

#[tokio::test]
async fn test_enrich_empty_batch() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let mut chunks: Vec<SemanticChunk> = vec![];
    let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();

    assert!(methods.is_empty());
}

#[tokio::test]
async fn test_enrich_different_document_types() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let tutorial_content = r#"
# Getting Started Tutorial

Follow these steps to install and configure the application.
First, download the installer. Next, run the setup wizard.
"#;

    let reference_content = r#"
# API Reference

Function: authenticate(username, password)
Returns: AuthToken
Throws: AuthenticationError
"#;

    let explanation_content = r#"
# Architecture Explanation

The system uses microservices architecture with event-driven communication.
Services communicate through message queues for loose coupling.
"#;

    let mut chunks = vec![
        create_test_chunk(0, tutorial_content, "docs/tutorials/getting_started.md"),
        create_test_chunk(1, reference_content, "docs/reference/api.md"),
        create_test_chunk(2, explanation_content, "docs/explanations/architecture.md"),
    ];

    let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();

    assert_eq!(methods.len(), 3);
    for chunk in &chunks {
        assert!(!chunk.metadata.keywords.is_empty());
    }

    // Tutorial should have action-oriented keywords
    assert!(chunks[0]
        .metadata
        .keywords
        .iter()
        .any(|k| k.contains("install") || k.contains("setup") || k.contains("configure")));

    // Reference should have technical keywords
    assert!(chunks[1]
        .metadata
        .keywords
        .iter()
        .any(|k| k.contains("function") || k.contains("api") || k.contains("authenticate")));

    // Explanation should have conceptual keywords
    assert!(chunks[2]
        .metadata
        .keywords
        .iter()
        .any(|k| k.contains("architecture")
            || k.contains("microservices")
            || k.contains("system")));
}

#[tokio::test]
async fn test_keyword_deduplication() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = "Rust Rust Rust programming programming language language Rust programming.";

    let mut chunk = create_test_chunk(0, content, "test.md");

    enricher.enrich_chunk(&mut chunk).await.unwrap();

    // Check that keywords are deduplicated
    let keywords = &chunk.metadata.keywords;
    let unique_keywords: std::collections::HashSet<_> = keywords.iter().collect();

    assert_eq!(keywords.len(), unique_keywords.len());
}

#[tokio::test]
async fn test_enrich_code_heavy_content() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = r#"
# Rust Code Example

```rust
fn main() {
    let mut vector = Vec::new();
    vector.push(1);
    vector.push(2);
    println!("Vector: {:?}", vector);
}
```

This example demonstrates vector usage in Rust.
"#;

    let mut chunk = create_test_chunk(0, content, "examples/vectors.md");

    enricher.enrich_chunk(&mut chunk).await.unwrap();

    assert!(!chunk.metadata.keywords.is_empty());
    assert!(chunk
        .metadata
        .keywords
        .iter()
        .any(|k| k.contains("rust") || k.contains("vector") || k.contains("example")));
}

#[tokio::test]
async fn test_parallel_batch_processing_performance() {
    let config = EnrichmentConfig {
        batch_size: 5,
        batch_workers: 4,
        ..EnrichmentConfig::frequency_only()
    };
    let enricher = DocumentEnricher::new(config).await.unwrap();

    // Create 20 test chunks
    let mut chunks: Vec<SemanticChunk> = (0..20)
        .map(|i| {
            create_test_chunk(
                i,
                &format!(
                    "Document {} about software development with Rust programming language.",
                    i
                ),
                &format!("doc_{}.md", i),
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(methods.len(), 20);
    assert!(duration.as_secs() < 10); // Should complete quickly with parallelization

    for chunk in &chunks {
        assert!(!chunk.metadata.keywords.is_empty());
    }
}

#[tokio::test]
async fn test_enricher_with_llm_disabled() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    assert!(!enricher.is_llm_enabled().await);
}

#[tokio::test]
async fn test_config_access() {
    let config = EnrichmentConfig {
        max_keywords_per_chunk: 20,
        ..EnrichmentConfig::default()
    };
    let enricher = DocumentEnricher::new(config.clone()).await.unwrap();

    assert_eq!(enricher.config().max_keywords_per_chunk, 20);
}

#[tokio::test]
async fn test_enrich_multilingual_content() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = "Rust programming language. Unicode support: 你好世界. Emoji test: 🚀.";

    let mut chunk = create_test_chunk(0, content, "test.md");

    let result = enricher.enrich_chunk(&mut chunk).await;
    assert!(result.is_ok());
    assert!(!chunk.metadata.keywords.is_empty());
}

#[tokio::test]
async fn test_keyword_extractor_integration() {
    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config);
    assert!(extractor.is_ok());

    let extractor = extractor.unwrap();
    let content = "Rust programming language provides memory safety and performance.";
    let result = extractor.extract_with_frequency(content);

    assert!(result.is_ok());
    let keywords = result.unwrap();
    assert!(!keywords.all_keywords().is_empty());
}

#[tokio::test]
async fn test_batch_extraction_with_extractor() {
    let config = KeywordExtractorConfig::default();
    let extractor = KeywordExtractor::new(config).unwrap();

    let docs = vec![
        "Kubernetes orchestrates containers.",
        "Docker provides container runtime.",
        "Terraform manages infrastructure.",
    ];

    let results = extractor.extract_batch(&docs).await;
    assert_eq!(results.len(), 3);

    for result in results {
        assert!(result.is_ok());
        let keywords = result.unwrap();
        assert!(!keywords.all_keywords().is_empty());
    }
}

#[tokio::test]
async fn test_chunk_metadata_preservation() {
    let config = EnrichmentConfig::frequency_only();
    let enricher = DocumentEnricher::new(config).await.unwrap();

    let content = "Test content for metadata preservation. This chunk contains enough text to pass the minimum content length filter and trigger keyword extraction using the frequency-based method.";
    let mut chunk = SemanticChunk::new(
        content.to_string(),
        5,
        10,
        3,
        7,
        0.87,
        ChunkMetadata {
            source_file: "test.md".to_string(),
            title: Some("Test Title".to_string()),
            category: Some("tutorial".to_string()),
            keywords: vec!["existing".to_string()],
            word_count: 28,
            char_count: 180,
        },
    );

    let original_title = chunk.metadata.title.clone();
    let original_category = chunk.metadata.category.clone();
    let original_index = chunk.chunk_index;

    enricher.enrich_chunk(&mut chunk).await.unwrap();

    // Verify metadata is preserved
    assert_eq!(chunk.metadata.title, original_title);
    assert_eq!(chunk.metadata.category, original_category);
    assert_eq!(chunk.chunk_index, original_index);

    // Keywords should be enriched (extracted from content)
    assert!(!chunk.metadata.keywords.is_empty());
    // Verify keywords are extracted from the actual content
    assert!(chunk.metadata.keywords.iter().any(|k| k.contains("test")
        || k.contains("content")
        || k.contains("metadata")
        || k.contains("preservation")));
}

#[tokio::test]
#[ignore] // Slow test - only run when explicitly requested
async fn test_large_scale_enrichment() {
    let config = EnrichmentConfig {
        batch_size: 50,
        batch_workers: 8,
        ..EnrichmentConfig::frequency_only()
    };
    let enricher = DocumentEnricher::new(config).await.unwrap();

    // Create 1000 test chunks
    let mut chunks: Vec<SemanticChunk> = (0..1000)
        .map(|i| {
            create_test_chunk(
                i,
                &format!(
                    "Document {} discusses software engineering practices and development methodologies.",
                    i
                ),
                &format!("docs/doc_{}.md", i),
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let methods = enricher.enrich_chunks(&mut chunks).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(methods.len(), 1000);
    println!("Processed 1000 chunks in {:.2}s", duration.as_secs_f64());
    println!("Average: {:.2} chunks/sec", 1000.0 / duration.as_secs_f64());

    let stats = enricher.stats().await;
    assert_eq!(stats.total_enriched, 1000);
    assert!(stats.avg_keywords_per_chunk > 0.0);
}
