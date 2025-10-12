//! Pipeline job scheduler for managing job execution order and concurrency

use crate::{
    error::{Result, XzeError},
    pipeline::job::{JobQueueEntry, PipelineJob},
    types::{JobId, JobStatus},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BinaryHeap, HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, info, warn};

/// Job scheduler for managing pipeline job execution
pub struct JobScheduler {
    /// Scheduler configuration
    config: SchedulerConfig,
    /// Job queue (priority queue)
    queue: Arc<Mutex<BinaryHeap<JobQueueEntry>>>,
    /// Running jobs
    running_jobs: Arc<RwLock<HashMap<JobId, RunningJob>>>,
    /// Completed jobs (for history)
    completed_jobs: Arc<RwLock<VecDeque<CompletedJob>>>,
    /// Semaphore for controlling concurrency
    semaphore: Arc<Semaphore>,
    /// Scheduler statistics
    stats: Arc<RwLock<SchedulerStats>>,
}

impl JobScheduler {
    /// Create a new job scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));

        Self {
            config,
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
            completed_jobs: Arc::new(RwLock::new(VecDeque::new())),
            semaphore,
            stats: Arc::new(RwLock::new(SchedulerStats::default())),
        }
    }

    /// Submit a job to the scheduler
    pub async fn submit_job(&self, job: PipelineJob) -> Result<()> {
        let job_id = job.id.clone();
        info!("Submitting job {} to scheduler", job_id);

        // Create queue entry
        let entry = JobQueueEntry::new(job);

        // Add to queue
        {
            let mut queue = self.queue.lock().await;
            queue.push(entry);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_submitted += 1;
            stats.queued += 1;
        }

        debug!("Job {} added to queue", job_id);
        Ok(())
    }

    /// Submit multiple jobs
    pub async fn submit_jobs(&self, jobs: Vec<PipelineJob>) -> Result<Vec<JobId>> {
        let mut job_ids = Vec::new();

        for job in jobs {
            let job_id = job.id.clone();
            self.submit_job(job).await?;
            job_ids.push(job_id);
        }

        info!("Submitted {} jobs to scheduler", job_ids.len());
        Ok(job_ids)
    }

    /// Get the next job from the queue
    pub async fn next_job(&self) -> Option<PipelineJob> {
        let mut queue = self.queue.lock().await;
        queue.pop().map(|entry| entry.job)
    }

    /// Start a job (move from queue to running)
    pub async fn start_job(&self, mut job: PipelineJob) -> Result<()> {
        let job_id = job.id.clone();

        // Acquire semaphore permit
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| XzeError::pipeline("Failed to acquire execution permit"))?;

        // Update job status
        job.start();

        // Create running job entry
        let running_job = RunningJob {
            job: job.clone(),
            started_at: Instant::now(),
            _permit: permit,
        };

        // Add to running jobs
        {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.insert(job_id.clone(), running_job);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.queued = stats.queued.saturating_sub(1);
            stats.running += 1;
        }

        info!("Started job {}", job_id);
        Ok(())
    }

    /// Complete a job (move from running to completed)
    pub async fn complete_job(&self, job_id: &JobId, result: JobCompletionResult) -> Result<()> {
        let running_job = {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.remove(job_id)
        };

        let Some(running_job) = running_job else {
            return Err(XzeError::not_found(format!(
                "Running job {} not found",
                job_id
            )));
        };

        let mut job = running_job.job;
        let execution_time = running_job.started_at.elapsed();

        // Update job based on result
        match result {
            JobCompletionResult::Success => {
                job.complete();
            }
            JobCompletionResult::Failed(ref error) => {
                job.fail(error.to_string());
            }
            JobCompletionResult::Cancelled => {
                job.cancel();
            }
        }

        // Create completed job entry
        let completed_job = CompletedJob {
            job,
            execution_time_ms: execution_time.as_millis() as u64,
            completed_at: Utc::now(),
        };

        // Add to completed jobs (with size limit)
        {
            let mut completed_jobs = self.completed_jobs.write().await;
            completed_jobs.push_back(completed_job);

            // Maintain size limit
            while completed_jobs.len() > self.config.max_completed_history {
                completed_jobs.pop_front();
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.running = stats.running.saturating_sub(1);
            match result {
                JobCompletionResult::Success => stats.completed += 1,
                JobCompletionResult::Failed(_) => stats.failed += 1,
                JobCompletionResult::Cancelled => stats.cancelled += 1,
            }
            stats.total_execution_time += execution_time;

            // Update average execution time
            let total_finished = stats.completed + stats.failed + stats.cancelled;
            if total_finished > 0 {
                stats.avg_execution_time = stats.total_execution_time / total_finished as u32;
            }
        }

        info!("Completed job {} in {:?}", job_id, execution_time);
        Ok(())
    }

    /// Cancel a job
    pub async fn cancel_job(&self, job_id: &JobId) -> Result<()> {
        // Check if job is running
        if self.running_jobs.read().await.contains_key(job_id) {
            self.complete_job(job_id, JobCompletionResult::Cancelled)
                .await?;
            return Ok(());
        }

        // Check if job is in queue
        {
            let mut queue = self.queue.lock().await;
            let mut remaining_jobs = Vec::new();
            let mut found = false;

            while let Some(entry) = queue.pop() {
                if entry.job.id == *job_id {
                    found = true;
                    info!("Cancelled queued job {}", job_id);
                    break;
                } else {
                    remaining_jobs.push(entry);
                }
            }

            // Put back remaining jobs
            for entry in remaining_jobs {
                queue.push(entry);
            }

            if found {
                let mut stats = self.stats.write().await;
                stats.queued = stats.queued.saturating_sub(1);
                stats.cancelled += 1;
                return Ok(());
            }
        }

        Err(XzeError::not_found(format!("Job {} not found", job_id)))
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        // Check running jobs
        if let Some(running_job) = self.running_jobs.read().await.get(job_id) {
            return Some(running_job.job.status.clone());
        }

        // Check completed jobs
        let completed_jobs = self.completed_jobs.read().await;
        for completed_job in completed_jobs.iter() {
            if completed_job.job.id == *job_id {
                return Some(completed_job.job.status.clone());
            }
        }

        // Check queue
        let queue = self.queue.lock().await;
        for entry in queue.iter() {
            if entry.job.id == *job_id {
                return Some(entry.job.status.clone());
            }
        }

        None
    }

    /// Get scheduler statistics
    pub async fn get_stats(&self) -> SchedulerStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Get running jobs count
    pub async fn running_count(&self) -> usize {
        let running_jobs = self.running_jobs.read().await;
        running_jobs.len()
    }

    /// List all running jobs
    pub async fn list_running_jobs(&self) -> Vec<JobId> {
        let running_jobs = self.running_jobs.read().await;
        running_jobs.keys().cloned().collect()
    }

    /// List recently completed jobs
    pub async fn list_completed_jobs(&self, limit: Option<usize>) -> Vec<CompletedJob> {
        let completed_jobs = self.completed_jobs.read().await;
        let limit = limit.unwrap_or(completed_jobs.len());

        completed_jobs
            .iter()
            .rev() // Most recent first
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clean up old completed jobs
    pub async fn cleanup_completed_jobs(&self, max_age: Duration) -> usize {
        let mut completed_jobs = self.completed_jobs.write().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        let initial_len = completed_jobs.len();

        completed_jobs.retain(|job| job.completed_at > cutoff);

        let removed = initial_len - completed_jobs.len();
        if removed > 0 {
            info!("Cleaned up {} old completed jobs", removed);
        }
        removed
    }

    /// Shutdown the scheduler
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down job scheduler");

        // Cancel all queued jobs
        let mut queue = self.queue.lock().await;
        let queued_count = queue.len();
        queue.clear();

        // Wait for running jobs to complete or cancel them
        let running_jobs: Vec<JobId> = {
            let running_jobs = self.running_jobs.read().await;
            running_jobs.keys().cloned().collect()
        };

        for job_id in running_jobs {
            if let Err(e) = self.cancel_job(&job_id).await {
                warn!("Failed to cancel job {} during shutdown: {}", job_id, e);
            }
        }

        info!(
            "Scheduler shutdown complete. Cancelled {} queued jobs",
            queued_count
        );
        Ok(())
    }

    /// Get available execution slots
    pub fn available_slots(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Check if scheduler can accept more jobs
    pub async fn can_accept_jobs(&self) -> bool {
        let queue_size = self.queue_size().await;
        queue_size < self.config.max_queue_size
    }
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum number of concurrent jobs
    pub max_concurrent_jobs: usize,
    /// Maximum number of jobs in queue
    pub max_queue_size: usize,
    /// Maximum number of completed jobs to keep in history
    pub max_completed_history: usize,
    /// Job timeout in seconds
    pub default_job_timeout: u64,
    /// Cleanup interval for old jobs in seconds
    pub cleanup_interval_seconds: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 4,
            max_queue_size: 1000,
            max_completed_history: 100,
            default_job_timeout: 3600,      // 1 hour
            cleanup_interval_seconds: 3600, // 1 hour
        }
    }
}

