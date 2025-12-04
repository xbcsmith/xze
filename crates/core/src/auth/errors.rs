use crate::error::XzeError;
use thiserror::Error;

/// Errors related to authentication and authorization
#[derive(Error, Debug)]
pub enum AuthError {
    /// Error accessing token storage
    #[error("Token storage error: {0}")]
    Storage(String),

    /// OAuth protocol error
    #[error("OAuth error: {0}")]
    OAuth(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network error during auth flow
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Token has expired and cannot be refreshed
    #[error("Token expired")]
    TokenExpired,

    /// Token validation failed
    #[error("Invalid token: {0}")]
    InvalidToken(String),
}

impl From<AuthError> for XzeError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Storage(msg) => XzeError::Auth {
                message: format!("Storage: {}", msg),
            },
            AuthError::OAuth(msg) => XzeError::Auth {
                message: format!("OAuth: {}", msg),
            },
            AuthError::Config(msg) => XzeError::Auth {
                message: format!("Config: {}", msg),
            },
            AuthError::Network(e) => XzeError::Network {
                message: e.to_string(),
            },
            AuthError::TokenExpired => XzeError::Auth {
                message: "Token expired".to_string(),
            },
            AuthError::InvalidToken(msg) => XzeError::Auth {
                message: format!("Invalid token: {}", msg),
            },
        }
    }
}
