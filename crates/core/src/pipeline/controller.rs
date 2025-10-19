//! Pipeline controller for orchestrating documentation workflows
//!
//! This module provides a comprehensive pipeline controller that manages:
//! - Job submission and execution
//! - Timeout handling
//! - Error recovery and retry logic
//! - Rollback capabilities
//! - Progress tracking with estimated time remaining
//! - Concurrency control

use crate::{
    ai::AIAnalysisService,
    error::{Result, XzeError},
    git::GitOperations,
    pipeline::{
        job::{JobConfig, PipelineJob},
        scheduler::{JobCompletionResult, JobScheduler, SchedulerConfig},
        PipelineConfig, PipelineExecutor, PipelineResult, PipelineStats,
    },
    repository::RepositoryManager,
    types::{JobId, JobStatus, RepositoryId},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration as StdDuration};
use tokio::{
    sync::RwLock,
    time::{sleep, timeout},
};
use tracing::{debug, error, info, warn};

/// Pipeline controller for managing documentation generation workflows
pub struct PipelineController {
    config: PipelineConfig,
    executor: Arc<PipelineExecutor>,
    scheduler: Arc<JobScheduler>,
    job_trackers: Arc<RwLock<HashMap<JobId, JobTracker>>>,
    retry_manager: Arc<RetryManager>,
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

        let scheduler_config = SchedulerConfig {
            max_concurrent_jobs: config.max_concurrent_jobs,
            default_job_timeout: config.job_timeout_seconds,
            ..Default::default()
        };

        let scheduler = Arc::new(JobScheduler::new(scheduler_config));
        let retry_manager = Arc::new(RetryManager::new(RetryConfig::default()));

