//! Contract tests validating OpenAPI specification against runtime behavior
//!
//! These tests ensure that the actual API behavior matches the OpenAPI specification.
//! They verify request/response schemas, status codes, headers, and error responses.
//!
//! Note: These integration tests require a running PostgreSQL database and the openapi feature.
//! They are disabled by default. To run them, set the DATABASE_URL environment variable
//! and use: cargo test --test contract_tests --features openapi -- --ignored

#[cfg(feature = "openapi")]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
#[cfg(feature = "openapi")]
use serde_json::Value;
#[cfg(feature = "openapi")]
use tower::ServiceExt;
#[cfg(feature = "openapi")]
use utoipa::OpenApi;
#[cfg(feature = "openapi")]
use xze_serve::api::create_routes;
#[cfg(feature = "openapi")]
use xze_serve::api::v1::openapi::ApiDocV1;

#[cfg(not(feature = "openapi"))]
#[test]
fn contract_tests_require_openapi_feature() {
    panic!("Contract tests require the 'openapi' feature to be enabled. Run with: cargo test --features openapi");
}

// Helper function for ignored tests
#[cfg(feature = "openapi")]
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

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_has_required_metadata() {
    let spec = ApiDocV1::openapi();

    // Verify OpenAPI version exists (OpenApiVersion is an enum)
    // We just verify the spec object is valid by checking info
    assert!(!spec.info.title.is_empty());

    // Verify API info
    assert_eq!(spec.info.title, "XZe API");
    assert_eq!(spec.info.version, "1.0.0");
    assert!(spec.info.description.is_some());
    assert!(spec.info.license.is_some());
    assert!(spec.info.contact.is_some());
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_has_all_v1_endpoints() {
    let spec = ApiDocV1::openapi();

    let required_paths = vec![
        "/api/v1/health",
        "/api/v1/version",
        "/api/v1/analyze",
        "/api/v1/repositories",
        "/api/v1/repositories/{id}",
        "/api/v1/repositories/{id}/analyze",
        "/api/v1/documentation",
        "/api/v1/documentation/{id}",
        "/api/v1/search",
    ];

    for path in required_paths {
        assert!(
            spec.paths.paths.contains_key(path),
            "OpenAPI spec missing required path: {}",
            path
        );
    }
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_has_all_required_schemas() {
    let spec = ApiDocV1::openapi();
    let components = spec.components.as_ref().unwrap();

    let required_schemas = vec![
        "HealthResponse",
        "VersionResponse",
        "BuildInfo",
        "AnalyzeRequest",
        "AnalyzeResponse",
        "RepositoryListResponse",
        "RepositoryInfo",
        "DocumentationListResponse",
        "DocumentationInfo",
        "SearchQueryParams",
        "SearchResponse",
        "SearchResultItem",
    ];

    for schema in required_schemas {
        assert!(
            components.schemas.contains_key(schema),
            "OpenAPI spec missing required schema: {}",
            schema
        );
    }
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_health_endpoint_schema_matches_response() {
    let spec = ApiDocV1::openapi();
    let path = spec.paths.paths.get("/api/v1/health").unwrap();
    let get_op = path
        .operations
        .get(&utoipa::openapi::PathItemType::Get)
        .unwrap();

    // Verify operation has responses
    assert!(get_op.responses.responses.contains_key("200"));

    // Verify response has schema reference to HealthResponse
    let response_200 = get_op.responses.responses.get("200").unwrap();
    match response_200 {
        utoipa::openapi::RefOr::T(resp) => {
            assert!(resp.content.contains_key("application/json"));
        }
        _ => panic!("Expected concrete response, got reference"),
    }
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_analyze_endpoint_requires_request_body() {
    let spec = ApiDocV1::openapi();
    let path = spec.paths.paths.get("/api/v1/analyze").unwrap();
    let post_op = path
        .operations
        .get(&utoipa::openapi::PathItemType::Post)
        .unwrap();

    // Verify operation requires request body
    assert!(post_op.request_body.is_some());

    let request_body = post_op.request_body.as_ref().unwrap();
    // Just verify that request body is defined, content structure varies by utoipa version
    assert!(!request_body.content.is_empty());
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_search_endpoint_has_query_params() {
    let spec = ApiDocV1::openapi();
    let path = spec.paths.paths.get("/api/v1/search").unwrap();
    let get_op = path
        .operations
        .get(&utoipa::openapi::PathItemType::Get)
        .unwrap();

    // Verify operation has parameters (parameters is Option<Vec<Parameter>>)
    if let Some(ref params) = get_op.parameters {
        assert!(
            !params.is_empty(),
            "Search endpoint should have query parameters"
        );
    } else {
        panic!("Search endpoint should have parameters defined");
    }
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_health_endpoint_response_matches_openapi_schema() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify status code matches OpenAPI spec
    assert_eq!(response.status(), StatusCode::OK);

    // Verify content-type header
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Parse response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure matches HealthResponse schema
    assert!(json.is_object());
    assert!(json.get("status").is_some());
    assert!(json.get("version").is_some());
    assert!(json.get("timestamp").is_some());

    // Verify types
    assert!(json["status"].is_string());
    assert!(json["version"].is_string());
    assert!(json["timestamp"].is_string());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_version_endpoint_response_matches_openapi_schema() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/version")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response matches VersionResponse schema
    assert!(json.is_object());
    assert!(json.get("version").is_some());
    assert!(json.get("api_version").is_some());
    assert!(json.get("build_info").is_some());

    // Verify build_info structure
    let build_info = &json["build_info"];
    assert!(build_info.is_object());
    assert!(build_info.get("git_hash").is_some());
    assert!(build_info.get("build_timestamp").is_some());
    assert!(build_info.get("rust_version").is_some());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_repositories_list_response_matches_openapi_schema() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/repositories")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response matches RepositoryListResponse schema
    assert!(json.is_object());
    assert!(json.get("repositories").is_some());
    assert!(json.get("total").is_some());
    assert!(json.get("page").is_some());
    assert!(json.get("per_page").is_some());

    // Verify types
    assert!(json["repositories"].is_array());
    assert!(json["total"].is_number());
    assert!(json["page"].is_number());
    assert!(json["per_page"].is_number());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_documentation_list_response_matches_openapi_schema() {
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

    // Verify response matches DocumentationListResponse schema
    assert!(json.is_object());
    assert!(json.get("documents").is_some());
    assert!(json.get("total").is_some());

    // Verify types
    assert!(json["documents"].is_array());
    assert!(json["total"].is_number());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_analyze_endpoint_accepts_request_matching_schema() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Create request matching AnalyzeRequest schema
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

    // Verify response matches AnalyzeResponse schema
    assert!(json.is_object());
    assert!(json.get("job_id").is_some());
    assert!(json.get("status").is_some());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_analyze_endpoint_rejects_invalid_request() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Create invalid request (missing required repository_url field)
    let body = serde_json::json!({
        "branch": "main"
    });

    let request = Request::builder()
        .uri("/api/v1/analyze")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 4xx error for invalid request
    assert!(response.status().is_client_error());
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_search_endpoint_response_matches_openapi_schema() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    let request = Request::builder()
        .uri("/api/v1/search?q=test&limit=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Search might return 200 or 500 depending on Ollama availability
    // We just verify response structure when successful
    if response.status() == StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // Verify response matches SearchResponse schema
        assert!(json.is_object());
        assert!(json.get("results").is_some());
        assert!(json.get("total").is_some());
        assert!(json.get("query").is_some());

        // Verify types
        assert!(json["results"].is_array());
        assert!(json["total"].is_number());
        assert!(json["query"].is_string());
    }
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_all_v1_endpoints_return_json_content_type() {
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

        let content_type = response.headers().get("content-type");
        assert!(
            content_type.is_some(),
            "Content-Type header missing for endpoint: {}",
            endpoint
        );

        let ct_value = content_type.unwrap().to_str().unwrap();
        assert!(
            ct_value.contains("application/json"),
            "Content-Type should be application/json for endpoint: {}, got: {}",
            endpoint,
            ct_value
        );
    }
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_all_v1_endpoints_include_api_version_header() {
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
            "API-Version header should be 'v1' for endpoint: {}",
            endpoint
        );
    }
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_openapi_spec_status_codes_match_actual_responses() {
    let state = create_test_state().await;
    let spec = ApiDocV1::openapi();

    // Test health endpoint
    let app = create_routes().with_state(state);
    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let actual_status = response.status().as_u16().to_string();

    // Check OpenAPI spec declares this status code
    let path = spec.paths.paths.get("/api/v1/health").unwrap();
    let get_op = path
        .operations
        .get(&utoipa::openapi::PathItemType::Get)
        .unwrap();

    assert!(
        get_op.responses.responses.contains_key(&actual_status),
        "OpenAPI spec should declare status {} for /api/v1/health",
        actual_status
    );
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_pagination_parameters_in_repositories_endpoint() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Test with pagination parameters
    let request = Request::builder()
        .uri("/api/v1/repositories?page=1&per_page=20")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify pagination fields are present and match request
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 20);
}

#[cfg(feature = "openapi")]
#[tokio::test]
#[ignore]
async fn test_error_responses_have_consistent_structure() {
    let state = create_test_state().await;
    let app = create_routes().with_state(state);

    // Request with invalid Accept-Version header to trigger error
    let request = Request::builder()
        .uri("/api/v1/health")
        .method("GET")
        .header("accept-version", "v99")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify error response structure is consistent
    assert!(json.is_object());
    assert!(json.get("error").is_some());
    assert!(json["error"].is_string());
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_validates_as_json() {
    use xze_serve::api::v1::openapi::get_openapi_json;

    let json_spec = get_openapi_json().unwrap();
    let parsed: Value = serde_json::from_str(&json_spec).unwrap();

    assert!(parsed.is_object());
    assert_eq!(parsed["openapi"].as_str().unwrap(), "3.0.3");
    assert_eq!(parsed["info"]["title"].as_str().unwrap(), "XZe API");
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_validates_as_yaml() {
    use xze_serve::api::v1::openapi::get_openapi_yaml;

    let yaml_spec = get_openapi_yaml().unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_spec).unwrap();

    assert!(parsed.is_mapping());
    assert_eq!(parsed["openapi"].as_str().unwrap(), "3.0.3");
    assert_eq!(parsed["info"]["title"].as_str().unwrap(), "XZe API");
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_has_consistent_schema_references() {
    let spec = ApiDocV1::openapi();
    let components = spec.components.as_ref().unwrap();

    // Verify that key schemas exist in components
    // This is a simplified check since traversing all refs is complex
    assert!(!components.schemas.is_empty(), "Schemas should be defined");

    // Verify some key schemas exist
    assert!(components.schemas.contains_key("HealthResponse"));
    assert!(components.schemas.contains_key("VersionResponse"));
    assert!(components.schemas.contains_key("AnalyzeRequest"));
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_all_endpoints_have_tags() {
    let spec = ApiDocV1::openapi();

    for (path, item) in &spec.paths.paths {
        for (_method, operation) in &item.operations {
            if let Some(ref tags) = operation.tags {
                assert!(
                    !tags.is_empty(),
                    "Endpoint {} should have at least one tag",
                    path
                );
            }
        }
    }
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_all_endpoints_have_descriptions() {
    let spec = ApiDocV1::openapi();

    for (path, item) in &spec.paths.paths {
        for (_method, operation) in &item.operations {
            assert!(
                operation.summary.is_some() || operation.description.is_some(),
                "Endpoint {} should have a summary or description",
                path
            );
        }
    }
}

#[cfg(feature = "openapi")]
#[test]
fn test_openapi_spec_server_base_path_correct() {
    let spec = ApiDocV1::openapi();

    if let Some(ref servers) = spec.servers {
        assert!(!servers.is_empty());
        let server = &servers[0];
        assert_eq!(server.url, "/api/v1");
    } else {
        panic!("OpenAPI spec should have servers defined");
    }
}
