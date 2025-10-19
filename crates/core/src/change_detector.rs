//! Change detection system for monitoring repository updates
//!
//! This module provides comprehensive change detection capabilities including:
//! - Git-based change detection via polling
//! - Webhook support for event-driven updates
//! - Change significance analysis
//! - Documentation impact mapping
//! - File pattern matching for selective monitoring

use crate::{error::Result, git::GitOperations, types::RepositoryId, XzeError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use tracing::{debug, info};

/// Change detector for monitoring repository updates
#[derive(Debug)]
pub struct ChangeDetector {
    git_ops: GitOperations,
    config: ChangeDetectorConfig,
    last_checked: HashMap<RepositoryId, DateTime<Utc>>,
    last_commit: HashMap<RepositoryId, String>,
}

impl ChangeDetector {
    /// Create a new change detector
    pub fn new(git_ops: GitOperations, config: ChangeDetectorConfig) -> Self {
        Self {
            git_ops,
            config,
            last_checked: HashMap::new(),
            last_commit: HashMap::new(),
        }
    }

    /// Detect changes in a repository since last check
    pub async fn detect_changes(
        &mut self,
        repo_id: &RepositoryId,
        repo_path: &Path,
        branch: &str,
    ) -> Result<Option<RepositoryChanges>> {
        info!(
            "Detecting changes for repository {} on branch {}",
            repo_id, branch
        );

        // Open repository and fetch latest changes
        let repo = git2::Repository::open(repo_path)?;
        self.git_ops.fetch(&repo)?;

        // Get current HEAD commit
        let current_commit = self.get_head_commit(repo_path)?;

        // Check if we have a previous commit to compare against
        let previous_commit = self.last_commit.get(repo_id);

        let changes = if let Some(prev_commit) = previous_commit {
            if prev_commit == &current_commit {
                debug!("No changes detected for repository {}", repo_id);
                None
            } else {
                debug!("Changes detected: {} -> {}", prev_commit, current_commit);
                Some(self.analyze_changes(repo_path, prev_commit, &current_commit, branch)?)
            }
        } else {
            // First time checking this repository
            info!(
                "First time checking repository {}, recording baseline",
                repo_id
            );
            Some(self.get_initial_state(repo_path, &current_commit, branch)?)
        };

        // Update tracking state
        if changes.is_some() {
            self.last_commit
                .insert(repo_id.clone(), current_commit.clone());
            self.last_checked.insert(repo_id.clone(), Utc::now());
        }

        Ok(changes)
    }

    /// Poll for changes (blocking)
    pub fn poll_changes(
        &mut self,
        repo_id: &RepositoryId,
        repo_path: &Path,
        branch: &str,
    ) -> Result<Option<RepositoryChanges>> {
        // Create a new runtime for blocking context
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| XzeError::pipeline(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(self.detect_changes(repo_id, repo_path, branch))
    }

    /// Process a webhook event
    pub fn process_webhook(&mut self, event: WebhookEvent) -> Result<Option<RepositoryChanges>> {
        info!("Processing webhook event: {:?}", event.event_type);

        match event.event_type {
            WebhookEventType::Push => self.process_push_event(event),
            WebhookEventType::PullRequest => self.process_pr_event(event),
            WebhookEventType::Tag => self.process_tag_event(event),
            WebhookEventType::Other(_) => {
                debug!("Ignoring unsupported webhook event type");
                Ok(None)
            }
        }
    }

    /// Analyze change significance
    pub fn analyze_significance(&self, changes: &RepositoryChanges) -> ChangeSignificance {
        let mut score = 0;
        let mut reasons = Vec::new();

        // Check number of files changed
        if changes.files_changed.len() > self.config.significant_change_threshold {
            score += 3;
            reasons.push(format!(
                "{} files changed (threshold: {})",
                changes.files_changed.len(),
                self.config.significant_change_threshold
            ));
        }

        // Check for major file changes
        for file in &changes.files_changed {
            if self.is_major_file(&file.path) {
                score += 2;
                reasons.push(format!("Major file modified: {}", file.path.display()));
            }
        }

        // Check total lines changed
        let total_lines = changes.total_additions + changes.total_deletions;
        if total_lines > 500 {
            score += 2;
            reasons.push(format!("{} lines changed", total_lines));
        }

        // Check for breaking changes in commit messages
        if changes
            .commits
            .iter()
            .any(|c| c.message.contains("BREAKING") || c.message.contains("breaking change"))
        {
            score += 5;
            reasons.push("Breaking change detected in commit messages".to_string());
        }

        // Determine significance level
        let level = if score >= 7 {
            SignificanceLevel::Critical
        } else if score >= 4 {
            SignificanceLevel::Major
        } else if score >= 2 {
            SignificanceLevel::Minor
        } else {
            SignificanceLevel::Trivial
        };

        ChangeSignificance {
            level,
            score,
            reasons,
        }
    }

    /// Map changes to affected documentation
    pub fn map_documentation_impact(&self, changes: &RepositoryChanges) -> DocumentationImpact {
        let mut affected_sections = HashSet::new();
        let mut update_priority = UpdatePriority::Low;

        for file in &changes.files_changed {
            let path_str = file.path.to_string_lossy();

            // Map file patterns to documentation sections
            if path_str.contains("/api/") || path_str.ends_with("_api.rs") {
                affected_sections.insert(DocSection::ApiReference);
                update_priority = update_priority.max(UpdatePriority::High);
            }

            if path_str.contains("/src/") && path_str.ends_with(".rs") {
                affected_sections.insert(DocSection::CodeExamples);
            }

            if path_str.contains("README") || path_str.contains("docs/") {
                affected_sections.insert(DocSection::UserGuide);
                update_priority = update_priority.max(UpdatePriority::Medium);
            }

            if path_str.contains("/config") || path_str.ends_with(".yaml") {
                affected_sections.insert(DocSection::Configuration);
            }

            if path_str.contains("/test") {
                affected_sections.insert(DocSection::TestingGuide);
            }
        }

        // Check for breaking changes
        if changes
            .commits
            .iter()
            .any(|c| c.message.contains("BREAKING"))
        {
            affected_sections.insert(DocSection::MigrationGuide);
            update_priority = UpdatePriority::Critical;
        }

        DocumentationImpact {
            affected_sections: affected_sections.into_iter().collect(),
            update_priority,
            suggested_updates: self.suggest_updates(changes),
        }
    }

    /// Check if changes should trigger documentation update
    pub fn should_update_documentation(&self, changes: &RepositoryChanges) -> bool {
        // Skip if only trivial changes
        let significance = self.analyze_significance(changes);
        if matches!(significance.level, SignificanceLevel::Trivial) {
            return false;
        }

        // Skip if all changes are in excluded patterns
        let all_excluded = changes
            .files_changed
            .iter()
            .all(|f| self.is_excluded_path(&f.path));

        if all_excluded {
            debug!("All changes are in excluded paths, skipping documentation update");
            return false;
        }

        // Update if any significant files changed
        changes
            .files_changed
            .iter()
            .any(|f| self.is_documentation_relevant(&f.path))
    }

    // Private helper methods

    fn get_head_commit(&self, repo_path: &Path) -> Result<String> {
        let repo = git2::Repository::open(repo_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    fn analyze_changes(
        &self,
        repo_path: &Path,
        from_commit: &str,
        to_commit: &str,
        branch: &str,
    ) -> Result<RepositoryChanges> {
        let repo = git2::Repository::open(repo_path)?;

        // Get diff between commits
        let from_oid = git2::Oid::from_str(from_commit)?;
        let to_oid = git2::Oid::from_str(to_commit)?;

        let from_tree = repo.find_commit(from_oid)?.tree()?;
        let to_tree = repo.find_commit(to_oid)?.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

        let mut files_changed = Vec::new();
        let mut total_additions = 0;
        let mut total_deletions = 0;

        diff.foreach(
            &mut |delta, _progress| {
                if let Some(path) = delta.new_file().path() {
                    files_changed.push(FileChange {
                        path: path.to_path_buf(),
                        change_type: match delta.status() {
                            git2::Delta::Added => FileChangeType::Added,
                            git2::Delta::Deleted => FileChangeType::Deleted,
                            git2::Delta::Modified => FileChangeType::Modified,
                            git2::Delta::Renamed => FileChangeType::Renamed,
                            _ => FileChangeType::Modified,
                        },
                        additions: 0, // Will be filled in by line callback
                        deletions: 0,
                    });
                }
                true
            },
            None,
            None,
            Some(&mut |_delta, _hunk, line| {
                match line.origin() {
                    '+' => total_additions += 1,
                    '-' => total_deletions += 1,
                    _ => {}
                }
                true
            }),
        )?;

        // Get commits between the two refs
        let commits = self.get_commits_between(&repo, from_oid, to_oid)?;

        Ok(RepositoryChanges {
            repository_id: RepositoryId::from(repo_path.file_name().unwrap().to_str().unwrap()),
            branch: branch.to_string(),
            from_commit: from_commit.to_string(),
            to_commit: to_commit.to_string(),
            commits,
            files_changed,
            total_additions,
            total_deletions,
            detected_at: Utc::now(),
        })
    }

    fn get_initial_state(
        &self,
        repo_path: &Path,
        commit: &str,
        branch: &str,
    ) -> Result<RepositoryChanges> {
        Ok(RepositoryChanges {
            repository_id: RepositoryId::from(repo_path.file_name().unwrap().to_str().unwrap()),
            branch: branch.to_string(),
            from_commit: commit.to_string(),
            to_commit: commit.to_string(),
            commits: vec![],
            files_changed: vec![],
            total_additions: 0,
            total_deletions: 0,
            detected_at: Utc::now(),
        })
    }

    fn get_commits_between(
        &self,
        repo: &git2::Repository,
        from: git2::Oid,
        to: git2::Oid,
    ) -> Result<Vec<CommitInfo>> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push(to)?;
        revwalk.hide(from)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            commits.push(CommitInfo {
                hash: oid.to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                email: commit.author().email().unwrap_or("").to_string(),
                message: commit.message().unwrap_or("").to_string(),
                timestamp: DateTime::from_timestamp(commit.time().seconds(), 0)
                    .unwrap_or_else(Utc::now),
            });
        }

        Ok(commits)
    }

    fn process_push_event(&mut self, event: WebhookEvent) -> Result<Option<RepositoryChanges>> {
        // Extract push event details from payload
        if let Some(payload) = event.payload.as_object() {
            let branch = payload
                .get("ref")
                .and_then(|v| v.as_str())
                .unwrap_or("main")
                .trim_start_matches("refs/heads/");

            let before = payload.get("before").and_then(|v| v.as_str()).unwrap_or("");
            let after = payload.get("after").and_then(|v| v.as_str()).unwrap_or("");

            Ok(Some(RepositoryChanges {
                repository_id: event.repository_id,
                branch: branch.to_string(),
                from_commit: before.to_string(),
                to_commit: after.to_string(),
                commits: vec![],
                files_changed: vec![],
                total_additions: 0,
                total_deletions: 0,
                detected_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    fn process_pr_event(&mut self, event: WebhookEvent) -> Result<Option<RepositoryChanges>> {
        debug!("Processing PR webhook event for {}", event.repository_id);
        // PR events don't necessarily trigger documentation updates
        // unless they're merged
        if let Some(payload) = event.payload.as_object() {
            let action = payload.get("action").and_then(|v| v.as_str());
            if action == Some("closed")
                && payload
                    .get("pull_request")
                    .and_then(|pr| pr.get("merged"))
                    .and_then(|m| m.as_bool())
                    == Some(true)
            {
                // PR was merged, this might warrant doc update
                debug!("PR merged, may trigger documentation update");
            }
        }
        Ok(None)
    }

    fn process_tag_event(&mut self, event: WebhookEvent) -> Result<Option<RepositoryChanges>> {
        debug!("Processing tag webhook event for {}", event.repository_id);
        // Tag events might trigger release documentation
        Ok(None)
    }

    fn is_major_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.config.major_file_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
        })
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.config.exclude_patterns.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
        })
    }

    fn is_documentation_relevant(&self, path: &Path) -> bool {
        !self.is_excluded_path(path) && (self.is_major_file(path) || self.is_source_file(path))
    }

    fn is_source_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        path_str.ends_with(".rs")
            || path_str.ends_with(".py")
            || path_str.ends_with(".js")
            || path_str.ends_with(".ts")
            || path_str.ends_with(".go")
            || path_str.ends_with(".java")
    }

    fn suggest_updates(&self, changes: &RepositoryChanges) -> Vec<String> {
        let mut suggestions = Vec::new();

        if changes.total_additions + changes.total_deletions > 100 {
            suggestions.push("Review and update API documentation".to_string());
        }

        if changes
            .commits
            .iter()
            .any(|c| c.message.to_lowercase().contains("api"))
        {
            suggestions.push("Update API reference documentation".to_string());
        }

        if changes
            .commits
            .iter()
            .any(|c| c.message.to_lowercase().contains("config"))
        {
            suggestions.push("Update configuration guide".to_string());
        }

        suggestions
    }
}

/// Change detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectorConfig {
    /// Patterns for files that trigger major updates
    #[serde(default = "default_major_patterns")]
    pub major_file_patterns: Vec<String>,

    /// Patterns to exclude from change detection
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// Number of files changed to be considered significant
    #[serde(default = "default_threshold")]
    pub significant_change_threshold: usize,

    /// Enable webhook support
    #[serde(default = "default_true")]
    pub enable_webhooks: bool,

    /// Polling interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
}

