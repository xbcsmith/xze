//! Integration tests for incremental loading functionality
//!
//! These tests validate the complete incremental loading workflow including:
//! - Resume after full load
//! - Update modified files
//! - Cleanup deleted files
//! - Dry run mode

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xze_core::kb::{
    error::Result,
    loader::{IncrementalLoader, LoaderConfig},
};

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
    sqlx::query("TRUNCATE TABLE documents, document_chunks CASCADE")
        .execute(&pool)
        .await
        .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;

    Ok(pool)
}

/// Create test files in a temporary directory
///
/// # Arguments
///
/// * `base_path` - Base directory to create files in
/// * `files` - Vector of (relative_path, content) tuples
fn create_test_files(base_path: &Path, files: &[(&str, &str)]) -> Result<()> {
    for (path, content) in files {
        let file_path = base_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
    }
    Ok(())
}

/// Delete test files from a temporary directory
///
/// # Arguments
///
/// * `base_path` - Base directory containing files
/// * `paths` - Vector of relative paths to delete
fn delete_test_files(base_path: &Path, paths: &[&str]) -> Result<()> {
    for path in paths {
        let file_path = base_path.join(path);
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
    }
    Ok(())
}

/// Modify test files in place
///
/// # Arguments
///
/// * `base_path` - Base directory containing files
/// * `files` - Vector of (relative_path, new_content) tuples
fn modify_test_files(base_path: &Path, files: &[(&str, &str)]) -> Result<()> {
    for (path, content) in files {
        let file_path = base_path.join(path);
        fs::write(file_path, content)?;
    }
    Ok(())
}

/// Count chunks in database for a specific file
async fn count_chunks_for_file(pool: &sqlx::PgPool, file_path: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_chunks dc
         JOIN documents d ON dc.document_id = d.id
         WHERE d.file_path = $1",
    )
    .bind(file_path)
    .fetch_one(pool)
    .await
    .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;

    Ok(count)
}

