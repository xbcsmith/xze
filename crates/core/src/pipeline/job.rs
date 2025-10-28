//! Pipeline job definitions and management

use crate::types::{JobId, JobStatus, RepositoryId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineJob {
    /// Unique job identifier
    pub id: JobId,
    /// Source repository being processed
    pub source_repo: RepositoryId,
    /// Target repository for documentation
    pub target_repo: Option<RepositoryId>,
    /// Current job status
    pub status: JobStatus,
    /// Job metadata
    pub metadata: JobMetadata,
    /// Job configuration
    pub config: JobConfig,
    /// Execution results
    pub results: JobResults,
}

impl PipelineJob {
    /// Create a new pipeline job
    pub fn new(id: JobId, source_repo: RepositoryId) -> Self {
        Self {
            id,
            source_repo,
            target_repo: None,
            status: JobStatus::Queued,
            metadata: JobMetadata::new(),
            config: JobConfig::default(),
            results: JobResults::default(),
        }
    }

    /// Create a new job with target repository
    pub fn with_target(id: JobId, source_repo: RepositoryId, target_repo: RepositoryId) -> Self {
        let mut job = Self::new(id, source_repo);
        job.target_repo = Some(target_repo);
        job
    }

    /// Start the job
    pub fn start(&mut self) {
        self.status = JobStatus::Running;
        self.metadata.started_at = Some(chrono::Utc::now());
    }

    /// Mark job as completed
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.metadata.completed_at = Some(chrono::Utc::now());
        self.calculate_duration();
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: String) {
        self.status = JobStatus::Failed(error);
        self.metadata.completed_at = Some(chrono::Utc::now());
        self.calculate_duration();
    }

    /// Cancel the job
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.metadata.completed_at = Some(chrono::Utc::now());
        self.calculate_duration();
    }

    /// Calculate job duration
    fn calculate_duration(&mut self) {
        if let (Some(started), Some(completed)) =
            (self.metadata.started_at, self.metadata.completed_at)
        {
            self.metadata.duration = Some(completed - started);
        }
    }

    /// Check if job is finished
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed(_) | JobStatus::Cancelled
        )
    }

    /// Check if job is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::Running)
    }

    /// Check if job is queued
    pub fn is_queued(&self) -> bool {
        matches!(self.status, JobStatus::Queued)
    }

    /// Add a result to the job
    pub fn add_result(&mut self, key: String, value: String) {
        self.results.data.insert(key, value);
    }

    /// Add an error to the job
    pub fn add_error(&mut self, error: String) {
        self.results.errors.push(error);
    }

    /// Add a warning to the job
    pub fn add_warning(&mut self, warning: String) {
        self.results.warnings.push(warning);
    }

    /// Add a generated file to the job
    pub fn add_generated_file(&mut self, file_path: String) {
        self.results.generated_files.push(file_path);
    }

    /// Get job duration in seconds
    pub fn duration_seconds(&self) -> Option<u64> {
        self.metadata.duration.map(|d| d.num_seconds() as u64)
    }

    /// Get job progress percentage (0-100)
    pub fn progress(&self) -> f32 {
        match &self.status {
            JobStatus::Queued => 0.0,
            JobStatus::Running => self.metadata.progress,
            JobStatus::Completed => 100.0,
            JobStatus::Failed(_) | JobStatus::Cancelled => self.metadata.progress,
        }
    }

    /// Update job progress
    pub fn set_progress(&mut self, progress: f32) {
        self.metadata.progress = progress.clamp(0.0, 100.0);
    }
}

/// Job metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    /// When the job was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the job started execution
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the job completed
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Job execution duration
    pub duration: Option<chrono::Duration>,
    /// Job priority (higher = more important)
    pub priority: i32,
    /// Job progress (0.0 to 100.0)
    pub progress: f32,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl JobMetadata {
    /// Create new job metadata
    pub fn new() -> Self {
        Self {
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration: None,
            priority: 0,
            progress: 0.0,
            custom: HashMap::new(),
        }
    }

    /// Set custom metadata
    pub fn set_custom(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }

    /// Get custom metadata
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }
}

impl Default for JobMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Job timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Number of retry attempts
    pub max_retries: u32,
    /// Current retry count
    pub retry_count: u32,
    /// Whether to run in dry-run mode
    pub dry_run: bool,
    /// Whether to create pull requests
    pub create_prs: bool,
    /// Documentation categories to generate
    pub doc_categories: Vec<String>,
    /// Intent filter for documentation generation
    #[serde(default)]
    pub intent_filter: Vec<String>,
    /// Custom job parameters
    pub parameters: HashMap<String, String>,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(3600), // 1 hour
            max_retries: 3,
            retry_count: 0,
            dry_run: false,
            create_prs: true,
            doc_categories: vec![
                "tutorial".to_string(),
                "howto".to_string(),
                "reference".to_string(),
                "explanation".to_string(),
            ],
            intent_filter: Vec::new(),
            parameters: HashMap::new(),
        }
    }
}

