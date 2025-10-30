//! API v1 handlers
//!
//! This module contains all v1 API endpoint handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::handlers::AppState;

/// Health check endpoint for API v1
///
/// # Returns
///
/// Returns a JSON response with service health status
///
/// # Examples
///
/// ```
/// use axum::http::StatusCode;
/// // GET /api/v1/health
/// // Returns: {"status":"healthy","version":"...","timestamp":"..."}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/health",
        tag = "health",
        responses(
            (status = 200, description = "Service is healthy", body = HealthResponse),
        )
    )
)]
pub async fn health_check(State(_state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get API version information
///
/// # Returns
///
/// Returns version details including API version, commit hash, and build date
///
/// # Examples
///
/// ```
/// // GET /api/v1/version
/// // Returns: {"version":"...","api_version":"v1","build_info":{...}}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/version",
        tag = "system",
        responses(
            (status = 200, description = "Version information", body = VersionResponse),
        )
    )
)]
pub async fn get_version(State(_state): State<AppState>) -> impl IntoResponse {
    Json(VersionResponse {
        version: crate::VERSION.to_string(),
        api_version: "v1".to_string(),
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
///
/// Queues a repository for AI-powered documentation analysis
///
/// # Arguments
///
/// * `request` - Repository analysis request containing URL, branch, and language
///
/// # Returns
///
/// Returns a job ID and status for tracking the analysis
///
/// # Examples
///
/// ```
/// // POST /api/v1/analyze
/// // Body: {"repository_url":"https://github.com/user/repo","branch":"main"}
/// // Returns: {"job_id":"...","status":"queued","message":"..."}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        post,
        path = "/api/v1/analyze",
        tag = "analysis",
        request_body = AnalyzeRequest,
        responses(
            (status = 200, description = "Analysis job queued successfully", body = AnalyzeResponse),
            (status = 400, description = "Invalid request parameters"),
        )
    )
)]
pub async fn analyze_repository(
    State(_state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    // TODO: Implement actual repository analysis
    // This should queue a job in the background processing system
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository: {}", request.repository_url),
    })
}

/// List repositories endpoint
///
/// Returns a paginated list of repositories
///
/// # Arguments
///
/// * `params` - Query parameters for pagination and filtering
///
/// # Returns
///
/// Returns a list of repositories with pagination metadata
///
/// # Examples
///
/// ```
/// // GET /api/v1/repositories?page=1&per_page=10
/// // Returns: {"repositories":[...],"total":0,"page":1,"per_page":10}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/repositories",
        tag = "repositories",
        params(
            ("page" = Option<usize>, Query, description = "Page number (default: 1)"),
            ("per_page" = Option<usize>, Query, description = "Items per page (default: 10, max: 100)"),
        ),
        responses(
            (status = 200, description = "List of repositories", body = RepositoryListResponse),
        )
    )
)]
pub async fn list_repositories(
    State(_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // TODO: Implement actual repository listing from database
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
///
/// Retrieves detailed information about a specific repository
///
/// # Arguments
///
/// * `id` - Repository ID
///
/// # Returns
///
/// Returns repository details or 404 if not found
///
/// # Examples
///
/// ```
/// // GET /api/v1/repositories/abc123
/// // Returns: {"repository":{...}} or 404
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/repositories/{id}",
        tag = "repositories",
        params(
            ("id" = String, Path, description = "Repository unique identifier"),
        ),
        responses(
            (status = 200, description = "Repository details", body = RepositoryInfo),
            (status = 404, description = "Repository not found"),
        )
    )
)]
pub async fn get_repository(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement repository retrieval from database
    StatusCode::NOT_FOUND
}

