//! Repository watcher for continuous monitoring and auto-triggering
//!
//! This module provides a comprehensive repository watching system that:
//! - Continuously monitors repositories for changes
//! - Automatically triggers documentation generation
//! - Manages polling intervals and backoff strategies
//! - Integrates with webhook systems for event-driven updates
//! - Coordinates with the pipeline controller for automated workflows

use crate::{
    change_detector::{ChangeDetector, ChangeDetectorConfig, RepositoryChanges},
    error::Result,
    git::GitOperations,
    pipeline::controller::PipelineController,
    types::RepositoryId,
    XzeError,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    sync::{mpsc, RwLock},
    time::interval,
};
use tracing::{debug, error, info, warn};

/// Repository watcher for auto-mode monitoring
pub struct RepositoryWatcher {
    config: WatcherConfig,
    change_detector: Arc<RwLock<ChangeDetector>>,
    pipeline_controller: Arc<PipelineController>,
    watched_repos: Arc<RwLock<HashMap<RepositoryId, WatchedRepository>>>,
    control_tx: mpsc::Sender<WatcherCommand>,
    control_rx: Arc<RwLock<mpsc::Receiver<WatcherCommand>>>,
    stats: Arc<RwLock<WatcherStats>>,
}

impl RepositoryWatcher {
    /// Create a new repository watcher
    pub fn new(
        config: WatcherConfig,
        git_ops: GitOperations,
        pipeline_controller: Arc<PipelineController>,
    ) -> Self {
        let change_detector = Arc::new(RwLock::new(ChangeDetector::new(
            git_ops,
            config.change_detector.clone(),
        )));

        let (control_tx, control_rx) = mpsc::channel(100);

        Self {
            config,
            change_detector,
            pipeline_controller,
            watched_repos: Arc::new(RwLock::new(HashMap::new())),
            control_tx,
            control_rx: Arc::new(RwLock::new(control_rx)),
            stats: Arc::new(RwLock::new(WatcherStats::default())),
        }
    }

    /// Add a repository to watch
    pub async fn watch_repository(
        &self,
        repo_id: RepositoryId,
        repo_path: PathBuf,
        branches: Vec<String>,
    ) -> Result<()> {
        info!(
            "Adding repository {} to watch list (branches: {:?})",
            repo_id, branches
        );

        let watched_repo = WatchedRepository {
            id: repo_id.clone(),
            path: repo_path,
            branches,
            last_check: None,
            last_trigger: None,
            check_count: 0,
            trigger_count: 0,
            status: WatchStatus::Active,
        };

        let mut repos = self.watched_repos.write().await;
        repos.insert(repo_id, watched_repo);

        Ok(())
    }

    /// Remove a repository from watch list
    pub async fn unwatch_repository(&self, repo_id: &RepositoryId) -> Result<()> {
        info!("Removing repository {} from watch list", repo_id);

        let mut repos = self.watched_repos.write().await;
        repos.remove(repo_id);

        Ok(())
    }

    /// Start watching all repositories
    pub async fn start(&self) -> Result<()> {
        info!("Starting repository watcher");

        let watcher = self.clone_handles();

        tokio::spawn(async move {
            if let Err(e) = watcher.run_watch_loop().await {
                error!("Watcher loop failed: {}", e);
            }
        });

        Ok(())
    }

    /// Stop watching repositories
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping repository watcher");

        self.control_tx
            .send(WatcherCommand::Stop)
            .await
            .map_err(|e| XzeError::pipeline(format!("Failed to send stop command: {}", e)))?;

