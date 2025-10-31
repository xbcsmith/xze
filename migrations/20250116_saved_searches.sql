-- Migration: Saved Searches
-- Description: Add table for storing user-saved search queries
-- Created: 2025-01-16

-- Create saved_searches table
CREATE TABLE IF NOT EXISTS saved_searches (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    search_request JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add indices for common queries
CREATE INDEX idx_saved_searches_user_id ON saved_searches(user_id);
CREATE INDEX idx_saved_searches_user_created ON saved_searches(user_id, created_at DESC);
CREATE INDEX idx_saved_searches_name ON saved_searches(name);

-- Add GIN index for JSONB search_request field for efficient querying
CREATE INDEX idx_saved_searches_request_gin ON saved_searches USING GIN (search_request);

-- Add constraint to ensure name is not empty
ALTER TABLE saved_searches ADD CONSTRAINT saved_searches_name_not_empty
    CHECK (length(trim(name)) > 0);

-- Add constraint to ensure user_id is not empty
ALTER TABLE saved_searches ADD CONSTRAINT saved_searches_user_id_not_empty
    CHECK (length(trim(user_id)) > 0);

-- Add updated_at trigger function if it doesn't exist
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Add trigger to automatically update updated_at
CREATE TRIGGER update_saved_searches_updated_at
    BEFORE UPDATE ON saved_searches
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add comment on table
COMMENT ON TABLE saved_searches IS 'Stores user-saved search queries for quick access';
COMMENT ON COLUMN saved_searches.id IS 'Unique identifier';
COMMENT ON COLUMN saved_searches.user_id IS 'ID of user who owns this saved search';
COMMENT ON COLUMN saved_searches.name IS 'Human-readable name for the search';
COMMENT ON COLUMN saved_searches.description IS 'Optional description of the search purpose';
COMMENT ON COLUMN saved_searches.search_request IS 'JSON representation of AdvancedSearchRequest';
COMMENT ON COLUMN saved_searches.created_at IS 'Timestamp when search was created';
COMMENT ON COLUMN saved_searches.updated_at IS 'Timestamp when search was last updated';

-- Analyze table for query planner
ANALYZE saved_searches;