/// Running job information
#[derive(Debug)]
struct RunningJob {
    job: PipelineJob,
    started_at: Instant,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

/// Completed job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedJob {
    pub job: PipelineJob,
    pub execution_time_ms: u64,
    pub completed_at: DateTime<Utc>,
}

/// Job completion result
#[derive(Debug, Clone)]
pub enum JobCompletionResult {
    Success,
    Failed(String),
    Cancelled,
}

/// Scheduler statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerStats {
    /// Total jobs submitted
    pub total_submitted: u64,
    /// Jobs currently queued
    pub queued: u64,
    /// Jobs currently running
    pub running: u64,
    /// Jobs completed successfully
    pub completed: u64,
    /// Jobs that failed
    pub failed: u64,
    /// Jobs that were cancelled
    pub cancelled: u64,
    /// Total execution time across all jobs
    pub total_execution_time: Duration,
    /// Average execution time per job
    pub avg_execution_time: Duration,
}

impl SchedulerStats {
    /// Get total finished jobs
    pub fn total_finished(&self) -> u64 {
        self.completed + self.failed + self.cancelled
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let finished = self.total_finished();
        if finished == 0 {
            0.0
        } else {
            (self.completed as f64 / finished as f64) * 100.0
        }
    }

    /// Get throughput (jobs per hour)
    pub fn throughput_per_hour(&self) -> f64 {
        let finished = self.total_finished();
        if finished == 0 || self.total_execution_time.is_zero() {
            0.0
        } else {
            let hours = self.total_execution_time.as_secs_f64() / 3600.0;
            finished as f64 / hours
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RepositoryId;

    fn create_test_job(repo_name: &str) -> PipelineJob {
        PipelineJob::new(JobId::new(), RepositoryId::from(repo_name))
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.total_submitted, 0);
        assert_eq!(stats.queued, 0);
        assert_eq!(stats.running, 0);
    }

    #[tokio::test]
    async fn test_job_submission() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        let job = create_test_job("test-repo");
        let job_id = job.id.clone();

        scheduler.submit_job(job).await.unwrap();

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.total_submitted, 1);
        assert_eq!(stats.queued, 1);

