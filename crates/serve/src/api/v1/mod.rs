//! API v1 module
//!
//! This module contains the v1 API implementation with all endpoints and handlers.

pub mod handlers;

#[cfg(feature = "openapi")]
pub mod openapi;

use axum::{routing::get, routing::post, Router};

use crate::handlers::AppState;

#[cfg(feature = "openapi")]
use utoipa::OpenApi;

#[cfg(feature = "openapi")]
use utoipa_swagger_ui::SwaggerUi;

/// Create API v1 routes
///
/// Returns a router configured with all v1 API endpoints.
///
/// # Returns
///
/// Returns an Axum router with the following endpoints:
/// - GET /health - Health check endpoint
/// - GET /version - Version information
/// - POST /analyze - Analyze repository
/// - GET /repositories - List repositories
/// - GET /repositories/:id - Get repository by ID
/// - POST /repositories/:id/analyze - Analyze repository by ID
/// - GET /documentation - List documentation
/// - GET /documentation/:id - Get documentation by ID
/// - GET /search - Search documentation (from handlers module)
///
/// # Examples
///
/// ```
/// use xze_serve::api::v1::create_v1_routes;
/// use xze_serve::handlers::AppState;
///
/// // Create router with application state
/// // let state = AppState::new(config).await?;
/// // let router = create_v1_routes().with_state(state);
/// ```
pub fn create_v1_routes() -> Router<AppState> {
    #[cfg(feature = "openapi")]
    let mut router = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/version", get(handlers::get_version))
        .route("/analyze", post(handlers::analyze_repository))
        .route("/repositories", get(handlers::list_repositories))
        .route("/repositories/:id", get(handlers::get_repository))
        .route(
            "/repositories/:id/analyze",
            post(handlers::analyze_repository_by_id),
        )
        .route("/documentation", get(handlers::list_documentation))
        .route("/documentation/:id", get(handlers::get_documentation))
        .route("/search", get(crate::handlers::handle_search));

    #[cfg(not(feature = "openapi"))]
    let router = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/version", get(handlers::get_version))
        .route("/analyze", post(handlers::analyze_repository))
        .route("/repositories", get(handlers::list_repositories))
        .route("/repositories/:id", get(handlers::get_repository))
        .route(
            "/repositories/:id/analyze",
            post(handlers::analyze_repository_by_id),
        )
        .route("/documentation", get(handlers::list_documentation))
        .route("/documentation/:id", get(handlers::get_documentation))
        .route("/search", get(crate::handlers::handle_search));

    // Add Swagger UI if openapi feature is enabled
    #[cfg(feature = "openapi")]
    {
        router = router
            .merge(SwaggerUi::new("/docs").url("/docs/openapi.json", openapi::ApiDocV1::openapi()));
    }

    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_v1_routes() {
        // Test that router creation doesn't panic
        let _router: Router<AppState> = create_v1_routes();
    }

    #[test]
    #[cfg(feature = "openapi")]
    fn test_create_v1_routes_includes_openapi() {
        // Test that OpenAPI docs are available when feature is enabled
        let router: Router<AppState> = create_v1_routes();
        // Router creation should succeed with openapi feature
        drop(router);
    }

    #[test]
    #[cfg(feature = "openapi")]
    fn test_openapi_spec_accessible() {
        use crate::api::v1::openapi::ApiDocV1;
        use utoipa::OpenApi;

        let spec = ApiDocV1::openapi();
        assert_eq!(spec.info.title, "XZe API");
    }
}