/// Check if file exists in database
async fn file_exists_in_db(pool: &sqlx::PgPool, file_path: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE file_path = $1")
        .bind(file_path)
        .fetch_one(pool)
        .await
        .map_err(|e| xze_core::kb::error::KbError::database(e.to_string()))?;

    Ok(count > 0)
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_resume_after_full_load() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create initial test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nInitial content"),
            ("doc2.md", "# Document 2\nInitial content"),
            ("doc3.md", "# Document 3\nInitial content"),
        ],
    )?;

    // Phase 1: Initial full load
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Verify initial load
    assert_eq!(stats.files_added, 3);
    assert_eq!(stats.files_skipped, 0);
    assert_eq!(stats.files_updated, 0);
    assert_eq!(stats.files_deleted, 0);

    // Phase 2: Resume with no changes (should skip all files)
    let config_resume = LoaderConfig {
        resume: true,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_resume)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Verify resume behavior - all files should be skipped
    assert_eq!(stats.files_skipped, 3);
    assert_eq!(stats.files_added, 0);
    assert_eq!(stats.files_updated, 0);
    assert_eq!(stats.files_deleted, 0);

    // All three files should exist in database
    assert!(file_exists_in_db(&pool, &format!("{}/doc1.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc2.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc3.md", base_path.display())).await?);

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_update_modified_files() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create initial test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nOriginal content"),
            ("doc2.md", "# Document 2\nOriginal content"),
            ("doc3.md", "# Document 3\nOriginal content"),
        ],
    )?;

    // Phase 1: Initial full load
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    assert_eq!(stats.files_added, 3);

    // Get initial chunk count for doc2
    let doc2_path = format!("{}/doc2.md", base_path.display());
    let initial_chunks = count_chunks_for_file(&pool, &doc2_path).await?;
    assert!(initial_chunks > 0, "Should have chunks after initial load");

    // Phase 2: Modify one file
    modify_test_files(
        base_path,
        &[(
            "doc2.md",
            "# Document 2\nModified content\nWith more lines\nTo generate different chunks",
        )],
    )?;

    // Phase 3: Update with --update flag
    let config_update = LoaderConfig {
        resume: false,
        update: true,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_update)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Verify update behavior
    assert_eq!(stats.files_skipped, 2, "doc1 and doc3 should be skipped");
    assert_eq!(stats.files_updated, 1, "doc2 should be updated");
    assert_eq!(stats.files_added, 0);
    assert_eq!(stats.files_deleted, 0);

    // Verify chunks were regenerated
    let updated_chunks = count_chunks_for_file(&pool, &doc2_path).await?;
    assert!(updated_chunks > 0, "Should still have chunks after update");

    // All three files should still exist in database
    assert!(file_exists_in_db(&pool, &format!("{}/doc1.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &doc2_path).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc3.md", base_path.display())).await?);

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_cleanup_deleted_files() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create initial test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nContent"),
            ("doc2.md", "# Document 2\nContent"),
            ("doc3.md", "# Document 3\nContent"),
            ("doc4.md", "# Document 4\nContent to be deleted"),
        ],
    )?;

    // Phase 1: Initial full load
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    assert_eq!(stats.files_added, 4);

    // Verify all files exist in database
    let doc4_path = format!("{}/doc4.md", base_path.display());
    assert!(file_exists_in_db(&pool, &doc4_path).await?);

    // Phase 2: Delete files from filesystem
    delete_test_files(base_path, &["doc4.md"])?;

    // Phase 3: Run with --update and --cleanup flags
    let config_cleanup = LoaderConfig {
        resume: false,
        update: true,
        cleanup: true,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_cleanup)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Verify cleanup behavior
    assert_eq!(stats.files_skipped, 3, "doc1, doc2, doc3 should be skipped");
    assert_eq!(stats.files_deleted, 1, "doc4 should be deleted");
    assert_eq!(stats.files_added, 0);
    assert_eq!(stats.files_updated, 0);

    // Verify doc4 no longer exists in database
    assert!(
        !file_exists_in_db(&pool, &doc4_path).await?,
        "doc4 should be removed from database"
    );

    // Verify other files still exist
    assert!(file_exists_in_db(&pool, &format!("{}/doc1.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc2.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc3.md", base_path.display())).await?);

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_dry_run_mode() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nContent"),
            ("doc2.md", "# Document 2\nContent"),
        ],
    )?;

    // Phase 1: Initial load (not dry-run)
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    assert_eq!(stats.files_added, 2);

    // Phase 2: Add new files and modify existing
    create_test_files(base_path, &[("doc3.md", "# Document 3\nNew content")])?;
    modify_test_files(base_path, &[("doc2.md", "# Document 2\nModified content")])?;

    // Phase 3: Run dry-run with --update
    let config_dry_run = LoaderConfig {
        resume: false,
        update: true,
        cleanup: false,
        dry_run: true,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_dry_run)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // In dry-run mode, stats should reflect what WOULD happen
    assert_eq!(stats.files_skipped, 1, "doc1 should be skipped");
    assert_eq!(stats.files_added, 1, "doc3 would be added");
    assert_eq!(stats.files_updated, 1, "doc2 would be updated");

    // Verify no changes were actually made
    let doc3_path = format!("{}/doc3.md", base_path.display());
    assert!(
        !file_exists_in_db(&pool, &doc3_path).await?,
        "doc3 should NOT be in database after dry-run"
    );

    // Phase 4: Run actual update (not dry-run)
    let config_real = LoaderConfig {
        resume: false,
        update: true,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_real)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Now changes should be made
    assert_eq!(stats.files_added, 1);
    assert_eq!(stats.files_updated, 1);

    // Verify doc3 now exists in database
    assert!(
        file_exists_in_db(&pool, &doc3_path).await?,
        "doc3 should be in database after real run"
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_force_full_reload() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create initial test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nContent"),
            ("doc2.md", "# Document 2\nContent"),
        ],
    )?;

    // Phase 1: Initial load
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    assert_eq!(stats.files_added, 2);

    // Phase 2: Force reload (should treat all as new even though unchanged)
    let config_force = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: true,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_force)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // With force, all files should be treated as new/updated
    assert_eq!(stats.files_added, 2, "Force should reload all files");
    assert_eq!(
        stats.files_skipped, 0,
        "No files should be skipped with force"
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_mixed_scenario_add_update_delete() -> Result<()> {
    // Setup
    let pool = setup_test_db().await?;
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create initial test files
    create_test_files(
        base_path,
        &[
            ("doc1.md", "# Document 1\nUnchanged"),
            ("doc2.md", "# Document 2\nWill be modified"),
            ("doc3.md", "# Document 3\nWill be deleted"),
        ],
    )?;

    // Phase 1: Initial load
    let config = LoaderConfig {
        resume: false,
        update: false,
        cleanup: false,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    assert_eq!(stats.files_added, 3);

    // Phase 2: Make mixed changes
    // - Leave doc1 unchanged
    // - Modify doc2
    // - Delete doc3
    // - Add doc4
    modify_test_files(
        base_path,
        &[("doc2.md", "# Document 2\nModified content with new text")],
    )?;
    delete_test_files(base_path, &["doc3.md"])?;
    create_test_files(base_path, &[("doc4.md", "# Document 4\nNew document")])?;

    // Phase 3: Run with --update and --cleanup
    let config_mixed = LoaderConfig {
        resume: false,
        update: true,
        cleanup: true,
        dry_run: false,
        force: false,
    };

    let loader = IncrementalLoader::new(pool.clone(), config_mixed)?;
    let stats = loader
        .load(&[base_path.to_string_lossy().to_string()])
        .await?;

    // Verify mixed operation results
    assert_eq!(stats.files_skipped, 1, "doc1 should be skipped");
    assert_eq!(stats.files_updated, 1, "doc2 should be updated");
    assert_eq!(stats.files_added, 1, "doc4 should be added");
    assert_eq!(stats.files_deleted, 1, "doc3 should be deleted");

    // Verify final database state
    assert!(file_exists_in_db(&pool, &format!("{}/doc1.md", base_path.display())).await?);
    assert!(file_exists_in_db(&pool, &format!("{}/doc2.md", base_path.display())).await?);
    assert!(
        !file_exists_in_db(&pool, &format!("{}/doc3.md", base_path.display())).await?,
        "doc3 should be deleted"
    );
    assert!(file_exists_in_db(&pool, &format!("{}/doc4.md", base_path.display())).await?);

    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_create_test_files_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_files(base_path, &[("subdir/nested/file.txt", "content")]).unwrap();

        assert!(base_path.join("subdir/nested/file.txt").exists());
    }

    #[test]
    fn test_modify_test_files_changes_content() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_files(base_path, &[("file.txt", "original")]).unwrap();
        modify_test_files(base_path, &[("file.txt", "modified")]).unwrap();

        let content = fs::read_to_string(base_path.join("file.txt")).unwrap();
        assert_eq!(content, "modified");
    }

    #[test]
    fn test_delete_test_files_removes_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        create_test_files(base_path, &[("file.txt", "content")]).unwrap();
        assert!(base_path.join("file.txt").exists());

        delete_test_files(base_path, &["file.txt"]).unwrap();
        assert!(!base_path.join("file.txt").exists());
    }
}
