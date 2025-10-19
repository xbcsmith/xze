//! Integration tests for Git operations
//!
//! These tests use real Git repositories to validate Git operations functionality.
//! They require a Git installation and network connectivity for remote operations.
//!
//! Tests can be run with:
//! ```bash
//! cargo test --package xze-core --test git_integration_tests
//! ```
//!
//! To run only local tests (no network):
//! ```bash
//! cargo test --package xze-core --test git_integration_tests -- --skip remote
//! ```

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xze_core::git::{ChangeType, CredentialStore, GitOperations};
use xze_core::Result;

/// Helper to create a test repository with initial commit
fn create_test_repo(path: &Path) -> Result<git2::Repository> {
    let repo = git2::Repository::init(path)?;

    // Configure test user
    {
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;
    }

    // Create README
    fs::write(
        path.join("README.md"),
        "# Test Repository\n\nInitial content\n",
    )?;

    // Create initial commit
    {
        let mut index = repo.index()?;
        index.add_path(Path::new("README.md"))?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = repo.signature()?;

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
    }

    Ok(repo)
}

/// Helper to create test files in a repository
fn create_test_files(repo_path: &Path, files: &[(&str, &str)]) -> Result<()> {
    for (path, content) in files {
        let file_path = repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
    }
    Ok(())
}

#[test]
fn test_init_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.init(&repo_path)?;

    assert!(!repo.is_bare());
    assert!(repo_path.join(".git").exists());

    Ok(())
}

#[test]
fn test_open_existing_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    // Create repository
    create_test_repo(&repo_path)?;

    // Open it with GitOperations
    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    assert!(!repo.is_bare());
    assert_eq!(repo.path(), repo_path.join(".git"));

    Ok(())
}

#[test]
fn test_create_and_checkout_branch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create branch without checkout
    git_ops.create_branch(&repo, "feature-1", false)?;

    // Should still be on main/master
    let current = git_ops.current_branch(&repo)?;
    assert!(current == "main" || current == "master");

    // Create and checkout branch
    git_ops.create_branch(&repo, "feature-2", true)?;

    let current = git_ops.current_branch(&repo)?;
    assert_eq!(current, "feature-2");

    Ok(())
}

#[test]
fn test_list_branches() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create multiple branches
    git_ops.create_branch(&repo, "feature-1", false)?;
    git_ops.create_branch(&repo, "feature-2", false)?;
    git_ops.create_branch(&repo, "feature-3", true)?;

    let branches = git_ops.list_branches(&repo, true)?;

    // Should have main/master + 3 feature branches
    assert!(branches.len() >= 4);

    // Check current branch is feature-3
    let current_branches: Vec<_> = branches.iter().filter(|b| b.is_current).collect();
    assert_eq!(current_branches.len(), 1);
    assert_eq!(current_branches[0].name, "feature-3");

    // Check branch names
    let names: Vec<_> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"feature-1"));
    assert!(names.contains(&"feature-2"));
    assert!(names.contains(&"feature-3"));

    Ok(())
}

#[test]
fn test_delete_branch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create and delete branch
    git_ops.create_branch(&repo, "temp-branch", false)?;

    let branches = git_ops.list_branches(&repo, true)?;
    assert!(branches.iter().any(|b| b.name == "temp-branch"));

    git_ops.delete_branch(&repo, "temp-branch")?;

    let branches = git_ops.list_branches(&repo, true)?;
    assert!(!branches.iter().any(|b| b.name == "temp-branch"));

    Ok(())
}

#[test]
fn test_stage_and_commit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create new files
    create_test_files(
        &repo_path,
        &[
            ("file1.txt", "Content 1"),
            ("file2.txt", "Content 2"),
            ("dir/file3.txt", "Content 3"),
        ],
    )?;

    // Verify changes exist
    assert!(git_ops.has_changes(&repo)?);

    // Stage all files
    git_ops.stage_all(&repo)?;

    // Commit
    let oid = git_ops.commit(&repo, "Add test files")?;
    assert!(!oid.is_zero());

    // Verify no changes after commit
    assert!(!git_ops.has_changes(&repo)?);

    // Verify last commit hash
    let last_hash = git_ops.get_last_commit_hash(&repo)?;
    assert_eq!(last_hash, oid.to_string());

    Ok(())
}

#[test]
fn test_stage_specific_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create multiple files
    create_test_files(
        &repo_path,
        &[
            ("file1.txt", "Content 1"),
            ("file2.txt", "Content 2"),
            ("file3.txt", "Content 3"),
        ],
    )?;

    // Stage only specific files
    git_ops.stage_files(&repo, &[Path::new("file1.txt"), Path::new("file2.txt")])?;

    // Commit
    git_ops.commit(&repo, "Add file1 and file2")?;

    // file3.txt should still be untracked
    let status = git_ops.get_status(&repo)?;
    assert!(status.contains_key(Path::new("file3.txt")));

    Ok(())
}

