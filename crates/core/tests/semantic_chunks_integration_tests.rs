//! Integration tests for semantic chunk database operations
//!
//! These tests validate the complete semantic chunk storage and retrieval workflow including:
//! - Storing semantic chunks with embeddings
//! - Retrieving chunks by file path
//! - Deleting chunks for a file
//! - Handling edge cases and errors

use std::path::Path;
use xze_core::kb::{error::Result, store::KbStore};
use xze_core::semantic::types::{ChunkMetadata, SemanticChunk};

/// Test database URL - uses environment variable or default
fn test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/xze_test".to_string())
}

/// Setup test database with schema
///
/// Creates a fresh database connection pool and runs migrations.
/// Assumes the database exists and is accessible.
async fn setup_test_db() -> Result<sqlx::PgPool> {
    let pool = sqlx::PgPool::connect(&test_db_url())
        .await
        .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;

    // Clean up any existing test data
    sqlx::query("TRUNCATE TABLE semantic_chunks CASCADE")
        .execute(&pool)
        .await
        .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;

    Ok(pool)
}

/// Helper to create a test chunk with embedding
fn create_test_chunk(
    index: usize,
    total: usize,
    content: &str,
    embedding: Vec<f32>,
) -> SemanticChunk {
    let metadata = ChunkMetadata {
        source_file: "test.md".to_string(),
        title: Some("Test Document".to_string()),
        category: Some("tutorial".to_string()),
        keywords: vec!["test".to_string(), "example".to_string()],
        word_count: content.split_whitespace().count(),
        char_count: content.chars().count(),
    };

    let mut chunk = SemanticChunk::new(
        content.to_string(),
        index,
        total,
        index * 3,
        index * 3 + 2,
        0.85,
        metadata,
    );
    chunk.set_embedding(embedding);
    chunk
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_store_and_retrieve_semantic_chunks() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/guide.md");
    let file_hash = "abc123def456";

    // Create test chunks with embeddings
    let chunks = vec![
        create_test_chunk(0, 3, "First chunk content.", vec![0.1, 0.2, 0.3]),
        create_test_chunk(1, 3, "Second chunk content.", vec![0.4, 0.5, 0.6]),
        create_test_chunk(2, 3, "Third chunk content.", vec![0.7, 0.8, 0.9]),
    ];

    // Store chunks
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve chunks
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;

    // Verify count
    assert_eq!(retrieved.len(), 3);

    // Verify chunk contents
    assert_eq!(retrieved[0].content, "First chunk content.");
    assert_eq!(retrieved[1].content, "Second chunk content.");
    assert_eq!(retrieved[2].content, "Third chunk content.");

    // Verify chunk indices
    assert_eq!(retrieved[0].chunk_index, 0);
    assert_eq!(retrieved[1].chunk_index, 1);
    assert_eq!(retrieved[2].chunk_index, 2);

    // Verify embeddings
    assert_eq!(retrieved[0].embedding, vec![0.1, 0.2, 0.3]);
    assert_eq!(retrieved[1].embedding, vec![0.4, 0.5, 0.6]);
    assert_eq!(retrieved[2].embedding, vec![0.7, 0.8, 0.9]);

    // Verify metadata
    assert_eq!(retrieved[0].metadata.source_file, "test.md");
    assert_eq!(
        retrieved[0].metadata.title,
        Some("Test Document".to_string())
    );
    assert_eq!(retrieved[0].metadata.category, Some("tutorial".to_string()));
    assert_eq!(retrieved[0].metadata.keywords.len(), 2);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_delete_semantic_chunks_for_file() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/deleteme.md");
    let file_hash = "xyz789";

    // Create and store test chunks
    let chunks = vec![
        create_test_chunk(0, 2, "Chunk to delete 1.", vec![0.1, 0.2]),
        create_test_chunk(1, 2, "Chunk to delete 2.", vec![0.3, 0.4]),
    ];

    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Verify chunks were stored
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved.len(), 2);

    // Delete chunks
    let deleted_count = store.delete_semantic_chunks_for_file(file_path).await?;
    assert_eq!(deleted_count, 2);

    // Verify chunks were deleted
    let retrieved_after = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved_after.len(), 0);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_delete_nonexistent_file_returns_zero() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/nonexistent.md");

    // Delete chunks for file that does not exist
    let deleted_count = store.delete_semantic_chunks_for_file(file_path).await?;
    assert_eq!(deleted_count, 0);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_retrieve_nonexistent_file_returns_empty() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/nonexistent.md");

    // Retrieve chunks for file that does not exist
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved.len(), 0);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_store_chunks_with_large_embeddings() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/large_embeddings.md");
    let file_hash = "large123";

    // Create chunk with large embedding (typical size: 384, 768, or 1536 dimensions)
    let large_embedding: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();

    let chunks = vec![create_test_chunk(
        0,
        1,
        "Chunk with large embedding.",
        large_embedding.clone(),
    )];

    // Store chunk
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve and verify
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].embedding.len(), 768);
    assert_eq!(retrieved[0].embedding, large_embedding);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_store_chunks_with_empty_optional_fields() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/minimal.md");
    let file_hash = "minimal123";

    // Create chunk with minimal metadata
    let metadata = ChunkMetadata {
        source_file: "minimal.md".to_string(),
        title: None,
        category: None,
        keywords: vec![],
        word_count: 5,
        char_count: 20,
    };

    let mut chunk = SemanticChunk::new(
        "Minimal chunk content.".to_string(),
        0,
        1,
        0,
        0,
        0.9,
        metadata,
    );
    chunk.set_embedding(vec![0.1, 0.2, 0.3]);

    let chunks = vec![chunk];

    // Store chunk
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve and verify
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].metadata.source_file, "minimal.md");
    assert!(retrieved[0].metadata.title.is_none());
    assert!(retrieved[0].metadata.category.is_none());
    assert!(retrieved[0].metadata.keywords.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_update_chunks_by_replacing() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/updateme.md");
    let file_hash_v1 = "v1hash";
    let file_hash_v2 = "v2hash";

    // Store initial chunks
    let initial_chunks = vec![
        create_test_chunk(0, 2, "Old chunk 1.", vec![0.1, 0.2]),
        create_test_chunk(1, 2, "Old chunk 2.", vec![0.3, 0.4]),
    ];

    store
        .store_semantic_chunks(file_path, file_hash_v1, &initial_chunks)
        .await?;

    // Verify initial storage
    let retrieved_v1 = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved_v1.len(), 2);
    assert_eq!(retrieved_v1[0].content, "Old chunk 1.");

    // Delete old chunks and store new ones
    store.delete_semantic_chunks_for_file(file_path).await?;

    let updated_chunks = vec![
        create_test_chunk(0, 3, "New chunk 1.", vec![0.5, 0.6]),
        create_test_chunk(1, 3, "New chunk 2.", vec![0.7, 0.8]),
        create_test_chunk(2, 3, "New chunk 3.", vec![0.9, 1.0]),
    ];

    store
        .store_semantic_chunks(file_path, file_hash_v2, &updated_chunks)
        .await?;

    // Verify updated storage
    let retrieved_v2 = store.get_semantic_chunks_for_file(file_path).await?;
    assert_eq!(retrieved_v2.len(), 3);
    assert_eq!(retrieved_v2[0].content, "New chunk 1.");
    assert_eq!(retrieved_v2[1].content, "New chunk 2.");
    assert_eq!(retrieved_v2[2].content, "New chunk 3.");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_chunks_ordered_by_index() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/ordered.md");
    let file_hash = "order123";

    // Create chunks in random order
    let chunks = vec![
        create_test_chunk(2, 5, "Chunk 2.", vec![0.2]),
        create_test_chunk(0, 5, "Chunk 0.", vec![0.0]),
        create_test_chunk(4, 5, "Chunk 4.", vec![0.4]),
        create_test_chunk(1, 5, "Chunk 1.", vec![0.1]),
        create_test_chunk(3, 5, "Chunk 3.", vec![0.3]),
    ];

    // Store chunks
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve chunks
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;

    // Verify they are ordered by chunk_index
    assert_eq!(retrieved.len(), 5);
    assert_eq!(retrieved[0].chunk_index, 0);
    assert_eq!(retrieved[1].chunk_index, 1);
    assert_eq!(retrieved[2].chunk_index, 2);
    assert_eq!(retrieved[3].chunk_index, 3);
    assert_eq!(retrieved[4].chunk_index, 4);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_chunks_preserve_similarity_scores() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/similarity.md");
    let file_hash = "sim123";

    // Create chunks with different similarity scores
    let metadata = ChunkMetadata::new("test.md".to_string(), "Content");

    let mut chunk1 =
        SemanticChunk::new("Content 1".to_string(), 0, 3, 0, 0, 0.95, metadata.clone());
    chunk1.set_embedding(vec![0.1]);

    let mut chunk2 =
        SemanticChunk::new("Content 2".to_string(), 1, 3, 1, 1, 0.75, metadata.clone());
    chunk2.set_embedding(vec![0.2]);

    let mut chunk3 = SemanticChunk::new("Content 3".to_string(), 2, 3, 2, 2, 0.60, metadata);
    chunk3.set_embedding(vec![0.3]);

    let chunks = vec![chunk1, chunk2, chunk3];

    // Store chunks
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve chunks
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;

    // Verify similarity scores are preserved
    assert_eq!(retrieved.len(), 3);
    assert!((retrieved[0].avg_similarity - 0.95).abs() < 0.001);
    assert!((retrieved[1].avg_similarity - 0.75).abs() < 0.001);
    assert!((retrieved[2].avg_similarity - 0.60).abs() < 0.001);

    Ok(())
}