        Ok(())
    }

    /// Pause watching
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing repository watcher");

        self.control_tx
            .send(WatcherCommand::Pause)
            .await
            .map_err(|e| XzeError::pipeline(format!("Failed to send pause command: {}", e)))?;

        Ok(())
    }

    /// Resume watching
    pub async fn resume(&self) -> Result<()> {
        info!("Resuming repository watcher");

        self.control_tx
            .send(WatcherCommand::Resume)
            .await
            .map_err(|e| XzeError::pipeline(format!("Failed to send resume command: {}", e)))?;

        Ok(())
    }

    /// Trigger immediate check for a repository
    pub async fn trigger_check(&self, repo_id: RepositoryId) -> Result<()> {
        info!("Triggering immediate check for repository {}", repo_id);

        self.control_tx
            .send(WatcherCommand::CheckRepository(repo_id))
            .await
            .map_err(|e| XzeError::pipeline(format!("Failed to send check command: {}", e)))?;

        Ok(())
    }

    /// Get watcher statistics
    pub async fn get_stats(&self) -> WatcherStats {
        self.stats.read().await.clone()
    }

    /// Get status of watched repositories
    pub async fn get_watched_repositories(&self) -> Vec<WatchedRepository> {
        let repos = self.watched_repos.read().await;
        repos.values().cloned().collect()
    }

    // Private methods

    fn clone_handles(&self) -> WatcherHandle {
        WatcherHandle {
            config: self.config.clone(),
            change_detector: Arc::clone(&self.change_detector),
            pipeline_controller: Arc::clone(&self.pipeline_controller),
            watched_repos: Arc::clone(&self.watched_repos),
            control_rx: Arc::clone(&self.control_rx),
            stats: Arc::clone(&self.stats),
        }
    }
}

/// Handle for running the watcher loop
struct WatcherHandle {
    config: WatcherConfig,
    change_detector: Arc<RwLock<ChangeDetector>>,
    pipeline_controller: Arc<PipelineController>,
    watched_repos: Arc<RwLock<HashMap<RepositoryId, WatchedRepository>>>,
    control_rx: Arc<RwLock<mpsc::Receiver<WatcherCommand>>>,
    stats: Arc<RwLock<WatcherStats>>,
}

impl WatcherHandle {
    async fn run_watch_loop(&self) -> Result<()> {
        let mut ticker = interval(std::time::Duration::from_secs(
            self.config.poll_interval_seconds,
        ));
        let mut paused = false;

        info!("Watch loop started");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if !paused {
                        if let Err(e) = self.check_all_repositories().await {
                            error!("Error checking repositories: {}", e);
                        }
                    }
                }
                cmd = self.receive_command() => {
                    match cmd {
                        Some(WatcherCommand::Stop) => {
                            info!("Received stop command");
                            break;
                        }
                        Some(WatcherCommand::Pause) => {
                            info!("Received pause command");
                            paused = true;
                        }
                        Some(WatcherCommand::Resume) => {
                            info!("Received resume command");
                            paused = false;
                        }
                        Some(WatcherCommand::CheckRepository(repo_id)) => {
                            if let Err(e) = self.check_repository(&repo_id).await {
                                error!("Error checking repository {}: {}", repo_id, e);
                            }
                        }
                        None => {
                            warn!("Control channel closed");
                            break;
                        }
                    }
                }
            }
        }

        info!("Watch loop stopped");
        Ok(())
    }

    async fn receive_command(&self) -> Option<WatcherCommand> {
        let mut rx = self.control_rx.write().await;
        rx.recv().await
    }

    async fn check_all_repositories(&self) -> Result<()> {
        debug!("Checking all repositories for changes");

        let repo_ids: Vec<RepositoryId> = {
            let repos = self.watched_repos.read().await;
            repos.keys().cloned().collect()
        };

        for repo_id in repo_ids {
            if let Err(e) = self.check_repository(&repo_id).await {
                error!("Error checking repository {}: {}", repo_id, e);
                self.update_stats(|stats| stats.failed_checks += 1).await;
            }
        }

        Ok(())
    }

    async fn check_repository(&self, repo_id: &RepositoryId) -> Result<()> {
        debug!("Checking repository {} for changes", repo_id);

        let (repo_path, branches) = {
            let repos = self.watched_repos.read().await;
            let repo = repos
                .get(repo_id)
                .ok_or_else(|| XzeError::not_found(format!("Repository {} not found", repo_id)))?;

            (repo.path.clone(), repo.branches.clone())
        };

        for branch in &branches {
            match self.check_branch(repo_id, &repo_path, branch).await {
                Ok(Some(changes)) => {
                    info!(
                        "Changes detected in {} on branch {}, triggering pipeline",
                        repo_id, branch
                    );

                    if self.should_trigger_pipeline(&changes).await {
                        self.trigger_pipeline(repo_id, changes).await?;
                    }
                }
                Ok(None) => {
                    debug!("No changes detected in {} on branch {}", repo_id, branch);
                }
                Err(e) => {
                    warn!(
                        "Error checking branch {} for repository {}: {}",
                        branch, repo_id, e
                    );
                }
            }
        }

        // Update repository metadata
        self.update_repository_metadata(repo_id).await;
        self.update_stats(|stats| stats.total_checks += 1).await;

        Ok(())
    }

    async fn check_branch(
        &self,
        repo_id: &RepositoryId,
        repo_path: &Path,
        branch: &str,
    ) -> Result<Option<RepositoryChanges>> {
        let mut detector = self.change_detector.write().await;
        detector.detect_changes(repo_id, repo_path, branch).await
    }

    async fn should_trigger_pipeline(&self, changes: &RepositoryChanges) -> bool {
        let detector = self.change_detector.read().await;

        // Check if changes are significant enough
        if !detector.should_update_documentation(changes) {
            debug!("Changes not significant enough for documentation update");
            return false;
        }

        // Check cooldown period
        let repos = self.watched_repos.read().await;
        if let Some(repo) = repos.get(&changes.repository_id) {
            if let Some(last_trigger) = repo.last_trigger {
                let cooldown = Duration::seconds(self.config.min_trigger_interval_seconds as i64);
                if Utc::now() - last_trigger < cooldown {
                    debug!("Repository in cooldown period, skipping trigger");
                    return false;
                }
            }
        }

        true
    }

    async fn trigger_pipeline(
        &self,
        repo_id: &RepositoryId,
        changes: RepositoryChanges,
    ) -> Result<()> {
        info!("Triggering pipeline for repository {}", repo_id);

        // Analyze significance and impact
        let detector = self.change_detector.read().await;
        let significance = detector.analyze_significance(&changes);
        let impact = detector.map_documentation_impact(&changes);

        info!(
            "Change significance: {:?}, Impact: {:?}",
            significance.level, impact.update_priority
        );

        // Submit to pipeline controller
        let job_id = self
            .pipeline_controller
            .submit_repository(repo_id.clone())
            .await?;

        info!(
            "Pipeline job {} submitted for repository {}",
            job_id, repo_id
        );

        // Update trigger metadata
        self.update_trigger_metadata(repo_id).await;
        self.update_stats(|stats| stats.total_triggers += 1).await;

        Ok(())
    }

    async fn update_repository_metadata(&self, repo_id: &RepositoryId) {
        let mut repos = self.watched_repos.write().await;
        if let Some(repo) = repos.get_mut(repo_id) {
            repo.last_check = Some(Utc::now());
            repo.check_count += 1;
        }
    }

    async fn update_trigger_metadata(&self, repo_id: &RepositoryId) {
        let mut repos = self.watched_repos.write().await;
        if let Some(repo) = repos.get_mut(repo_id) {
            repo.last_trigger = Some(Utc::now());
            repo.trigger_count += 1;
        }
    }

    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut WatcherStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut stats);
    }
}

/// Watcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Polling interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,

    /// Minimum time between triggers for the same repository (seconds)
    #[serde(default = "default_min_trigger_interval")]
    pub min_trigger_interval_seconds: u64,

    /// Maximum number of repositories to watch
    #[serde(default = "default_max_watched_repos")]
    pub max_watched_repos: usize,

    /// Enable auto-triggering of pipeline
    #[serde(default = "default_true")]
    pub auto_trigger: bool,

    /// Change detector configuration
    #[serde(default)]
    pub change_detector: ChangeDetectorConfig,

    /// Enable retry on failed checks
    #[serde(default = "default_true")]
    pub retry_on_failure: bool,

    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: default_poll_interval(),
            min_trigger_interval_seconds: default_min_trigger_interval(),
            max_watched_repos: default_max_watched_repos(),
            auto_trigger: true,
            change_detector: ChangeDetectorConfig::default(),
            retry_on_failure: true,
            max_retries: default_max_retries(),
        }
    }
}

/// Watched repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedRepository {
    pub id: RepositoryId,
    pub path: PathBuf,
    pub branches: Vec<String>,
    pub last_check: Option<DateTime<Utc>>,
    pub last_trigger: Option<DateTime<Utc>>,
    pub check_count: u64,
    pub trigger_count: u64,
    pub status: WatchStatus,
}

/// Watch status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchStatus {
    Active,
    Paused,
    Error,
    Disabled,
}

/// Watcher command for control
#[derive(Debug, Clone)]
enum WatcherCommand {
    Stop,
    Pause,
    Resume,
    CheckRepository(RepositoryId),
}

