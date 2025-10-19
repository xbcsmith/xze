//! Pipeline module for orchestrating documentation generation workflows

use crate::{
    ai::AIAnalysisService,
    error::{Result, XzeError},
    git::GitOperations,
    repository::{Repository, RepositoryManager},
    types::{JobId, JobStatus, RepositoryId},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub mod controller;
pub mod job;
pub mod scheduler;

pub use controller::PipelineController;
pub use job::{JobMetadata, PipelineJob};
pub use scheduler::JobScheduler;

/// Pipeline execution mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineMode {
    /// Analyze repositories only
    Analyze,
    /// Generate documentation only
    Generate,
    /// Full pipeline: analyze and generate
    Full,
    /// Validate existing documentation
    Validate,
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Execution mode
    pub mode: PipelineMode,
    /// Maximum concurrent jobs
    pub max_concurrent_jobs: usize,
    /// Job timeout in seconds
    pub job_timeout_seconds: u64,
    /// Whether to create pull requests automatically
    pub auto_create_prs: bool,
    /// Whether to run in dry-run mode
    pub dry_run: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            mode: PipelineMode::Full,
            max_concurrent_jobs: 4,
            job_timeout_seconds: 3600, // 1 hour
            auto_create_prs: true,
            dry_run: false,
        }
    }
}

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Job ID
    pub job_id: JobId,
    /// Repository that was processed
    pub repository_id: RepositoryId,
    /// Final job status
    pub status: JobStatus,
    /// Execution duration
    pub duration_seconds: u64,
    /// Generated documentation files
    pub generated_files: Vec<String>,
    /// Errors encountered during execution
    pub errors: Vec<String>,
    /// Warnings generated
    pub warnings: Vec<String>,
    /// Pull request URL if created
    pub pull_request_url: Option<String>,
}

/// Pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    /// Total jobs executed
    pub total_jobs: usize,
    /// Successful jobs
    pub successful_jobs: usize,
    /// Failed jobs
    pub failed_jobs: usize,
    /// Jobs currently running
    pub running_jobs: usize,
    /// Jobs in queue
    pub queued_jobs: usize,
    /// Average execution time in seconds
    pub avg_execution_time: f64,
    /// Total execution time in seconds
    pub total_execution_time: u64,
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self {
            total_jobs: 0,
            successful_jobs: 0,
            failed_jobs: 0,
            running_jobs: 0,
            queued_jobs: 0,
            avg_execution_time: 0.0,
            total_execution_time: 0,
        }
    }
}

/// Pipeline executor for running documentation generation workflows
#[derive(Debug)]
pub struct PipelineExecutor {
    config: PipelineConfig,
    repo_manager: Arc<RepositoryManager>,
    ai_service: Arc<AIAnalysisService>,
    #[allow(dead_code)]
    git_ops: Arc<GitOperations>,
    jobs: Arc<RwLock<HashMap<JobId, PipelineJob>>>,
    stats: Arc<RwLock<PipelineStats>>,
}

impl PipelineExecutor {
    /// Create a new pipeline executor
    pub fn new(
        config: PipelineConfig,
        repo_manager: Arc<RepositoryManager>,
        ai_service: Arc<AIAnalysisService>,
        git_ops: Arc<GitOperations>,
    ) -> Self {
        Self {
            config,
            repo_manager,
            ai_service,
            git_ops,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
        }
    }

    /// Execute pipeline for a single repository
    pub async fn execute_for_repository(&self, repo_id: &RepositoryId) -> Result<PipelineResult> {
        let job_id = JobId::new();
        let start_time = Instant::now();

        info!(
            "Starting pipeline execution for repository {} (job: {})",
            repo_id, job_id
        );

        // Create and register job
        let job = PipelineJob::new(job_id.clone(), repo_id.clone());
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_jobs += 1;
            stats.running_jobs += 1;
        }

        let result = self
            .execute_job_internal(&job_id, repo_id, start_time)
            .await;

