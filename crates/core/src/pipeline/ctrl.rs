//! Pipeline controller for high-level orchestration

use crate::{
    error::Result,
    pipeline::{PipelineService, PipelineState},
    types::{JobId, RepoId},
    XzeError,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

/// Pipeline controller
pub struct PipelineController {
    service: Arc<PipelineService>,
    state: Arc<RwLock<PipelineState>>,
}

impl PipelineController {
    pub fn new(service: PipelineService) -> Self {
        Self {
            service: Arc::new(service),
            state: Arc::new(RwLock::new(PipelineState::Idle)),
        }
    }

    /// Run pipeline for a single repository
    pub async fn run_for_repository(
        &self,
        repo_id: RepoId,
        target_path: PathBuf,
    ) -> Result<JobId> {
        // Check if already running
        {
            let state = self.state.read().await;
            if *state != PipelineState::Idle && *state != PipelineState::Completed {
                return Err(XzeError::pipeline("Pipeline is already running"));
            }
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = PipelineState::Cloning;
        }

        // Submit job
        let job_id = self.service.submit_job(repo_id, target_path).await?;

        // Execute job in background
        let service = self.service.clone();
        let state = self.state.clone();
        
        tokio::spawn(async move {
            match service.execute_job(job_id).await {
                Ok(_) => {
                    let mut s = state.write().await;
                    *s = PipelineState::Completed;
                    tracing::info!("Pipeline completed for job {}", job_id);
                }
                Err(e) => {
                    let mut s = state.write().await;
                    *s = PipelineState::Failed;
                    tracing::error!("Pipeline failed for job {}: {}", job_id, e);
                }
            }
        });

        Ok(job_id)
    }

    /// Get current pipeline state
    pub async fn get_state(&self) -> PipelineState {
        *self.state.read().await
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: &JobId) -> Option<crate::types::JobStatus> {
        self.service.get_job_status(job_id).await
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<(JobId, crate::types::JobStatus)> {
        self.service.list_jobs().await
    }

    /// Reset pipeline to idle state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = PipelineState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::AIAnalysisService,
        config::ModelConfig,
        git::{CredentialStore, GitOperations},
        pipeline::PipelineConfig,
        repository::RepositoryManager,
    };
    use tempfile::tempdir;

    fn create_test_controller() -> PipelineController {
        let temp = tempdir().unwrap();
        
        let repo_manager = RepositoryManager::new(temp.path()).unwrap();
        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));
        let git_ops = GitOperations::new(Cre