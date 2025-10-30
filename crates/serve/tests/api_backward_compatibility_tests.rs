//! API backward compatibility integration tests
//!
//! Tests legacy endpoints, deprecation headers, and migration path.
//!
//! Note: These integration tests require a running PostgreSQL database.
//! They are disabled by default. To run them, set the DATABASE_URL environment variable
//! and use: cargo test --test api_backward_compatibility_tests -- --ignored

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use xze_serve::api::create_routes;

#[test]
fn test_legacy_api_module_accessible() {
    // Compile-time test that legacy API functions are still accessible
    use xze_serve::api::create_routes;
    let _routes = create_routes();
}

#[test]
fn test_middleware_exists_for_deprecation() {
    // Compile-time test that deprecation middleware exists
    use xze_serve::middleware::legacy_deprecation_middleware;
    let _middleware = legacy_deprecation_middleware;
}

// The following tests require a database connection and are ignored by default.
// To run them: cargo test --test api_backward_compatibility_tests -- --ignored --test-threads=1
// Make sure DATABASE_URL environment variable is set to a test database.

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
async fn test_legacy_health_endpoint_still_works() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_legacy_version_endpoint_still_works() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn test_legacy_health_has_deprecation_header() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let deprecation = response.headers().get("deprecation");
    assert!(deprecation.is_some(), "Deprecation header missing");
    assert_eq!(deprecation.unwrap(), "true");
}

#[tokio::test]
#[ignore]
async fn test_legacy_health_has_sunset_header() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let sunset = response.headers().get("sunset");
    assert!(sunset.is_some(), "Sunset header missing");
    assert_eq!(sunset.unwrap(), "Sat, 01 Mar 2025 00:00:00 GMT");
}

#[tokio::test]
#[ignore]
async fn test_legacy_health_has_link_header() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let link = response.headers().get("link");
    assert!(link.is_some(), "Link header missing");
    assert_eq!(link.unwrap(), "</api/v1/docs>; rel=\"successor-version\"");
}

#[tokio::test]
#[ignore]
async fn test_legacy_health_has_warning_header() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let warning = response.headers().get("warning");
    assert!(warning.is_some(), "Warning header missing");
    assert!(warning.unwrap().to_str().unwrap().contains("deprecated"));
}

#[tokio::test]
#[ignore]
async fn test_legacy_version_has_all_deprecation_headers() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(response.headers().get("deprecation").is_some());
    assert!(response.headers().get("sunset").is_some());
    assert!(response.headers().get("link").is_some());
    assert!(response.headers().get("warning").is_some());
}

#[tokio::test]
#[ignore]
async fn test_legacy_repositories_endpoint_has_deprecation_headers() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/repositories")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("deprecation").is_some());
    assert!(response.headers().get("sunset").is_some());
}

#[tokio::test]
#[ignore]
async fn test_legacy_documentation_endpoint_has_deprecation_headers() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/documentation")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("deprecation").is_some());
}

#[tokio::test]
#[ignore]
async fn test_legacy_search_endpoint_has_deprecation_headers() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/search?q=test")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(response.headers().get("deprecation").is_some());
}

#[tokio::test]
#[ignore]
async fn test_v1_endpoints_do_not_have_deprecation_headers() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("deprecation").is_none());
    assert!(response.headers().get("sunset").is_none());
    assert!(response.headers().get("warning").is_none());
}

#[tokio::test]
#[ignore]
async fn test_legacy_and_v1_health_return_same_structure() {
    let state = create_test_state().await;

    // Test legacy endpoint
    let app_legacy = create_routes().with_state(state.clone());
    let request_legacy = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_legacy = app_legacy.oneshot(request_legacy).await.unwrap();
    let body_legacy = axum::body::to_bytes(response_legacy.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_legacy: Value = serde_json::from_slice(&body_legacy).unwrap();

    // Test v1 endpoint
    let app_v1 = create_routes().with_state(state);
    let request_v1 = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_v1 = app_v1.oneshot(request_v1).await.unwrap();
    let body_v1 = axum::body::to_bytes(response_v1.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_v1: Value = serde_json::from_slice(&body_v1).unwrap();

    // Both should have the same fields
    assert!(json_legacy.get("status").is_some());
    assert!(json_v1.get("status").is_some());
    assert!(json_legacy.get("version").is_some());
    assert!(json_v1.get("version").is_some());
    assert!(json_legacy.get("timestamp").is_some());
    assert!(json_v1.get("timestamp").is_some());
}

#[tokio::test]
#[ignore]
async fn test_legacy_and_v1_version_return_same_structure() {
    let state = create_test_state().await;

    // Test legacy endpoint
    let app_legacy = create_routes().with_state(state.clone());
    let request_legacy = Request::builder()
        .uri("/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_legacy = app_legacy.oneshot(request_legacy).await.unwrap();
    let body_legacy = axum::body::to_bytes(response_legacy.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_legacy: Value = serde_json::from_slice(&body_legacy).unwrap();

    // Test v1 endpoint
    let app_v1 = create_routes().with_state(state);
    let request_v1 = Request::builder()
        .uri("/api/v1/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response_v1 = app_v1.oneshot(request_v1).await.unwrap();
    let body_v1 = axum::body::to_bytes(response_v1.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_v1: Value = serde_json::from_slice(&body_v1).unwrap();

    // Both should have the same structure
    assert_eq!(
        json_legacy.get("api_version"),
        json_v1.get("api_version"),
        "API version mismatch"
    );
    assert!(json_legacy.get("build_info").is_some());
    assert!(json_v1.get("build_info").is_some());
}

#[tokio::test]
#[ignore]
async fn test_legacy_analyze_endpoint_still_works() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let body = serde_json::json!({
        "repository_url": "https://github.com/test/repo",
        "branch": "main"
    });

    let request = Request::builder()
        .uri("/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("deprecation").is_some());
}

#[tokio::test]
#[ignore]
async fn test_all_legacy_endpoints_functional() {
    let state = create_test_state().await;

    let endpoints = vec![
        ("/health", "GET"),
        ("/version", "GET"),
        ("/repositories", "GET"),
        ("/documentation", "GET"),
    ];

    for (endpoint, method) in endpoints {
        let app = create_routes().with_state(state.clone());

        let request = Request::builder()
            .uri(endpoint)
            .method(method)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Legacy endpoint {} {} should still work",
            method,
            endpoint
        );
        assert!(
            response.headers().get("deprecation").is_some(),
            "Legacy endpoint {} {} should have deprecation header",
            method,
            endpoint
        );
    }
}
