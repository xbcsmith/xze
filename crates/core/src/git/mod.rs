//! Git operations

use crate::{error::Result, XzeError};
use git2::{Commit, Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use std::path::Path;

pub mod credentials;
pub mod pr;

pub use credentials::CredentialStore;
pub use pr::{GitHubPrManager, PullRequest, PullRequestManager};

// Type alias for convenience
pub type PrManager = GitHubPrManager;

/// Git operations manager
#[derive(Debug)]
pub struct GitOperations {
    credentials: CredentialStore,
}

impl GitOperations {
    pub fn new(credentials: CredentialStore) -> Self {
        Self { credentials }
    }

    /// Clone a repository
    pub async fn clone(&self, url: &str, dest: &Path) -> Result<Repository> {
        tracing::info!("Cloning repository {} to {}", url, dest.display());

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder.clone(url, dest).map_err(XzeError::Git)?;

        Ok(repo)
    }

    /// Open an existing repository
    pub fn open<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        Repository::open(path).map_err(XzeError::Git)
    }

    /// Create a new branch
    pub fn create_branch(&self, repo: &Repository, name: &str) -> Result<()> {
        tracing::info!("Creating branch: {}", name);

        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        repo.branch(name, &head_commit, false)?;

        // Checkout the new branch
        repo.set_head(&format!("refs/heads/{}", name))?;

        Ok(())
    }

    /// Commit changes
    pub fn commit(&self, repo: &Repository, message: &str) -> Result<git2::Oid> {
        tracing::info!("Committing changes: {}", message);

        let mut index = repo.index()?;
        index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let signature = repo.signature()?;
        let parent_commit = self.get_head_commit(repo)?;

        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(oid)
    }

    /// Push to remote
    pub fn push(&self, repo: &Repository, branch: &str) -> Result<()> {
        tracing::info!("Pushing branch: {}", branch);

        let mut remote = repo.find_remote("origin")?;

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(
            &[&format!("refs/heads/{}:refs/heads/{}", branch, branch)],
            Some(&mut push_options),
        )?;

        Ok(())
    }

    /// Get the current HEAD commit
    fn get_head_commit<'repo>(&self, repo: &'repo Repository) -> Result<Commit<'repo>> {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit)
    }

    /// Get the last commit hash
    pub fn get_last_commit_hash(&self, repo: &Repository) -> Result<String> {
        let commit = self.get_head_commit(repo)?;
        Ok(commit.id().to_string())
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self, repo: &Repository) -> Result<bool> {
        let statuses = repo.statuses(None)?;
        Ok(!statuses.is_empty())
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

    /// Fetch from remote
    pub fn fetch(&self, repo: &Repository) -> Result<()> {
        tracing::info!("Fetching from remote");

        let mut remote = repo.find_remote("origin")?;

        let mut callbacks = RemoteCallbacks::new();
        self.setup_credentials(&mut callbacks)?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(
            &["refs/heads/*:refs/remotes/origin/*"],
            Some(&mut fetch_options),
            None,
        )?;

        Ok(())
    }

    /// Pull from remote (fetch + merge)
    pub fn pull(&self, repo: &Repository) -> Result<()> {
        self.fetch(repo)?;

        // Get current branch
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| XzeError::repository("Failed to get branch name"))?;

        // Perform merge
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        let analysis = repo.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            tracing::info!("Already up to date");
        } else if analysis.0.is_fast_forward() {
            tracing::info!("Fast-forwarding");
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        }

        Ok(())
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
        assert!(true); // Just test creation
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let creds = CredentialStore::new();
        let git_ops = GitOperations::new(creds);
        let temp = tempdir().unwrap();

        let result = git_ops.open(temp.path().join("nonexistent"));
        assert!(result.is_err());
    }
}
