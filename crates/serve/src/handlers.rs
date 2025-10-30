//! HTTP handlers for XZe serve crate

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub ollama_url: String,
    pub database_pool: PgPool,
    pub config: crate::ServerConfig,
}

impl AppState {
    /// Create new application state with database connection
    pub async fn new(config: crate::ServerConfig) -> Result<Self, sqlx::Error> {
        let database_pool = PgPool::connect(&config.database_url).await?;

        Ok(Self {
            ollama_url: config.ollama_url.clone(),
            database_pool,
            config,
        })
    }

    /// Create application state from existing pool (for testing)
    pub fn from_pool(config: crate::ServerConfig, pool: PgPool) -> Self {
        Self {
            ollama_url: config.ollama_url.clone(),
            database_pool: pool,
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

/// Handler for semantic search
#[cfg_attr(
    feature = "openapi",
    utoipa::path(
        get,
        path = "/api/v1/search",
        tag = "search",
        params(
            ("q" = String, Query, description = "Search query text"),
            ("max_results" = Option<usize>, Query, description = "Maximum number of results (default: 10)"),
            ("min_similarity" = Option<f32>, Query, description = "Minimum similarity threshold 0.0-1.0 (default: 0.0)"),
            ("category" = Option<String>, Query, description = "Filter by documentation category (tutorial, how-to, reference, explanation)"),
        ),
        responses(
            (status = 200, description = "Search results with similarity scores", body = SearchResponse),
            (status = 400, description = "Invalid search parameters", body = SearchErrorResponse),
            (status = 502, description = "Failed to generate embedding", body = SearchErrorResponse),
            (status = 500, description = "Internal search error", body = SearchErrorResponse),
        )
    )
)]
pub async fn handle_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    use xze_core::semantic::search::{search_with_chunks, SearchConfig};

    tracing::info!(
        "Search request: query='{}', max_results={}, min_similarity={}, category={:?}",
        params.q,
        params.max_results.unwrap_or(10),
        params.min_similarity.unwrap_or(0.0),
        params.category
    );

    // Validate query
    if params.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(SearchErrorResponse {
                error: "Query string cannot be empty".to_string(),
                details: None,
            }),
        )
            .into_response();
    }

    // Build search config
    let config = SearchConfig {
        max_results: params.max_results.unwrap_or(10),
        min_similarity: params.min_similarity.unwrap_or(0.0),
        category_filter: params.category.clone(),
    };

    // Perform search
    match search_with_chunks(&state.database_pool, &params.q, &state.ollama_url, &config).await {
        Ok(results) => {
            let total_results = results.len();
            let response = SearchResponse {
                query: params.q.clone(),
                results: results
                    .into_iter()
                    .map(|r| SearchResultItem {
                        id: r.id,
                        source_file: r.source_file,
                        content: r.content,
                        similarity: r.similarity,
                        chunk_index: r.chunk_index,
                        total_chunks: r.total_chunks,
                        title: r.title,
                        category: r.category,
                        sentence_range: r.sentence_range,
                        avg_chunk_similarity: r.avg_chunk_similarity,
                    })
                    .collect(),
                total_results,
                config: SearchConfigResponse {
                    max_results: config.max_results,
                    min_similarity: config.min_similarity,
                    category_filter: config.category_filter,
                },
            };

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Search error: {}", e);

            let (status, error_msg) = match e {
                xze_core::semantic::search::SearchError::EmptyQuery => {
                    (StatusCode::BAD_REQUEST, "Query string cannot be empty")
                }
                xze_core::semantic::search::SearchError::InvalidConfig(ref msg) => {
                    (StatusCode::BAD_REQUEST, msg.as_str())
                }
                xze_core::semantic::search::SearchError::Database(ref db_err) => {
                    tracing::error!("Database error: {}", db_err);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred")
                }
                xze_core::semantic::search::SearchError::EmbeddingGeneration(_) => (
                    StatusCode::BAD_GATEWAY,
                    "Failed to generate query embedding",
                ),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal search error"),
            };

            (
                status,
                Json(SearchErrorResponse {
                    error: error_msg.to_string(),
                    details: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

// Search request/response types

/// Search query parameters
#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchQueryParams {
    /// Search query text
    #[cfg_attr(
        feature = "openapi",
        schema(example = "how to implement authentication")
    )]
    pub q: String,
    /// Maximum number of results to return
    #[cfg_attr(feature = "openapi", schema(example = 10))]
    pub max_results: Option<usize>,
    /// Minimum similarity threshold (0.0 to 1.0)
    #[cfg_attr(feature = "openapi", schema(example = 0.7))]
    pub min_similarity: Option<f32>,
    /// Filter by documentation category
    #[cfg_attr(feature = "openapi", schema(example = "tutorial"))]
    pub category: Option<String>,
}

/// Search response with results
#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchResponse {
    /// Original search query
    #[cfg_attr(
        feature = "openapi",
        schema(example = "how to implement authentication")
    )]
    pub query: String,
    /// List of search results ordered by similarity
    pub results: Vec<SearchResultItem>,
    /// Total number of results found
    #[cfg_attr(feature = "openapi", schema(example = 5))]
    pub total_results: usize,
    /// Search configuration used
    pub config: SearchConfigResponse,
}

