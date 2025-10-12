//! Repository manager for handling repository operations

use crate::{
    config::{RepositoryConfig, XzeConfig},
    error::{Result, XzeError},
    repository::{ChangeAnalysis, Repository},
    types::{ProgrammingLanguage, RepositoryId},
};
use git2::{Cred, FetchOptions, Oid, RemoteCallbacks, Repository as GitRepository};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Repository manager handles cloning, updating, and managing repositories
#[derive(Debug)]
pub struct RepositoryManager {
    /// Cache directory for repositories
    cache_dir: PathBuf,
    /// Cached repositories
    repositories: Arc<RwLock<HashMap<RepositoryId, Repository>>>,
    /// Configuration
    config: XzeConfig,
}

impl RepositoryManager {
    /// Create a new repository manager
    pub fn new(cache_dir: PathBuf, config: XzeConfig) -> Result<Self> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).map_err(|e| {
                XzeError::filesystem(format!("Failed to create cache directory: {}", e))
            })?;
        }

        Ok(Self {
            cache_dir,
            repositories: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Get or clone a repository
    pub async fn get_repository(&self, repo_id: &RepositoryId) -> Result<Repository> {
        // Check if repository is already cached in memory
        {
            let repositories = self.repositories.read().await;
            if let Some(repo) = repositories.get(repo_id) {
                if repo.exists() {
                    debug!("Repository {} found in cache", repo_id);
                    return Ok(repo.clone());
                }
            }
        }

        // Get repository configuration
        let repo_config = self.config.get_repository(&repo_id.0).ok_or_else(|| {
            XzeError::repository(format!("Repository {} not found in configuration", repo_id))
        })?;

        // Clone or update repository
        let repo = self.clone_or_update_repository(repo_config).await?;

        // Cache the repository
        {
            let mut repositories = self.repositories.write().await;
            repositories.insert(repo_id.clone(), repo.clone());
        }

        Ok(repo)
    }

    /// Clone or update a repository
    async fn clone_or_update_repository(&self, config: &RepositoryConfig) -> Result<Repository> {
        let local_path = self.get_local_path(&config.name);
        let repo_id = RepositoryId::from(config.name.clone());

        info!("Processing repository {} at {}", config.name, config.url);

        if local_path.exists() {
            // Repository exists, update it
            self.update_repository(&local_path, config).await?;
        } else {
            // Clone the repository
            self.clone_repository(config, &local_path).await?;
        }

        // Detect language if not specified
        let language = if let Some(lang_str) = &config.language {
            ProgrammingLanguage::from(lang_str.as_str())
        } else {
            self.detect_repository_language(&local_path)?
        };

        // Create repository instance
        let mut repo = Repository::new(repo_id, config.url.to_string(), local_path, language);

        // Update metadata
        self.update_repository_metadata(&mut repo).await?;

        Ok(repo)
    }

    /// Clone a repository
    async fn clone_repository(&self, config: &RepositoryConfig, local_path: &Path) -> Result<()> {
        info!("Cloning repository {} to {:?}", config.url, local_path);

        // Prepare callbacks for authentication
        let url = config.url.to_string();
        let local_path = local_path.to_path_buf();
        let watch_branches = config.watch_branches.clone();

        let _git_repo = tokio::task::spawn_blocking(move || -> Result<GitRepository> {
            let callbacks = RemoteCallbacks::new();
            // Note: Authentication setup would need to be done here if needed
            // For now, we'll assume public repositories or SSH keys are configured

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            // Clone the repository
            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_options);

            if !watch_branches.is_empty() {
                builder.branch(&watch_branches[0]);
            }

            builder.clone(&url, &local_path).map_err(XzeError::Git)
        })
        .await
        .map_err(|e| XzeError::repository(format!("Clone task failed: {}", e)))??;

        info!("Successfully cloned repository {}", config.name);
        Ok(())
    }

    /// Update an existing repository
    async fn update_repository(&self, local_path: &Path, config: &RepositoryConfig) -> Result<()> {
        info!("Updating repository {} at {:?}", config.name, local_path);

        let local_path = local_path.to_path_buf();

        let name = config.name.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let git_repo = GitRepository::open(&local_path).map_err(XzeError::Git)?;

            // Fetch updates
            let mut remote = git_repo.find_remote("origin").map_err(XzeError::Git)?;

            let callbacks = RemoteCallbacks::new();
            // Note: Authentication setup would need to be done here if needed
            // For now, we'll assume public repositories or SSH keys are configured

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            remote
                .fetch(&[] as &[&str], Some(&mut fetch_options), None)
                .map_err(XzeError::Git)?;

            Ok(())
        })
        .await
        .map_err(|e| XzeError::repository(format!("Update task failed: {}", e)))??;

        info!("Successfully updated repository {}", name);
        Ok(())
    }

    /// Setup authentication callbacks
    #[allow(dead_code)]
    fn setup_auth_callbacks(
        &self,
        callbacks: &mut RemoteCallbacks,
        credentials: &crate::config::GitCredentials,
    ) -> Result<()> {
        use crate::config::GitAuth;

        match &credentials.auth {
            GitAuth::UserPass { username, password } => {
                let username = username.clone();
                let password = password.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    let user = username_from_url.unwrap_or(&username);
                    Cred::userpass_plaintext(user, &password)
                });
            }
            GitAuth::SshKey {
                username,
                private_key_path,
                passphrase,
            } => {
                let username = username.clone();
                let private_key_path = private_key_path.clone();
                let passphrase = passphrase.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    let user = username_from_url.unwrap_or(&username);
                    Cred::ssh_key(user, None, &private_key_path, passphrase.as_deref())
                });
            }
            GitAuth::SshAgent { username } => {
                let username = username.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    let user = username_from_url.unwrap_or(&username);
                    Cred::ssh_key_from_agent(user)
                });
            }
        }

        Ok(())
    }

    /// Get local path for a repository
    fn get_local_path(&self, repo_name: &str) -> PathBuf {
        self.cache_dir.join(repo_name)
    }

    /// Detect the primary language of a repository
    fn detect_repository_language(&self, path: &Path) -> Result<ProgrammingLanguage> {
        let mut language_counts: HashMap<ProgrammingLanguage, usize> = HashMap::new();

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                    let lang = ProgrammingLanguage::from(extension);
                    if !matches!(lang, ProgrammingLanguage::Unknown(_)) {
                        *language_counts.entry(lang).or_insert(0) += 1;
                    }
                }
            }
        }

        // Return the most common language
        Ok(language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or(ProgrammingLanguage::Unknown("mixed".to_string())))
    }

    /// Update repository metadata
    async fn update_repository_metadata(&self, repo: &mut Repository) -> Result<()> {
        let mut file_count = 0;
        let mut lines_of_code = 0;

        // Count files and lines
        for entry in walkdir::WalkDir::new(&repo.local_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.is_file() && !repo.should_ignore_path(path) {
                file_count += 1;

                // Count lines for source files
                if self.is_source_file(path, &repo.language) {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        lines_of_code += content.lines().count();
                    }
                }
            }
        }

        // Update metadata
        repo.metadata.update_stats(file_count, lines_of_code);
        repo.metadata.language = Some(repo.language.clone());

        // Get current commit hash
        if let Ok(git_repo) = GitRepository::open(&repo.local_path) {
            if let Ok(head) = git_repo.head() {
                if let Some(oid) = head.target() {
                    repo.metadata.last_commit = Some(oid.to_string());
                }
            }
        }

        Ok(())
    }

    /// Check if a file is a source file for the given language
    fn is_source_file(&self, path: &Path, language: &ProgrammingLanguage) -> bool {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match language {
                ProgrammingLanguage::Rust => matches!(extension, "rs"),
                ProgrammingLanguage::Go => matches!(extension, "go"),
                ProgrammingLanguage::Python => matches!(extension, "py"),
                ProgrammingLanguage::JavaScript => matches!(extension, "js"),
                ProgrammingLanguage::TypeScript => matches!(extension, "ts" | "tsx"),
                ProgrammingLanguage::Java => matches!(extension, "java"),
                ProgrammingLanguage::CSharp => matches!(extension, "cs"),
                ProgrammingLanguage::Cpp => matches!(extension, "cpp" | "cc" | "cxx"),
                ProgrammingLanguage::C => matches!(extension, "c"),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get all cached repositories
    pub async fn get_all_repositories(&self) -> Vec<Repository> {
        let repositories = self.repositories.read().await;
        repositories.values().cloned().collect()
    }

    /// Remove a repository from cache
    pub async fn remove_repository(&self, repo_id: &RepositoryId) -> Result<()> {
        let mut repositories = self.repositories.write().await;
        repositories.remove(repo_id);

        // Also remove from filesystem
        let local_path = self.get_local_path(&repo_id.0);
        if local_path.exists() {
            std::fs::remove_dir_all(&local_path).map_err(|e| {
                XzeError::filesystem(format!("Failed to remove repository directory: {}", e))
            })?;
        }

        Ok(())
    }

    /// Clear all cached repositories
    pub async fn clear_cache(&self) -> Result<()> {
        let mut repositories = self.repositories.write().await;
        repositories.clear();

        // Remove all repository directories
        if self.cache_dir.exists() {
            for entry in std::fs::read_dir(&self.cache_dir).map_err(|e| {
                XzeError::filesystem(format!("Failed to read cache directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    XzeError::filesystem(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();
                if path.is_dir() {
                    std::fs::remove_dir_all(&path).map_err(|e| {
                        XzeError::filesystem(format!("Failed to remove directory: {}", e))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Get repository statistics
    pub async fn get_stats(&self) -> RepositoryManagerStats {
        let repositories = self.repositories.read().await;
        let total_repos = repositories.len();
        let total_files: usize = repositories.values().map(|r| r.metadata.file_count).sum();
        let total_lines: usize = repositories
            .values()
            .map(|r| r.metadata.lines_of_code)
            .sum();

        let mut language_distribution = HashMap::new();
        for repo in repositories.values() {
            if let Some(lang) = &repo.metadata.language {
                *language_distribution.entry(lang.clone()).or_insert(0) += 1;
            }
        }

        RepositoryManagerStats {
            total_repositories: total_repos,
            total_files,
            total_lines_of_code: total_lines,
            language_distribution,
            cache_size_mb: self.calculate_cache_size().unwrap_or(0),
        }
    }

    /// Calculate cache size in MB
    fn calculate_cache_size(&self) -> Result<usize> {
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(&self.cache_dir) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            if entry.path().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        Ok((total_size / 1024 / 1024) as usize)
    }

    /// Analyze changes between two commits
    pub async fn analyze_changes(
        &self,
        repo_id: &RepositoryId,
        from_commit: &str,
        to_commit: &str,
    ) -> Result<ChangeAnalysis> {
        let repo = self.get_repository(repo_id).await?;

        let git_repo = GitRepository::open(&repo.local_path).map_err(XzeError::Git)?;

        let from_oid = Oid::from_str(from_commit).map_err(XzeError::Git)?;
        let to_oid = Oid::from_str(to_commit).map_err(XzeError::Git)?;

        let from_tree = git_repo
            .find_commit(from_oid)?
            .tree()
            .map_err(XzeError::Git)?;
        let to_tree = git_repo
            .find_commit(to_oid)?
            .tree()
            .map_err(XzeError::Git)?;

        let diff = git_repo
            .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
            .map_err(XzeError::Git)?;

        let mut analysis = ChangeAnalysis::new();

        diff.foreach(
            &mut |delta, _progress| {
                if let (Some(old_file), Some(new_file)) =
                    (delta.old_file().path(), delta.new_file().path())
                {
                    let change_type = match delta.status() {
                        git2::Delta::Added => crate::types::ChangeType::Added,
                        git2::Delta::Deleted => crate::types::ChangeType::Deleted,
                        git2::Delta::Modified => crate::types::ChangeType::Modified,
                        git2::Delta::Renamed => crate::types::ChangeType::Renamed {
                            from: old_file.to_string_lossy().to_string(),
                            to: new_file.to_string_lossy().to_string(),
                        },
                        _ => crate::types::ChangeType::Modified,
                    };

                    analysis.add_file_change(crate::types::FileChange {
                        path: new_file.to_string_lossy().to_string(),
                        change_type,
                        lines_added: 0,   // TODO: Calculate from diff stats
                        lines_removed: 0, // TODO: Calculate from diff stats
                    });
                }
                true
            },
            None,
            None,
            None,
        )
        .map_err(XzeError::Git)?;

        Ok(analysis)
    }
}

/// Repository manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryManagerStats {
    pub total_repositories: usize,
    pub total_files: usize,
    pub total_lines_of_code: usize,
    pub language_distribution: HashMap<ProgrammingLanguage, usize>,
    pub cache_size_mb: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DocumentationRepoConfig, RepositoryConfig};
    use std::fs;
    use tempfile::TempDir;
    use url::Url;

    #[tokio::test]
    async fn test_repository_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = XzeConfig::default();

        let manager = RepositoryManager::new(temp_dir.path().to_path_buf(), config).unwrap();
        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = XzeConfig::default();
        let manager = RepositoryManager::new(temp_dir.path().to_path_buf(), config).unwrap();

        // Create a test repository directory with Rust files
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir_all(repo_dir.join("src")).unwrap();
        fs::write(repo_dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(repo_dir.join("Cargo.toml"), "[package]").unwrap();

        let language = manager.detect_repository_language(&repo_dir).unwrap();
        // Should detect Rust or TOML as primary language
        assert!(matches!(
            language,
            ProgrammingLanguage::Rust | ProgrammingLanguage::Toml
        ));
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = XzeConfig::default();
        let manager = RepositoryManager::new(temp_dir.path().to_path_buf(), config).unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_repositories, 0);

        let all_repos = manager.get_all_repositories().await;
        assert!(all_repos.is_empty());
    }

    #[test]
    fn test_source_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = XzeConfig::default();
        let manager = RepositoryManager::new(temp_dir.path().to_path_buf(), config).unwrap();

        assert!(manager.is_source_file(Path::new("main.rs"), &ProgrammingLanguage::Rust));
        assert!(manager.is_source_file(Path::new("main.go"), &ProgrammingLanguage::Go));
        assert!(manager.is_source_file(Path::new("main.py"), &ProgrammingLanguage::Python));
        assert!(!manager.is_source_file(Path::new("README.md"), &ProgrammingLanguage::Rust));
    }
}
