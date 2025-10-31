//! Security middleware for XZe serve crate
//!
//! Provides security headers, CORS configuration, and input validation
//! to protect against common web vulnerabilities.

use axum::{
    extract::Request,
    http::{header::HeaderValue, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
};

/// Security headers middleware
///
/// Adds comprehensive security headers to all responses to protect
/// against common web vulnerabilities.
///
/// # Security Headers
///
/// - X-Content-Type-Options: Prevents MIME type sniffing
/// - X-Frame-Options: Prevents clickjacking
/// - X-XSS-Protection: Enables browser XSS protection
/// - Strict-Transport-Security: Enforces HTTPS
/// - Content-Security-Policy: Restricts resource loading
/// - Referrer-Policy: Controls referrer information
/// - Permissions-Policy: Restricts browser features
///
/// # Arguments
///
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns the response with security headers added
///
/// # Examples
///
/// ```no_run
/// use axum::{Router, middleware};
/// use xze_serve::middleware::security::security_headers_middleware;
///
/// let app = Router::new()
///     .layer(middleware::from_fn(security_headers_middleware));
/// ```
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let is_sensitive = is_sensitive_endpoint(response.headers());
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // Enable XSS protection (legacy browsers)
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Enforce HTTPS for 1 year including subdomains
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    // Content Security Policy - restrictive default
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';",
        ),
    );

    // Control referrer information
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Restrict browser features
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    // Add cache control for sensitive data
    if is_sensitive {
        headers.insert(
            "cache-control",
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
    }

    response
}

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins
    pub allow_origins: Vec<String>,
    /// Allowed HTTP methods
    pub allow_methods: Vec<String>,
    /// Allowed headers
    pub allow_headers: Vec<String>,
    /// Exposed headers
    pub expose_headers: Vec<String>,
    /// Max age for preflight cache in seconds
    pub max_age: u64,
    /// Allow credentials
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_origins: vec!["*".to_string()],
            allow_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
                "PATCH".to_string(),
            ],
            allow_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
                "accept".to_string(),
                "accept-version".to_string(),
            ],
            expose_headers: vec![
                "x-request-id".to_string(),
                "x-ratelimit-limit".to_string(),
                "x-ratelimit-remaining".to_string(),
                "retry-after".to_string(),
            ],
            max_age: 86400,
            allow_credentials: false,
        }
    }
}

impl CorsConfig {
    /// Creates a new CORS configuration
    ///
    /// # Arguments
    ///
    /// * `allow_origins` - List of allowed origins
    ///
    /// # Returns
    ///
    /// Returns a new CorsConfig instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::security::CorsConfig;
    ///
    /// let config = CorsConfig::new(vec!["https://example.com".to_string()]);
    /// assert_eq!(config.allow_origins.len(), 1);
    /// ```
    pub fn new(allow_origins: Vec<String>) -> Self {
        Self {
            allow_origins,
            ..Default::default()
        }
    }

    /// Creates a permissive CORS configuration for development
    ///
    /// # Returns
    ///
    /// Returns a CorsConfig allowing all origins
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::security::CorsConfig;
    ///
    /// let config = CorsConfig::permissive();
    /// assert!(config.allow_origins.contains(&"*".to_string()));
    /// ```
    pub fn permissive() -> Self {
        Self::default()
    }

    /// Creates a strict CORS configuration for production
    ///
    /// # Arguments
    ///
    /// * `allowed_origins` - Specific origins to allow
    ///
    /// # Returns
    ///
    /// Returns a CorsConfig with strict settings
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::security::CorsConfig;
    ///
    /// let config = CorsConfig::strict(vec!["https://api.example.com".to_string()]);
    /// assert_eq!(config.allow_credentials, true);
    /// ```
    pub fn strict(allowed_origins: Vec<String>) -> Self {
        Self {
            allow_origins: allowed_origins,
            allow_credentials: true,
            ..Default::default()
        }
    }
}

/// CORS middleware
///
/// Handles CORS preflight requests and adds CORS headers to responses.
///
/// # Arguments
///
/// * `config` - CORS configuration
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns the response with CORS headers added
///
/// # Examples
///
/// ```no_run
/// use axum::{Router, middleware};
/// use xze_serve::middleware::security::{CorsConfig, cors_middleware};
/// use std::sync::Arc;
///
/// let config = Arc::new(CorsConfig::default());
/// let app = Router::new()
///     .layer(middleware::from_fn(move |req, next| {
///         cors_middleware(config.clone(), req, next)
///     }));
/// ```
pub async fn cors_middleware(
    config: std::sync::Arc<CorsConfig>,
    request: Request,
    next: Next,
) -> Response {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Handle preflight request
    if request.method() == Method::OPTIONS {
        let mut response = Response::new(String::new().into());
        add_cors_headers(&mut response, &config, &origin);
        *response.status_mut() = StatusCode::NO_CONTENT;
        return response;
    }

    // Process normal request
    let mut response = next.run(request).await;
    add_cors_headers(&mut response, &config, &origin);

    response
}

