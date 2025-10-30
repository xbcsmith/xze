//! API versioning integration tests
//!
//! Tests API v1 endpoints, version negotiation, and header handling.
//!
//! Note: These integration tests require a running PostgreSQL database.
//! They are disabled by default. To run them, set the DATABASE_URL environment variable
//! and use: cargo test --test api_versioning_tests -- --ignored

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use xze_serve::api::create_routes;
use xze_serve::api::v1;

#[test]
fn test_v1_module_exists() {
    // Compile-time test that v1 module is accessible
    let _routes = v1::create_v1_routes();
}

#[test]
fn test_v1_handlers_module_exists() {
    // Compile-time test that v1 handlers are accessible
    use xze_serve::api::v1::handlers::*;

    // Verify request types exist
    let _request = AnalyzeRequest {
        repository_url: "https://github.com/test/repo".to_string(),
        branch: Some("main".to_string()),
        language: Some("rust".to_string()),
    };
}

#[test]
fn test_v1_response_types_serializable() {
    use xze_serve::api::v1::handlers::*;

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("healthy"));
}

#[test]
fn test_middleware_functions_exist() {
    // Compile-time test that middleware is accessible
    use xze_serve::middleware::{api_version_middleware, legacy_deprecation_middleware};

    let _api_version = api_version_middleware;
    let _legacy = legacy_deprecation_middleware;
}

// The following tests require a database connection and are ignored by default.
// To run them: cargo test --test api_versioning_tests -- --ignored --test-threads=1
// Make sure DATABASE_URL environment variable is set to a test database.

#[tokio::test]
#[ignore]
async fn test_v1_health_endpoint_returns_200() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check for API-Version header
    let api_version = response.headers().get("api-version");
    assert!(api_version.is_some());
    assert_eq!(api_version.unwrap(), "v1");
}

#[tokio::test]
#[ignore]
async fn test_v1_version_endpoint_returns_correct_format() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check response structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("version").is_some());
    assert!(json.get("api_version").is_some());
    assert!(json.get("build_info").is_some());
    assert_eq!(json["api_version"], "v1");
}

#[tokio::test]
#[ignore]
async fn test_accept_version_header_v1_is_accepted() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "v1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_accept_version_header_1_is_accepted() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_accept_version_header_1_0_is_accepted() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "1.0")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_accept_version_header_invalid_returns_400() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "v2")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Check error message
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("error").is_some());
    assert_eq!(json["error"], "Invalid API version");
}

#[tokio::test]
#[ignore]
async fn test_accept_version_header_v99_returns_400() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "v99")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn test_v1_repositories_endpoint_exists() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/repositories")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 200 with empty list (not 404)
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("repositories").is_some());
    assert!(json.get("total").is_some());
}

#[tokio::test]
#[ignore]
async fn test_v1_documentation_endpoint_exists() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/documentation")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("documents").is_some());
    assert!(json.get("total").is_some());
}

#[tokio::test]
#[ignore]
async fn test_v1_search_endpoint_exists() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/search?q=test")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should not return 404 (endpoint exists)
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore]
async fn test_api_version_header_present_in_all_v1_responses() {
    let state = create_test_state().await;

    let endpoints = vec![
        "/api/v1/health",
        "/api/v1/version",
        "/api/v1/repositories",
        "/api/v1/documentation",
    ];

    for endpoint in endpoints {
        let app = create_routes().with_state(state.clone());

        let request = Request::builder()
            .uri(endpoint)
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let api_version = response.headers().get("api-version");
        assert!(
            api_version.is_some(),
            "API-Version header missing for endpoint: {}",
            endpoint
        );
        assert_eq!(
            api_version.unwrap(),
            "v1",
            "API-Version header incorrect for endpoint: {}",
            endpoint
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_v1_analyze_endpoint_accepts_post() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let body = serde_json::json!({
        "repository_url": "https://github.com/test/repo",
        "branch": "main",
        "language": "rust"
    });

    let request = Request::builder()
        .uri("/api/v1/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("job_id").is_some());
    assert!(json.get("status").is_some());
    assert_eq!(json["status"], "queued");
}

#[tokio::test]
#[ignore]
async fn test_v1_endpoints_have_consistent_structure() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Test health endpoint structure
    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health: Value = serde_json::from_slice(&body).unwrap();

    assert!(health.is_object());
    assert!(health.get("status").is_some());
    assert!(health.get("version").is_some());
    assert!(health.get("timestamp").is_some());
}

// Helper function for ignored tests
#[allow(dead_code)]
async fn create_test_state() -> xze_serve::handlers::AppState {
    use xze_serve::{handlers::AppState, ServerConfig};

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/xze_test".to_string());

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        ollama_url: "http://localhost:11434".to_string(),
        database_url,
        cors_enabled: false,
        max_request_size: 10 * 1024 * 1024,
    };

    AppState::new(config)
        .await
        .expect("Failed to create test state - ensure DATABASE_URL is set to a valid test database")
}

#[tokio::test]
#[ignore]
async fn test_v1_pagination_query_params_work() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/repositories?page=2&per_page=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["page"], 2);
    assert_eq!(json["per_page"], 20);
}

