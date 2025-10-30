//! OpenAPI documentation generation for API v1
//!
//! This module provides OpenAPI 3.0 specification generation using utoipa.
//! It defines the complete API schema including all endpoints, request/response types,
//! and documentation.

#[cfg(feature = "openapi")]
use utoipa::OpenApi;

#[cfg(feature = "openapi")]
use super::handlers::*;

/// OpenAPI v1 API documentation
///
/// Generates the complete OpenAPI 3.0 specification for the XZe API v1.
/// This includes all endpoints, schemas, and examples.
///
/// # Features
///
/// This module is only available when the `openapi` feature is enabled.
///
/// # Examples
///
/// ```ignore
/// use xze_serve::api::v1::openapi::ApiDocV1;
/// use utoipa::OpenApi;
///
/// let openapi_spec = ApiDocV1::openapi();
/// let json = openapi_spec.to_json().unwrap();
/// ```
#[cfg(feature = "openapi")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "XZe API",
        version = "1.0.0",
        description = "AI-powered documentation generator API - Version 1",
        license(
            name = "MIT OR Apache-2.0",
            url = "https://github.com/xbcsmith/xze"
        ),
        contact(
            name = "XZe Project",
            url = "https://github.com/xbcsmith/xze"
        )
    ),
    servers(
        (url = "/api/v1", description = "API v1 base path")
    ),
    paths(
        super::handlers::health_check,
        super::handlers::get_version,
        super::handlers::analyze_repository,
        super::handlers::list_repositories,
        super::handlers::get_repository,
        super::handlers::analyze_repository_by_id,
        super::handlers::list_documentation,
        super::handlers::get_documentation,
        crate::handlers::handle_search,
    ),
    components(
        schemas(
            HealthResponse,
            VersionResponse,
            BuildInfo,
            AnalyzeResponse,
            AnalyzeRequest,
            RepositoryListResponse,
            RepositoryInfo,
            DocumentationListResponse,
            DocumentationInfo,
            crate::handlers::SearchQueryParams,
            crate::handlers::SearchResponse,
            crate::handlers::SearchResultItem,
            crate::handlers::SearchConfigResponse,
            crate::handlers::SearchErrorResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check and system status endpoints"),
        (name = "system", description = "System information and version endpoints"),
        (name = "analysis", description = "Repository analysis and processing endpoints"),
        (name = "repositories", description = "Repository management endpoints"),
        (name = "documentation", description = "Documentation retrieval endpoints"),
        (name = "search", description = "Semantic search endpoints"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDocV1;

/// Security scheme modifier
///
/// Adds optional API key authentication scheme to the OpenAPI spec.
/// This is placeholder for future authentication implementation.
#[cfg(feature = "openapi")]
struct SecurityAddon;

#[cfg(feature = "openapi")]
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}

/// Get OpenAPI specification as JSON string
///
/// # Returns
///
/// Returns the OpenAPI 3.0 specification in JSON format
///
/// # Errors
///
/// Returns an error if JSON serialization fails
#[cfg(feature = "openapi")]
pub fn get_openapi_json() -> Result<String, serde_json::Error> {
    let spec = ApiDocV1::openapi();
    serde_json::to_string_pretty(&spec)
}

/// Get OpenAPI specification as YAML string
///
/// # Returns
///
/// Returns the OpenAPI 3.0 specification in YAML format
///
/// # Errors
///
/// Returns an error if YAML serialization fails
#[cfg(feature = "openapi")]
pub fn get_openapi_yaml() -> Result<String, serde_yaml::Error> {
    let spec = ApiDocV1::openapi();
    serde_yaml::to_string(&spec)
}

#[cfg(all(test, feature = "openapi"))]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = ApiDocV1::openapi();
        assert_eq!(spec.info.title, "XZe API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[test]
    fn test_openapi_json_generation() {
        let json = get_openapi_json().unwrap();
        assert!(json.contains("XZe API"));
        assert!(json.contains("openapi"));
        assert!(json.contains("3.0"));
    }

    #[test]
    fn test_openapi_yaml_generation() {
        let yaml = get_openapi_yaml().unwrap();
        assert!(yaml.contains("XZe API"));
        assert!(yaml.contains("openapi:"));
    }

    #[test]
    fn test_openapi_has_all_tags() {
        let spec = ApiDocV1::openapi();
        let tags = spec.tags.as_ref().unwrap();

        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();

        assert!(tag_names.contains(&"health"));
        assert!(tag_names.contains(&"system"));
        assert!(tag_names.contains(&"analysis"));
        assert!(tag_names.contains(&"repositories"));
        assert!(tag_names.contains(&"documentation"));
        assert!(tag_names.contains(&"search"));
    }

    #[test]
    fn test_openapi_has_security_scheme() {
        let spec = ApiDocV1::openapi();
        let components = spec.components.as_ref().unwrap();

        assert!(components.security_schemes.contains_key("api_key"));
    }

    #[test]
    fn test_openapi_has_paths() {
        let spec = ApiDocV1::openapi();

        // Verify key paths are present
        assert!(spec.paths.paths.contains_key("/api/v1/health"));
        assert!(spec.paths.paths.contains_key("/api/v1/version"));
        assert!(spec.paths.paths.contains_key("/api/v1/analyze"));
        assert!(spec.paths.paths.contains_key("/api/v1/repositories"));
        assert!(spec.paths.paths.contains_key("/api/v1/documentation"));
        assert!(spec.paths.paths.contains_key("/api/v1/search"));
    }

    #[test]
    fn test_openapi_has_schemas() {
        let spec = ApiDocV1::openapi();
        let components = spec.components.as_ref().unwrap();

        // Verify key schemas are present
        assert!(components.schemas.contains_key("HealthResponse"));
        assert!(components.schemas.contains_key("VersionResponse"));
        assert!(components.schemas.contains_key("AnalyzeRequest"));
        assert!(components.schemas.contains_key("AnalyzeResponse"));
        assert!(components.schemas.contains_key("RepositoryInfo"));
        assert!(components.schemas.contains_key("SearchResponse"));
        assert!(components.schemas.contains_key("SearchResultItem"));
    }

    #[test]
    fn test_openapi_json_is_valid_json() {
        let json = get_openapi_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_object());
        assert_eq!(parsed["openapi"].as_str().unwrap(), "3.0.3");
        assert_eq!(parsed["info"]["title"].as_str().unwrap(), "XZe API");
    }

    #[test]
    fn test_openapi_yaml_is_valid_yaml() {
        let yaml = get_openapi_yaml().unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();

        assert!(parsed.is_mapping());
        assert_eq!(parsed["openapi"].as_str().unwrap(), "3.0.3");
        assert_eq!(parsed["info"]["title"].as_str().unwrap(), "XZe API");
    }
}