impl Default for ChangeDetectorConfig {
    fn default() -> Self {
        Self {
            major_file_patterns: default_major_patterns(),
            exclude_patterns: default_exclude_patterns(),
            significant_change_threshold: default_threshold(),
            enable_webhooks: true,
            poll_interval_seconds: default_poll_interval(),
        }
    }
}

/// Repository changes detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryChanges {
    pub repository_id: RepositoryId,
    pub branch: String,
    pub from_commit: String,
    pub to_commit: String,
    pub commits: Vec<CommitInfo>,
    pub files_changed: Vec<FileChange>,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub detected_at: DateTime<Utc>,
}

/// Individual commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// File change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub additions: usize,
    pub deletions: usize,
}

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Webhook event received from Git platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub repository_id: RepositoryId,
    pub event_type: WebhookEventType,
    pub payload: serde_json::Value,
    pub received_at: DateTime<Utc>,
}

/// Type of webhook event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    Push,
    PullRequest,
    Tag,
    Other(String),
}

/// Change significance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSignificance {
    pub level: SignificanceLevel,
    pub score: u32,
    pub reasons: Vec<String>,
}

/// Significance level of changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignificanceLevel {
    Trivial,
    Minor,
    Major,
    Critical,
}

/// Documentation impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationImpact {
    pub affected_sections: Vec<DocSection>,
    pub update_priority: UpdatePriority,
    pub suggested_updates: Vec<String>,
}

