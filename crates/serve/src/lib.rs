//! XZe Serve Library
//!
//! Web server interface for the XZe documentation pipeline tool.

use xze_core::Result;

pub mod api;
pub mod handlers;
pub mod middleware;
pub mod server;

pub use handlers::*;
pub use server::*;

/// Server version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub ollama_url: String,
    pub cors_enabled: bool,
    pub max_request_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            ollama_url: "http://localhost:11434".to_string(),
            cors_enabled: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Initialize the server
pub async fn init_server(config: ServerConfig) -> Result<()> {
    tracing::info!("Initializing XZe server v{}", VERSION);
    tracing::info!("Server will bind to {}:{}", config.host, config.port);

    // TODO: Implement actual server initialization
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert!(config.cors_enabled);
    }

    #[tokio::test]
    async fn test_server_init() {
        let config = ServerConfig::default();
        assert!(init_server(config).await.is_ok());
    }
}
