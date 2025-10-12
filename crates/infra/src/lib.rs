//! XZe Infrastructure Library
//!
//! Infrastructure components for the XZe documentation pipeline tool.
//! This includes Ollama client, file system abstractions, and other
//! infrastructure-level services.

use xze_core::{Result, XzeError};

pub mod cache;
pub mod filesystem;
pub mod logger;
pub mod ollama;

pub use cache::*;
pub use filesystem::*;
pub use logger::*;
pub use ollama::*;

/// Infrastructure version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Infrastructure configuration
#[derive(Debug, Clone)]
pub struct InfraConfig {
    pub ollama_url: String,
    pub ollama_timeout_seconds: u64,
    pub cache_dir: std::path::PathBuf,
    pub max_cache_size_mb: usize,
    pub log_level: String,
}

impl Default for InfraConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            ollama_timeout_seconds: 300,
            cache_dir: std::env::temp_dir().join("xze-cache"),
            max_cache_size_mb: 1024,
            log_level: "info".to_string(),
        }
    }
}

/// Initialize infrastructure components
pub async fn init_infrastructure(config: InfraConfig) -> Result<()> {
    tracing::info!("Initializing XZe infrastructure v{}", VERSION);

    // Create cache directory if it doesn't exist
    if !config.cache_dir.exists() {
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| {
            XzeError::filesystem(format!("Failed to create cache directory: {}", e))
        })?;
    }

    tracing::info!("Cache directory: {:?}", config.cache_dir);
    tracing::info!("Ollama URL: {}", config.ollama_url);

    Ok(())
}

/// Health check for infrastructure components
pub async fn health_check(config: &InfraConfig) -> Result<HealthStatus> {
    let mut status = HealthStatus::new();

    // Check cache directory
    if config.cache_dir.exists() && config.cache_dir.is_dir() {
        status.cache_accessible = true;
    }

    // TODO: Check Ollama connectivity
    status.ollama_accessible = false; // Placeholder

    Ok(status)
}

/// Health status for infrastructure components
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub cache_accessible: bool,
    pub ollama_accessible: bool,
    pub filesystem_accessible: bool,
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            cache_accessible: false,
            ollama_accessible: false,
            filesystem_accessible: true, // Assume true by default
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.cache_accessible && self.filesystem_accessible
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = InfraConfig::default();
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.ollama_timeout_seconds, 300);
        assert_eq!(config.max_cache_size_mb, 1024);
    }

    #[tokio::test]
    async fn test_infrastructure_init() {
        let temp_dir = TempDir::new().unwrap();
        let config = InfraConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        assert!(init_infrastructure(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let config = InfraConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let status = health_check(&config).await.unwrap();
        assert!(status.cache_accessible);
        assert!(status.filesystem_accessible);
    }

    #[test]
    fn test_health_status() {
        let status = HealthStatus::new();
        assert!(!status.is_healthy()); // Should be false by default

        let healthy_status = HealthStatus {
            cache_accessible: true,
            ollama_accessible: true,
            filesystem_accessible: true,
        };
        assert!(healthy_status.is_healthy());
    }
}
