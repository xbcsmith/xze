//! Common test helpers for Git integration tests

use std::fs;
use std::path::Path;
use xze_core::Result;

/// Create a Git repository with initial configuration
pub fn create_test_repo(path: &Path) -> Result<git2::Repository> {
    let repo = git2::Repository::init(path)?;

    // Configure test user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;

    // Create initial commit with README
    fs::write(
        path.join("README.md"),
        "# Test Repository\n\nInitial content\n",
    )?;

    let mut index = repo.index()?;
    index.add_path(Path::new("README.md"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let sig = repo.signature()?;

    {
        let tree = repo.find_tree(tree_id)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
    }

    Ok(repo)
}

/// Create multiple test files in a repository
pub fn create_test_files(repo_path: &Path, files: &[(&str, &str)]) -> Result<()> {
    for (path, content) in files {
        let file_path = repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
    }
    Ok(())
}

/// Create a test repository with multiple commits
pub fn create_repo_with_history(
    path: &Path,
    commits: &[(&str, &[(&str, &str)])],
) -> Result<git2::Repository> {
    let repo = create_test_repo(path)?;

    for (message, files) in commits {
        create_test_files(path, files)?;

        let mut index = repo.index()?;
        for (file, _) in *files {
            index.add_path(Path::new(file))?;
        }
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = repo.signature()?;
        let parent = repo.head()?.peel_to_commit()?;

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
    }

    Ok(repo)
}

/// Check if a test should be skipped based on environment
pub fn should_skip_remote_test() -> bool {
    std::env::var("SKIP_REMOTE_TESTS").is_ok()
}

/// Check if credentials are available for authenticated tests
pub fn has_test_credentials() -> bool {
    std::env::var("GIT_USERNAME").is_ok() && std::env::var("GIT_PASSWORD").is_ok()
        || std::env::var("GIT_SSH_KEY").is_ok()
}

/// Get test repository URL from environment or return default
pub fn get_test_repo_url() -> Option<String> {
    std::env::var("TEST_REPO_URL").ok()
}

/// Create a bare repository for testing remote operations
pub fn create_bare_repo(path: &Path) -> Result<git2::Repository> {
    let repo = git2::Repository::init_bare(path)?;
    Ok(repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_test_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");

        let repo = create_test_repo(&repo_path).unwrap();

        assert!(!repo.is_bare());
        assert!(repo_path.join("README.md").exists());
    }

    #[test]
    fn test_create_test_files() {
        let temp_dir = TempDir::new().unwrap();

        create_test_files(
            temp_dir.path(),
            &[("file1.txt", "content1"), ("dir/file2.txt", "content2")],
        )
        .unwrap();

        assert!(temp_dir.path().join("file1.txt").exists());
        assert!(temp_dir.path().join("dir/file2.txt").exists());
    }
}
