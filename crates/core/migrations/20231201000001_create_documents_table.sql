-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create documents table for RAG storage
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_file TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding vector(1536),  -- Dimension for nomic-embed-text
    chunk_index INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL,
    
    -- Diataxis classification
    diataxis_type VARCHAR(50),  -- 'tutorial', 'howto', 'reference', 'explanation'
    
    -- Chunking metadata
    chunk_strategy VARCHAR(50),  -- Strategy used for chunking
    
    -- Additional metadata
    title TEXT,
    category TEXT,
    keywords TEXT[],
    code_blocks INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_diataxis_type CHECK (
        diataxis_type IS NULL OR 
        diataxis_type IN ('tutorial', 'howto', 'reference', 'explanation')
    )
);

-- Create indexes for fast retrieval
CREATE INDEX IF NOT EXISTS idx_documents_source_file ON documents(source_file);
CREATE INDEX IF NOT EXISTS idx_documents_diataxis_type ON documents(diataxis_type);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- HNSW index for vector similarity search (fast approximate nearest neighbor)
CREATE INDEX IF NOT EXISTS idx_documents_embedding_hnsw 
    ON documents USING hnsw (embedding vector_cosine_ops);

-- GIN index for full-text search (BM25)
CREATE INDEX IF NOT EXISTS idx_documents_content_fts 
    ON documents USING gin (to_tsvector('english', content));

-- Function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically update updated_at
CREATE TRIGGER update_documents_updated_at
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