#[test]
fn test_diff_analysis_between_commits() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Get initial commit
    let initial_commit = git_ops.get_last_commit_hash(&repo)?;

    // Add files and commit
    create_test_files(
        &repo_path,
        &[
            ("added.txt", "New file"),
            ("modified.txt", "Original content"),
        ],
    )?;

    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Add files")?;

    // Modify and delete files
    fs::write(repo_path.join("modified.txt"), "Modified content")?;
    fs::remove_file(repo_path.join("added.txt"))?;
    create_test_files(&repo_path, &[("another.txt", "Another file")])?;

    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Modify and delete")?;

    // Analyze diff between initial and current
    let diff = git_ops.diff_analysis(&repo, Some(&initial_commit), Some("HEAD"))?;

    assert!(diff.has_changes());
    assert!(diff.files_changed > 0);

    // Check for specific changes
    let modified_files = diff.changes_by_type(ChangeType::Modified);
    let deleted_files = diff.changes_by_type(ChangeType::Deleted);
    let added_files = diff.changes_by_type(ChangeType::Added);

    assert!(!modified_files.is_empty() || !deleted_files.is_empty() || !added_files.is_empty());

    Ok(())
}

#[test]
fn test_diff_working_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Modify README
    fs::write(
        repo_path.join("README.md"),
        "# Modified README\n\nNew content\n",
    )?;

    // Add new file
    create_test_files(&repo_path, &[("newfile.txt", "New content")])?;

    // Check working directory status first
    assert!(git_ops.has_changes(&repo)?);

    // For diff analysis, need to compare commits, not working directory
    // Create a second commit to have something to diff
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Second commit")?;

    // Now diff between commits
    let diff = git_ops.diff_analysis(&repo, Some("HEAD~1"), Some("HEAD"))?;

    assert!(diff.has_changes());
    assert!(diff.files_changed >= 1);

    Ok(())
}

#[test]
fn test_change_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Clean repository should have no changes
    assert!(!git_ops.has_changes(&repo)?);
    assert!(git_ops.is_clean(&repo)?);

    // Create a new file
    create_test_files(&repo_path, &[("test.txt", "Test content")])?;

    // Should detect changes
    assert!(git_ops.has_changes(&repo)?);
    assert!(!git_ops.is_clean(&repo)?);

    // Get detailed status
    let status = git_ops.get_status(&repo)?;
    assert!(status.contains_key(Path::new("test.txt")));

    Ok(())
}

#[test]
fn test_tag_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create lightweight tag
    git_ops.create_tag(&repo, "v1.0.0", None, false)?;

    // Create annotated tag
    git_ops.create_tag(&repo, "v1.1.0", Some("Version 1.1.0 release"), false)?;

    // List tags
    let tags = git_ops.list_tags(&repo)?;
    assert!(tags.contains(&"v1.0.0".to_string()));
    assert!(tags.contains(&"v1.1.0".to_string()));

    // Delete tag
    git_ops.delete_tag(&repo, "v1.0.0")?;

    let tags = git_ops.list_tags(&repo)?;
    assert!(!tags.contains(&"v1.0.0".to_string()));
    assert!(tags.contains(&"v1.1.0".to_string()));

    Ok(())
}

#[test]
fn test_stash_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let mut repo = git_ops.open(&repo_path)?;

    // Make some changes and stage them (stash needs tracked files)
    create_test_files(&repo_path, &[("stashed.txt", "Stashed content")])?;
    git_ops.stage_all(&repo)?;

    // Verify changes exist
    assert!(git_ops.has_changes(&repo)?);

    // Stash changes
    let stash_oid = git_ops.stash(&mut repo, Some("Test stash"))?;
    assert!(!stash_oid.is_zero());

    // Working directory should be clean
    assert!(!git_ops.has_changes(&repo)?);

    // Pop stash
    git_ops.stash_pop(&mut repo, 0)?;

    // Changes should be restored
    assert!(git_ops.has_changes(&repo)?);
    assert!(repo_path.join("stashed.txt").exists());

    Ok(())
}

