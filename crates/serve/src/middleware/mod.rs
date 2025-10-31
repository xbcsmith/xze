//! Middleware module for XZe serve crate
//!
//! Provides comprehensive middleware for rate limiting, security,
//! authentication, and request processing.

pub mod rate_limit;
pub mod security;

pub use rate_limit::{
    api_key_middleware, create_rate_limiter, rate_limit_middleware, request_validation_middleware,
    RateLimitConfig, SharedRateLimiter,
};
pub use security::{
    cors_middleware, input_sanitization_middleware, security_headers_middleware, CorsConfig,
};