/// Documentation sections that might be affected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocSection {
    ApiReference,
    UserGuide,
    Configuration,
    TestingGuide,
    CodeExamples,
    MigrationGuide,
    Architecture,
    Tutorials,
}

/// Priority for documentation updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdatePriority {
    Low,
    Medium,
    High,
    Critical,
}

// Default value functions
fn default_major_patterns() -> Vec<String> {
    vec![
        "src/**/*.rs".to_string(),
        "lib/**/*.rs".to_string(),
        "src/api/**/*".to_string(),
        "Cargo.toml".to_string(),
        "package.json".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "**/.git/**".to_string(),
        "**/target/**".to_string(),
        "**/node_modules/**".to_string(),
        "**/*.log".to_string(),
        "**/.DS_Store".to_string(),
        "**/tmp/**".to_string(),
    ]
}

fn default_threshold() -> usize {
    5
}

fn default_true() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    300 // 5 minutes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::CredentialStore;

    fn create_test_detector() -> ChangeDetector {
        let git_ops = GitOperations::new(CredentialStore::new());
        let config = ChangeDetectorConfig::default();
        ChangeDetector::new(git_ops, config)
    }

    #[test]
    fn test_change_detector_creation() {
        let detector = create_test_detector();
        assert_eq!(detector.config.significant_change_threshold, 5);
        assert!(detector.config.enable_webhooks);
    }

    #[test]
    fn test_significance_analysis() {
        let detector = create_test_detector();

        let changes = RepositoryChanges {
            repository_id: RepositoryId::from("test-repo"),
            branch: "main".to_string(),
            from_commit: "abc123".to_string(),
            to_commit: "def456".to_string(),
            commits: vec![CommitInfo {
                hash: "def456".to_string(),
                author: "Test Author".to_string(),
                email: "test@example.com".to_string(),
                message: "BREAKING: Major API change".to_string(),
                timestamp: Utc::now(),
            }],
            files_changed: vec![],
            total_additions: 100,
            total_deletions: 50,
            detected_at: Utc::now(),
        };

        let significance = detector.analyze_significance(&changes);
        assert!(matches!(
            significance.level,
            SignificanceLevel::Major | SignificanceLevel::Critical
        ));
    }

    #[test]
    fn test_documentation_impact() {
        let detector = create_test_detector();

        let changes = RepositoryChanges {
            repository_id: RepositoryId::from("test-repo"),
            branch: "main".to_string(),
            from_commit: "abc123".to_string(),
            to_commit: "def456".to_string(),
            commits: vec![],
            files_changed: vec![FileChange {
                path: PathBuf::from("src/api/handler.rs"),
                change_type: FileChangeType::Modified,
                additions: 50,
                deletions: 20,
            }],
            total_additions: 50,
            total_deletions: 20,
            detected_at: Utc::now(),
        };

        let impact = detector.map_documentation_impact(&changes);
        assert!(impact.affected_sections.contains(&DocSection::ApiReference));
    }

    #[test]
    fn test_should_update_documentation() {
        let detector = create_test_detector();

        let changes = RepositoryChanges {
            repository_id: RepositoryId::from("test-repo"),
            branch: "main".to_string(),
            from_commit: "abc123".to_string(),
            to_commit: "def456".to_string(),
            commits: vec![],
            files_changed: vec![FileChange {
                path: PathBuf::from("src/main.rs"),
                change_type: FileChangeType::Modified,
                additions: 100,
                deletions: 50,
            }],
            total_additions: 100,
            total_deletions: 50,
            detected_at: Utc::now(),
        };

        assert!(detector.should_update_documentation(&changes));
    }

    #[test]
    fn test_exclude_patterns() {
        let detector = create_test_detector();

        assert!(detector.is_excluded_path(Path::new("target/debug/output")));
        assert!(detector.is_excluded_path(Path::new(".git/config")));
        assert!(!detector.is_excluded_path(Path::new("src/main.rs")));
    }

    #[test]
    fn test_webhook_event_types() {
        let push_event = WebhookEvent {
            repository_id: RepositoryId::from("test-repo"),
            event_type: WebhookEventType::Push,
            payload: serde_json::json!({}),
            received_at: Utc::now(),
        };

        assert_eq!(push_event.event_type, WebhookEventType::Push);
    }

    #[test]
    fn test_config_defaults() {
        let config = ChangeDetectorConfig::default();
        assert!(!config.major_file_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert_eq!(config.poll_interval_seconds, 300);
    }
}
