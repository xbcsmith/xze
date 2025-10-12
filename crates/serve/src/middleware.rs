//! Middleware module for XZe serve crate

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tower::{Layer, Service};
use uuid::Uuid;

/// Request ID middleware for tracing
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();

    // Add request ID to headers
    request
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());

    // Add request ID to tracing span
    let span = tracing::info_span!("request", request_id = %request_id);
    let _enter = span.enter();

    let response = next.run(request).await;
    response
}

/// Timing middleware to log request duration
pub async fn timing_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let duration = start.elapsed();
    tracing::info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

/// Authentication middleware (placeholder)
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: Implement actual authentication
    // For now, just check for Authorization header presence
    if let Some(_auth_header) = headers.get("authorization") {
        tracing::debug!("Authentication header present");
    } else {
        tracing::debug!("No authentication header found");
    }

    Ok(next.run(request).await)
}

/// Rate limiting middleware (basic implementation)
pub struct RateLimitLayer {
    max_requests: u32,
    window_seconds: u64,
}

impl RateLimitLayer {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            max_requests: self.max_requests,
            window_seconds: self.window_seconds,
        }
    }
}

pub struct RateLimitService<S> {
    inner: S,
    max_requests: u32,
    window_seconds: u64,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let inner = self.inner.clone();
        let _max_requests = self.max_requests;
        let _window_seconds = self.window_seconds;

        Box::pin(async move {
            // TODO: Implement actual rate limiting logic
            // For now, just pass through
            let mut inner = inner;
            inner.call(request).await
        })
    }
}

/// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Add security headers
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("x-xss-protection", "1; mode=block".parse().unwrap());
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    headers.insert(
        "content-security-policy",
        "default-src 'self'".parse().unwrap(),
    );

    response
}

/// Error handling middleware
pub async fn error_handling_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    // Log errors if status code indicates an error
    if response.status().is_server_error() {
        tracing::error!(
            status = %response.status(),
            "Server error occurred"
        );
    } else if response.status().is_client_error() {
        tracing::warn!(
            status = %response.status(),
            "Client error occurred"
        );
    }

    response
}

/// CORS middleware configuration
pub struct CorsConfig {
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub max_age: u64,
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
            ],
            allow_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
            ],
            max_age: 86400, // 24 hours
        }
    }
}

/// Health check bypass middleware
pub async fn health_check_bypass_middleware(request: Request, next: Next) -> Response {
    // Skip expensive middleware for health check endpoints
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    // For other endpoints, continue with normal processing
    next.run(request).await
}

/// Request size limit middleware
pub fn request_size_limit_layer(max_size: usize) -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(max_size)
}

/// Compression middleware
pub fn compression_layer() -> tower_http::compression::CompressionLayer {
    tower_http::compression::CompressionLayer::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Method};

    #[tokio::test]
    async fn test_rate_limit_layer() {
        let layer = RateLimitLayer::new(100, 60);
        assert_eq!(layer.max_requests, 100);
        assert_eq!(layer.window_seconds, 60);
    }

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allow_origins.contains(&"*".to_string()));
        assert!(config.allow_methods.contains(&"GET".to_string()));
        assert_eq!(config.max_age, 86400);
    }

    #[test]
    fn test_middleware_functions_exist() {
        // This test ensures all middleware functions are properly defined
        // and can be referenced (compilation test)
        let _timing = timing_middleware;
        let _security = security_headers_middleware;
        let _error = error_handling_middleware;
        let _health = health_check_bypass_middleware;
        let _request_id = request_id_middleware;
    }
}
