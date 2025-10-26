-- Migration: Add semantic_chunks table for semantic chunking
-- Created: 2025-01-01
-- Phase: 4 - Semantic Chunking Database Integration

-- Create semantic_chunks table
CREATE TABLE IF NOT EXISTS semantic_chunks (
    -- Primary key
    id BIGSERIAL PRIMARY KEY,

    -- File identification
    file_path TEXT NOT NULL,
    file_hash VARCHAR(64) NOT NULL,

    -- Chunk identification and positioning
    chunk_index INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL,
    start_sentence INTEGER NOT NULL,
    end_sentence INTEGER NOT NULL,

    -- Content
    content TEXT NOT NULL,

    -- Embedding vector for semantic search (stored as bytea)
    embedding BYTEA NOT NULL,

    -- Similarity metrics
    avg_similarity REAL NOT NULL,

    -- Metadata fields
    source_file TEXT,
    title TEXT,
    category TEXT,
    keywords TEXT[],
    word_count INTEGER NOT NULL DEFAULT 0,
    char_count INTEGER NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT semantic_chunks_file_chunk_unique UNIQUE (file_path, chunk_index),
    CONSTRAINT semantic_chunks_chunk_index_positive CHECK (chunk_index >= 0),
    CONSTRAINT semantic_chunks_total_chunks_positive CHECK (total_chunks > 0),
    CONSTRAINT semantic_chunks_sentence_range_valid CHECK (start_sentence <= end_sentence),
    CONSTRAINT semantic_chunks_word_count_nonnegative CHECK (word_count >= 0),
    CONSTRAINT semantic_chunks_char_count_nonnegative CHECK (char_count >= 0)
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_semantic_chunks_file_path
ON semantic_chunks(file_path);

CREATE INDEX IF NOT EXISTS idx_semantic_chunks_file_hash
ON semantic_chunks(file_hash);

CREATE INDEX IF NOT EXISTS idx_semantic_chunks_file_path_hash
ON semantic_chunks(file_path, file_hash);

CREATE INDEX IF NOT EXISTS idx_semantic_chunks_chunk_index
ON semantic_chunks(file_path, chunk_index);

CREATE INDEX IF NOT EXISTS idx_semantic_chunks_category
ON semantic_chunks(category);

CREATE INDEX IF NOT EXISTS idx_semantic_chunks_created_at
ON semantic_chunks(created_at DESC);

-- Add comments for documentation
COMMENT ON TABLE semantic_chunks IS 'Stores semantically coherent document chunks with embeddings for semantic search';
COMMENT ON COLUMN semantic_chunks.id IS 'Auto-incrementing primary key';
COMMENT ON COLUMN semantic_chunks.file_path IS 'Path to the source file';
COMMENT ON COLUMN semantic_chunks.file_hash IS 'SHA-256 hash of the source file for change detection';
COMMENT ON COLUMN semantic_chunks.chunk_index IS 'Zero-based index of this chunk within the document';
COMMENT ON COLUMN semantic_chunks.total_chunks IS 'Total number of chunks in the document';
COMMENT ON COLUMN semantic_chunks.start_sentence IS 'Index of the first sentence in this chunk';
COMMENT ON COLUMN semantic_chunks.end_sentence IS 'Index of the last sentence in this chunk (inclusive)';
COMMENT ON COLUMN semantic_chunks.content IS 'Text content of the chunk';
COMMENT ON COLUMN semantic_chunks.embedding IS 'Embedding vector as byte array (f32 values in little-endian)';
COMMENT ON COLUMN semantic_chunks.avg_similarity IS 'Average cosine similarity between sentences in this chunk';
COMMENT ON COLUMN semantic_chunks.source_file IS 'Original source file name from metadata';
COMMENT ON COLUMN semantic_chunks.title IS 'Document title from metadata';
COMMENT ON COLUMN semantic_chunks.category IS 'Document category (Diataxis framework)';
COMMENT ON COLUMN semantic_chunks.keywords IS 'Array of keywords for the document';
COMMENT ON COLUMN semantic_chunks.word_count IS 'Number of words in the chunk';
COMMENT ON COLUMN semantic_chunks.char_count IS 'Number of characters in the chunk';

-- Migration rollback (if needed):
-- DROP TABLE IF EXISTS semantic_chunks CASCADE;
