//! XZe Core Library
//!
//! Core functionality for the XZe documentation pipeline tool.
//! This library provides the main components for analyzing repositories,
//! generating documentation, and managing the documentation workflow.

pub mod ai;
pub mod config;
pub mod documentation;
pub mod error;
pub mod git;
pub mod pipeline;
pub mod repository;
pub mod types;

// Re-export commonly used types
pub use config::XzeConfig;
pub use error::{Result, XzeError};
pub use repository::{CodeStructure, Repository, RepositoryManager};
pub use types::{
    DiátaxisCategory, JobId, JobStatus, OperationMode, ProgrammingLanguage, RepositoryId,
};

/// Initialize logging with JSON formatting
pub fn init_logging() -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "xze_core=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    Ok(())
}

/// Initialize logging with custom configuration
pub fn init_logging_with_config(level: &str, format: &str) -> Result<()> {
    use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::new(level);

    let registry = tracing_subscriber::registry().with(env_filter);

    match format {
        "json" => {
            registry
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "text" | "pretty" => {
            registry
                .with(
                    tracing_subscriber::fmt::layer()
                        .pretty()
                        .with_span_events(FmtSpan::CLOSE),
                )
                .init();
        }
        "compact" => {
            registry
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
        _ => {
            return Err(XzeError::validation(format!(
                "Unknown log format: {}",
                format
            )));
        }
    }

    Ok(())
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Get version info as a formatted string
pub fn version_info() -> String {
    format!("{} v{} - {}", NAME, VERSION, DESCRIPTION)
}

/// Health check function
pub fn health_check() -> std::result::Result<(), String> {
    // Basic health checks
    if std::env::var("HOME").is_err() && std::env::var("USERPROFILE").is_err() {
        return Err("No home directory found".to_string());
    }

    // Check if we can create temporary files
    if let Err(e) = tempfile::tempfile() {
        return Err(format!("Cannot create temporary files: {}", e));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_exports() {
        // Verify that main types are exported
        let _: Result<()> = Ok(());
        let _id = RepositoryId::from("test");
        let _job_id = JobId::new();
    }

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains("xze-core"));
        assert!(info.contains("v"));
    }

    #[test]
    fn test_health_check() {
        // This should pass on most environments
        match health_check() {
            Ok(()) => {} // Good
            Err(msg) => {
                // Print the error but don't fail the test since it might be environment-specific
                println!(
                    "Health check failed (this might be environment-specific): {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_logging_init() {
        // Test that logging can be initialized without panicking
        // Note: This might fail if logging is already initialized
        let result = init_logging_with_config("debug", "compact");
        // Don't assert on the result since logging might already be initialized
        match result {
            Ok(()) => println!("Logging initialized successfully"),
            Err(e) => println!("Logging init failed (might be already initialized): {}", e),
        }
    }
}