// Breaking Change Detection Tests

#[tokio::test]
#[ignore]
async fn test_v1_health_response_structure_stable() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // These fields MUST exist for v1 compatibility
    let required_fields = vec!["status", "version", "timestamp"];

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Breaking change: v1 health response missing required field '{}'",
            field
        );
    }

    // Field types must remain stable
    assert!(
        json["status"].is_string(),
        "Breaking change: 'status' field type changed"
    );
    assert!(
        json["version"].is_string(),
        "Breaking change: 'version' field type changed"
    );
    assert!(
        json["timestamp"].is_string(),
        "Breaking change: 'timestamp' field type changed"
    );
}

#[tokio::test]
#[ignore]
async fn test_v1_version_response_structure_stable() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Required top-level fields
    let required_fields = vec!["version", "api_version", "build_info"];

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Breaking change: v1 version response missing required field '{}'",
            field
        );
    }

    // Build info required fields
    let build_info = &json["build_info"];
    let build_info_fields = vec!["git_hash", "build_timestamp", "rust_version"];

    for field in build_info_fields {
        assert!(
            build_info.get(field).is_some(),
            "Breaking change: build_info missing required field '{}'",
            field
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_v1_repositories_response_structure_stable() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/repositories")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Required fields for v1 compatibility
    let required_fields = vec!["repositories", "total", "page", "per_page"];

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Breaking change: v1 repositories response missing required field '{}'",
            field
        );
    }

    // Type stability
    assert!(
        json["repositories"].is_array(),
        "Breaking change: 'repositories' field must be array"
    );
    assert!(
        json["total"].is_number(),
        "Breaking change: 'total' field must be number"
    );
}

#[tokio::test]
#[ignore]
async fn test_v1_analyze_request_accepts_all_documented_fields() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Test that all documented fields are accepted
    let body = serde_json::json!({
        "repository_url": "https://github.com/test/repo",
        "branch": "main",
        "language": "rust"
    });

    let request = Request::builder()
        .uri("/api/v1/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Breaking change: v1 analyze endpoint rejected valid request format"
    );
}

#[tokio::test]
#[ignore]
async fn test_v1_analyze_response_structure_stable() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let body = serde_json::json!({
        "repository_url": "https://github.com/test/repo",
        "branch": "main"
    });

    let request = Request::builder()
        .uri("/api/v1/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Required response fields
    let required_fields = vec!["job_id", "status"];

    for field in required_fields {
        assert!(
            json.get(field).is_some(),
            "Breaking change: v1 analyze response missing required field '{}'",
            field
        );
    }
}

// Migration Path Validation Tests

#[tokio::test]
#[ignore]
async fn test_migration_path_legacy_to_v1_health() {
    let state = create_test_state().await;

    // Step 1: Test legacy endpoint works
    let app_legacy = create_routes().with_state(state.clone());
    let request_legacy = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_legacy = app_legacy.oneshot(request_legacy).await.unwrap();
    assert_eq!(response_legacy.status(), StatusCode::OK);

    // Verify deprecation headers present
    assert!(response_legacy.headers().get("deprecation").is_some());
    assert!(response_legacy.headers().get("sunset").is_some());
    assert!(response_legacy.headers().get("link").is_some());

    // Step 2: Test v1 endpoint works
    let app_v1 = create_routes().with_state(state);
    let request_v1 = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_v1 = app_v1.oneshot(request_v1).await.unwrap();
    assert_eq!(response_v1.status(), StatusCode::OK);

    // Verify no deprecation headers on v1
    assert!(response_v1.headers().get("deprecation").is_none());
    assert!(response_v1.headers().get("sunset").is_none());
}

