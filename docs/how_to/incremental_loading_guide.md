# Incremental Loading Guide

## Overview

XZe's incremental loading system allows you to efficiently manage large documentation repositories by only processing files that have changed. This guide explains how to use the various loading modes and when to apply them.

## Quick Start

### Full Load (Default)

Load all files in a directory:

```bash
xze load /path/to/docs
```

This performs a complete scan and loads all discovered files into the knowledge base. Files are hashed to enable future incremental operations.

### Resume After Interruption

If a load operation is interrupted, resume it without reprocessing already-loaded files:

```bash
xze load --resume /path/to/docs
```

Files that already exist in the database with matching hashes are skipped. Only new or missing files are processed.

### Incremental Updates

Update the knowledge base when files have changed:

```bash
xze load --update /path/to/docs
```

This mode:
- Skips files with unchanged hashes
- Updates files with modified content
- Adds newly created files
- Does NOT remove files deleted from the filesystem

### Incremental Updates with Cleanup

Update the knowledge base and remove files that no longer exist:

```bash
xze load --update --cleanup /path/to/docs
```

This mode performs all update operations plus removes database entries for files that have been deleted from the filesystem.

### Dry Run

Preview what would happen without making changes:

```bash
xze load --update --dry-run /path/to/docs
```

Shows categorization results and statistics without modifying the database. Useful for validating changes before applying them.

### Force Full Reload

Force reprocessing of all files regardless of hash status:

```bash
xze load --force /path/to/docs
```

Treats all files as new and regenerates all chunks and embeddings. Use when:
- Chunking algorithm has changed
- Embedding model has been updated
- Database schema has been modified
- Corruption is suspected

## Use Cases

### Scenario 1: Initial Load

**Situation**: First time loading a documentation repository

**Command**:
```bash
xze load /path/to/docs
```

**Expected Result**: All files are discovered, hashed, chunked, and stored in the database.

### Scenario 2: Load Interrupted

**Situation**: Loading process was killed or crashed partway through

**Command**:
```bash
xze load --resume /path/to/docs
```

**Expected Result**: Already-processed files are skipped, remaining files are loaded.

### Scenario 3: Daily Documentation Updates

**Situation**: Documentation is updated regularly, need to sync changes

**Command**:
```bash
xze load --update --cleanup /path/to/docs
```

**Expected Result**: Modified files are updated, new files are added, deleted files are removed from database.

### Scenario 4: Preview Changes

**Situation**: Want to see what would change without modifying database

**Command**:
```bash
xze load --update --cleanup --dry-run /path/to/docs
```

**Expected Result**: Detailed report of files to add, update, skip, and delete without database modifications.

## Flag Combinations

### Valid Combinations

| Flags | Behavior |
|-------|----------|
| (none) | Full load, processes all files |
| `--resume` | Skip files already in database |
| `--update` | Update changed files, add new files |
| `--update --cleanup` | Update changed files, add new files, remove deleted files |
| `--update --dry-run` | Preview update operations |
| `--update --cleanup --dry-run` | Preview update and cleanup operations |
| `--force` | Force full reload of all files |
| `--force --cleanup` | Force reload and remove stale entries |

### Invalid Combinations

These combinations are rejected with an error:

- `--force --resume` (conflicting: force implies no resume)
- `--force --update` (conflicting: force implies treating all as new)
- `--resume` alone without other flags (no useful operation)

## Performance Tips

### Optimize for Large Repositories

For repositories with thousands of files:

1. Use `--update` instead of full reloads when possible
2. Run with `--dry-run` first to estimate processing time
3. Consider processing subdirectories independently if files are logically grouped

### Parallel Processing

The loader processes files concurrently. Performance is primarily limited by:
- Disk I/O for reading files
- CPU for hashing and chunking
- Database connection pool size for writes

### Hash Caching

File hashes are stored in the database. Once a file is loaded:
- Hash is calculated from file content (SHA-256)
- Subsequent runs compare filesystem file hash to stored hash
- Only files with different hashes are reprocessed

## Troubleshooting

### Issue: Files Not Being Updated

**Symptoms**: Files show as modified but are marked as skipped

**Solution**: Ensure you are using the `--update` flag:
```bash
xze load --update /path/to/docs
```

Without `--update`, the default mode is resume, which skips existing files.

### Issue: Database Growing Too Large

**Symptoms**: Database size increases even when files are deleted

**Solution**: Use the `--cleanup` flag periodically:
```bash
xze load --update --cleanup /path/to/docs
```

This removes database entries for files that no longer exist on the filesystem.

### Issue: Slow Performance

**Symptoms**: Loading takes excessive time

**Diagnostics**:
1. Check file count: `find /path/to/docs -type f | wc -l`
2. Run with `--dry-run` to see categorization without processing
3. Review database connection pool settings

**Solutions**:
- Exclude irrelevant files (configure ignored patterns)
- Ensure database has proper indexes
- Use SSD storage for both filesystem and database

### Issue: Hash Mismatches After Schema Changes

**Symptoms**: Files show as unchanged but need reprocessing

**Solution**: Use `--force` to regenerate all chunks:
```bash
xze load --force /path/to/docs
```

This ignores stored hashes and treats all files as new.

## Technical Details

### Hash Algorithm

Files are hashed using SHA-256. The hash is calculated from:
- Complete file content (binary read)
- No normalization of line endings or encoding

This ensures exact detection of any file changes.

### Database Schema

The loader interacts with two main tables:

**documents**:
- `id`: Primary key
- `file_path`: Absolute path to file
- `file_hash`: SHA-256 hash of file content
- `created_at`: Initial load timestamp
- `updated_at`: Last modification timestamp

**document_chunks**:
- `id`: Primary key
- `document_id`: Foreign key to documents
- `chunk_index`: Position in document
- `content`: Text content of chunk
- `embedding`: Vector embedding for semantic search

### Transaction Safety

Operations are executed with the following guarantees:

- **Add files**: Each file insertion is atomic
- **Update files**: Chunk deletion and reinsertion for a file happens in a transaction
- **Delete files**: Each file deletion (document + chunks) is atomic
- **Dry run**: No transactions are started, read-only operations

Note: Currently, cleanup of multiple files is not wrapped in a single transaction. Each file deletion is independent.

## Advanced Usage

### Multiple Paths

Load from multiple directories in one operation:

```bash
xze load --update /path/to/docs /path/to/other/docs
```

All paths are discovered and categorized together before processing begins.

### Environment Variables

Control behavior via environment variables:

- `RUST_LOG=debug` - Enable detailed logging
- `DATABASE_URL` - Override default database connection

### Integration with CI/CD

Example GitHub Actions workflow:

```yaml
name: Update Documentation KB

on:
  push:
    paths:
      - 'docs/**'

jobs:
  update-kb:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Update knowledge base
        run: |
          xze load --update --cleanup ./docs
        env:
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
```

## Best Practices

1. **Use `--dry-run` first** - Always preview changes before applying them to production databases

2. **Schedule regular cleanup** - Run `--cleanup` periodically to remove stale entries

3. **Monitor statistics** - Review the summary output to track database growth and processing performance

4. **Version control integration** - Run incremental updates on post-commit hooks to keep documentation synchronized

5. **Backup before force** - Always backup your database before running `--force` operations

## Getting Help

For more information:

- Architecture details: See `docs/explanations/incremental_loading_architecture.md`
- Implementation details: See `docs/explanations/phase*_implementation.md`
- Report issues: GitHub repository issue tracker
