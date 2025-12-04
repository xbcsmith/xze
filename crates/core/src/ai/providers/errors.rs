use crate::error::XzeError;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur within AI providers
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimit { retry_after: Duration },

    /// Context length exceeded
    #[error("Context length exceeded: max {max}, actual {actual}")]
    ContextLength { max: usize, actual: usize },

    /// Server error (5xx)
    #[error("Server error: {status}")]
    ServerError { status: u16 },

    /// Request error (4xx)
    #[error("Request error: {message}")]
    RequestError { message: String },

    /// Network error
    #[error("Network error: {source}")]
    Network {
        #[from]
        source: reqwest::Error,
    },

    /// Feature not implemented
    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<ProviderError> for XzeError {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::Authentication(msg) => XzeError::Auth { message: msg },
            ProviderError::RateLimit { retry_after } => XzeError::AiService {
                message: format!("Rate limit exceeded. Retry after {:?}", retry_after),
            },
            ProviderError::ContextLength { max, actual } => XzeError::Validation {
                message: format!("Context length exceeded: max {}, actual {}", max, actual),
            },
            ProviderError::ServerError { status } => XzeError::AiService {
                message: format!("Provider server error: status {}", status),
            },
            ProviderError::RequestError { message } => XzeError::AiService {
                message: format!("Provider request error: {}", message),
            },
            ProviderError::Network { source } => XzeError::Http(source),
            ProviderError::NotImplemented { feature } => {
                XzeError::UnsupportedOperation { operation: feature }
            }
            ProviderError::Config(msg) => XzeError::AiService {
                message: format!("Provider configuration error: {}", msg),
            },
            ProviderError::Serialization(err) => XzeError::Json(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_conversion() {
        let auth_err = ProviderError::Authentication("invalid key".into());
        let xze_err: XzeError = auth_err.into();
        assert!(matches!(xze_err, XzeError::Auth { .. }));

        let not_impl = ProviderError::NotImplemented {
            feature: "streaming".into(),
        };
        let xze_err: XzeError = not_impl.into();
        assert!(matches!(xze_err, XzeError::UnsupportedOperation { .. }));
    }

    #[test]
    fn test_retry_after_formatting() {
        let err = ProviderError::RateLimit {
            retry_after: Duration::from_secs(60),
        };
        assert!(err.to_string().contains("60s"));
    }
}