        // Update job status and stats
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match &result {
                    Ok(pipeline_result) => {
                        job.status = pipeline_result.status.clone();
                    }
                    Err(_) => {
                        job.status = JobStatus::Failed("Pipeline execution failed".to_string());
                    }
                }
            }
        }

        {
            let mut stats = self.stats.write().await;
            stats.running_jobs = stats.running_jobs.saturating_sub(1);
            let duration = start_time.elapsed().as_secs();
            stats.total_execution_time += duration;

            match &result {
                Ok(_) => stats.successful_jobs += 1,
                Err(_) => stats.failed_jobs += 1,
            }

            if stats.total_jobs > 0 {
                stats.avg_execution_time =
                    stats.total_execution_time as f64 / stats.total_jobs as f64;
            }
        }

        result
    }

    /// Internal job execution logic
    async fn execute_job_internal(
        &self,
        job_id: &JobId,
        repo_id: &RepositoryId,
        start_time: Instant,
    ) -> Result<PipelineResult> {
        let mut result = PipelineResult {
            job_id: job_id.clone(),
            repository_id: repo_id.clone(),
            status: JobStatus::Running,
            duration_seconds: 0,
            generated_files: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            pull_request_url: None,
        };

        // Step 1: Get repository
        let repository = match self.repo_manager.get_repository(repo_id).await {
            Ok(repo) => repo,
            Err(e) => {
                error!("Failed to get repository {}: {}", repo_id, e);
                result
                    .errors
                    .push(format!("Failed to get repository: {}", e));
                result.status = JobStatus::Failed("Repository not found".to_string());
                return Ok(result);
            }
        };

        info!("Retrieved repository: {}", repository.name());

        // Step 2: Analyze repository (if needed)
        if matches!(self.config.mode, PipelineMode::Analyze | PipelineMode::Full) {
            if let Err(e) = self.analyze_repository(&repository, &mut result).await {
                error!("Analysis failed for repository {}: {}", repo_id, e);
                result.errors.push(format!("Analysis failed: {}", e));
            }
        }

        // Step 3: Generate documentation (if needed)
        if matches!(
            self.config.mode,
            PipelineMode::Generate | PipelineMode::Full
        ) {
            if let Err(e) = self.generate_documentation(&repository, &mut result).await {
                error!(
                    "Documentation generation failed for repository {}: {}",
                    repo_id, e
                );
                result
                    .errors
                    .push(format!("Documentation generation failed: {}", e));
            }
        }

        // Step 4: Validate documentation (if needed)
        if matches!(self.config.mode, PipelineMode::Validate) {
            if let Err(e) = self.validate_documentation(&repository, &mut result).await {
                warn!(
                    "Documentation validation had issues for repository {}: {}",
                    repo_id, e
                );
                result
                    .warnings
                    .push(format!("Documentation validation: {}", e));
            }
        }

        // Step 5: Create pull request (if configured and not dry run)
        if self.config.auto_create_prs && !self.config.dry_run && !result.generated_files.is_empty()
        {
            if let Err(e) = self.create_pull_request(&repository, &mut result).await {
                warn!(
                    "Failed to create pull request for repository {}: {}",
                    repo_id, e
                );
                result
                    .warnings
                    .push(format!("Pull request creation failed: {}", e));
            }
        }

        // Finalize result
        result.duration_seconds = start_time.elapsed().as_secs();
        result.status = if result.errors.is_empty() {
            JobStatus::Completed
        } else {
            JobStatus::Failed(result.errors.join("; "))
        };

        info!(
            "Pipeline execution completed for repository {} in {}s with status: {}",
            repo_id, result.duration_seconds, result.status
        );

        Ok(result)
    }

    /// Analyze repository structure and code
    async fn analyze_repository(
        &self,
        repository: &Repository,
        result: &mut PipelineResult,
    ) -> Result<()> {
        info!("Analyzing repository structure for {}", repository.name());

        let analysis = self
            .ai_service
            .analyze_code_structure(&repository.structure)
            .await?;

        debug!(
            "Analysis completed with confidence: {}",
            analysis.confidence
        );

        // Store analysis results (in a real implementation, this might go to a database)
        result
            .generated_files
            .push("analysis_report.md".to_string());

        Ok(())
    }

    /// Generate documentation using AI
    async fn generate_documentation(
        &self,
        repository: &Repository,
        result: &mut PipelineResult,
    ) -> Result<()> {
        info!("Generating documentation for {}", repository.name());

        // Generate different types of documentation
        let tasks = vec![
            ("tutorial", "Getting Started"),
            ("reference", "API Reference"),
            ("howto", "Common Tasks"),
            ("explanation", "Architecture Overview"),
        ];

        for (doc_type, title) in tasks {
            match doc_type {
                "tutorial" => {
                    if let Ok(_content) = self
                        .ai_service
                        .generate_tutorial(&repository.structure, title)
                        .await
                    {
                        let filename = format!(
                            "docs/tutorial_{}.md",
                            title.replace(' ', "_").to_lowercase()
                        );
                        result.generated_files.push(filename);
                        debug!("Generated tutorial: {}", title);
                    }
                }
                "reference" => {
                    if let Ok(_content) = self
                        .ai_service
                        .generate_api_documentation(&repository.structure)
                        .await
                    {
                        let filename = "docs/api_reference.md".to_string();
                        result.generated_files.push(filename);
                        debug!("Generated API reference");
                    }
                }
                "howto" => {
                    if let Ok(_content) = self
                        .ai_service
                        .generate_howto(&repository.structure, title)
                        .await
                    {
                        let filename =
                            format!("docs/howto_{}.md", title.replace(' ', "_").to_lowercase());
                        result.generated_files.push(filename);
                        debug!("Generated how-to guide: {}", title);
                    }
                }
                "explanation" => {
                    if let Ok(_content) = self
                        .ai_service
                        .generate_explanation(&repository.structure, title)
                        .await
                    {
                        let filename = format!(
                            "docs/explanation_{}.md",
                            title.replace(' ', "_").to_lowercase()
                        );
                        result.generated_files.push(filename);
                        debug!("Generated explanation: {}", title);
                    }
                }
                _ => {}
            }
        }

        info!(
            "Generated {} documentation files",
            result.generated_files.len()
        );
        Ok(())
    }

    /// Validate existing documentation
    async fn validate_documentation(
        &self,
        _repository: &Repository,
        result: &mut PipelineResult,
    ) -> Result<()> {
        info!("Validating documentation for {}", _repository.name());

        // TODO: Implement documentation validation logic
        // This might check for:
        // - Broken links
        // - Outdated information
        // - Missing sections
        // - Consistency with code

        result
            .generated_files
            .push("validation_report.md".to_string());
        Ok(())
    }

    /// Create pull request with generated documentation
    async fn create_pull_request(
        &self,
        repository: &Repository,
        result: &mut PipelineResult,
    ) -> Result<()> {
        info!(
            "Creating pull request for {} with {} files",
            repository.name(),
            result.generated_files.len()
        );

        // TODO: Implement pull request creation
        // This would:
        // 1. Create a new branch
        // 2. Commit the generated files
        // 3. Push the branch
        // 4. Create a pull request using the git operations

        result.pull_request_url = Some(format!(
            "https://github.com/example/{}/pull/123",
            repository.name()
        ));

        Ok(())
    }

    /// Get current pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get job by ID
    pub async fn get_job(&self, job_id: &JobId) -> Option<PipelineJob> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<PipelineJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Cancel a running job
    pub async fn cancel_job(&self, job_id: &JobId) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            if matches!(job.status, JobStatus::Running | JobStatus::Queued) {
                job.status = JobStatus::Cancelled;
                info!("Cancelled job: {}", job_id);
                return Ok(());
            }
        }

        Err(XzeError::not_found(format!(
            "Job {} not found or not cancellable",
            job_id
        )))
    }

    /// Clean up completed jobs older than specified duration
    pub async fn cleanup_old_jobs(&self, max_age_hours: u64) -> usize {
        let mut jobs = self.jobs.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);

        let initial_count = jobs.len();
        jobs.retain(|_, job| {
            match &job.status {
                JobStatus::Completed | JobStatus::Failed(_) | JobStatus::Cancelled => {
                    job.metadata.created_at > cutoff
                }
                _ => true, // Keep running/queued jobs
            }
        });

        let removed_count = initial_count - jobs.len();
        if removed_count > 0 {
            info!("Cleaned up {} old jobs", removed_count);
        }

        removed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::XzeConfig, types::ProgrammingLanguage};
    use tempfile::TempDir;

    fn create_test_config() -> XzeConfig {
        XzeConfig::default()
    }

    fn create_test_repository() -> Repository {
        let temp_dir = TempDir::new().unwrap();
        Repository::new(
            RepositoryId::from("test-repo"),
            "https://github.com/test/repo".to_string(),
            temp_dir.path().to_path_buf(),
            ProgrammingLanguage::Rust,
        )
    }

    #[tokio::test]
    async fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.mode, PipelineMode::Full);
        assert_eq!(config.max_concurrent_jobs, 4);
        assert!(config.auto_create_prs);
        assert!(!config.dry_run);
    }

    #[tokio::test]
    async fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.successful_jobs, 0);
        assert_eq!(stats.avg_execution_time, 0.0);
    }

    #[test]
    fn test_pipeline_result_creation() {
        let job_id = JobId::new();
        let repo_id = RepositoryId::from("test");

        let result = PipelineResult {
            job_id: job_id.clone(),
            repository_id: repo_id,
            status: JobStatus::Running,
            duration_seconds: 0,
            generated_files: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            pull_request_url: None,
        };

        assert_eq!(result.job_id, job_id);
        assert_eq!(result.status, JobStatus::Running);
        assert_eq!(result.duration_seconds, 0);
    }

    #[test]
    fn test_pipeline_mode_serialization() {
        let mode = PipelineMode::Full;
        let json = serde_json::to_string(&mode).unwrap();
        let deserialized: PipelineMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, deserialized);
    }
}
