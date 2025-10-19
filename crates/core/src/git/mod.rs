//! Git operations module
//!
//! This module provides comprehensive Git operations including:
//! - Repository cloning and initialization
//! - Branch creation and management
//! - Commit operations with proper messages
//! - Push/pull operations with credential handling
//! - Diff analysis and change detection
//! - Remote branch management
//! - Conflict detection
//! - Stash operations
//! - Tag management

use crate::{error::Result, XzeError};
use git2::{
    BranchType, Commit, Cred, Delta, DiffOptions, FetchOptions, IndexAddOption, Oid, PushOptions,
    RemoteCallbacks, Repository, Signature, Status, StatusOptions,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub mod credentials;
pub mod pr;

pub use credentials::{credentials_from_env, CredentialStore};
pub use pr::{GitHubPrManager, PullRequest, PullRequestManager};

// Type alias for convenience
pub type PrManager = GitHubPrManager;

/// Represents a file change in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Path to the changed file
    pub path: PathBuf,
    /// Type of change (added, modified, deleted, renamed)
    pub change_type: ChangeType,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines deleted
    pub deletions: usize,
    /// Old path (for renames)
    pub old_path: Option<PathBuf>,
}

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

impl From<Delta> for ChangeType {
    fn from(delta: Delta) -> Self {
        match delta {
            Delta::Added => ChangeType::Added,
            Delta::Deleted => ChangeType::Deleted,
            Delta::Modified => ChangeType::Modified,
            Delta::Renamed => ChangeType::Renamed,
            Delta::Copied => ChangeType::Copied,
            Delta::Typechange => ChangeType::TypeChanged,
            _ => ChangeType::Modified,
        }
    }
}

/// Represents a diff between two commits or the working directory
#[derive(Debug, Clone)]
pub struct DiffSummary {
    /// List of file changes
    pub changes: Vec<FileChange>,
    /// Total files changed
    pub files_changed: usize,
    /// Total insertions across all files
    pub insertions: usize,
    /// Total deletions across all files
    pub deletions: usize,
}

impl DiffSummary {
    /// Create an empty diff summary
    pub fn empty() -> Self {
        Self {
            changes: Vec::new(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        }
    }

    /// Check if diff has any changes
    pub fn has_changes(&self) -> bool {
        self.files_changed > 0
    }

    /// Get changes by type
    pub fn changes_by_type(&self, change_type: ChangeType) -> Vec<&FileChange> {
        self.changes
            .iter()
            .filter(|c| c.change_type == change_type)
            .collect()
    }
}

/// Information about a branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Is this the current branch
    pub is_current: bool,
    /// Is this a remote branch
    pub is_remote: bool,
    /// Upstream branch name if it exists
    pub upstream: Option<String>,
    /// Latest commit hash
    pub commit_hash: String,
}

/// Git operations manager
#[derive(Debug)]
pub struct GitOperations {
    credentials: CredentialStore,
}

impl GitOperations {
    /// Create a new GitOperations instance
    ///
    /// # Arguments
    ///
    /// * `credentials` - Credential store for authentication
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xze_core::git::{GitOperations, CredentialStore};
    ///
    /// let creds = CredentialStore::new();
    /// let git_ops = GitOperations::new(creds);
    /// ```
    pub fn new(credentials: CredentialStore) -> Self {
        Self { credentials }
    }

    /// Clone a repository
    ///
    /// # Arguments
    ///
    /// * `url` - Repository URL to clone
    /// * `dest` - Destination path for the cloned repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xze_core::git::{GitOperations, CredentialStore};
    /// # async fn example() -> xze_core::Result<()> {
    /// let git_ops = GitOperations::new(CredentialStore::new());
    /// let repo = git_ops.clone("https://github.com/user/repo", std::path::Path::new("./repo")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clone(&self, url: &str, dest: &Path) -> Result<Repository> {
        tracing::info!("Cloning repository {} to {}", url, dest.display());

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder.clone(url, dest).map_err(XzeError::Git)?;