#[tokio::test]
#[ignore] // Requires database connection
async fn test_chunks_preserve_sentence_ranges() -> Result<()> {
    let pool = setup_test_db().await?;
    let store = KbStore::new(pool);

    let file_path = Path::new("docs/ranges.md");
    let file_hash = "range123";

    // Create chunks with specific sentence ranges
    let chunks = vec![
        create_test_chunk(0, 3, "Sentences 0-5.", vec![0.1]),
        create_test_chunk(1, 3, "Sentences 6-12.", vec![0.2]),
        create_test_chunk(2, 3, "Sentences 13-20.", vec![0.3]),
    ];

    // Store chunks
    store
        .store_semantic_chunks(file_path, file_hash, &chunks)
        .await?;

    // Retrieve chunks
    let retrieved = store.get_semantic_chunks_for_file(file_path).await?;

    // Verify sentence ranges
    assert_eq!(retrieved.len(), 3);
    assert_eq!(retrieved[0].start_sentence, 0);
    assert_eq!(retrieved[0].end_sentence, 2);
    assert_eq!(retrieved[1].start_sentence, 3);
    assert_eq!(retrieved[1].end_sentence, 5);
    assert_eq!(retrieved[2].start_sentence, 6);
    assert_eq!(retrieved[2].end_sentence, 8);

    Ok(())
}
