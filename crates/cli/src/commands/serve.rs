//! Serve command implementation

use clap::Args;
use std::net::SocketAddr;
use std::path::PathBuf;
use xze_core::{Result, XzeError};

use crate::commands::CliCommand;

/// Start the XZe web server
#[derive(Debug, Args)]
pub struct ServeCommand {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "3000")]
    pub port: u16,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Ollama API URL
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Enable development mode with hot reload
    #[arg(long)]
    pub dev: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Enable JSON logging format
    #[arg(long)]
    pub json_logs: bool,

    /// Worker threads for the server
    #[arg(long)]
    pub workers: Option<usize>,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Maximum request body size in bytes
    #[arg(long, default_value = "10485760")] // 10MB
    pub max_body_size: usize,

    /// Enable CORS
    #[arg(long)]
    pub cors: bool,

    /// TLS certificate file path
    #[arg(long)]
    pub tls_cert: Option<PathBuf>,

    /// TLS private key file path
    #[arg(long)]
    pub tls_key: Option<PathBuf>,
}

impl CliCommand for ServeCommand {
    async fn execute(&self) -> Result<()> {
        // Initialize logging
        self.init_logging()?;

        tracing::info!("Starting XZe server");
        tracing::info!("Host: {}", self.host);
        tracing::info!("Port: {}", self.port);
        tracing::info!("Ollama URL: {}", self.ollama_url);

        // Parse socket address
        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| XzeError::validation(format!("Invalid host:port combination: {}", e)))?;

        // Load configuration
        let config = self.load_config().await?;

        // Build server
        let server = self.build_server(config).await?;

        // Start server
        tracing::info!("🚀 XZe server starting on http://{}", addr);

        if let Err(e) = server.serve(addr).await {
            return Err(XzeError::network(format!("Server error: {}", e)));
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "serve"
    }

    fn validate(&self) -> Result<()> {
        // Validate port range
        if self.port == 0 {
            return Err(XzeError::validation("Port cannot be 0".to_string()));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            return Err(XzeError::validation(format!(
                "Invalid log level: {}. Valid levels: {}",
                self.log_level,
                valid_levels.join(", ")
            )));
        }

        // Validate timeout
        if self.timeout == 0 {
            return Err(XzeError::validation("Timeout cannot be 0".to_string()));
        }

        // Validate max body size
        if self.max_body_size == 0 {
            return Err(XzeError::validation(
                "Max body size cannot be 0".to_string(),
            ));
        }

        // Validate TLS configuration
        match (&self.tls_cert, &self.tls_key) {
            (Some(_), None) => {
                return Err(XzeError::validation(
                    "TLS private key is required when certificate is provided".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(XzeError::validation(
                    "TLS certificate is required when private key is provided".to_string(),
                ));
            }
            _ => {}
        }

        // Validate TLS files exist
        if let Some(cert_path) = &self.tls_cert {
            if !cert_path.exists() {
                return Err(XzeError::filesystem(format!(
                    "TLS certificate file not found: {}",
                    cert_path.display()
                )));
            }
        }

        if let Some(key_path) = &self.tls_key {
            if !key_path.exists() {
                return Err(XzeError::filesystem(format!(
                    "TLS private key file not found: {}",
                    key_path.display()
                )));
            }
        }

        Ok(())
    }
}

impl ServeCommand {
    /// Initialize logging based on command arguments
    fn init_logging(&self) -> Result<()> {
        use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

        let filter = EnvFilter::new(&self.log_level);

        if self.json_logs {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json())
                .try_init()
                .map_err(|e| XzeError::validation(format!("Failed to initialize logger: {}", e)))?;
        } else {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_target(self.verbose))
                .try_init()
                .map_err(|e| XzeError::validation(format!("Failed to initialize logger: {}", e)))?;
        }

        Ok(())
    }

    /// Load server configuration
    async fn load_config(&self) -> Result<ServerConfig> {
        let mut config = ServerConfig {
            host: self.host.clone(),
            port: self.port,
            ollama_url: self.ollama_url.clone(),
            dev_mode: self.dev,
            timeout_seconds: self.timeout,
            max_body_size: self.max_body_size,
            enable_cors: self.cors,
            tls_cert: self.tls_cert.clone(),
            tls_key: self.tls_key.clone(),
            worker_threads: self.workers.unwrap_or_else(num_cpus::get),
        };

        // Load from config file if provided
        if let Some(config_path) = &self.config {
            config = self.load_config_file(config_path, config).await?;
        }

        Ok(config)
    }

