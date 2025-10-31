//! Server module for XZe serve crate

use crate::api::create_routes;
use crate::handlers::AppState;

use crate::ServerConfig;
use anyhow;
use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use xze_core::{Result, XzeError};

/// XZe HTTP server
pub struct XzeServer {
    config: ServerConfig,
    app: Router,
}

impl XzeServer {
    /// Create a new server instance with async initialization
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let app = create_app(&config).await?;

        Ok(Self { config, app })
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e| XzeError::validation(format!("Invalid address {}: {}", addr, e)))?;

        tracing::info!("Starting XZe server on {}", addr);

        let listener = tokio::net::TcpListener::bind(socket_addr)
            .await
            .map_err(|e| XzeError::network(format!("Failed to bind to {}: {}", addr, e)))?;

        axum::serve(listener, self.app)
            .await
            .map_err(|e| XzeError::network(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

/// Create the Axum application with middleware
async fn create_app(config: &ServerConfig) -> Result<Router> {
    // Initialize application state with database connection
    let state = AppState::new(config.clone())
        .await
        .map_err(|e| XzeError::Generic(anyhow::anyhow!("Failed to connect to database: {}", e)))?;

    let mut app = create_routes().with_state(state);

    // Add middleware layers
    app = app.layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(RequestBodyLimitLayer::new(config.max_request_size)),
    );

    // Add CORS if enabled
    if config.cors_enabled {
        let cors = CorsLayer::new()
            .allow_origin("*".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE]);

        app = app.layer(cors);
    }

    Ok(app)
}

/// Server builder for configuration
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }

    /// Set the host address
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Set the Ollama URL
    pub fn ollama_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.ollama_url = url.into();
        self
    }

    /// Set the database URL
    pub fn database_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.database_url = url.into();
        self
    }

    /// Enable or disable CORS
    pub fn cors(mut self, enabled: bool) -> Self {
        self.config.cors_enabled = enabled;
        self
    }

    /// Set maximum request size
    pub fn max_request_size(mut self, size: usize) -> Self {
        self.config.max_request_size = size;
        self
    }

    /// Build the server with async initialization
    pub async fn build(self) -> Result<XzeServer> {
        XzeServer::new(self.config).await
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_builder() {
        // Note: This test will fail without a real database connection
        // For now, we just test the builder configuration
        let builder = ServerBuilder::new()
            .host("0.0.0.0")
            .port(8080)
            .ollama_url("http://localhost:11435")
            .database_url("postgresql://localhost/xze_test")
            .cors(false)
            .max_request_size(5 * 1024 * 1024);

        assert_eq!(builder.config.host, "0.0.0.0");
        assert_eq!(builder.config.port, 8080);
        assert_eq!(builder.config.ollama_url, "http://localhost:11435");
        assert_eq!(
            builder.config.database_url,
            "postgresql://localhost/xze_test"
        );
        assert!(!builder.config.cors_enabled);
        assert_eq!(builder.config.max_request_size, 5 * 1024 * 1024);
    }
}