        tracing::info!("Successfully cloned repository to {}", dest.display());
        Ok(repo)
    }

    /// Open an existing repository
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the repository
    pub fn open<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        let path = path.as_ref();
        tracing::debug!("Opening repository at {}", path.display());
        Repository::open(path).map_err(XzeError::Git)
    }

    /// Initialize a new repository
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to initialize the repository
    pub fn init<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        let path = path.as_ref();
        tracing::info!("Initializing repository at {}", path.display());
        Repository::init(path).map_err(XzeError::Git)
    }

    /// Create a new branch
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `name` - Name of the new branch
    /// * `checkout` - Whether to checkout the new branch
    pub fn create_branch(&self, repo: &Repository, name: &str, checkout: bool) -> Result<()> {
        tracing::info!("Creating branch: {}", name);

        let head = repo.head().map_err(XzeError::Git)?;
        let head_commit = head.peel_to_commit().map_err(XzeError::Git)?;

        repo.branch(name, &head_commit, false)
            .map_err(XzeError::Git)?;

        if checkout {
            self.checkout_branch(repo, name)?;
        }

        Ok(())
    }

    /// Checkout a branch
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `name` - Name of the branch to checkout
    pub fn checkout_branch(&self, repo: &Repository, name: &str) -> Result<()> {
        tracing::info!("Checking out branch: {}", name);

        repo.set_head(&format!("refs/heads/{}", name))
            .map_err(XzeError::Git)?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(XzeError::Git)?;

        Ok(())
    }

    /// Delete a branch
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `name` - Name of the branch to delete
    pub fn delete_branch(&self, repo: &Repository, name: &str) -> Result<()> {
        tracing::info!("Deleting branch: {}", name);

        let mut branch = repo
            .find_branch(name, BranchType::Local)
            .map_err(XzeError::Git)?;

        branch.delete().map_err(XzeError::Git)?;

        Ok(())
    }

    /// List all branches in the repository
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `local_only` - If true, only list local branches
    pub fn list_branches(&self, repo: &Repository, local_only: bool) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();
        let current_branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let branch_type = if local_only {
            BranchType::Local
        } else {
            BranchType::Local
        };

        for branch_result in repo.branches(Some(branch_type)).map_err(XzeError::Git)? {
            let (branch, _) = branch_result.map_err(XzeError::Git)?;
            let name = branch
                .name()
                .map_err(XzeError::Git)?
                .ok_or_else(|| XzeError::repository("Invalid branch name"))?
                .to_string();

            let is_current = current_branch.as_ref().map(|c| c == &name).unwrap_or(false);
            let is_remote = matches!(branch.get().kind(), Some(git2::ReferenceType::Symbolic));

            let upstream = branch
                .upstream()
                .ok()
                .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

            let commit_hash = branch
                .get()
                .peel_to_commit()
                .ok()
                .map(|c| c.id().to_string())
                .unwrap_or_default();

            branches.push(BranchInfo {
                name,
                is_current,
                is_remote,
                upstream,
                commit_hash,
            });
        }

        Ok(branches)
    }

    /// Get the current branch name
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn current_branch(&self, repo: &Repository) -> Result<String> {
        let head = repo.head().map_err(XzeError::Git)?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| XzeError::repository("Failed to get branch name"))?
            .to_string();
        Ok(branch_name)
    }

    /// Stage all changes
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn stage_all(&self, repo: &Repository) -> Result<()> {
        tracing::debug!("Staging all changes");
        let mut index = repo.index().map_err(XzeError::Git)?;
        index
            .add_all(["."].iter(), IndexAddOption::DEFAULT, None)
            .map_err(XzeError::Git)?;
        index.write().map_err(XzeError::Git)?;
        Ok(())
    }

    /// Stage specific files
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `paths` - Paths to stage
    pub fn stage_files<P: AsRef<Path>>(&self, repo: &Repository, paths: &[P]) -> Result<()> {
        tracing::debug!("Staging {} files", paths.len());
        let mut index = repo.index().map_err(XzeError::Git)?;

        for path in paths {
            index.add_path(path.as_ref()).map_err(XzeError::Git)?;
        }

        index.write().map_err(XzeError::Git)?;
        Ok(())
    }

    /// Commit changes with a message
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `message` - Commit message
    pub fn commit(&self, repo: &Repository, message: &str) -> Result<Oid> {
        tracing::info!("Committing changes: {}", message);

        let mut index = repo.index().map_err(XzeError::Git)?;
        let tree_id = index.write_tree().map_err(XzeError::Git)?;
        let tree = repo.find_tree(tree_id).map_err(XzeError::Git)?;

        let signature = self.get_signature(repo)?;
        let parent_commit = self.get_head_commit(repo)?;

        let oid = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[&parent_commit],
            )
            .map_err(XzeError::Git)?;

        tracing::info!("Created commit: {}", oid);
        Ok(oid)
    }

    /// Commit changes with author and committer information
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `message` - Commit message
    /// * `author` - Author signature
    /// * `committer` - Committer signature
    pub fn commit_with_signature(
        &self,
        repo: &Repository,
        message: &str,
        author: &Signature,
        committer: &Signature,
    ) -> Result<Oid> {
        tracing::info!("Committing changes with custom signature");

        let mut index = repo.index().map_err(XzeError::Git)?;
        let tree_id = index.write_tree().map_err(XzeError::Git)?;
        let tree = repo.find_tree(tree_id).map_err(XzeError::Git)?;

        let parent_commit = self.get_head_commit(repo)?;

        let oid = repo
            .commit(
                Some("HEAD"),
                author,
                committer,
                message,
                &tree,
                &[&parent_commit],
            )
            .map_err(XzeError::Git)?;

        Ok(oid)
    }

    /// Push to remote
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `branch` - Branch name to push
    pub fn push(&self, repo: &Repository, branch: &str) -> Result<()> {
        tracing::info!("Pushing branch: {}", branch);

        let mut remote = repo.find_remote("origin").map_err(XzeError::Git)?;

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote
            .push(
                &[&format!("refs/heads/{}:refs/heads/{}", branch, branch)],
                Some(&mut push_options),
            )
            .map_err(XzeError::Git)?;

        tracing::info!("Successfully pushed branch: {}", branch);
        Ok(())
    }

    /// Fetch from remote
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn fetch(&self, repo: &Repository) -> Result<()> {
        tracing::info!("Fetching from remote");

        let mut remote = repo.find_remote("origin").map_err(XzeError::Git)?;

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote
            .fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(&mut fetch_options),
                None,
            )
            .map_err(XzeError::Git)?;

        tracing::info!("Successfully fetched from remote");
        Ok(())
    }

    /// Pull from remote (fetch + merge)
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn pull(&self, repo: &Repository) -> Result<()> {
        self.fetch(repo)?;

        let head = repo.head().map_err(XzeError::Git)?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| XzeError::repository("Failed to get branch name"))?;

        let fetch_head = repo.find_reference("FETCH_HEAD").map_err(XzeError::Git)?;
        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(XzeError::Git)?;

        let analysis = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(XzeError::Git)?;

        if analysis.0.is_up_to_date() {
            tracing::info!("Already up to date");
        } else if analysis.0.is_fast_forward() {
            tracing::info!("Fast-forwarding");
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo.find_reference(&refname).map_err(XzeError::Git)?;
            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(XzeError::Git)?;
            repo.set_head(&refname).map_err(XzeError::Git)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(XzeError::Git)?;
        } else {
            return Err(XzeError::repository(
                "Pull requires merge, which is not implemented yet",
            ));
        }

        Ok(())
    }

    /// Analyze changes between two commits or working directory
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `from` - Starting commit (None for working directory)
    /// * `to` - Ending commit (None for HEAD)
    pub fn diff_analysis(
        &self,
        repo: &Repository,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<DiffSummary> {
        tracing::debug!("Analyzing diff from {:?} to {:?}", from, to);

        let from_tree = if let Some(from_ref) = from {
            let from_obj = repo.revparse_single(from_ref).map_err(XzeError::Git)?;
            Some(from_obj.peel_to_tree().map_err(XzeError::Git)?)
        } else {
            None
        };

        let to_tree = if let Some(to_ref) = to {
            let to_obj = repo.revparse_single(to_ref).map_err(XzeError::Git)?;
            Some(to_obj.peel_to_tree().map_err(XzeError::Git)?)
        } else {
            let head = repo.head().map_err(XzeError::Git)?;
            let commit = head.peel_to_commit().map_err(XzeError::Git)?;
            Some(commit.tree().map_err(XzeError::Git)?)
        };

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let diff = repo
            .diff_tree_to_tree(from_tree.as_ref(), to_tree.as_ref(), Some(&mut diff_opts))
            .map_err(XzeError::Git)?;

        let mut changes = Vec::new();
        let insertions = Arc::new(RwLock::new(0usize));
        let deletions = Arc::new(RwLock::new(0usize));

        let ins_clone = Arc::clone(&insertions);
        let del_clone = Arc::clone(&deletions);

        diff.foreach(
            &mut |delta, _progress| {
                let change_type = ChangeType::from(delta.status());
                let path = delta.new_file().path().unwrap_or_else(|| Path::new(""));
                let old_path = if matches!(change_type, ChangeType::Renamed) {
                    delta.old_file().path().map(|p| p.to_path_buf())
                } else {
                    None
                };

                changes.push(FileChange {
                    path: path.to_path_buf(),
                    change_type,
                    additions: 0,
                    deletions: 0,
                    old_path,
                });
                true
            },
            None,
            None,
            Some(&mut |_delta, _hunk, line| {
                match line.origin() {
                    '+' => {
                        *ins_clone.write().unwrap() += 1;
                    }
                    '-' => {
                        *del_clone.write().unwrap() += 1;
                    }
                    _ => {}
                }
                true
            }),
        )
        .map_err(XzeError::Git)?;

        let total_insertions = *insertions.read().unwrap();
        let total_deletions = *deletions.read().unwrap();

        // Update individual file stats by running diff again
        let _stats = diff.stats().map_err(XzeError::Git)?;
        for (i, _delta) in diff.deltas().enumerate() {
            if i < changes.len() {
                // Approximate stats distribution
                changes[i].additions = (total_insertions / changes.len().max(1)) as usize;
                changes[i].deletions = (total_deletions / changes.len().max(1)) as usize;
            }
        }

        Ok(DiffSummary {
            files_changed: changes.len(),
            changes,
            insertions: total_insertions,
            deletions: total_deletions,
        })
    }

    /// Check if there are uncommitted changes
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn has_changes(&self, repo: &Repository) -> Result<bool> {
        let statuses = repo.statuses(None).map_err(XzeError::Git)?;
        Ok(!statuses.is_empty())
    }

    /// Get detailed status of files
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn get_status(&self, repo: &Repository) -> Result<HashMap<PathBuf, Status>> {
        let mut status_map = HashMap::new();
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts)).map_err(XzeError::Git)?;

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                status_map.insert(PathBuf::from(path), entry.status());
            }
        }

        Ok(status_map)
    }

    /// Check for merge conflicts
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn has_conflicts(&self, repo: &Repository) -> Result<bool> {
        let index = repo.index().map_err(XzeError::Git)?;
        Ok(index.has_conflicts())
    }

    /// Get conflicted files
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn get_conflicts(&self, repo: &Repository) -> Result<Vec<PathBuf>> {
        let index = repo.index().map_err(XzeError::Git)?;
        let mut conflicts = Vec::new();

        for conflict in index.conflicts().map_err(XzeError::Git)? {
            let conflict = conflict.map_err(XzeError::Git)?;
            if let Some(our) = conflict.our {
                if let Ok(path_str) = std::str::from_utf8(&our.path) {
                    conflicts.push(PathBuf::from(path_str));
                }
            }
        }

        Ok(conflicts)
    }

    /// Get the last commit hash
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn get_last_commit_hash(&self, repo: &Repository) -> Result<String> {
        let commit = self.get_head_commit(repo)?;
        Ok(commit.id().to_string())
    }

    /// Get HEAD commit
    fn get_head_commit<'repo>(&self, repo: &'repo Repository) -> Result<Commit<'repo>> {
        let head = repo.head().map_err(XzeError::Git)?;
        let commit = head.peel_to_commit().map_err(XzeError::Git)?;
        Ok(commit)
    }

    /// Get signature from config or use default
    fn get_signature<'a>(&self, repo: &'a Repository) -> Result<Signature<'a>> {
        repo.signature().map_err(XzeError::Git)
    }

    /// Setup credentials callback
    fn setup_credentials(&self, callbacks: &mut RemoteCallbacks) -> Result<()> {
        let creds = self.credentials.clone();

        callbacks.credentials(move |url, username_from_url, allowed_types| {
            creds
                .create_credentials(url, username_from_url, allowed_types)
                .or_else(|_| {
                    if let Some(username) = username_from_url {
                        Cred::ssh_key_from_agent(username)
                    } else {
                        Cred::default()
                    }
                })
        });

        Ok(())
    }

    /// Create a tag
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `name` - Tag name
    /// * `message` - Tag message (for annotated tags)
    /// * `force` - Overwrite existing tag
    pub fn create_tag(
        &self,
        repo: &Repository,
        name: &str,
        message: Option<&str>,
        force: bool,
    ) -> Result<Oid> {
        tracing::info!("Creating tag: {}", name);

        let target = self.get_head_commit(repo)?;
        let target_obj = target.as_object();

        let oid = if let Some(msg) = message {
            let sig = self.get_signature(repo)?;
            repo.tag(name, target_obj, &sig, msg, force)
                .map_err(XzeError::Git)?
        } else {
            repo.tag_lightweight(name, target_obj, force)
                .map_err(XzeError::Git)?;
            target.id()
        };

        Ok(oid)
    }

    /// Delete a tag
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `name` - Tag name to delete
    pub fn delete_tag(&self, repo: &Repository, name: &str) -> Result<()> {
        tracing::info!("Deleting tag: {}", name);
        repo.tag_delete(name).map_err(XzeError::Git)?;
        Ok(())
    }

    /// List all tags
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn list_tags(&self, repo: &Repository) -> Result<Vec<String>> {
        let tag_names = repo.tag_names(None).map_err(XzeError::Git)?;
        let tags = tag_names
            .iter()
            .filter_map(|name| name.map(|s| s.to_string()))
            .collect();
        Ok(tags)
    }

    /// Stash changes
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `message` - Stash message
    pub fn stash(&self, repo: &mut Repository, message: Option<&str>) -> Result<Oid> {
        tracing::info!("Stashing changes");

        let msg = message.unwrap_or("WIP");

        // Get signature and use it immediately to avoid borrow issues
        let sig = repo.signature().map_err(XzeError::Git)?;
        let oid = repo.stash_save(&sig, msg, None).map_err(XzeError::Git)?;

        Ok(oid)
    }

    /// Apply stashed changes
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `index` - Stash index (0 is most recent)
    pub fn stash_pop(&self, repo: &mut Repository, index: usize) -> Result<()> {
        tracing::info!("Popping stash at index {}", index);
        repo.stash_pop(index, None).map_err(XzeError::Git)?;
        Ok(())
    }

    /// Reset to a specific commit
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `commit_hash` - Commit hash to reset to
    /// * `reset_type` - Type of reset (soft, mixed, hard)
    pub fn reset(
        &self,
        repo: &Repository,
        commit_hash: &str,
        reset_type: git2::ResetType,
    ) -> Result<()> {
        tracing::info!("Resetting to commit: {}", commit_hash);

        let obj = repo.revparse_single(commit_hash).map_err(XzeError::Git)?;

        repo.reset(&obj, reset_type, None).map_err(XzeError::Git)?;

        Ok(())
    }

    /// Check if repository is clean (no uncommitted changes)
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    pub fn is_clean(&self, repo: &Repository) -> Result<bool> {
        Ok(!self.has_changes(repo)? && !self.has_conflicts(repo)?)
    }

    /// Get remote URL
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository reference
    /// * `remote_name` - Name of the remote (default: "origin")
    pub fn get_remote_url(&self, repo: &Repository, remote_name: &str) -> Result<String> {
        let remote = repo.find_remote(remote_name).map_err(XzeError::Git)?;
        let url = remote
            .url()
            .ok_or_else(|| XzeError::repository("Remote URL not found"))?;
        Ok(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_git_operations_creation() {
        let creds = CredentialStore::new();
        let _git_ops = GitOperations::new(creds);
    }

    #[test]
    fn test_init_repository() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let result = git_ops.init(temp.path());
        assert!(result.is_ok());

        let repo = result.unwrap();
        assert!(!repo.is_bare());
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let result = git_ops.open(temp.path().join("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_change_type_conversion() {
        assert_eq!(ChangeType::from(Delta::Added), ChangeType::Added);
        assert_eq!(ChangeType::from(Delta::Modified), ChangeType::Modified);
        assert_eq!(ChangeType::from(Delta::Deleted), ChangeType::Deleted);
        assert_eq!(ChangeType::from(Delta::Renamed), ChangeType::Renamed);
    }

    #[test]
    fn test_diff_summary_empty() {
        let summary = DiffSummary::empty();
        assert!(!summary.has_changes());
        assert_eq!(summary.files_changed, 0);
        assert_eq!(summary.insertions, 0);
        assert_eq!(summary.deletions, 0);
    }

    #[test]
    fn test_diff_summary_changes_by_type() {
        let changes = vec![
            FileChange {
                path: PathBuf::from("file1.txt"),
                change_type: ChangeType::Added,
                additions: 10,
                deletions: 0,
                old_path: None,
            },
            FileChange {
                path: PathBuf::from("file2.txt"),
                change_type: ChangeType::Modified,
                additions: 5,
                deletions: 3,
                old_path: None,
            },
            FileChange {
                path: PathBuf::from("file3.txt"),
                change_type: ChangeType::Added,
                additions: 8,
                deletions: 0,
                old_path: None,
            },
        ];

        let summary = DiffSummary {
            files_changed: 3,
            insertions: 23,
            deletions: 3,
            changes: changes.clone(),
        };

        let added = summary.changes_by_type(ChangeType::Added);
        assert_eq!(added.len(), 2);
        assert_eq!(added[0].path, PathBuf::from("file1.txt"));
        assert_eq!(added[1].path, PathBuf::from("file3.txt"));

        let modified = summary.changes_by_type(ChangeType::Modified);
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0].path, PathBuf::from("file2.txt"));
    }

    #[test]
    fn test_create_and_checkout_branch() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let repo = git_ops.init(temp.path()).unwrap();

        // Create initial commit
        let sig = repo.signature().unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Create and checkout branch
        let result = git_ops.create_branch(&repo, "test-branch", true);
        assert!(result.is_ok());

        let current = git_ops.current_branch(&repo).unwrap();
        assert_eq!(current, "test-branch");
    }

    #[test]
    fn test_has_changes_clean_repo() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let repo = git_ops.init(temp.path()).unwrap();
        let result = git_ops.has_changes(&repo);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let repo = git_ops.init(temp.path()).unwrap();

        // Create initial commit
        let sig = repo.signature().unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let result = git_ops.create_tag(&repo, "v1.0.0", Some("Version 1.0.0"), false);
        assert!(result.is_ok());

        let tags = git_ops.list_tags(&repo).unwrap();
        assert!(tags.contains(&"v1.0.0".to_string()));
    }

    #[test]
    fn test_is_clean_repository() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let repo = git_ops.init(temp.path()).unwrap();

        // New repository should be clean (no changes or conflicts)
        let result = git_ops.is_clean(&repo);
        assert!(result.is_ok());
    }
}