        Self {
            config,
            executor,
            scheduler,
            job_trackers: Arc::new(RwLock::new(HashMap::new())),
            retry_manager,
        }
    }

    /// Submit a repository for processing with timeout and retry support
    pub async fn submit_repository(&self, repo_id: RepositoryId) -> Result<JobId> {
        self.submit_repository_with_config(repo_id, JobConfig::default())
            .await
    }

    /// Submit a repository with custom job configuration
    pub async fn submit_repository_with_config(
        &self,
        repo_id: RepositoryId,
        config: JobConfig,
    ) -> Result<JobId> {
        info!("Submitting repository {} for processing", repo_id);

        // Check if we can accept more jobs
        if !self.scheduler.can_accept_jobs().await {
            return Err(XzeError::pipeline("Job queue is full"));
        }

        let job_id = JobId::new();
        let mut job = PipelineJob::with_target(job_id.clone(), repo_id.clone(), repo_id.clone());
        job.config = config;

        // Create job tracker for progress monitoring
        let tracker = JobTracker::new(job_id.clone(), repo_id.clone());
        {
            let mut trackers = self.job_trackers.write().await;
            trackers.insert(job_id.clone(), tracker);
        }

        // Submit to scheduler
        self.scheduler.submit_job(job.clone()).await?;

        // Spawn execution task
        let controller = self.clone_for_task();
        let job_id_clone = job_id.clone();

        tokio::spawn(async move {
            if let Err(e) = controller.execute_job_with_monitoring(job_id_clone).await {
                error!("Job execution failed: {}", e);
            }
        });

        Ok(job_id)
    }

    /// Execute a job with full monitoring, timeout, and retry logic
    async fn execute_job_with_monitoring(&self, job_id: JobId) -> Result<()> {
        // Get job from scheduler
        let job = self.scheduler.next_job().await;
        let Some(job) = job else {
            return Err(XzeError::not_found("Job not found in queue"));
        };

        if job.id != job_id {
            // Put it back and wait
            self.scheduler.submit_job(job).await?;
            return Ok(());
        }

        // Start the job
        self.scheduler.start_job(job.clone()).await?;
        self.update_tracker_status(&job_id, JobStatus::Running)
            .await;

        let timeout_duration = job
            .config
            .timeout_seconds
            .map(StdDuration::from_secs)
            .unwrap_or(StdDuration::from_secs(3600));

        // Execute with timeout
        let result = timeout(timeout_duration, self.execute_job_with_retry(&job_id, &job)).await;

        let completion_result = match result {
            Ok(Ok(pipeline_result)) => {
                info!("Job {} completed successfully", job_id);
                self.update_tracker_result(&job_id, &pipeline_result).await;
                JobCompletionResult::Success
            }
            Ok(Err(e)) => {
                error!("Job {} failed: {}", job_id, e);
                self.update_tracker_error(&job_id, &e.to_string()).await;
                JobCompletionResult::Failed(e.to_string())
            }
            Err(_) => {
                let error_msg = format!("Job {} timed out after {:?}", job_id, timeout_duration);
                error!("{}", error_msg);
                self.update_tracker_error(&job_id, &error_msg).await;

                // Attempt rollback on timeout
                if let Err(e) = self.rollback_job(&job).await {
                    warn!("Rollback failed for job {}: {}", job_id, e);
                }

                JobCompletionResult::Failed(error_msg)
            }
        };

        // Complete the job in scheduler
        self.scheduler
            .complete_job(&job_id, completion_result)
            .await?;

        Ok(())
    }

    /// Execute job with retry logic
    async fn execute_job_with_retry(
        &self,
        job_id: &JobId,
        job: &PipelineJob,
    ) -> Result<PipelineResult> {
        let max_retries = job.config.max_retries;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                info!(
                    "Retrying job {} (attempt {}/{})",
                    job_id, attempt, max_retries
                );

                // Calculate backoff delay
                let backoff = self.retry_manager.get_backoff_delay(attempt);
                self.update_tracker_retry(job_id, attempt, backoff).await;

                sleep(backoff).await;
            }

            // Update progress
            self.update_tracker_progress(job_id, 0.0, Some("Starting execution"))
                .await;

            // Execute the pipeline
            match self.executor.execute_for_repository(&job.source_repo).await {
                Ok(result) => {
                    self.update_tracker_progress(job_id, 100.0, Some("Completed"))
                        .await;
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Job {} failed on attempt {}: {}", job_id, attempt + 1, e);

                    // Check if error is retryable
                    let is_retryable = self.retry_manager.is_retryable(&e);
                    last_error = Some(e);

                    if !is_retryable {
                        info!("Error is not retryable, stopping attempts");
                        break;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| XzeError::pipeline("Unknown error")))
    }

    /// Rollback a failed job
    async fn rollback_job(&self, job: &PipelineJob) -> Result<()> {
        info!("Attempting rollback for job {}", job.id);

        // Rollback steps:
        // 1. Remove any partially generated files
        // 2. Revert any git changes if applicable
        // 3. Clean up temporary resources

        // TODO: Implement actual rollback logic based on what was done
        // For now, this is a placeholder that logs the intent

        debug!(
            "Rollback completed for job {} on repository {}",
            job.id, job.source_repo
        );

        Ok(())
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

    /// Get job status with detailed information
    pub async fn get_job_status(&self, job_id: &JobId) -> Option<JobStatusDetail> {
        // Check scheduler first
        let status = self.scheduler.get_job_status(job_id).await?;

        // Get tracker information
        let tracker = {
            let trackers = self.job_trackers.read().await;
            trackers.get(job_id).cloned()
        };

        tracker.map(|t| JobStatusDetail {
            job_id: job_id.clone(),
            status,
            progress: t.progress,
            current_step: t.current_step.clone(),
            estimated_completion: t.estimate_completion_time(),
            retry_count: t.retry_count,
            last_error: t.last_error.clone(),
        })
    }

    /// Get job result
    pub async fn get_job_result(&self, job_id: &JobId) -> Option<PipelineResult> {
        let trackers = self.job_trackers.read().await;
        trackers.get(job_id).and_then(|t| t.result.clone())
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<JobSummary> {
        let mut summaries = Vec::new();

        // Add running jobs
        for job_id in self.scheduler.list_running_jobs().await {
            if let Some(detail) = self.get_job_status(&job_id).await {
                summaries.push(JobSummary {
                    job_id: detail.job_id,
                    repository_id: self
                        .get_repo_id_for_job(&job_id)
                        .await
                        .unwrap_or_else(|| RepositoryId::from("unknown")),
                    status: detail.status,
                    progress: detail.progress,
                    created_at: Utc::now(), // Would need to store this
                    started_at: Some(Utc::now()),
                    completed_at: None,
                    is_running: true,
                });
            }
        }

        // Add completed jobs
        for completed in self.scheduler.list_completed_jobs(Some(50)).await {
            summaries.push(JobSummary {
                job_id: completed.job.id.clone(),
                repository_id: completed.job.source_repo.clone(),
                status: completed.job.status.clone(),
                progress: completed.job.progress(),
                created_at: completed.job.metadata.created_at,
                started_at: completed.job.metadata.started_at,
                completed_at: Some(completed.completed_at),
                is_running: false,
            });
        }

        summaries
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: &JobId) -> Result<()> {
        info!("Attempting to cancel job: {}", job_id);

        // Cancel in scheduler
        self.scheduler.cancel_job(job_id).await?;

        // Update tracker
        self.update_tracker_status(job_id, JobStatus::Cancelled)
            .await;

        Ok(())
    }

    /// Get pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        let scheduler_stats = self.scheduler.get_stats().await;

        PipelineStats {
            total_jobs: scheduler_stats.total_submitted as usize,
            successful_jobs: scheduler_stats.completed as usize,
            failed_jobs: scheduler_stats.failed as usize,
            running_jobs: scheduler_stats.running as usize,
            queued_jobs: scheduler_stats.queued as usize,
            avg_execution_time: scheduler_stats.avg_execution_time.as_secs() as f64,
            total_execution_time: scheduler_stats.total_execution_time.as_secs(),
        }
    }

    /// Wait for all jobs to complete
    pub async fn wait_for_all_jobs(&self) -> Result<()> {
        info!("Waiting for all jobs to complete");

        loop {
            let running_count = self.scheduler.running_count().await;
            let queue_size = self.scheduler.queue_size().await;

            if running_count == 0 && queue_size == 0 {
                break;
            }

            debug!(
                "Waiting for {} running jobs and {} queued jobs",
                running_count, queue_size
            );
            sleep(StdDuration::from_secs(1)).await;
        }

        info!("All jobs completed");
        Ok(())
    }

    /// Shutdown the controller and cancel all running jobs
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down pipeline controller");

        // Shutdown scheduler (cancels all jobs)
        self.scheduler.shutdown().await?;

        // Clear trackers
        {
            let mut trackers = self.job_trackers.write().await;
            trackers.clear();
        }

        info!("Pipeline controller shutdown complete");
        Ok(())
    }

    /// Clean up old completed jobs
    pub async fn cleanup_old_jobs(&self, max_age_hours: u64) -> usize {
        let max_age = StdDuration::from_secs(max_age_hours * 3600);
        self.scheduler.cleanup_completed_jobs(max_age).await
    }

    /// Get controller configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get available execution slots
    pub fn available_slots(&self) -> usize {
        self.scheduler.available_slots()
    }

    // Internal helper methods

    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            executor: Arc::clone(&self.executor),
            scheduler: Arc::clone(&self.scheduler),
            job_trackers: Arc::clone(&self.job_trackers),
            retry_manager: Arc::clone(&self.retry_manager),
        }
    }

    async fn get_repo_id_for_job(&self, job_id: &JobId) -> Option<RepositoryId> {
        let trackers = self.job_trackers.read().await;
        trackers.get(job_id).map(|t| t.repository_id.clone())
    }

    async fn update_tracker_status(&self, job_id: &JobId, status: JobStatus) {
        let mut trackers = self.job_trackers.write().await;
        if let Some(tracker) = trackers.get_mut(job_id) {
            tracker.status = status;
        }
    }

    async fn update_tracker_progress(&self, job_id: &JobId, progress: f32, step: Option<&str>) {
        let mut trackers = self.job_trackers.write().await;
        if let Some(tracker) = trackers.get_mut(job_id) {
            tracker.progress = progress;
            if let Some(step) = step {
                tracker.current_step = Some(step.to_string());
            }
            tracker.last_update = Utc::now();
        }
    }

    async fn update_tracker_result(&self, job_id: &JobId, result: &PipelineResult) {
        let mut trackers = self.job_trackers.write().await;
        if let Some(tracker) = trackers.get_mut(job_id) {
            tracker.result = Some(result.clone());
            tracker.status = result.status.clone();
        }
    }

    async fn update_tracker_error(&self, job_id: &JobId, error: &str) {
        let mut trackers = self.job_trackers.write().await;
        if let Some(tracker) = trackers.get_mut(job_id) {
            tracker.last_error = Some(error.to_string());
        }
    }

    async fn update_tracker_retry(&self, job_id: &JobId, retry_count: u32, backoff: StdDuration) {
        let mut trackers = self.job_trackers.write().await;
        if let Some(tracker) = trackers.get_mut(job_id) {
            tracker.retry_count = retry_count;
            tracker.current_step = Some(format!("Retrying after {:?}", backoff));
        }
    }
}