/// Job execution results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobResults {
    /// Generated files
    pub generated_files: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Warnings generated
    pub warnings: Vec<String>,
    /// Pull request URLs created
    pub pull_requests: Vec<String>,
    /// Analysis results
    pub analysis_data: HashMap<String, String>,
    /// Custom result data
    pub data: HashMap<String, String>,
}

impl JobResults {
    /// Check if job has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if job has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get total number of generated files
    pub fn file_count(&self) -> usize {
        self.generated_files.len()
    }

    /// Get total number of pull requests created
    pub fn pr_count(&self) -> usize {
        self.pull_requests.len()
    }
}

/// Job queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQueueEntry {
    /// The job
    pub job: PipelineJob,
    /// When the job was queued
    pub queued_at: chrono::DateTime<chrono::Utc>,
    /// Queue priority
    pub queue_priority: i32,
}

impl JobQueueEntry {
    /// Create a new queue entry
    pub fn new(job: PipelineJob) -> Self {
        Self {
            queue_priority: job.metadata.priority,
            job,
            queued_at: chrono::Utc::now(),
        }
    }

    /// Create a queue entry with custom priority
    pub fn with_priority(job: PipelineJob, priority: i32) -> Self {
        let mut entry = Self::new(job);
        entry.queue_priority = priority;
        entry
    }
}

impl PartialEq for JobQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.job.id == other.job.id
    }
}

impl Eq for JobQueueEntry {}

impl PartialOrd for JobQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JobQueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then older jobs first
        match other.queue_priority.cmp(&self.queue_priority) {
            std::cmp::Ordering::Equal => self.queued_at.cmp(&other.queued_at),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_job_creation() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let job = PipelineJob::new(job_id.clone(), repo_id.clone());

        assert_eq!(job.id, job_id);
        assert_eq!(job.source_repo, repo_id);
        assert_eq!(job.status, JobStatus::Queued);
        assert!(job.is_queued());
        assert!(!job.is_running());
        assert!(!job.is_finished());
    }

    #[test]
    fn test_job_lifecycle() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let mut job = PipelineJob::new(job_id, repo_id);

        // Start job
        job.start();
        assert!(job.is_running());
        assert!(job.metadata.started_at.is_some());

        // Set progress
        job.set_progress(50.0);
        assert_eq!(job.progress(), 50.0);

        // Complete job
        job.complete();
        assert!(job.is_finished());
        assert_eq!(job.progress(), 100.0);
        assert!(job.metadata.completed_at.is_some());
        assert!(job.duration_seconds().is_some());
    }

    #[test]
    fn test_job_failure() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let mut job = PipelineJob::new(job_id, repo_id);

        job.start();
        job.fail("Test error".to_string());

        assert!(job.is_finished());
        assert!(matches!(job.status, JobStatus::Failed(_)));
        if let JobStatus::Failed(error) = &job.status {
            assert_eq!(error, "Test error");
        }
    }

    #[test]
    fn test_job_results() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let mut job = PipelineJob::new(job_id, repo_id);

        job.add_generated_file("docs/api.md".to_string());
        job.add_warning("Test warning".to_string());
        job.add_error("Test error".to_string());

        assert_eq!(job.results.file_count(), 1);
        assert!(job.results.has_warnings());
        assert!(job.results.has_errors());
    }

    #[test]
    fn test_job_queue_entry_ordering() {
        let job1 = PipelineJob::new(JobId::new(), RepositoryId::from("repo1"));
        let job2 = PipelineJob::new(JobId::new(), RepositoryId::from("repo2"));

        let entry1 = JobQueueEntry::with_priority(job1, 10);
        let entry2 = JobQueueEntry::with_priority(job2, 5);

        // Higher priority should come first
        assert!(entry1 < entry2);
    }

    #[test]
    fn test_job_metadata() {
        let mut metadata = JobMetadata::new();
        assert_eq!(metadata.priority, 0);
        assert_eq!(metadata.progress, 0.0);

        metadata.set_custom("key".to_string(), "value".to_string());
        assert_eq!(metadata.get_custom("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_job_config_default() {
        let config = JobConfig::default();
        assert_eq!(config.timeout_seconds, Some(3600));
        assert_eq!(config.max_retries, 3);
        assert!(config.create_prs);
        assert!(!config.dry_run);
        assert_eq!(config.doc_categories.len(), 4);
    }

    #[test]
    fn test_progress_clamping() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test-repo");
        let mut job = PipelineJob::new(job_id, repo_id);

        job.set_progress(-10.0);
        assert_eq!(job.progress(), 0.0);

        job.set_progress(150.0);
        assert_eq!(job.progress(), 100.0);

        job.set_progress(75.5);
        assert_eq!(job.progress(), 75.5);
    }
}