/// Analyze repository by ID
///
/// Queues an existing repository for re-analysis
///
/// # Arguments
///
/// * `id` - Repository ID
///
/// # Returns
///
/// Returns a job ID and status for tracking the analysis
///
/// # Examples
///
/// ```
/// // POST /api/v1/repositories/abc123/analyze
/// // Returns: {"job_id":"...","status":"queued","message":"..."}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        post,
        path = "/api/v1/repositories/{id}/analyze",
        tag = "analysis",
        params(
            ("id" = String, Path, description = "Repository unique identifier"),
        ),
        responses(
            (status = 200, description = "Re-analysis job queued", body = AnalyzeResponse),
            (status = 404, description = "Repository not found"),
        )
    )
)]
pub async fn analyze_repository_by_id(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement repository analysis by ID
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository ID: {}", id),
    })
}

/// List documentation endpoint
///
/// Returns a list of generated documentation
///
/// # Returns
///
/// Returns a list of documentation items
///
/// # Examples
///
/// ```
/// // GET /api/v1/documentation
/// // Returns: {"documents":[...],"total":0}
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/documentation",
        tag = "documentation",
        responses(
            (status = 200, description = "List of documentation", body = DocumentationListResponse),
        )
    )
)]
pub async fn list_documentation(State(_state): State<AppState>) -> impl IntoResponse {
    // TODO: Implement documentation listing from database
    Json(DocumentationListResponse {
        documents: vec![],
        total: 0,
    })
}

/// Get documentation by ID
///
/// Retrieves a specific documentation item
///
/// # Arguments
///
/// * `id` - Documentation ID
///
/// # Returns
///
/// Returns documentation details or 404 if not found
///
/// # Examples
///
/// ```
/// // GET /api/v1/documentation/xyz789
/// // Returns: documentation content or 404
/// ```
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/documentation/{id}",
        tag = "documentation",
        params(
            ("id" = String, Path, description = "Documentation unique identifier"),
        ),
        responses(
            (status = 200, description = "Documentation content", body = DocumentationInfo),
            (status = 404, description = "Documentation not found"),
        )
    )
)]
pub async fn get_documentation(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Implement documentation retrieval from database
    StatusCode::NOT_FOUND
}

// Response types

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct HealthResponse {
    /// Service status (healthy, degraded, unhealthy)
    #[cfg_attr(feature = "openapi", schema(example = "healthy"))]
    pub status: String,
    /// Application version
    #[cfg_attr(feature = "openapi", schema(example = "1.0.0"))]
    pub version: String,
    /// Current timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Version information response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct VersionResponse {
    /// Application version
    #[cfg_attr(feature = "openapi", schema(example = "1.0.0"))]
    pub version: String,
    /// API version
    #[cfg_attr(feature = "openapi", schema(example = "v1"))]
    pub api_version: String,
    /// Build information
    pub build_info: BuildInfo,
}

/// Build information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct BuildInfo {
    /// Git commit hash
    #[cfg_attr(feature = "openapi", schema(example = "abc123def456"))]
    pub commit: String,
    /// Build timestamp
    #[cfg_attr(feature = "openapi", schema(example = "2024-01-15T10:30:00Z"))]
    pub build_date: String,
}

/// Analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AnalyzeResponse {
    /// Unique job identifier
    #[cfg_attr(
        feature = "openapi",
        schema(example = "550e8400-e29b-41d4-a716-446655440000")
    )]
    pub job_id: String,
    /// Job status (queued, processing, completed, failed)
    #[cfg_attr(feature = "openapi", schema(example = "queued"))]
    pub status: String,
    /// Human-readable message
    #[cfg_attr(
        feature = "openapi",
        schema(example = "Analysis queued for repository")
    )]
    pub message: String,
}