/// Add CORS headers to response
fn add_cors_headers(response: &mut Response, config: &CorsConfig, origin: &str) {
    let headers = response.headers_mut();

    // Check if origin is allowed
    let allowed = config.allow_origins.contains(&"*".to_string())
        || config.allow_origins.contains(&origin.to_string());

    if allowed {
        let origin_header = if config.allow_origins.contains(&"*".to_string()) {
            "*"
        } else {
            origin
        };

        headers.insert(
            "access-control-allow-origin",
            HeaderValue::from_str(origin_header).unwrap_or(HeaderValue::from_static("*")),
        );
    }

    // Add other CORS headers
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_str(&config.allow_methods.join(", "))
            .unwrap_or(HeaderValue::from_static("GET, POST")),
    );

    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_str(&config.allow_headers.join(", "))
            .unwrap_or(HeaderValue::from_static("content-type")),
    );

    headers.insert(
        "access-control-expose-headers",
        HeaderValue::from_str(&config.expose_headers.join(", "))
            .unwrap_or(HeaderValue::from_static("")),
    );

    headers.insert(
        "access-control-max-age",
        HeaderValue::from_str(&config.max_age.to_string())
            .unwrap_or(HeaderValue::from_static("86400")),
    );

    if config.allow_credentials {
        headers.insert(
            "access-control-allow-credentials",
            HeaderValue::from_static("true"),
        );
    }
}

/// Input sanitization middleware
///
/// Validates and sanitizes request inputs to prevent injection attacks.
///
/// # Arguments
///
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns the response or a 400 Bad Request error
///
/// # Errors
///
/// Returns HTTP 400 when input validation fails
pub async fn input_sanitization_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Validate query parameters
    if let Some(query) = request.uri().query() {
        if contains_suspicious_patterns(query) {
            tracing::warn!(query = query, "Suspicious query parameters detected");
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Validate path
    let path = request.uri().path();
    if contains_path_traversal(path) {
        tracing::warn!(path = path, "Path traversal attempt detected");
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(next.run(request).await)
}

/// Check for suspicious patterns in input
fn contains_suspicious_patterns(input: &str) -> bool {
    let patterns = [
        "<script",
        "javascript:",
        "onerror=",
        "onload=",
        "eval(",
        "document.cookie",
        "select * from",
        "drop table",
        "union select",
        "../",
        "..\\",
    ];

    let lower = input.to_lowercase();
    patterns.iter().any(|p| lower.contains(p))
}

/// Check for path traversal attempts
fn contains_path_traversal(path: &str) -> bool {
    path.contains("../") || path.contains("..\\") || path.contains("%2e%2e")
}

/// Check if endpoint handles sensitive data
fn is_sensitive_endpoint(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allow_origins.contains(&"*".to_string()));
        assert!(config.allow_methods.contains(&"GET".to_string()));
        assert_eq!(config.max_age, 86400);
        assert!(!config.allow_credentials);
    }

    #[test]
    fn test_cors_config_new() {
        let origins = vec!["https://example.com".to_string()];
        let config = CorsConfig::new(origins.clone());
        assert_eq!(config.allow_origins, origins);
    }

    #[test]
    fn test_cors_config_permissive() {
        let config = CorsConfig::permissive();
        assert!(config.allow_origins.contains(&"*".to_string()));
    }

    #[test]
    fn test_cors_config_strict() {
        let origins = vec!["https://api.example.com".to_string()];
        let config = CorsConfig::strict(origins.clone());
        assert_eq!(config.allow_origins, origins);
        assert!(config.allow_credentials);
    }

    #[test]
    fn test_contains_suspicious_patterns_with_script_tag() {
        assert!(contains_suspicious_patterns(
            "<script>alert('xss')</script>"
        ));
        assert!(contains_suspicious_patterns(
            "<SCRIPT>alert('xss')</SCRIPT>"
        ));
    }

    #[test]
    fn test_contains_suspicious_patterns_with_javascript() {
        assert!(contains_suspicious_patterns("javascript:alert('xss')"));
        assert!(contains_suspicious_patterns("JAVASCRIPT:alert('xss')"));
    }

    #[test]
    fn test_contains_suspicious_patterns_with_sql() {
        assert!(contains_suspicious_patterns("SELECT * FROM users"));
        assert!(contains_suspicious_patterns("DROP TABLE users"));
        assert!(contains_suspicious_patterns("UNION SELECT password"));
    }

    #[test]
    fn test_contains_suspicious_patterns_with_safe_input() {
        assert!(!contains_suspicious_patterns("hello world"));
        assert!(!contains_suspicious_patterns("test query"));
        assert!(!contains_suspicious_patterns("user@example.com"));
    }

    #[test]
    fn test_contains_path_traversal_with_unix_style() {
        assert!(contains_path_traversal("../../../etc/passwd"));
        assert!(contains_path_traversal("/api/../../../etc/passwd"));
    }

    #[test]
    fn test_contains_path_traversal_with_windows_style() {
        assert!(contains_path_traversal("..\\..\\..\\windows\\system32"));
    }

    #[test]
    fn test_contains_path_traversal_with_encoded() {
        assert!(contains_path_traversal("%2e%2e/etc/passwd"));
    }

    #[test]
    fn test_contains_path_traversal_with_safe_path() {
        assert!(!contains_path_traversal("/api/v1/search"));
        assert!(!contains_path_traversal("/health"));
        assert!(!contains_path_traversal("/api/v1/docs"));
    }

    #[test]
    fn test_is_sensitive_endpoint_with_json() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        assert!(is_sensitive_endpoint(&headers));
    }

    #[test]
    fn test_is_sensitive_endpoint_with_html() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html"));
        assert!(!is_sensitive_endpoint(&headers));
    }

    #[test]
    fn test_is_sensitive_endpoint_without_content_type() {
        let headers = HeaderMap::new();
        assert!(!is_sensitive_endpoint(&headers));
    }

    #[test]
    fn test_cors_config_clone() {
        let config = CorsConfig::default();
        let cloned = config.clone();
        assert_eq!(config.allow_origins, cloned.allow_origins);
        assert_eq!(config.max_age, cloned.max_age);
    }

    #[test]
    fn test_cors_config_debug() {
        let config = CorsConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("allow_origins"));
    }
}
