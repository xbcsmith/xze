//! Logging infrastructure for XZe
//!
//! This module provides centralized logging configuration and utilities
//! using the tracing ecosystem.

use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use xze_core::{Result, XzeError};

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Whether to use JSON format
    pub json_format: bool,
    /// Whether to include timestamps
    pub with_timestamps: bool,
    /// Whether to include file/line information
    pub with_file_info: bool,
    /// Custom log format string
    pub custom_format: Option<String>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            with_timestamps: true,
            with_file_info: false,
            custom_format: None,
        }
    }
}

/// Initialize the global logger with the given configuration
pub fn init_logger(config: LoggerConfig) -> Result<()> {
    let level = Level::from_str(&config.level).map_err(|e| {
        XzeError::validation(format!("Invalid log level '{}': {}", config.level, e))
    })?;

    let env_filter = EnvFilter::from_default_env()
        .add_directive(level.into())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap());

    let fmt_layer = if config.json_format {
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_file(config.with_file_info)
            .with_line_number(config.with_file_info)
            .boxed()
    } else {
        let layer = fmt::layer()
            .with_target(true)
            .with_file(config.with_file_info)
            .with_line_number(config.with_file_info);

        if config.with_timestamps {
            layer.boxed()
        } else {
            layer.without_time().boxed()
        }
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| XzeError::validation(format!("Failed to initialize logger: {}", e)))?;

    tracing::info!("Logger initialized with level: {}", config.level);
    Ok(())
}

/// Initialize logger with default configuration
pub fn init_default_logger() -> Result<()> {
    init_logger(LoggerConfig::default())
}

/// Initialize logger for testing (reduces noise)
pub fn init_test_logger() -> Result<()> {
    let config = LoggerConfig {
        level: "warn".to_string(),
        json_format: false,
        with_timestamps: false,
        with_file_info: false,
        custom_format: None,
    };

    // Ignore errors if already initialized
    let _ = init_logger(config);
    Ok(())
}

/// Create a logger configuration from environment variables
pub fn logger_config_from_env() -> LoggerConfig {
    LoggerConfig {
        level: std::env::var("XZE_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        json_format: std::env::var("XZE_LOG_JSON")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false),
        with_timestamps: std::env::var("XZE_LOG_TIMESTAMPS")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true),
        with_file_info: std::env::var("XZE_LOG_FILE_INFO")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false),
        custom_format: std::env::var("XZE_LOG_FORMAT").ok(),
    }
}

/// Log level utilities
pub struct LogLevel;

impl LogLevel {
    /// Parse log level from string
    pub fn parse(level: &str) -> Result<Level> {
        Level::from_str(level)
            .map_err(|e| XzeError::validation(format!("Invalid log level '{}': {}", level, e)))
    }

    /// Get all available log levels
    pub fn all_levels() -> Vec<&'static str> {
        vec!["trace", "debug", "info", "warn", "error"]
    }

    /// Check if a log level string is valid
    pub fn is_valid(level: &str) -> bool {
        Self::all_levels().contains(&level.to_lowercase().as_str())
    }
}

/// Structured logging utilities
pub struct StructuredLog;

impl StructuredLog {
    /// Log an event with structured data
    pub fn event(level: Level, message: &str, fields: &[(&str, &dyn std::fmt::Display)]) {
        // Simple structured logging without dynamic fields
        match level {
            Level::ERROR => tracing::error!("{}", message),
            Level::WARN => tracing::warn!("{}", message),
            Level::INFO => tracing::info!("{}", message),
            Level::DEBUG => tracing::debug!("{}", message),
            Level::TRACE => tracing::trace!("{}", message),
        }

        // Log fields separately
        for (key, value) in fields {
            tracing::info!("  {}: {}", key, value);
        }
    }

    /// Create a timed operation span
    pub fn timed_operation(name: &str) -> TimedSpan {
        let span = tracing::info_span!("operation", name = name);
        let start = std::time::Instant::now();
        TimedSpan {
            span,
            start,
            name: name.to_string(),
        }
    }
}

/// A span that tracks elapsed time
pub struct TimedSpan {
    span: tracing::Span,
    start: std::time::Instant,
    name: String,
}

impl TimedSpan {
    /// Execute a closure within this span's context
    pub fn in_scope<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.span.in_scope(f)
    }

    /// Record the elapsed time manually
    pub fn record_elapsed(&self) {
        let elapsed = self.start.elapsed();
        self.span.in_scope(|| {
            tracing::info!(
                "Operation '{}' completed in {}ms",
                self.name,
                elapsed.as_millis()
            );
        });
    }
}

impl Drop for TimedSpan {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        self.span.in_scope(|| {
            tracing::info!(
                "Operation '{}' completed in {}ms",
                self.name,
                elapsed.as_millis()
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json_format);
        assert!(config.with_timestamps);
        assert!(!config.with_file_info);
    }

    #[test]
    fn test_log_level_parse() {
        assert!(LogLevel::parse("info").is_ok());
        assert!(LogLevel::parse("debug").is_ok());
        assert!(LogLevel::parse("invalid").is_err());
    }

    #[test]
    fn test_log_level_validation() {
        assert!(LogLevel::is_valid("info"));
        assert!(LogLevel::is_valid("ERROR"));
        assert!(!LogLevel::is_valid("invalid"));
    }

    #[test]
    fn test_all_log_levels() {
        let levels = LogLevel::all_levels();
        assert_eq!(levels.len(), 5);
        assert!(levels.contains(&"info"));
        assert!(levels.contains(&"error"));
    }

    #[test]
    fn test_logger_config_from_env() {
        // Test with no env vars set
        let config = logger_config_from_env();
        assert_eq!(config.level, "info"); // default

        // Test with env vars set
        std::env::set_var("XZE_LOG_LEVEL", "debug");
        std::env::set_var("XZE_LOG_JSON", "true");

        let config = logger_config_from_env();
        assert_eq!(config.level, "debug");
        assert!(config.json_format);

        // Clean up
        std::env::remove_var("XZE_LOG_LEVEL");
        std::env::remove_var("XZE_LOG_JSON");
    }

    #[tokio::test]
    async fn test_init_test_logger() {
        // Should not panic even if called multiple times
        assert!(init_test_logger().is_ok());
        assert!(init_test_logger().is_ok());
    }

    #[test]
    fn test_timed_span() {
        let span = StructuredLog::timed_operation("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        span.record_elapsed();
        // Span will be dropped here and should log elapsed time again
    }

    #[test]
    fn test_timed_span_in_scope() {
        let span = StructuredLog::timed_operation("scoped_operation");
        let result = span.in_scope(|| {
            std::thread::sleep(std::time::Duration::from_millis(5));
            42
        });
        assert_eq!(result, 42);
    }
}