/// Repository list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RepositoryListResponse {
    /// List of repositories
    pub repositories: Vec<RepositoryInfo>,
    /// Total number of repositories
    #[cfg_attr(feature = "openapi", schema(example = 42))]
    pub total: usize,
    /// Current page number
    #[cfg_attr(feature = "openapi", schema(example = 1))]
    pub page: usize,
    /// Items per page
    #[cfg_attr(feature = "openapi", schema(example = 10))]
    pub per_page: usize,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RepositoryInfo {
    /// Repository ID
    #[cfg_attr(feature = "openapi", schema(example = "repo-123"))]
    pub id: String,
    /// Repository name
    #[cfg_attr(feature = "openapi", schema(example = "xze"))]
    pub name: String,
    /// Repository URL
    #[cfg_attr(
        feature = "openapi",
        schema(example = "https://github.com/xbcsmith/xze")
    )]
    pub url: String,
    /// Primary language
    #[cfg_attr(feature = "openapi", schema(example = "rust"))]
    pub language: String,
    /// Last analysis timestamp
    pub last_analyzed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Documentation list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DocumentationListResponse {
    /// List of documentation items
    pub documents: Vec<DocumentationInfo>,
    /// Total number of documents
    #[cfg_attr(feature = "openapi", schema(example = 15))]
    pub total: usize,
}

/// Documentation information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DocumentationInfo {
    /// Documentation ID
    #[cfg_attr(feature = "openapi", schema(example = "doc-456"))]
    pub id: String,
    /// Document title
    #[cfg_attr(feature = "openapi", schema(example = "Getting Started Guide"))]
    pub title: String,
    /// Document category (tutorial, how-to, reference, explanation)
    #[cfg_attr(feature = "openapi", schema(example = "tutorial"))]
    pub category: String,
    /// Source repository ID
    #[cfg_attr(feature = "openapi", schema(example = "repo-123"))]
    pub repository_id: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Request types

/// Repository analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct AnalyzeRequest {
    /// Repository URL (https or git)
    #[cfg_attr(
        feature = "openapi",
        schema(example = "https://github.com/xbcsmith/xze")
    )]
    pub repository_url: String,
    /// Branch to analyze (defaults to main/master)
    #[cfg_attr(feature = "openapi", schema(example = "main"))]
    pub branch: Option<String>,
    /// Primary language (auto-detected if not provided)
    #[cfg_attr(feature = "openapi", schema(example = "rust"))]
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_version_response_serialization() {
        let response = VersionResponse {
            version: "1.0.0".to_string(),
            api_version: "v1".to_string(),
            build_info: BuildInfo {
                commit: "abc123".to_string(),
                build_date: "2024-01-01".to_string(),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("v1"));
        assert!(json.contains("abc123"));
    }

    #[test]
    fn test_analyze_request_deserialization() {
        let json = r#"{"repository_url":"https://github.com/test/repo","branch":"main","language":"rust"}"#;
        let request: AnalyzeRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.repository_url, "https://github.com/test/repo");
        assert_eq!(request.branch, Some("main".to_string()));
        assert_eq!(request.language, Some("rust".to_string()));
    }

    #[test]
    fn test_analyze_response_serialization() {
        let response = AnalyzeResponse {
            job_id: "job-123".to_string(),
            status: "queued".to_string(),
            message: "Analysis queued".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("job-123"));
        assert!(json.contains("queued"));
    }

    #[test]
    fn test_repository_info_serialization() {
        let info = RepositoryInfo {
            id: "repo-1".to_string(),
            name: "test-repo".to_string(),
            url: "https://github.com/test/repo".to_string(),
            language: "rust".to_string(),
            last_analyzed: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("repo-1"));
        assert!(json.contains("test-repo"));
        assert!(json.contains("rust"));
    }

    #[test]
    fn test_repository_list_response_serialization() {
        let response = RepositoryListResponse {
            repositories: vec![],
            total: 0,
            page: 1,
            per_page: 10,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"per_page\":10"));
    }

    #[test]
    fn test_documentation_info_fields() {
        let doc = DocumentationInfo {
            id: "doc-1".to_string(),
            title: "Getting Started".to_string(),
            category: "tutorial".to_string(),
            repository_id: "repo-1".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(doc.id, "doc-1");
        assert_eq!(doc.category, "tutorial");
        assert_eq!(doc.repository_id, "repo-1");
    }
}
