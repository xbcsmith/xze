//! API module for XZe serve crate

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version
pub const API_VERSION: &str = "v1";

/// API routes configuration
pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/version", get(get_version))
        .route("/analyze", post(analyze_repository))
        .route("/repositories", get(list_repositories))
        .route("/repositories/:id", get(get_repository))
        .route("/repositories/:id/analyze", post(analyze_repository_by_id))
        .route("/documentation", get(list_documentation))
        .route("/documentation/:id", get(get_documentation))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get version information
pub async fn get_version() -> impl IntoResponse {
    Json(VersionResponse {
        version: crate::VERSION.to_string(),
        api_version: API_VERSION.to_string(),
        build_info: BuildInfo {
            commit: option_env!("VERGEN_GIT_SHA")
                .unwrap_or("unknown")
                .to_string(),
            build_date: option_env!("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or("unknown")
                .to_string(),
        },
    })
}

/// Analyze repository endpoint
pub async fn analyze_repository(Json(request): Json<AnalyzeRequest>) -> impl IntoResponse {
    // TODO: Implement repository analysis
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository: {}", request.repository_url),
    })
}

/// List repositories endpoint
pub async fn list_repositories(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    // TODO: Implement repository listing
    Json(RepositoryListResponse {
        repositories: vec![],
        total: 0,
        page: params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1),
        per_page: params
            .get("per_page")
            .and_then(|p| p.parse().ok())
            .unwrap_or(10),
    })
}

/// Get repository by ID
pub async fn get_repository(Path(_id): Path<String>) -> impl IntoResponse {
    // TODO: Implement repository retrieval
    StatusCode::NOT_FOUND
}

/// Analyze repository by ID
pub async fn analyze_repository_by_id(Path(id): Path<String>) -> impl IntoResponse {
    // TODO: Implement repository analysis by ID
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository ID: {}", id),
    })
}

/// List documentation endpoint
pub async fn list_documentation() -> impl IntoResponse {
    // TODO: Implement documentation listing
    Json(DocumentationListResponse {
        documents: vec![],
        total: 0,
    })
}

/// Get documentation by ID
pub async fn get_documentation(Path(_id): Path<String>) -> impl IntoResponse {
    // TODO: Implement documentation retrieval
    StatusCode::NOT_FOUND
}

// Response types

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub api_version: String,
    pub build_info: BuildInfo,
}

#[derive(Serialize)]
pub struct BuildInfo {
    pub commit: String,
    pub build_date: String,
}

#[derive(Serialize)]
pub struct AnalyzeResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
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
}

#[derive(Serialize)]
pub struct DocumentationListResponse {
    pub documents: Vec<DocumentationInfo>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct DocumentationResponse {
    pub document: DocumentationInfo,
}

#[derive(Serialize)]
pub struct DocumentationInfo {
    pub id: String,
    pub title: String,
    pub category: String,
    pub repository_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Request types

#[derive(Deserialize)]
pub struct AnalyzeRequest {
    pub repository_url: String,
    pub branch: Option<String>,
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_check() {
        let app = create_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_endpoint() {
        let app = create_routes();
        let server = TestServer::new(app).unwrap();

        let response = server.get("/version").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let version_response: VersionResponse = response.json();
        assert_eq!(version_response.api_version, API_VERSION);
    }

    #[tokio::test]
    async fn test_analyze_endpoint() {
        let app = create_routes();
        let server = TestServer::new(app).unwrap();

        let request = AnalyzeRequest {
            repository_url: "https://github.com/test/repo".to_string(),
            branch: Some("main".to_string()),
            language: Some("rust".to_string()),
        };

        let response = server.post("/analyze").json(&request).await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let analyze_response: AnalyzeResponse = response.json();
        assert_eq!(analyze_response.status, "queued");
    }
}