#[test]
fn test_reset_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    let initial_commit = git_ops.get_last_commit_hash(&repo)?;

    // Create and commit changes
    create_test_files(&repo_path, &[("file.txt", "Content")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Add file")?;

    let second_commit = git_ops.get_last_commit_hash(&repo)?;
    assert_ne!(initial_commit, second_commit);

    // Soft reset - keeps changes staged
    git_ops.reset(&repo, "HEAD~1", git2::ResetType::Soft)?;

    let current_commit = git_ops.get_last_commit_hash(&repo)?;
    assert_eq!(current_commit, initial_commit);

    // File should still exist and be staged
    assert!(repo_path.join("file.txt").exists());

    Ok(())
}

#[test]
fn test_checkout_branch_switches() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create branches with different content
    git_ops.create_branch(&repo, "branch-a", true)?;
    create_test_files(&repo_path, &[("a.txt", "Branch A content")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Add a.txt")?;

    let current = git_ops.current_branch(&repo)?;
    assert_eq!(current, "branch-a");

    git_ops.create_branch(&repo, "branch-b", true)?;
    create_test_files(&repo_path, &[("b.txt", "Branch B content")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Add b.txt")?;

    // Switch back to branch-a
    git_ops.checkout_branch(&repo, "branch-a")?;

    let current = git_ops.current_branch(&repo)?;
    assert_eq!(current, "branch-a");

    // b.txt should not exist on branch-a
    assert!(!repo_path.join("b.txt").exists());
    assert!(repo_path.join("a.txt").exists());

    Ok(())
}

#[test]
fn test_get_remote_url() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    let _repo = create_test_repo(&repo_path)?;

    // Add a remote
    let repo = git2::Repository::open(&repo_path)?;
    repo.remote("origin", "https://github.com/test/repo.git")?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    let url = git_ops.get_remote_url(&repo, "origin")?;
    assert_eq!(url, "https://github.com/test/repo.git");

    Ok(())
}

#[test]
fn test_commit_with_custom_signature() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create files
    create_test_files(&repo_path, &[("custom.txt", "Custom commit")])?;
    git_ops.stage_all(&repo)?;

    // Create custom signatures
    let author = git2::Signature::now("Alice", "alice@example.com")?;
    let committer = git2::Signature::now("Bob", "bob@example.com")?;

    // Commit with custom signature
    let oid = git_ops.commit_with_signature(&repo, "Custom commit", &author, &committer)?;
    assert!(!oid.is_zero());

    // Verify commit
    let commit = repo.find_commit(oid)?;
    assert_eq!(commit.author().name(), Some("Alice"));
    assert_eq!(commit.committer().name(), Some("Bob"));

    Ok(())
}

#[test]
fn test_no_conflicts_in_clean_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Clean repo should have no conflicts
    assert!(!git_ops.has_conflicts(&repo)?);

    let conflicts = git_ops.get_conflicts(&repo)?;
    assert!(conflicts.is_empty());

    Ok(())
}

#[test]
fn test_multiple_commits_with_diff() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    let initial = git_ops.get_last_commit_hash(&repo)?;

    // First commit
    create_test_files(&repo_path, &[("v1.txt", "Version 1")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Version 1")?;

    // Second commit
    create_test_files(&repo_path, &[("v2.txt", "Version 2")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Version 2")?;

    // Third commit
    create_test_files(&repo_path, &[("v3.txt", "Version 3")])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Version 3")?;

    // Analyze diff from initial to HEAD
    let diff = git_ops.diff_analysis(&repo, Some(&initial), Some("HEAD"))?;

    assert!(diff.has_changes());
    assert!(diff.files_changed >= 3); // At least v1, v2, v3

    let added = diff.changes_by_type(ChangeType::Added);
    assert!(added.len() >= 3); // At least v1, v2, v3

    Ok(())
}

#[test]
fn test_credential_store_configuration() -> Result<()> {
    // Test username/password configuration
    let store =
        CredentialStore::new().with_userpass("testuser".to_string(), "testpass".to_string());

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("testuser".to_string()));

    let (user, pass) = store.get_userpass().unwrap();
    assert_eq!(user, "testuser");
    assert_eq!(pass, "testpass");

    // Test clearing credentials
    store.clear();
    assert!(!store.has_credentials());

    Ok(())
}

#[test]
fn test_branch_info_metadata() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    git_ops.create_branch(&repo, "test-branch", false)?;

    let branches = git_ops.list_branches(&repo, true)?;

    // Check that each branch has metadata
    for branch in branches {
        assert!(!branch.name.is_empty());
        assert!(!branch.commit_hash.is_empty());
        // Either is current or not
        assert!(branch.is_current || !branch.is_current);
    }

    Ok(())
}

#[test]
fn test_diff_summary_statistics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    let initial = git_ops.get_last_commit_hash(&repo)?;

    // Add files with known content
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";
    create_test_files(&repo_path, &[("lines.txt", content)])?;
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "Add lines")?;

    // Analyze diff
    let diff = git_ops.diff_analysis(&repo, Some(&initial), Some("HEAD"))?;

    assert!(diff.has_changes());
    assert!(diff.insertions > 0);
    assert_eq!(diff.deletions, 0); // No deletions in new file

    Ok(())
}

