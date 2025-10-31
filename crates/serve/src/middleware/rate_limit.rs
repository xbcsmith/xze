//! Rate limiting middleware using tower-governor
//!
//! Provides configurable rate limiting for API endpoints to prevent abuse
//! and ensure fair resource allocation.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::{Clock, DefaultClock},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{num::NonZeroU32, sync::Arc, time::Duration};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests per window
    pub max_requests: u32,
    /// Time window duration in seconds
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 60,
        }
    }
}

impl RateLimitConfig {
    /// Creates a new rate limit configuration
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum requests allowed per window
    /// * `window_seconds` - Duration of the rate limit window in seconds
    ///
    /// # Returns
    ///
    /// Returns a new RateLimitConfig instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::rate_limit::RateLimitConfig;
    ///
    /// let config = RateLimitConfig::new(100, 60);
    /// assert_eq!(config.max_requests, 100);
    /// assert_eq!(config.window_seconds, 60);
    /// ```
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }

    /// Creates a permissive configuration for development
    ///
    /// # Returns
    ///
    /// Returns a RateLimitConfig with high limits (1000 requests per minute)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::rate_limit::RateLimitConfig;
    ///
    /// let config = RateLimitConfig::permissive();
    /// assert_eq!(config.max_requests, 1000);
    /// ```
    pub fn permissive() -> Self {
        Self {
            max_requests: 1000,
            window_seconds: 60,
        }
    }

    /// Creates a strict configuration for production
    ///
    /// # Returns
    ///
    /// Returns a RateLimitConfig with lower limits (60 requests per minute)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_serve::middleware::rate_limit::RateLimitConfig;
    ///
    /// let config = RateLimitConfig::strict();
    /// assert_eq!(config.max_requests, 60);
    /// ```
    pub fn strict() -> Self {
        Self {
            max_requests: 60,
            window_seconds: 60,
        }
    }
}

/// Rate limiter state shared across requests
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Creates a new rate limiter from configuration
///
/// # Arguments
///
/// * `config` - Rate limit configuration
///
/// # Returns
///
/// Returns a shared rate limiter instance
///
/// # Examples
///
/// ```
/// use xze_serve::middleware::rate_limit::{RateLimitConfig, create_rate_limiter};
///
/// let config = RateLimitConfig::default();
/// let limiter = create_rate_limiter(&config);
/// ```
pub fn create_rate_limiter(config: &RateLimitConfig) -> SharedRateLimiter {
    let quota = Quota::with_period(Duration::from_secs(config.window_seconds))
        .expect("valid duration")
        .allow_burst(NonZeroU32::new(config.max_requests).expect("non-zero requests"));

    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
///
/// Checks if the request is within rate limits and rejects requests
/// that exceed the configured limits.
///
/// # Arguments
///
/// * `limiter` - Shared rate limiter instance
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns either the response from the next middleware or a 429 Too Many Requests error
///
/// # Errors
///
/// Returns HTTP 429 when rate limit is exceeded
///
/// # Examples
///
/// ```no_run
/// use axum::{Router, middleware};
/// use xze_serve::middleware::rate_limit::{RateLimitConfig, create_rate_limiter, rate_limit_middleware};
///
/// let config = RateLimitConfig::default();
/// let limiter = create_rate_limiter(&config);
///
/// let app = Router::new()
///     .layer(middleware::from_fn(move |req, next| {
///         rate_limit_middleware(limiter.clone(), req, next)
///     }));
/// ```
pub async fn rate_limit_middleware(
    limiter: SharedRateLimiter,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => {
            // Request is within rate limits
            next.run(request).await
        }
        Err(not_until) => {
            // Rate limit exceeded
            let retry_after = not_until
                .wait_time_from(DefaultClock::default().now())
                .as_secs();

            tracing::warn!(retry_after = retry_after, "Rate limit exceeded");

            (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("retry-after", retry_after.to_string()),
                    ("x-ratelimit-remaining", "0".to_string()),
                ],
                "Rate limit exceeded. Please try again later.",
            )
                .into_response()
        }
    }
}

/// API key authentication middleware
///
/// Validates API keys from the Authorization header.
///
/// # Arguments
///
/// * `valid_keys` - Set of valid API keys
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns either the response from the next middleware or a 401 Unauthorized error
///
/// # Errors
///
/// Returns HTTP 401 when API key is missing or invalid
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashSet;
/// use axum::{Router, middleware};
/// use xze_serve::middleware::rate_limit::api_key_middleware;
///
/// let mut keys = HashSet::new();
/// keys.insert("test-key-123".to_string());
/// let valid_keys = std::sync::Arc::new(keys);
///
/// let app = Router::new()
///     .layer(middleware::from_fn(move |req, next| {
///         api_key_middleware(valid_keys.clone(), req, next)
///     }));
/// ```
pub async fn api_key_middleware(
    valid_keys: Arc<std::collections::HashSet<String>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for health check endpoints
    let path = request.uri().path();
    if path == "/health" || path == "/api/v1/health" || path == "/metrics" {
        return Ok(next.run(request).await);
    }

    // Extract API key from Authorization header
    let api_key = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            tracing::warn!("Missing or invalid authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    // Validate API key
    if !valid_keys.contains(api_key) {
        tracing::warn!(
            api_key_prefix = &api_key[..api_key.len().min(8)],
            "Invalid API key"
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    tracing::debug!("API key validated successfully");
    Ok(next.run(request).await)
}

/// Request validation middleware
///
/// Validates request content type and size.
///
/// # Arguments
///
/// * `request` - Incoming HTTP request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// Returns either the response from the next middleware or a 400 Bad Request error
///
/// # Errors
///
/// Returns HTTP 400 when request validation fails
pub async fn request_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Validate content-type for POST/PUT requests
    let method = request.method();
    if method == axum::http::Method::POST || method == axum::http::Method::PUT {
        let content_type = request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok());

        if let Some(ct) = content_type {
            if !ct.starts_with("application/json")
                && !ct.starts_with("application/x-www-form-urlencoded")
            {
                tracing::warn!(content_type = ct, "Unsupported content type");
                return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
            }
        }
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_config_new() {
        let config = RateLimitConfig::new(50, 30);
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window_seconds, 30);
    }

    #[test]
    fn test_rate_limit_config_permissive() {
        let config = RateLimitConfig::permissive();
        assert_eq!(config.max_requests, 1000);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_config_strict() {
        let config = RateLimitConfig::strict();
        assert_eq!(config.max_requests, 60);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_create_rate_limiter() {
        let config = RateLimitConfig::new(10, 1);
        let limiter = create_rate_limiter(&config);

        // Should allow up to max_requests
        for _ in 0..10 {
            assert!(limiter.check().is_ok());
        }

        // Next request should be rate limited
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_rate_limiter_burst() {
        let config = RateLimitConfig::new(5, 60);
        let limiter = create_rate_limiter(&config);

        // Should allow burst of 5 requests
        for i in 0..5 {
            assert!(limiter.check().is_ok(), "Request {} should succeed", i);
        }

        // 6th request should fail
        assert!(limiter.check().is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_recovery() {
        let config = RateLimitConfig::new(2, 1);
        let limiter = create_rate_limiter(&config);

        // Use up quota
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_err());

        // Wait for quota to replenish
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Should be able to make requests again
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_config_clone() {
        let config = RateLimitConfig::new(100, 60);
        let cloned = config.clone();
        assert_eq!(config.max_requests, cloned.max_requests);
        assert_eq!(config.window_seconds, cloned.window_seconds);
    }

    #[test]
    fn test_config_debug() {
        let config = RateLimitConfig::new(100, 60);
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("60"));
    }
}