/// Watcher statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatcherStats {
    pub total_checks: u64,
    pub total_triggers: u64,
    pub failed_checks: u64,
    pub active_watches: usize,
    pub last_check_time: Option<DateTime<Utc>>,
}

// Default value functions
fn default_poll_interval() -> u64 {
    300 // 5 minutes
}

fn default_min_trigger_interval() -> u64 {
    600 // 10 minutes
}

fn default_max_watched_repos() -> usize {
    100
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::AIAnalysisService,
        config::{ModelConfig, XzeConfig},
        git::CredentialStore,
        pipeline::{PipelineConfig, PipelineExecutor},
        repository::RepositoryManager,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_watcher() -> RepositoryWatcher {
        let temp_dir = TempDir::new().unwrap();
        let git_ops = GitOperations::new(CredentialStore::new());
        let config = WatcherConfig::default();

        let xze_config = XzeConfig::default();
        let pipeline_config = PipelineConfig::default();

        let repo_manager = Arc::new(
            RepositoryManager::new(temp_dir.path().to_path_buf(), xze_config.clone()).unwrap(),
        );

        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));

        let git_ops_arc = Arc::new(GitOperations::new(CredentialStore::new()));

        let executor = Arc::new(PipelineExecutor::new(
            pipeline_config.clone(),
            repo_manager,
            ai_service,
            git_ops_arc,
        ));

        let controller = Arc::new(crate::pipeline::controller::PipelineController::new(
            pipeline_config,
            Arc::new(RepositoryManager::new(temp_dir.path().to_path_buf(), xze_config).unwrap()),
            Arc::new(AIAnalysisService::new(
                "http://localhost:11434".to_string(),
                ModelConfig::default(),
            )),
            Arc::new(GitOperations::new(CredentialStore::new())),
        ));

        RepositoryWatcher::new(config, git_ops, controller)
    }

    #[tokio::test]
    async fn test_watcher_creation() {
        let watcher = create_test_watcher().await;
        let stats = watcher.get_stats().await;
        assert_eq!(stats.total_checks, 0);
        assert_eq!(stats.total_triggers, 0);
    }

    #[tokio::test]
    async fn test_watch_repository() {
        let watcher = create_test_watcher().await;
        let repo_id = RepositoryId::from("test-repo");
        let temp_dir = TempDir::new().unwrap();

        let result = watcher
            .watch_repository(
                repo_id.clone(),
                temp_dir.path().to_path_buf(),
                vec!["main".to_string()],
            )
            .await;

        assert!(result.is_ok());

        let repos = watcher.get_watched_repositories().await;
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, repo_id);
    }

    #[tokio::test]
    async fn test_unwatch_repository() {
        let watcher = create_test_watcher().await;
        let repo_id = RepositoryId::from("test-repo");
        let temp_dir = TempDir::new().unwrap();

        watcher
            .watch_repository(
                repo_id.clone(),
                temp_dir.path().to_path_buf(),
                vec!["main".to_string()],
            )
            .await
            .unwrap();

        let result = watcher.unwatch_repository(&repo_id).await;
        assert!(result.is_ok());

        let repos = watcher.get_watched_repositories().await;
        assert_eq!(repos.len(), 0);
    }

    #[tokio::test]
    async fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert_eq!(config.poll_interval_seconds, 300);
        assert_eq!(config.min_trigger_interval_seconds, 600);
        assert!(config.auto_trigger);
    }

    #[tokio::test]
    async fn test_watch_status() {
        let repo = WatchedRepository {
            id: RepositoryId::from("test"),
            path: PathBuf::from("/tmp/test"),
            branches: vec!["main".to_string()],
            last_check: None,
            last_trigger: None,
            check_count: 0,
            trigger_count: 0,
            status: WatchStatus::Active,
        };

        assert_eq!(repo.status, WatchStatus::Active);
    }

    #[tokio::test]
    async fn test_watcher_stats() {
        let stats = WatcherStats {
            total_checks: 10,
            total_triggers: 3,
            failed_checks: 1,
            active_watches: 5,
            last_check_time: Some(Utc::now()),
        };

        assert_eq!(stats.total_checks, 10);
        assert_eq!(stats.total_triggers, 3);
        assert_eq!(stats.failed_checks, 1);
    }
}