#[tokio::test]
#[ignore]
async fn test_migration_path_legacy_to_v1_analyze() {
    let state = create_test_state().await;

    let body = serde_json::json!({
        "repository_url": "https://github.com/test/repo",
        "branch": "main"
    });

    // Step 1: Legacy endpoint works
    let app_legacy = create_routes().with_state(state.clone());
    let request_legacy = Request::builder()
        .uri("/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response_legacy = app_legacy.oneshot(request_legacy).await.unwrap();
    assert_eq!(response_legacy.status(), StatusCode::OK);
    assert!(response_legacy.headers().get("deprecation").is_some());

    // Step 2: V1 endpoint works with same request
    let app_v1 = create_routes().with_state(state);
    let request_v1 = Request::builder()
        .uri("/api/v1/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response_v1 = app_v1.oneshot(request_v1).await.unwrap();
    assert_eq!(response_v1.status(), StatusCode::OK);
    assert!(response_v1.headers().get("deprecation").is_none());
}

#[tokio::test]
#[ignore]
async fn test_migration_path_client_can_use_both_endpoints() {
    let state = create_test_state().await;

    // Simulate client using both endpoints during migration
    let endpoints = vec![
        ("/health", "/api/v1/health"),
        ("/version", "/api/v1/version"),
        ("/repositories", "/api/v1/repositories"),
        ("/documentation", "/api/v1/documentation"),
    ];

    for (legacy, v1) in endpoints {
        // Test legacy
        let app_legacy = create_routes().with_state(state.clone());
        let request_legacy = Request::builder()
            .uri(legacy)
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response_legacy = app_legacy.oneshot(request_legacy).await.unwrap();
        assert_eq!(
            response_legacy.status(),
            StatusCode::OK,
            "Migration path broken: legacy endpoint {} not working",
            legacy
        );

        // Test v1
        let app_v1 = create_routes().with_state(state.clone());
        let request_v1 = Request::builder()
            .uri(v1)
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response_v1 = app_v1.oneshot(request_v1).await.unwrap();
        assert_eq!(
            response_v1.status(),
            StatusCode::OK,
            "Migration path broken: v1 endpoint {} not working",
            v1
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_deprecation_headers_provide_migration_information() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify sunset date is in the future and provides time to migrate
    let sunset = response.headers().get("sunset").unwrap().to_str().unwrap();
    assert!(
        !sunset.is_empty(),
        "Sunset header must provide specific date"
    );

    // Verify link header provides successor version
    let link = response.headers().get("link").unwrap().to_str().unwrap();
    assert!(link.contains("/api/v1"), "Link header must point to v1 API");
    assert!(
        link.contains("successor-version"),
        "Link header must indicate successor version"
    );

    // Verify warning provides migration guidance
    let warning = response.headers().get("warning").unwrap().to_str().unwrap();
    assert!(
        warning.contains("deprecated"),
        "Warning must indicate deprecation"
    );
    assert!(
        warning.contains("/api/v1"),
        "Warning must mention v1 migration path"
    );
}

// Version Negotiation Tests

#[tokio::test]
#[ignore]
async fn test_version_negotiation_with_exact_match() {
    let versions = vec!["v1", "1", "1.0"];

    for version in versions {
        let state = create_test_state().await;
        let app = create_routes().with_state(state);
        let request = Request::builder()
            .uri("/api/v1/health")
            .method("GET")
            .header("accept-version", version)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Version negotiation failed for accept-version: {}",
            version
        );

        let api_version = response.headers().get("api-version").unwrap();
        assert_eq!(api_version, "v1");
    }
}

#[tokio::test]
#[ignore]
async fn test_version_negotiation_rejects_unsupported_versions() {
    let state = create_test_state().await;

    let invalid_versions = vec!["v0", "v2", "v99", "2.0", "latest", "alpha"];

    for version in invalid_versions {
        let app = create_routes().with_state(state.clone());
        let request = Request::builder()
            .uri("/api/v1/health")
            .method("GET")
            .header("accept-version", version)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "Should reject unsupported version: {}",
            version
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert!(json.get("error").is_some());
        assert!(json.get("supported_versions").is_some());
    }
}

#[tokio::test]
#[ignore]
async fn test_version_negotiation_without_header_defaults_to_path_version() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Request without accept-version header
    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Should still return api-version header
    let api_version = response.headers().get("api-version").unwrap();
    assert_eq!(api_version, "v1");
}

#[tokio::test]
#[ignore]
async fn test_all_v1_endpoints_support_version_negotiation() {
    let state = create_test_state().await;

    let endpoints = vec![
        "/api/v1/health",
        "/api/v1/version",
        "/api/v1/repositories",
        "/api/v1/documentation",
    ];

    for endpoint in endpoints {
        let app = create_routes().with_state(state.clone());
        let request = Request::builder()
            .uri(endpoint)
            .method("GET")
            .header("accept-version", "v1")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Version negotiation failed for endpoint: {}",
            endpoint
        );
    }
}
