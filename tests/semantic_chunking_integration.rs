//! Integration tests for semantic chunking functionality
//!
//! These tests verify end-to-end chunking behavior including:
//! - Document processing
//! - Code block preservation
//! - Boundary detection
//! - Search integration

use xze_core::semantic::{ChunkMetadata, ChunkerConfig, SemanticChunk, SentenceSplitter};

/// Test end-to-end chunking of a sample document
#[tokio::test]
async fn test_end_to_end_chunking() {
    // Sample document with multiple topics
    let content = r#"
# Getting Started

This is an introduction to the system. It provides basic information.
You will learn how to use the core features.

## Installation

First, install the required dependencies. Run the installation command.
Make sure you have the prerequisites installed.

## Configuration

Configure the system using YAML files. The configuration is straightforward.
You can customize many options to fit your needs.

## Usage

Start using the system with simple commands. The CLI is user-friendly.
Advanced features are available for power users.
"#;

    // Create a mock chunker configuration
    let config = ChunkerConfig {
        similarity_threshold: 0.7,
        min_chunk_sentences: 3,
        max_chunk_sentences: 30,
        similarity_percentile: 0.5,
        min_sentence_length: 10,
        embedding_batch_size: 32,
        model_name: "nomic-embed-text".to_string(),
    };

    // Validate configuration
    assert!(config.validate().is_ok(), "Configuration should be valid");

    // Test sentence splitting
    let splitter = SentenceSplitter::new(config.min_sentence_length);
    let sentences = splitter.split(content);

    // Verify sentences were extracted
    assert!(
        sentences.len() > 5,
        "Should extract multiple sentences from content"
    );

    // Verify no empty sentences
    for sentence in &sentences {
        assert!(!sentence.trim().is_empty(), "Sentences should not be empty");
        assert!(
            sentence.len() >= config.min_sentence_length,
            "Sentences should meet minimum length"
        );
    }

    // Test metadata creation
    let metadata = ChunkMetadata::new("test.md".to_string(), content);

    assert_eq!(metadata.source_file, "test.md");
    assert!(metadata.word_count > 0, "Should count words");
    assert!(metadata.char_count > 0, "Should count characters");
    // Keywords are not auto-extracted in new() - they must be provided via with_details()
    assert!(
        metadata.keywords.is_empty(),
        "new() should not auto-extract keywords"
    );

    // Note: Full chunking test requires Ollama server
    // In CI/CD, this would be mocked or require test infrastructure
}

/// Test that chunking preserves code blocks
#[test]
fn test_chunking_preserves_code_blocks() {
    let content = r#"
Here is some explanation text before the code.

```rust
fn main() {
    println!("Hello, world!");
}
```

And here is text after the code block.
More explanation continues here.
"#;

    let splitter = SentenceSplitter::new(10);
    let sentences = splitter.split(content);

    // Find the code block in sentences
    let has_code_block = sentences.iter().any(|s| s.contains("fn main()"));

    assert!(
        has_code_block,
        "Code block should be preserved in sentence splitting"
    );

    // Verify code block syntax is maintained
    let code_sentence = sentences
        .iter()
        .find(|s| s.contains("fn main()"))
        .expect("Should find code block");

    assert!(
        code_sentence.contains("```"),
        "Code block markers should be preserved"
    );
}

/// Test that chunking respects semantic boundaries
#[test]
fn test_chunking_respects_boundaries() {
    // Create content with clear topic boundaries
    let content = r#"
Topic A starts here. This is about topic A.
More information about topic A follows.

Topic B begins now.
This discusses topic B entirely.
Topic B has different content from A.

Topic C is introduced here.
It covers different material.
Topic C continues with new information.
"#;

    let splitter = SentenceSplitter::new(10);
    let sentences = splitter.split(content);

    // Should have multiple sentences
    assert!(sentences.len() >= 6, "Should split into multiple sentences");

    // Each sentence should be properly trimmed
    for sentence in &sentences {
        assert_eq!(sentence.trim(), *sentence, "Sentences should be trimmed");
    }

    // Verify topic markers are in separate sentences
    let topic_a_count = sentences.iter().filter(|s| s.contains("Topic A")).count();
    let topic_b_count = sentences.iter().filter(|s| s.contains("Topic B")).count();
    let topic_c_count = sentences.iter().filter(|s| s.contains("Topic C")).count();

    assert_eq!(topic_a_count, 1, "Should have one Topic A sentence");
    assert!(
        topic_b_count >= 1,
        "Should have at least one Topic B sentence"
    );
    assert!(
        topic_c_count >= 1,
        "Should have at least one Topic C sentence"
    );
}

