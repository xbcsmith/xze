//! XZe CLI Library
//!
//! Command-line interface components for the XZe documentation pipeline tool.

use xze_core::{Result, XzeError};

pub mod commands;
pub mod config;
pub mod output;

pub use commands::*;
pub use config::*;
pub use output::*;

/// CLI version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the CLI environment
pub fn init() -> Result<()> {
    // Set up panic handler
    std::panic::set_hook(Box::new(|info| {
        eprintln!("XZe CLI encountered an error: {}", info);
    }));

    Ok(())
}

/// Check if running in CI environment
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
}

/// Get the appropriate exit code for an error
pub fn exit_code_for_error(error: &XzeError) -> i32 {
    match error {
        XzeError::Validation { .. } => 2,
        XzeError::NotFound { .. } => 3,
        XzeError::PermissionDenied { .. } => 4,
        XzeError::Network { .. } => 5,
        XzeError::AiService { .. } => 6,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }

    #[test]
    fn test_exit_codes() {
        let validation_error = XzeError::validation("test");
        assert_eq!(exit_code_for_error(&validation_error), 2);

        let not_found_error = XzeError::not_found("test");
        assert_eq!(exit_code_for_error(&not_found_error), 3);
    }
}
