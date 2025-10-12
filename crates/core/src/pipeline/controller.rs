//! Pipeline controller for orchestrating documentation workflows

use crate::{
    ai::AIAnalysisService,
    error::Result,
    git::GitOperations,
    pipeline::{PipelineConfig, PipelineExecutor, PipelineResult, PipelineStats},
    repository::RepositoryManager,
    types::{JobId, JobStatus, RepositoryId},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Pipeline controller for managing documentation generation workflows
pub struct PipelineController {
    config: PipelineConfig,
    executor: Arc<PipelineExecutor>,
    running_jobs: Arc<RwLock<HashMap<JobId, tokio::task::JoinHandle<()>>>>,
    semaphore: Arc<Semaphore>,
}

impl PipelineController {
    /// Create a new pipeline controller
    pub fn new(
        config: PipelineConfig,
        repo_manager: Arc<RepositoryManager>,
        ai_service: Arc<AIAnalysisService>,
        git_ops: Arc<GitOperations>,
    ) -> Self {
        let executor = Arc::new(PipelineExecutor::new(
            config.clone(),
            repo_manager,
            ai_service,
            git_ops,
        ));

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));

        Self {
            config,
            executor,
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
            semaphore,
        }
    }

    /// Submit a repository for processing
    pub async fn submit_repository(&self, repo_id: RepositoryId) -> Result<JobId> {
        info!("Submitting repository {} for processing", repo_id);

        let job_id = JobId::new();
        let executor = Arc::clone(&self.executor);
        let semaphore = Arc::clone(&self.semaphore);
        let running_jobs = Arc::clone(&self.running_jobs);
        let job_id_for_task = job_id.clone();

        // Spawn the job
        let handle = tokio::spawn(async move {
            // Acquire semaphore permit to limit concurrency
            let _permit = semaphore.acquire().await.expect("Semaphore closed");

            match executor.execute_for_repository(&repo_id).await {
                Ok(result) => {
                    info!(
                        "Pipeline completed successfully for repository {}: {:?}",
                        repo_id, result.status
                    );
                }
                Err(e) => {
                    error!("Pipeline failed for repository {}: {}", repo_id, e);
                }
            }

            // Remove from running jobs when complete
            let mut jobs = running_jobs.write().await;
            jobs.remove(&job_id_for_task);
        });

        // Store the job handle
        {
            let mut jobs = self.running_jobs.write().await;
            jobs.insert(job_id.clone(), handle);
        }

        Ok(job_id)
    }

    /// Submit multiple repositories for processing
    pub async fn submit_repositories(&self, repo_ids: Vec<RepositoryId>) -> Result<Vec<JobId>> {
        let mut job_ids = Vec::new();

        for repo_id in repo_ids {
            match self.submit_repository(repo_id.clone()).await {
                Ok(job_id) => job_ids.push(job_id),
                Err(e) => {
                    warn!("Failed to submit repository {}: {}", repo_id, e);
                }
            }
        }

        info!("Submitted {} repositories for processing", job_ids.len());
        Ok(job_ids)
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        if let Some(job) = self.executor.get_job(job_id).await {
            Some(job.status)
        } else {
            None
        }
    }

    /// Get job result
    pub async fn get_job_result(&self, job_id: &JobId) -> Option<PipelineResult> {
        // Check if job is still running
        {
            let running_jobs = self.running_jobs.read().await;
            if running_jobs.contains_key(job_id) {
                return None; // Job is still running
            }
        }

        // Try to get completed job result
        if let Some(job) = self.executor.get_job(job_id).await {
            // Convert job to result (simplified)
            Some(PipelineResult {
                job_id: job_id.clone(),
                repository_id: job.source_repo.clone(),
                status: job.status,
                duration_seconds: 0, // Would need to calculate from timestamps
                generated_files: Vec::new(), // Would need to store this in job
                errors: Vec::new(),
                warnings: Vec::new(),
                pull_request_url: None,
            })
        } else {
            None
        }
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<JobSummary> {
        let jobs = self.executor.list_jobs().await;
        let running_jobs = self.running_jobs.read().await;

        jobs.into_iter()
            .map(|job| JobSummary {
                job_id: job.id.clone(),
                repository_id: job.source_repo,
                status: job.status,
                created_at: job.metadata.created_at,
                started_at: job.metadata.started_at,
                completed_at: job.metadata.completed_at,
                is_running: running_jobs.contains_key(&job.id),
            })
            .collect()
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: &JobId) -> Result<()> {
        info!("Attempting to cancel job: {}", job_id);

        // First, try to cancel in the executor
        self.executor.cancel_job(job_id).await?;

        // Then abort the task if it's running
        let mut running_jobs = self.running_jobs.write().await;
        if let Some(handle) = running_jobs.remove(job_id) {
            handle.abort();
            info!("Cancelled job task: {}", job_id);
        }

        Ok(())
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        let mut stats = self.executor.get_stats().await;

        // Update running jobs count from actual running tasks
        let running_jobs = self.running_jobs.read().await;
        stats.running_jobs = running_jobs.len();

        stats
    }

    /// Wait for all jobs to complete
    pub async fn wait_for_all_jobs(&self) -> Result<()> {
        info!("Waiting for all jobs to complete");

        loop {
            let running_jobs = {
                let jobs = self.running_jobs.read().await;
                jobs.len()
            };

            if running_jobs == 0 {
                break;
            }

            debug!("Waiting for {} jobs to complete", running_jobs);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        info!("All jobs completed");
        Ok(())
    }

    /// Shutdown the controller and cancel all running jobs
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down pipeline controller");

        let mut running_jobs = self.running_jobs.write().await;

        for (job_id, handle) in running_jobs.drain() {
            debug!("Aborting job: {}", job_id);
            handle.abort();
        }

        info!("Pipeline controller shutdown complete");
        Ok(())
    }

    /// Clean up old completed jobs
    pub async fn cleanup_old_jobs(&self, max_age_hours: u64) -> usize {
        self.executor.cleanup_old_jobs(max_age_hours).await
    }

    /// Get controller configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }
}

