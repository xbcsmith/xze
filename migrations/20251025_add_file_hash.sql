-- Migration: Add file_hash column for incremental loading
-- Created: 2025-10-25
-- Phase: 1 - Hash Tracking Infrastructure

-- Add file_hash column to documents table
-- This column stores the SHA-256 hash of the file content to detect changes
ALTER TABLE IF EXISTS documents
ADD COLUMN IF NOT EXISTS file_hash VARCHAR(64);

-- Add index on file_hash for efficient lookups
CREATE INDEX IF NOT EXISTS idx_documents_file_hash
ON documents(file_hash);

-- Add index on file path and hash combination for quick change detection
CREATE INDEX IF NOT EXISTS idx_documents_path_hash
ON documents(file_path, file_hash);

-- Add comment explaining the column purpose
COMMENT ON COLUMN documents.file_hash IS 'SHA-256 hash of file content for change detection in incremental loading';

-- Migration rollback (if needed):
-- ALTER TABLE documents DROP COLUMN IF EXISTS file_hash;
-- DROP INDEX IF EXISTS idx_documents_file_hash;
-- DROP INDEX IF EXISTS idx_documents_path_hash;