/// Test configuration validation
#[test]
fn test_configuration_validation() {
    // Valid configuration
    let valid_config = ChunkerConfig::default();
    assert!(valid_config.validate().is_ok());

    // Invalid similarity threshold (too high)
    let invalid_config = ChunkerConfig {
        similarity_threshold: 1.5,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid similarity threshold (negative)
    let invalid_config = ChunkerConfig {
        similarity_threshold: -0.1,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid min/max relationship
    let invalid_config = ChunkerConfig {
        min_chunk_sentences: 10,
        max_chunk_sentences: 5,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid percentile (too high) - percentile is 0.0-1.0 not 0-100
    let invalid_config = ChunkerConfig {
        similarity_percentile: 1.5,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Invalid percentile (negative)
    let invalid_config = ChunkerConfig {
        similarity_percentile: -0.1,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // Valid configuration with valid percentile
    let valid_config = ChunkerConfig {
        similarity_percentile: 0.75,
        ..Default::default()
    };
    assert!(valid_config.validate().is_ok());
}

/// Test configuration presets
#[test]
fn test_configuration_presets() {
    // Technical docs preset
    let tech_config = ChunkerConfig::technical_docs();
    assert!(tech_config.validate().is_ok());
    assert_eq!(tech_config.similarity_threshold, 0.75);
    assert_eq!(tech_config.max_chunk_sentences, 40);

    // Narrative preset
    let narrative_config = ChunkerConfig::narrative();
    assert!(narrative_config.validate().is_ok());
    assert_eq!(narrative_config.similarity_threshold, 0.65);
    assert_eq!(narrative_config.max_chunk_sentences, 20);

    // Default preset
    let default_config = ChunkerConfig::default();
    assert!(default_config.validate().is_ok());
    assert_eq!(default_config.similarity_threshold, 0.7);
    assert_eq!(default_config.max_chunk_sentences, 30);
}

/// Test sentence splitting with abbreviations
#[test]
fn test_sentence_splitting_with_abbreviations() {
    let content = "Dr. Smith works at the hospital. Mr. Jones is his colleague. They collaborate on research.";

    let splitter = SentenceSplitter::new(5);
    let sentences = splitter.split(content);

    // Should not split on abbreviations
    assert_eq!(sentences.len(), 3, "Should have exactly 3 sentences");

    assert!(
        sentences[0].contains("Dr. Smith"),
        "First sentence should contain Dr. Smith"
    );
    assert!(
        sentences[1].contains("Mr. Jones"),
        "Second sentence should contain Mr. Jones"
    );
}

/// Test sentence splitting filters short fragments
#[test]
fn test_sentence_splitting_filters_short_fragments() {
    let content = "This is a valid sentence. OK. Another valid sentence here.";

    let splitter = SentenceSplitter::new(10);
    let sentences = splitter.split(content);

    // "OK." should be filtered out (too short)
    assert_eq!(sentences.len(), 2, "Should filter out short fragments");

    for sentence in &sentences {
        assert!(
            sentence.len() >= 10,
            "All sentences should meet minimum length"
        );
    }
}

/// Test metadata keyword creation
#[test]
fn test_metadata_with_keywords() {
    let content = "The API provides authentication endpoints. Configure the database connection.";

    let keywords = vec![
        "api".to_string(),
        "authentication".to_string(),
        "endpoints".to_string(),
    ];

    let metadata = ChunkMetadata::with_details(
        "test.md".to_string(),
        Some("API Guide".to_string()),
        Some("reference".to_string()),
        keywords.clone(),
        content,
    );

    // Should have assigned keywords
    assert!(!metadata.keywords.is_empty(), "Should have keywords");
    assert_eq!(metadata.keywords, keywords);

    // Should calculate word count
    assert!(metadata.word_count > 0, "Should count words");

    // Should have title and category
    assert_eq!(metadata.title, Some("API Guide".to_string()));
    assert_eq!(metadata.category, Some("reference".to_string()));
}

/// Test chunk creation
#[test]
fn test_chunk_creation() {
    let metadata = ChunkMetadata::new(
        "test.md".to_string(),
        "Sample content for testing with more words",
    );

    let chunk = SemanticChunk::new(
        "This is chunk content.".to_string(),
        0,
        3,
        0,
        5,
        0.85,
        metadata,
    );

    assert_eq!(chunk.chunk_index, 0);
    assert_eq!(chunk.total_chunks, 3);
    assert_eq!(chunk.start_sentence, 0);
    assert_eq!(chunk.end_sentence, 5);
    assert_eq!(chunk.sentence_count(), 6); // end - start + 1 = 5 - 0 + 1 = 6
    assert_eq!(chunk.avg_similarity, 0.85);
}

/// Test sentence splitting with empty input
#[test]
fn test_sentence_splitting_empty_input() {
    let splitter = SentenceSplitter::new(10);

    let empty_sentences = splitter.split("");
    assert!(
        empty_sentences.is_empty(),
        "Empty input should produce no sentences"
    );

    let whitespace_sentences = splitter.split("   \n\n   ");
    assert!(
        whitespace_sentences.is_empty(),
        "Whitespace-only input should produce no sentences"
    );
}

/// Test sentence splitting with multiple code blocks
#[test]
fn test_sentence_splitting_multiple_code_blocks() {
    let content = r#"
First explanation paragraph.

```rust
let x = 1;
```

Middle explanation.

```bash
echo "test"
```

Final explanation.
"#;

    let splitter = SentenceSplitter::new(5);
    let sentences = splitter.split(content);

    // Count code blocks
    let code_block_count = sentences.iter().filter(|s| s.contains("```")).count();

    assert!(
        code_block_count >= 2,
        "Should preserve multiple code blocks"
    );
}

/// Test chunker configuration bounds
#[test]
fn test_chunker_configuration_bounds() {
    // Test minimum valid values
    let min_config = ChunkerConfig {
        similarity_threshold: 0.0,
        min_chunk_sentences: 1,
        max_chunk_sentences: 1,
        similarity_percentile: 0.0,
        min_sentence_length: 1,
        embedding_batch_size: 1,
        model_name: "test".to_string(),
    };
    assert!(min_config.validate().is_ok());

    // Test maximum valid values (percentile is 0.0-1.0, not 0-100)
    let max_config = ChunkerConfig {
        similarity_threshold: 1.0,
        min_chunk_sentences: 1,
        max_chunk_sentences: 1000,
        similarity_percentile: 1.0,
        min_sentence_length: 1000,
        embedding_batch_size: 1000,
        model_name: "test".to_string(),
    };
    assert!(max_config.validate().is_ok());
}