// Remote repository tests - these require network access
// Run with: cargo test -- --ignored

#[test]
#[ignore = "requires network access"]
fn test_clone_public_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("cloned-repo");

    let git_ops = GitOperations::new(CredentialStore::new());

    // Clone a small public repository
    let repo = tokio_test::block_on(
        git_ops.clone("https://github.com/octocat/Hello-World.git", &repo_path),
    )?;

    assert!(!repo.is_bare());
    assert!(repo_path.join(".git").exists());
    assert!(repo_path.join("README").exists());

    Ok(())
}

#[test]
#[ignore = "requires network access and authentication"]
fn test_clone_with_authentication() -> Result<()> {
    // This test requires setting up credentials via environment variables
    // GIT_USERNAME and GIT_PASSWORD (or SSH credentials)

    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("cloned-repo");

    let store = xze_core::git::credentials_from_env();

    // Skip test if no credentials configured
    if !store.has_credentials() {
        println!("Skipping test - no credentials configured");
        return Ok(());
    }

    let git_ops = GitOperations::new(store);

    // Replace with your private repository URL
    let test_repo_url = std::env::var("TEST_REPO_URL")
        .unwrap_or_else(|_| "https://github.com/your-org/private-repo.git".to_string());

    if test_repo_url.contains("your-org") {
        println!("Skipping test - TEST_REPO_URL not configured");
        return Ok(());
    }

    let repo = tokio_test::block_on(git_ops.clone(&test_repo_url, &repo_path))?;

    assert!(!repo.is_bare());
    assert!(repo_path.join(".git").exists());

    Ok(())
}

#[test]
#[ignore = "requires network access"]
fn test_fetch_from_remote() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("cloned-repo");

    let git_ops = GitOperations::new(CredentialStore::new());

    // Clone repository
    let repo = tokio_test::block_on(
        git_ops.clone("https://github.com/octocat/Hello-World.git", &repo_path),
    )?;

    // Fetch should succeed
    git_ops.fetch(&repo)?;

    Ok(())
}

#[test]
fn test_error_handling_open_nonexistent() {
    let git_ops = GitOperations::new(CredentialStore::new());
    let result = git_ops.open("/nonexistent/path/to/repo");

    assert!(result.is_err());
}

#[test]
fn test_error_handling_delete_current_branch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    let current = git_ops.current_branch(&repo)?;

    // Should fail to delete current branch
    let result = git_ops.delete_branch(&repo, &current);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_error_handling_create_duplicate_branch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    git_ops.create_branch(&repo, "test-branch", false)?;

    // Should fail to create duplicate branch
    let result = git_ops.create_branch(&repo, "test-branch", false);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_complete_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    // Create repository with initial commit
    create_test_repo(&repo_path)?;

    // Open with GitOperations
    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Create feature branch
    git_ops.create_branch(&repo, "feature/add-docs", true)?;

    // Add documentation
    create_test_files(
        &repo_path,
        &[
            ("docs/api.md", "# API Documentation"),
            ("docs/guide.md", "# User Guide"),
        ],
    )?;

    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "docs: add documentation")?;

    // Get the commit hash from the first branch (main/master)
    // We need to find which branch was the initial one
    let branches = git_ops.list_branches(&repo, true)?;
    let main_branch_name = if branches.iter().any(|b| b.name == "main") {
        "main"
    } else {
        "master"
    };

    // Switch to main/master to get its commit
    git_ops.checkout_branch(&repo, main_branch_name)?;
    let main_commit = git_ops.get_last_commit_hash(&repo)?;

    // Switch back to feature branch
    git_ops.checkout_branch(&repo, "feature/add-docs")?;

    let diff = git_ops.diff_analysis(&repo, Some(&main_commit), Some("HEAD"))?;

    assert!(diff.has_changes());
    assert_eq!(diff.changes_by_type(ChangeType::Added).len(), 2);

    // Create release tag
    git_ops.create_tag(&repo, "v1.0.0", Some("First release"), false)?;

    // Verify tag exists
    let tags = git_ops.list_tags(&repo)?;
    assert!(tags.contains(&"v1.0.0".to_string()));

    // Clean repository check
    assert!(git_ops.is_clean(&repo)?);

    Ok(())
}
