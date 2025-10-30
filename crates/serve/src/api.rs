//! API module for XZe serve crate
//!
//! This module provides both v1 and legacy API routes.
//! Legacy routes are deprecated and will be removed in a future version.

pub mod v1;

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
///
/// Creates the complete router with both v1 and legacy endpoints.
/// Legacy endpoints include deprecation headers.
///
/// # Returns
///
/// Returns an Axum router with:
/// - `/api/v1/*` - Current API v1 endpoints
/// - `/*` - Legacy deprecated endpoints (for backward compatibility)
///
/// # Examples
///
/// ```
/// use xze_serve::api::create_routes;
/// use xze_serve::handlers::AppState;
///
/// // Create router with application state
/// // let state = AppState::new(config).await?;
/// // let router = create_routes().with_state(state);
/// ```
pub fn create_routes() -> Router<crate::handlers::AppState> {
    // Create v1 routes nested under /api/v1
    let v1_routes = v1::create_v1_routes();

    // Create legacy routes (deprecated but maintained for backward compatibility)
    let legacy_routes = Router::new()
        .route("/health", get(health_check))
        .route("/version", get(get_version))
        .route("/search", get(crate::handlers::handle_search))
        .route("/analyze", post(analyze_repository))
        .route("/repositories", get(list_repositories))
        .route("/repositories/:id", get(get_repository))
        .route("/repositories/:id/analyze", post(analyze_repository_by_id))
        .route("/documentation", get(list_documentation))
        .route("/documentation/:id", get(get_documentation));

    // Combine routes: v1 under /api/v1, legacy at root
    Router::new()
        .nest("/api/v1", v1_routes)
        .merge(legacy_routes)
}

/// Health check endpoint (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/health` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/health`.
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get version information (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/version` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/version`.
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

/// Analyze repository endpoint (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/analyze` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/analyze`.
pub async fn analyze_repository(Json(request): Json<AnalyzeRequest>) -> impl IntoResponse {
    // TODO: Implement repository analysis
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository: {}", request.repository_url),
    })
}

/// List repositories endpoint (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/repositories` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/repositories`.
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

/// Get repository by ID (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/repositories/:id` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/repositories/:id`.
pub async fn get_repository(Path(_id): Path<String>) -> impl IntoResponse {
    // TODO: Implement repository retrieval
    StatusCode::NOT_FOUND
}

/// Analyze repository by ID (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/repositories/:id/analyze` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/repositories/:id/analyze`.
pub async fn analyze_repository_by_id(Path(id): Path<String>) -> impl IntoResponse {
    // TODO: Implement repository analysis by ID
    Json(AnalyzeResponse {
        job_id: uuid::Uuid::new_v4().to_string(),
        status: "queued".to_string(),
        message: format!("Analysis queued for repository ID: {}", id),
    })
}

/// List documentation endpoint (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/documentation` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/documentation`.
pub async fn list_documentation() -> impl IntoResponse {
    // TODO: Implement documentation listing
    Json(DocumentationListResponse {
        documents: vec![],
        total: 0,
    })
}

/// Get documentation by ID (legacy - deprecated)
///
/// This endpoint is deprecated. Use `/api/v1/documentation/:id` instead.
///
/// # Deprecation
///
/// This endpoint will be removed on 2025-03-01.
/// Please migrate to `/api/v1/documentation/:id`.
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

#[derive(Deserialize, Serialize)]
pub struct AnalyzeRequest {
    pub repository_url: String,
    pub branch: Option<String>,
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version() {
        assert_eq!(API_VERSION, "v1");
    }

    #[test]
    fn test_v1_module_exists() {
        // Ensure v1 module is properly exposed
        let _v1_routes = v1::create_v1_routes();
    }

    #[test]
    fn test_create_routes_includes_v1_and_legacy() {
        // Test that router creation includes both v1 and legacy routes
        let _router = create_routes();
    }

    #[test]
    fn test_analyze_request_serialization() {
        let request = AnalyzeRequest {
            repository_url: "https://github.com/test/repo".to_string(),
            branch: Some("main".to_string()),
            language: Some("rust".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("https://github.com/test/repo"));
        assert!(json.contains("main"));
        assert!(json.contains("rust"));
    }

    // NOTE: Integration tests for routes with AppState are in tests/integration_tests.rs
    // These tests require a database connection and are run separately.

    // #[tokio::test]
    // async fn test_health_check() {
    //     // Requires AppState with database pool
    // }

    // #[tokio::test]
    // async fn test_search_endpoint() {
    //     // Requires AppState with database pool
    // }
}