/// Job tracker for monitoring job progress and state
#[derive(Debug, Clone)]
struct JobTracker {
    #[allow(dead_code)]
    job_id: JobId,
    repository_id: RepositoryId,
    status: JobStatus,
    progress: f32,
    current_step: Option<String>,
    started_at: DateTime<Utc>,
    last_update: DateTime<Utc>,
    retry_count: u32,
    last_error: Option<String>,
    result: Option<PipelineResult>,
}

impl JobTracker {
    fn new(job_id: JobId, repository_id: RepositoryId) -> Self {
        let now = Utc::now();
        Self {
            job_id,
            repository_id,
            status: JobStatus::Queued,
            progress: 0.0,
            current_step: None,
            started_at: now,
            last_update: now,
            retry_count: 0,
            last_error: None,
            result: None,
        }
    }

    /// Estimate completion time based on progress
    fn estimate_completion_time(&self) -> Option<DateTime<Utc>> {
        if self.progress <= 0.0 {
            return None;
        }

        let elapsed = Utc::now() - self.started_at;
        let total_estimated = Duration::milliseconds(
            (elapsed.num_milliseconds() as f32 / self.progress * 100.0) as i64,
        );

        Some(self.started_at + total_estimated)
    }
}

/// Retry manager for handling job retry logic
struct RetryManager {
    config: RetryConfig,
}