        let status = scheduler.get_job_status(&job_id).await;
        assert_eq!(status, Some(JobStatus::Queued));
    }

    #[tokio::test]
    async fn test_job_execution_lifecycle() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        let job = create_test_job("test-repo");
        let job_id = job.id.clone();

        // Submit job
        scheduler.submit_job(job).await.unwrap();
        assert_eq!(scheduler.queue_size().await, 1);

        // Get next job and start it
        let job = scheduler.next_job().await.unwrap();
        scheduler.start_job(job).await.unwrap();

        assert_eq!(scheduler.running_count().await, 1);
        assert_eq!(scheduler.queue_size().await, 0);

        // Complete the job
        scheduler
            .complete_job(&job_id, JobCompletionResult::Success)
            .await
            .unwrap();

        assert_eq!(scheduler.running_count().await, 0);

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.running, 0);
    }

    #[tokio::test]
    async fn test_job_cancellation() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        let job = create_test_job("test-repo");
        let job_id = job.id.clone();

        scheduler.submit_job(job).await.unwrap();
        scheduler.cancel_job(&job_id).await.unwrap();

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.cancelled, 1);
        assert_eq!(stats.queued, 0);
    }

    #[tokio::test]
    async fn test_multiple_job_submission() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        let jobs = vec![
            create_test_job("repo1"),
            create_test_job("repo2"),
            create_test_job("repo3"),
        ];

        let job_ids = scheduler.submit_jobs(jobs).await.unwrap();
        assert_eq!(job_ids.len(), 3);

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.total_submitted, 3);
        assert_eq!(stats.queued, 3);
    }

    #[tokio::test]
    async fn test_scheduler_stats() {
        let stats = SchedulerStats {
            completed: 80,
            failed: 15,
            cancelled: 5,
            ..Default::default()
        };

        assert_eq!(stats.total_finished(), 100);
        assert_eq!(stats.success_rate(), 80.0);
    }

    #[tokio::test]
    async fn test_scheduler_shutdown() {
        let config = SchedulerConfig::default();
        let scheduler = JobScheduler::new(config);

        // Submit some jobs
        let jobs = vec![create_test_job("repo1"), create_test_job("repo2")];
        scheduler.submit_jobs(jobs).await.unwrap();

        // Shutdown should clear the queue
        scheduler.shutdown().await.unwrap();

        assert_eq!(scheduler.queue_size().await, 0);
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_concurrent_jobs, 4);
        assert_eq!(config.max_queue_size, 1000);
        assert_eq!(config.max_completed_history, 100);
    }
}
