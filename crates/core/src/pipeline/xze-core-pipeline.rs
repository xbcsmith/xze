//! Pipeline orchestration

use crate::{
    ai::AIAnalysisService,
    documentation::DocumentationService,
    error::Result,
    git::{GitOperations, PrManager, PullRequest},
    repository::{Repository, RepositoryManager},
    types::{JobId, JobMetadata, JobStatus, RepoId},
    XzeError,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

pub mod controller;
pub mod job;

pub use controller::PipelineController;
pub use job::{PipelineJob, JobState};

/// Pipeline state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    Idle,
    Cloning,
    Analyzing,
    Generating,
    Reviewing,
    CreatingPr,
    Completed,
    Failed,
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub documentation_repo_url: String,
    pub documentation_repo_path: PathBuf,
    pub cache_dir: PathBuf,
}

/// Pipeline service for orchestrating documentation generation
pub struct PipelineService {
    repo_manager: Arc<RwLock<RepositoryManager>>,
    ai_service: Arc<AIAnalysisService>,
    doc_service: Arc<DocumentationService>,
    git_ops: Arc<GitOperations>,
    pr_manager: Arc<PrManager>,
    jobs: Arc<RwLock<HashMap<JobId, PipelineJob>>>,
    config: PipelineConfig,
}

impl PipelineService {
    pub fn new(
        repo_manager: RepositoryManager,
        ai_service: Arc<AIAnalysisService>,
        git_ops: GitOperations,
        config: PipelineConfig,
    ) -> Self {
        let doc_service = Arc::new(DocumentationService::new(ai_service.clone()));

        Self {
            repo_manager: Arc::new(RwLock::new(repo_manager)),
            ai_service,
            doc_service,
            git_ops: Arc::new(git_ops),
            pr_manager: Arc::new(PrManager::new()),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Submit a new job
    pub async fn submit_job(&self, repo_id: RepoId, target_repo_path: PathBuf) -> Result<JobId> {
        let job_id = JobId::new();
        
        let job = PipelineJob::new(job_id, repo_id.clone(), target_repo_path);
        
        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id, job);
        
        tracing::info!("Submitted job {} for repository {}", job_id, repo_id.as_str());
        
        Ok(job_id)
    }

    /// Execute a job
    pub async fn execute_job(&self, job_id: JobId) -> Result<()> {
        tracing::info!("Executing job {}", job_id);

        // Update job state to running
        {
            let mut jobs = self.jobs.write().await;
            let job = jobs.get_mut(&job_id)
                .ok_or_else(|| XzeError::not_found("Job not found"))?;
            job.metadata.start();
            job.state = JobState::Analyzing;
        }

        // Execute pipeline stages
        if let Err(e) = self.run_pipeline(&job_id).await {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.metadata.fail(e.to_string());
                job.state = JobState::Failed;
            }
            return Err(e);
        }

        // Mark job as completed
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.metadata.complete();
                job.state = JobState::Completed;
            }
        }

        tracing::info!("Job {} completed successfully", job_id);
        Ok(())
    }

    /// Run the complete pipeline
    async fn run_pipeline(&self, job_id: &JobId) -> Result<()> {
        let (repo_id, target_path) = {
            let jobs = self.jobs.read().await;
            let job = jobs.get(job_id)
                .ok_or_else(|| XzeError::not_found("Job not found"))?;
            (job.source_repo.clone(), job.target_repo.clone())
        };

        // Stage 1: Analyze repository
        tracing::info!("Stage 1: Analyzing repository");
        self.analyze_repository(&repo_id).await?;

        // Stage 2: Generate documentation
        tracing::info!("Stage 2: Generating documentation");
        let documents = self.generate_documentation(&repo_id).await?;

        // Stage 3: Write documentation to target repository
        tracing::info!("Stage 3: Writing documentation");
        self.write_documentation(&target_path, &documents).await?;

        // Stage 4: Create pull request
        tracing::info!("Stage 4: Creating pull request");
        self.create_pull_request(&repo_id, &target_path).await?;

        Ok(())
    }

    /// Analyze a repository
    async fn analyze_repository(&self, repo_id: &RepoId) -> Result<()> {
        let mut manager = self.repo_manager.write().await;
        manager.analyze_repository(repo_id).await
    }

    /// Generate documentation for a repository
    async fn generate_documentation(&self, repo_id: &RepoId) -> Result<Vec<crate::documentation::Document>> {
        let manager = self.repo_manager.read().await;
        let repo = manager.get_repository(repo_id)
            .ok_or_else(|| XzeError::not_found("Repository not found"))?;

        self.doc_service.generate_all(repo).await
    }

    /// Write documentation to files
    async fn write_documentation(
        &self,
        target_path: &PathBuf,
        documents: &[crate::documentation::Document],
    ) -> Result<()> {
        for doc in documents {
            let file_path = target_path.join(doc.file_path("service")); // TODO: Use actual service name
            
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(&file_path, &doc.content).await?;
            tracing::info!("Wrote documentation to {}", file_path.display());
        }

        Ok(())
    }

    /// Create a pull request
    async fn create_pull_request(&self, repo_id: &RepoId, target_path: &PathBuf) -> Result<()> {
        // Open the documentation repository
        let doc_repo = self.git_ops.open(target_path)?;

        // Create a new branch
        let branch_name = format!("docs-update-{}", repo_id.as_str());
        self.git_ops.create_branch(&doc_repo, &branch_name)?;

        // Commit changes
        let commit_msg = format!("Update documentation for {}", repo_id.as_str());
        self.git_ops.commit(&doc_repo, &commit_msg)?;

        // Push to remote
        self.git_ops.push(&doc_repo, &branch_name)?;

        // Create PR
        let pr = PullRequest::new(
            format!("Update {} documentation", repo_id.as_str()),
            branch_name,
            repo_id.as_str().to_string(),
        );

        let pr_id = self.pr_manager.create_pr(pr).await?;
        tracing::info!("Created pull request: {}", pr_id);

        Ok(())
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).map(|job| job.metadata.status)
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<(JobId, JobStatus)> {
        let jobs = self.jobs.read().await;
        jobs.iter()
            .map(|(id, job)| (*id, job.metadata.status))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ModelConfig,
        git::CredentialStore,
        types::ProgrammingLanguage,
    };
    use tempfile::tempdir;

    fn create_test_service() -> PipelineService {
        let temp = tempdir().unwrap();
        
        let repo_manager = RepositoryManager::new(temp.path()).unwrap();
        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));
        let git_ops = GitOperations::new(CredentialStore::new());
        
        let config = PipelineConfig {
            documentation_repo_url: "https://example.com/docs".to_string(),
            documentation_repo_path: temp.path().to_path_buf(),
            cache_dir: temp.path().to_path_buf(),
        };

        PipelineService::new(repo_manager, ai_service, git_ops, config)
    }

    #[tokio::test]
    async fn test_submit_job() {
        let service = create_test_service();
        let temp = tempdir().unwrap();
        
        let job_id = service.submit_job(
            RepoId::from("test-repo"),
            temp.path().to_path_buf(),
        ).await.unwrap();

        assert!(service.get_job_status(&job_id).await.is_some());
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let service = create_test_service();
        let temp = tempdir().unwrap();
        
        let _ = service.submit_job(
            RepoId::from("test-repo-1"),
            temp.path().to_path_buf(),
        ).await.unwrap();

        let _ = service.submit_job(
            RepoId::from("test-repo-2"),
            temp.path().to_path_buf(),
        ).await.unwrap();

        let jobs = service.list_jobs().await;
        assert_eq!(jobs.len(), 2);
    }
}