impl RetryManager {
    fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Check if an error is retryable
    fn is_retryable(&self, error: &XzeError) -> bool {
        // Network errors, timeouts, and temporary failures are retryable
        match error {
            XzeError::Network { .. } => true,
            XzeError::Io(_) => true,
            XzeError::Git(_) => true,
            XzeError::AiService { .. } => true,
            XzeError::Timeout { .. } => true,
            XzeError::Http(_) => true,
            _ => false, // Configuration and validation errors are not retryable
        }
    }

    /// Get backoff delay for retry attempt
    fn get_backoff_delay(&self, attempt: u32) -> StdDuration {
        let base_delay = self.config.initial_backoff_ms;
        let max_delay = self.config.max_backoff_ms;

        // Exponential backoff with jitter
        let delay = (base_delay as f64 * self.config.backoff_multiplier.powi(attempt as i32))
            .min(max_delay as f64);

        // Add jitter (random 0-20%)
        let jitter = delay * 0.2 * rand::random::<f64>();
        StdDuration::from_millis((delay + jitter) as u64)
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
struct RetryConfig {
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_backoff_ms: 1000, // 1 second
            max_backoff_ms: 60000,    // 1 minute
            backoff_multiplier: 2.0,  // Double each time
        }
    }
}

/// Detailed job status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusDetail {
    pub job_id: JobId,
    pub status: JobStatus,
    pub progress: f32,
    pub current_step: Option<String>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

/// Job summary for listing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub job_id: JobId,
    pub repository_id: RepositoryId,
    pub status: JobStatus,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_running: bool,
}

impl JobSummary {
    /// Get job duration if completed
    pub fn duration(&self) -> Option<Duration> {
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
            crate::config::ModelConfig::default(),
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
        let now = Utc::now();

        let summary = JobSummary {
            job_id: job_id.clone(),
            repository_id: repo_id,
            status: JobStatus::Completed,
            progress: 100.0,
            created_at: now,
            started_at: Some(now),
            completed_at: Some(now + Duration::seconds(30)),
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

    #[tokio::test]
    async fn test_available_slots() {
        let controller = create_test_controller().await;
        assert_eq!(controller.available_slots(), 4);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.initial_backoff_ms, 1000);
        assert_eq!(config.max_backoff_ms, 60000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_job_tracker_creation() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let tracker = JobTracker::new(job_id.clone(), repo_id.clone());

        assert_eq!(tracker.job_id, job_id);
        assert_eq!(tracker.repository_id, repo_id);
        assert_eq!(tracker.progress, 0.0);
        assert_eq!(tracker.retry_count, 0);
    }
}
