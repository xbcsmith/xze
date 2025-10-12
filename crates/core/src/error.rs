//! Error handling for XZe core library

use std::fmt;
use thiserror::Error;

/// Result type alias for XZe operations
pub type Result<T> = std::result::Result<T, XzeError>;

/// Main error type for XZe operations
#[derive(Error, Debug)]
pub enum XzeError {
    /// IO-related errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Git-related errors
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML serialization/deserialization errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// Template rendering errors
    #[error("Template error: {0}")]
    Template(#[from] handlebars::RenderError),

    /// URL parsing errors
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Generic errors
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),

    /// Repository-related errors
    #[error("Repository error: {message}")]
    Repository { message: String },

    /// AI service errors
    #[error("AI service error: {message}")]
    AiService { message: String },

    /// Documentation generation errors
    #[error("Documentation error: {message}")]
    Documentation { message: String },

    /// Pipeline execution errors
    #[error("Pipeline error: {message}")]
    Pipeline { message: String },

    /// File system errors
    #[error("File system error: {message}")]
    FileSystem { message: String },

    /// Authentication/authorization errors
    #[error("Authentication error: {message}")]
    Auth { message: String },

    /// Network connectivity errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Model not available error
    #[error("Model '{model}' is not available")]
    ModelNotAvailable { model: String },

    /// Timeout errors
    #[error("Operation timed out: {operation}")]
    Timeout { operation: String },

    /// Resource not found errors
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Permission denied errors
    #[error("Permission denied: {resource}")]
    PermissionDenied { resource: String },

    /// Invalid state errors
    #[error("Invalid state: {message}")]
    InvalidState { message: String },

    /// Unsupported operation errors
    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },
}

impl XzeError {
    /// Create a repository error
    pub fn repository<S: Into<String>>(message: S) -> Self {
        Self::Repository {
            message: message.into(),
        }
    }

    /// Create an AI service error
    pub fn ai<S: Into<String>>(message: S) -> Self {
        Self::AiService {
            message: message.into(),
        }
    }

    /// Create a documentation error
    pub fn documentation<S: Into<String>>(message: S) -> Self {
        Self::Documentation {
            message: message.into(),
        }
    }

    /// Create a pipeline error
    pub fn pipeline<S: Into<String>>(message: S) -> Self {
        Self::Pipeline {
            message: message.into(),
        }
    }

    /// Create a file system error
    pub fn filesystem<S: Into<String>>(message: S) -> Self {
        Self::FileSystem {
            message: message.into(),
        }
    }

    /// Create an auth error
    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a model not available error
    pub fn model_not_available<S: Into<String>>(model: S) -> Self {
        Self::ModelNotAvailable {
            model: model.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout<S: Into<String>>(operation: S) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied<S: Into<String>>(resource: S) -> Self {
        Self::PermissionDenied {
            resource: resource.into(),
        }
    }

    /// Create an invalid state error
    pub fn invalid_state<S: Into<String>>(message: S) -> Self {
        Self::InvalidState {
            message: message.into(),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported<S: Into<String>>(operation: S) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. } | Self::Timeout { .. } | Self::Http(_) => true,
            Self::AiService { .. } => true, // AI services might be temporarily down
            _ => false,
        }
    }

    /// Get error category for logging/metrics
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Io(_) | Self::FileSystem { .. } => ErrorCategory::FileSystem,
            Self::Git(_) => ErrorCategory::Git,
            Self::Http(_) | Self::Network { .. } => ErrorCategory::Network,
            Self::Json(_) | Self::Yaml(_) => ErrorCategory::Serialization,
            Self::Config(_) => ErrorCategory::Configuration,
            Self::Template(_) => ErrorCategory::Template,
            Self::Url(_) => ErrorCategory::Url,
            Self::Repository { .. } => ErrorCategory::Repository,
            Self::AiService { .. } | Self::ModelNotAvailable { .. } => ErrorCategory::AI,
            Self::Documentation { .. } => ErrorCategory::Documentation,
            Self::Pipeline { .. } => ErrorCategory::Pipeline,
            Self::Auth { .. } | Self::PermissionDenied { .. } => ErrorCategory::Security,
            Self::Validation { .. } => ErrorCategory::Validation,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::NotFound { .. } => ErrorCategory::NotFound,
            Self::InvalidState { .. } => ErrorCategory::State,
            Self::UnsupportedOperation { .. } => ErrorCategory::Unsupported,
            Self::Generic(_) => ErrorCategory::Generic,
        }
    }
}

/// Error categories for metrics and logging
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    FileSystem,
    Git,
    Network,
    Serialization,
    Configuration,
    Template,
    Url,
    Repository,
    AI,
    Documentation,
    Pipeline,
    Security,
    Validation,
    Timeout,
    NotFound,
    State,
    Unsupported,
    Generic,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileSystem => write!(f, "filesystem"),
            Self::Git => write!(f, "git"),
            Self::Network => write!(f, "network"),
            Self::Serialization => write!(f, "serialization"),
            Self::Configuration => write!(f, "configuration"),
            Self::Template => write!(f, "template"),
            Self::Url => write!(f, "url"),
            Self::Repository => write!(f, "repository"),
            Self::AI => write!(f, "ai"),
            Self::Documentation => write!(f, "documentation"),
            Self::Pipeline => write!(f, "pipeline"),
            Self::Security => write!(f, "security"),
            Self::Validation => write!(f, "validation"),
            Self::Timeout => write!(f, "timeout"),
            Self::NotFound => write!(f, "not_found"),
            Self::State => write!(f, "state"),
            Self::Unsupported => write!(f, "unsupported"),
            Self::Generic => write!(f, "generic"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = XzeError::repository("test message");
        assert!(matches!(err, XzeError::Repository { .. }));
        assert_eq!(err.to_string(), "Repository error: test message");
    }

    #[test]
    fn test_error_categories() {
        let err = XzeError::ai("test");
        assert_eq!(err.category(), ErrorCategory::AI);

        let err = XzeError::network("test");
        assert_eq!(err.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_retryable_errors() {
        assert!(XzeError::network("test").is_retryable());
        assert!(XzeError::timeout("test").is_retryable());
        assert!(!XzeError::validation("test").is_retryable());
        assert!(!XzeError::permission_denied("test").is_retryable());
    }

    #[test]
    fn test_error_from_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let xze_err: XzeError = io_err.into();
        assert!(matches!(xze_err, XzeError::Io(_)));

        let json_err = serde_json::from_str::<i32>("invalid json").unwrap_err();
        let xze_err: XzeError = json_err.into();
        assert!(matches!(xze_err, XzeError::Json(_)));
    }

    #[test]
    fn test_error_display() {
        let err = XzeError::model_not_available("gpt-4");
        assert_eq!(err.to_string(), "Model 'gpt-4' is not available");

        let err = XzeError::timeout("analysis");
        assert_eq!(err.to_string(), "Operation timed out: analysis");
    }
}
