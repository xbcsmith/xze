//! HTTP handlers for XZe serve crate

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub ollama_url: String,
    pub config: crate::ServerConfig,
}

impl AppState {
    /// Create new application state
    pub fn new(config: crate::ServerConfig) -> Self {
        Self {
            ollama_url: config.ollama_url.clone(),
            config,
        }
    }
}

/// Handler for repository analysis
pub async fn handle_analyze_repository(
    State(_state): State<AppState>,
    Json(payload): Json<AnalyzeRepositoryRequest>,
) -> impl IntoResponse {
    tracing::info!("Analyzing repository: {}", payload.repository_url);

    // TODO: Implement actual repository analysis
    let job_id = uuid::Uuid::new_v4().to_string();

    Json(AnalyzeRepositoryResponse {
        job_id,
        status: "queued".to_string(),
        message: format!("Repository {} queued for analysis", payload.repository_url),
        estimated_duration: Some(300), // 5 minutes
    })
}

/// Handler for getting analysis job status
pub async fn handle_job_status(
    State(_state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Getting status for job: {}", job_id);

    // TODO: Implement actual job status lookup
    Json(JobStatusResponse {
        job_id,
        status: "completed".to_string(),
        progress: 100.0,
        message: "Analysis completed successfully".to_string(),
        result: Some("Documentation generated".to_string()),
        error: None,
    })
}

/// Handler for listing repositories
pub async fn handle_list_repositories(
    State(_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let page = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let per_page = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(10);

    tracing::info!(
        "Listing repositories - page: {}, per_page: {}",
        page,
        per_page
    );

    // TODO: Implement actual repository listing
    Json(RepositoryListResponse {
        repositories: vec![RepositoryInfo {
            id: "repo-1".to_string(),
            name: "example-repo".to_string(),
            url: "https://github.com/example/repo".to_string(),
            language: "rust".to_string(),
            last_analyzed: Some(chrono::Utc::now()),
            doc_coverage: 0.85,
        }],
        total: 1,
        page,
        per_page,
    })
}

/// Handler for getting repository details
pub async fn handle_get_repository(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Getting repository: {}", id);

    // TODO: Implement actual repository lookup
    if id == "repo-1" {
        Json(RepositoryResponse {
            repository: RepositoryInfo {
                id: id.clone(),
                name: "example-repo".to_string(),
                url: "https://github.com/example/repo".to_string(),
                language: "rust".to_string(),
                last_analyzed: Some(chrono::Utc::now()),
                doc_coverage: 0.85,
            },
        })
        .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Handler for generating documentation
pub async fn handle_generate_documentation(
    State(_state): State<AppState>,
    Json(payload): Json<GenerateDocumentationRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Generating documentation for repository: {}",
        payload.repository_id
    );

    // TODO: Implement actual documentation generation
    let job_id = uuid::Uuid::new_v4().to_string();

    Json(GenerateDocumentationResponse {
        job_id,
        status: "queued".to_string(),
        message: "Documentation generation queued".to_string(),
        categories: payload.categories.unwrap_or_else(|| {
            vec![
                "tutorial".to_string(),
                "how-to".to_string(),
                "reference".to_string(),
                "explanation".to_string(),
            ]
        }),
    })
}

/// Handler for getting documentation
pub async fn handle_get_documentation(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Getting documentation: {}", id);

    // TODO: Implement actual documentation lookup
    Json(DocumentationResponse {
        id,
        title: "Example Documentation".to_string(),
        category: "reference".to_string(),
        content: "# Example Documentation\n\nThis is example documentation content.".to_string(),
        repository_id: "repo-1".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata: DocumentationMetadata {
            author: "XZe".to_string(),
            tags: vec!["rust".to_string(), "documentation".to_string()],
            word_count: 150,
        },
    })
}

/// Handler for server health check
pub async fn handle_health_check() -> impl IntoResponse {
    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        timestamp: chrono::Utc::now(),
        services: ServiceStatus {
            ollama: true,   // TODO: Check actual Ollama connectivity
            database: true, // TODO: Check actual database connectivity
            filesystem: true,
        },
    })
}

// Request types

#[derive(Deserialize)]
pub struct AnalyzeRepositoryRequest {
    pub repository_url: String,
    pub branch: Option<String>,
    pub language: Option<String>,
    pub force_reanalysis: Option<bool>,
}

#[derive(Deserialize)]
pub struct GenerateDocumentationRequest {
    pub repository_id: String,
    pub categories: Option<Vec<String>>,
    pub template: Option<String>,
    pub output_format: Option<String>,
}

// Response types

#[derive(Serialize)]
pub struct AnalyzeRepositoryResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
    pub estimated_duration: Option<u64>,
}

#[derive(Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub progress: f32,
    pub message: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct RepositoryListResponse {
    pub repositories: Vec<RepositoryInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Serialize)]
pub struct RepositoryResponse {
    pub repository: RepositoryInfo,
}

#[derive(Serialize)]
pub struct RepositoryInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub language: String,
    pub last_analyzed: Option<chrono::DateTime<chrono::Utc>>,
    pub doc_coverage: f32,
}

#[derive(Serialize)]
pub struct GenerateDocumentationResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
    pub categories: Vec<String>,
}

#[derive(Serialize)]
pub struct DocumentationResponse {
    pub id: String,
    pub title: String,
    pub category: String,
    pub content: String,
    pub repository_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: DocumentationMetadata,
}

#[derive(Serialize)]
pub struct DocumentationMetadata {
    pub author: String,
    pub tags: Vec<String>,
    pub word_count: usize,
}

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub services: ServiceStatus,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub ollama: bool,
    pub database: bool,
    pub filesystem: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let config = crate::ServerConfig::default();
        let state = AppState::new(config.clone());
        assert_eq!(state.ollama_url, config.ollama_url);
    }

    #[tokio::test]
    async fn test_health_check_handler() {
        let response = handle_health_check().await;
        assert_eq!(response.0.status, "healthy");
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"repository_url": "https://github.com/test/repo", "branch": "main"}"#;
        let request: AnalyzeRepositoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.repository_url, "https://github.com/test/repo");
        assert_eq!(request.branch, Some("main".to_string()));
    }

    #[test]
    fn test_response_serialization() {
        let response = AnalyzeRepositoryResponse {
            job_id: "test-job-id".to_string(),
            status: "queued".to_string(),
            message: "Test message".to_string(),
            estimated_duration: Some(300),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-job-id"));
        assert!(json.contains("queued"));
    }
}