/// Job summary for listing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub job_id: JobId,
    pub repository_id: RepositoryId,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_running: bool,
}

impl JobSummary {
    /// Get job duration if completed
    pub fn duration(&self) -> Option<chrono::Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            Some(completed - started)
        } else {
            None
        }
    }

    /// Check if job is finished (completed, failed, or cancelled)
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed(_) | JobStatus::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::XzeConfig, git::CredentialStore};
    use tempfile::TempDir;

    async fn create_test_controller() -> PipelineController {
        let temp_dir = TempDir::new().unwrap();
        let xze_config = XzeConfig::default();
        let pipeline_config = PipelineConfig::default();

        let repo_manager = Arc::new(
            RepositoryManager::new(temp_dir.path().to_path_buf(), xze_config.clone()).unwrap(),
        );

        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));

        let git_ops = Arc::new(GitOperations::new(CredentialStore::new()));

        PipelineController::new(pipeline_config, repo_manager, ai_service, git_ops)
    }

    #[tokio::test]
    async fn test_controller_creation() {
        let controller = create_test_controller().await;
        let stats = controller.get_stats().await;
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.running_jobs, 0);
    }

    #[tokio::test]
    async fn test_job_summary() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let now = chrono::Utc::now();

        let summary = JobSummary {
            job_id: job_id.clone(),
            repository_id: repo_id,
            status: JobStatus::Completed,
            created_at: now,
            started_at: Some(now),
            completed_at: Some(now + chrono::Duration::seconds(30)),
            is_running: false,
        };

        assert!(summary.is_finished());
        assert!(!summary.is_running);
        assert!(summary.duration().is_some());
        assert_eq!(summary.duration().unwrap().num_seconds(), 30);
    }

    #[tokio::test]
    async fn test_controller_config() {
        let controller = create_test_controller().await;
        let config = controller.config();
        assert_eq!(config.max_concurrent_jobs, 4);
    }

    #[tokio::test]
    async fn test_controller_shutdown() {
        let controller = create_test_controller().await;
        assert!(controller.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_empty_job_list() {
        let controller = create_test_controller().await;
        let jobs = controller.list_jobs().await;
        assert!(jobs.is_empty());
    }
}