    /// Load configuration from file
    async fn load_config_file(
        &self,
        path: &PathBuf,
        mut config: ServerConfig,
    ) -> Result<ServerConfig> {
        if !path.exists() {
            return Err(XzeError::filesystem(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read config file: {}", e)))?;

        // Try YAML first, then JSON
        let file_config: ServerConfig = if path.extension().and_then(|s| s.to_str()) == Some("json")
        {
            serde_json::from_str(&content)
                .map_err(|e| XzeError::validation(format!("Invalid JSON config: {}", e)))?
        } else {
            serde_yaml::from_str(&content)
                .map_err(|e| XzeError::validation(format!("Invalid YAML config: {}", e)))?
        };

        // Merge configurations (command line takes precedence)
        config.merge_with(file_config);
        Ok(config)
    }

    /// Build the server with the given configuration
    async fn build_server(&self, config: ServerConfig) -> Result<XzeServer> {
        // TODO: Initialize actual server components
        // This would integrate with xze-serve crate
        tracing::info!("Building server with configuration: {:?}", config);

        Ok(XzeServer::new(config))
    }
}

/// Server configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub ollama_url: String,
    pub dev_mode: bool,
    pub timeout_seconds: u64,
    pub max_body_size: usize,
    pub enable_cors: bool,
    pub worker_threads: usize,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            ollama_url: "http://localhost:11434".to_string(),
            dev_mode: false,
            timeout_seconds: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            enable_cors: false,
            worker_threads: num_cpus::get(),
            tls_cert: None,
            tls_key: None,
        }
    }
}

impl ServerConfig {
    /// Merge with another configuration, keeping self's values where present
    pub fn merge_with(&mut self, other: ServerConfig) {
        // Only merge values that weren't explicitly set via command line
        if self.host == "127.0.0.1" && other.host != "127.0.0.1" {
            self.host = other.host;
        }
        if self.port == 3000 && other.port != 3000 {
            self.port = other.port;
        }
        if self.ollama_url == "http://localhost:11434"
            && other.ollama_url != "http://localhost:11434"
        {
            self.ollama_url = other.ollama_url;
        }
        // Add more fields as needed
    }
}

/// Mock server implementation
/// TODO: Replace with actual xze-serve integration
pub struct XzeServer {
    config: ServerConfig,
}

impl XzeServer {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    pub async fn serve(&self, addr: SocketAddr) -> Result<()> {
        tracing::info!("Mock server would start on {}", addr);
        tracing::info!("Server configuration: {:?}", self.config);

        // TODO: Implement actual server startup
        // This would use xze-serve crate to start the HTTP server

        // For now, just simulate server running
        tokio::signal::ctrl_c().await.map_err(|e| {
            XzeError::network(format!("Failed to listen for shutdown signal: {}", e))
        })?;

        tracing::info!("Shutting down server");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_valid_config() {
        let cmd = ServeCommand {
            host: "0.0.0.0".to_string(),
            port: 8080,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "info".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: None,
            tls_key: None,
        };

        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_port() {
        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 0,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "info".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: None,
            tls_key: None,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 3000,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "invalid".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: None,
            tls_key: None,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_validate_tls_config_mismatch() {
        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 3000,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "info".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: Some(PathBuf::from("cert.pem")),
            tls_key: None,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert!(!config.dev_mode);
    }

    #[test]
    fn test_server_config_merge() {
        let mut config1 = ServerConfig::default();
        let mut config2 = ServerConfig::default();
        config2.host = "0.0.0.0".to_string();
        config2.port = 8080;

        config1.merge_with(config2);
        assert_eq!(config1.host, "0.0.0.0");
        assert_eq!(config1.port, 8080);
    }

    #[tokio::test]
    async fn test_load_config_file_not_found() {
        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 3000,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "info".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: None,
            tls_key: None,
        };

        let result = cmd
            .load_config_file(
                &PathBuf::from("/nonexistent/config.yaml"),
                ServerConfig::default(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_yaml_config_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
host: "0.0.0.0"
port: 8080
ollama_url: "http://localhost:11434"
dev_mode: true
timeout_seconds: 60
max_body_size: 20971520
enable_cors: true
worker_threads: 4
"#;

        std::io::Write::write_all(&mut temp_file, config_content.as_bytes()).unwrap();

        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 3000,
            config: None,
            ollama_url: "http://localhost:11434".to_string(),
            dev: false,
            verbose: false,
            log_level: "info".to_string(),
            json_logs: false,
            workers: None,
            timeout: 30,
            max_body_size: 1024 * 1024,
            cors: false,
            tls_cert: None,
            tls_key: None,
        };

        let result = cmd
            .load_config_file(&temp_file.path().to_path_buf(), ServerConfig::default())
            .await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(config.dev_mode);
    }
}
