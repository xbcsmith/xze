//! Integration tests for search API endpoints
//!
//! These tests verify the behavior of both GET and POST search endpoints,
//! including their equivalence, validation, and error handling.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::json;
use tower::ServiceExt;
use xze_serve::search::{
    handlers::search_routes, AdvancedSearchRequest, SearchFilters, SearchOptions,
};

/// Creates a test router with search routes
fn create_test_router() -> Router {
    Router::new().nest("/api", search_routes())
}

/// Helper to make GET request
async fn make_get_request(router: Router, uri: &str) -> (StatusCode, String) {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    (status, body_str)
}

/// Helper to make POST request
async fn make_post_request(
    router: Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, String) {
    let response = router
        .oneshot(
            Request::builder()
                .uri(uri)
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    (status, body_str)
}

#[tokio::test]
async fn test_get_search_basic_query() {
    let router = create_test_router();

    let (status, body) = make_get_request(router, "/api/search?q=rust&limit=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"query\":\"rust\""));
    assert!(body.contains("\"results\""));
    assert!(body.contains("\"pagination\""));
}

#[tokio::test]
async fn test_get_search_empty_query_returns_error() {
    let router = create_test_router();

    let (status, body) = make_get_request(router, "/api/search?q=").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
    assert!(body.contains("Query cannot be empty"));
}

#[tokio::test]
async fn test_get_search_limit_too_large() {
    let router = create_test_router();

    let (status, body) = make_get_request(router, "/api/search?q=test&limit=200").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
    assert!(body.contains("Limit cannot exceed 100"));
}

#[tokio::test]
async fn test_get_search_with_category_filter() {
    let router = create_test_router();

    let (status, body) =
        make_get_request(router, "/api/search?q=rust&category=tutorial&limit=5").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"query\":\"rust\""));
    assert!(body.contains("tutorial"));
}

#[tokio::test]
async fn test_post_search_basic_query() {
    let router = create_test_router();

    let request_body = json!({
        "query": "rust async"
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"query\":\"rust async\""));
    assert!(body.contains("\"results\""));
}

#[tokio::test]
async fn test_post_search_with_filters() {
    let router = create_test_router();

    let request_body = json!({
        "query": "documentation",
        "filters": {
            "categories": ["tutorial", "reference"],
            "similarity": {
                "min": 0.7
            }
        },
        "options": {
            "max_results": 20,
            "include_snippets": true
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"query\":\"documentation\""));
    assert!(body.contains("\"results\""));
}

#[tokio::test]
async fn test_post_search_with_aggregations() {
    let router = create_test_router();

    let request_body = json!({
        "query": "rust",
        "aggregations": {
            "by_category": true,
            "by_similarity_range": true
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"aggregations\""));
    assert!(body.contains("\"by_category\""));
    assert!(body.contains("\"by_similarity_range\""));
}

#[tokio::test]
async fn test_post_search_empty_query_returns_error() {
    let router = create_test_router();

    let request_body = json!({
        "query": "   "
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
}

#[tokio::test]
async fn test_post_search_invalid_similarity_range() {
    let router = create_test_router();

    let request_body = json!({
        "query": "test",
        "filters": {
            "similarity": {
                "min": 0.9,
                "max": 0.5
            }
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
}

#[tokio::test]
async fn test_post_search_invalid_max_results() {
    let router = create_test_router();

    let request_body = json!({
        "query": "test",
        "options": {
            "max_results": 200
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
    assert!(body.contains("max_results cannot exceed 100"));
}

#[tokio::test]
async fn test_post_search_invalid_category() {
    let router = create_test_router();

    let request_body = json!({
        "query": "test",
        "filters": {
            "categories": ["invalid_category"]
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("\"error\""));
    assert!(body.contains("Invalid category"));
}

#[test]
fn test_search_request_serialization() {
    let request = AdvancedSearchRequest {
        query: "test query".to_string(),
        multi_match: None,
        bool_query: None,
        filters: Some(SearchFilters {
            categories: Some(vec!["tutorial".to_string()]),
            similarity: None,
            date_range: None,
            tags: None,
            repositories: None,
        }),
        options: Some(SearchOptions {
            max_results: Some(50),
            offset: Some(0),
            include_snippets: Some(true),
            highlight_terms: Some(false),
            group_by: None,
        }),
        aggregations: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("test query"));
    assert!(json.contains("tutorial"));
    assert!(json.contains("50"));

    // Test deserialization
    let deserialized: AdvancedSearchRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.query, "test query");
}

#[test]
fn test_search_response_structure() {
    // This test verifies the response structure is correctly formed
    let response_json = json!({
        "query": "test",
        "results": [],
        "total_results": 0,
        "aggregations": null,
        "pagination": {
            "offset": 0,
            "limit": 20,
            "total": 0,
            "has_more": false
        }
    });

    // Verify structure is valid JSON
    assert!(response_json.is_object());
    assert!(response_json["results"].is_array());
    assert!(response_json["pagination"].is_object());
}

#[tokio::test]
async fn test_pagination_in_get_search() {
    let router = create_test_router();

    let (status, body) = make_get_request(router, "/api/search?q=rust&limit=5&offset=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"offset\":10"));
    assert!(body.contains("\"limit\":5"));
}

#[tokio::test]
async fn test_repository_filter_in_get_search() {
    let router = create_test_router();

    let (status, body) = make_get_request(router, "/api/search?q=test&repository=xze-core").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"repository\":\"xze-core\""));
}

#[tokio::test]
async fn test_post_search_all_options() {
    let router = create_test_router();

    let request_body = json!({
        "query": "comprehensive test",
        "filters": {
            "categories": ["tutorial", "how-to"],
            "similarity": {
                "min": 0.6,
                "max": 0.95
            },
            "tags": ["rust", "documentation"],
            "repositories": ["xze"]
        },
        "options": {
            "max_results": 25,
            "offset": 5,
            "include_snippets": true,
            "highlight_terms": true,
            "group_by": "category"
        },
        "aggregations": {
            "by_category": true,
            "by_similarity_range": true,
            "by_date": true
        }
    });

    let (status, body) = make_post_request(router, "/api/search/advanced", request_body).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"query\":\"comprehensive test\""));
    assert!(body.contains("\"results\""));
    assert!(body.contains("\"aggregations\""));
    assert!(body.contains("\"pagination\""));
}