/// Individual search result item
#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchResultItem {
    /// Unique identifier for this document chunk
    pub id: i64,
    /// Source file path
    #[cfg_attr(feature = "openapi", schema(example = "docs/how_to/authentication.md"))]
    pub source_file: String,
    /// Content text of the matching chunk
    #[cfg_attr(
        feature = "openapi",
        schema(example = "To implement authentication, first configure the JWT middleware...")
    )]
    pub content: String,
    /// Similarity score (0.0 to 1.0, higher is better)
    #[cfg_attr(feature = "openapi", schema(example = 0.87))]
    pub similarity: f32,
    /// Index of this chunk within the document
    #[cfg_attr(feature = "openapi", schema(example = 2))]
    pub chunk_index: i32,
    /// Total number of chunks in the document
    #[cfg_attr(feature = "openapi", schema(example = 5))]
    pub total_chunks: i32,
    /// Document title
    #[cfg_attr(feature = "openapi", schema(example = "Authentication Guide"))]
    pub title: Option<String>,
    /// Documentation category
    #[cfg_attr(feature = "openapi", schema(example = "how-to"))]
    pub category: Option<String>,
    /// Range of sentences in this chunk (start, end)
    pub sentence_range: (i32, i32),
    /// Average similarity across all chunks in document
    #[cfg_attr(feature = "openapi", schema(example = 0.82))]
    pub avg_chunk_similarity: f32,
}

/// Search configuration details
#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchConfigResponse {
    /// Maximum results requested
    #[cfg_attr(feature = "openapi", schema(example = 10))]
    pub max_results: usize,
    /// Minimum similarity threshold applied
    #[cfg_attr(feature = "openapi", schema(example = 0.0))]
    pub min_similarity: f32,
    /// Category filter applied
    #[cfg_attr(feature = "openapi", schema(example = "tutorial"))]
    pub category_filter: Option<String>,
}

/// Search error response
#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SearchErrorResponse {
    /// Error message
    #[cfg_attr(feature = "openapi", schema(example = "Query string cannot be empty"))]
    pub error: String,
    /// Additional error details
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_handler() {
        // Just verify the handler can be called without panicking
        // Full integration tests are in tests/ directory
        let _response = handle_health_check().await;
        // Handler returns impl IntoResponse, actual testing requires integration tests
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

    #[test]
    fn test_search_query_params_deserialization() {
        // Test basic query
        let params = SearchQueryParams {
            q: "test query".to_string(),
            max_results: Some(10),
            min_similarity: Some(0.5),
            category: None,
        };
        assert_eq!(params.q, "test query");
        assert_eq!(params.max_results, Some(10));
        assert_eq!(params.min_similarity, Some(0.5));
        assert!(params.category.is_none());

        // Test with category
        let params_with_category = SearchQueryParams {
            q: "another query".to_string(),
            max_results: None,
            min_similarity: None,
            category: Some("tutorial".to_string()),
        };
        assert_eq!(params_with_category.category, Some("tutorial".to_string()));
    }

    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse {
            query: "test query".to_string(),
            results: vec![SearchResultItem {
                id: 1,
                source_file: "docs/test.md".to_string(),
                content: "Test content".to_string(),
                similarity: 0.95,
                chunk_index: 0,
                total_chunks: 3,
                title: Some("Test Document".to_string()),
                category: Some("tutorial".to_string()),
                sentence_range: (0, 5),
                avg_chunk_similarity: 0.85,
            }],
            total_results: 1,
            config: SearchConfigResponse {
                max_results: 10,
                min_similarity: 0.0,
                category_filter: None,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("docs/test.md"));
        assert!(json.contains("Test content"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_search_error_response_serialization() {
        let error_response = SearchErrorResponse {
            error: "Query string cannot be empty".to_string(),
            details: Some("Additional error details".to_string()),
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("Query string cannot be empty"));
        assert!(json.contains("Additional error details"));
    }

    #[test]
    fn test_search_result_item_fields() {
        let item = SearchResultItem {
            id: 42,
            source_file: "path/to/file.md".to_string(),
            content: "Sample content".to_string(),
            similarity: 0.87,
            chunk_index: 2,
            total_chunks: 5,
            title: Some("Sample Title".to_string()),
            category: Some("how_to".to_string()),
            sentence_range: (10, 20),
            avg_chunk_similarity: 0.75,
        };

        assert_eq!(item.id, 42);
        assert_eq!(item.source_file, "path/to/file.md");
        assert_eq!(item.similarity, 0.87);
        assert_eq!(item.chunk_index, 2);
        assert_eq!(item.total_chunks, 5);
        assert_eq!(item.sentence_range, (10, 20));
    }

    #[test]
    fn test_search_config_response() {
        let config = SearchConfigResponse {
            max_results: 20,
            min_similarity: 0.6,
            category_filter: Some("reference".to_string()),
        };

        assert_eq!(config.max_results, 20);
        assert_eq!(config.min_similarity, 0.6);
        assert_eq!(config.category_filter, Some("reference".to_string()));
    }
}
