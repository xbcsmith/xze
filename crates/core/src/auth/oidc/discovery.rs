use crate::auth::errors::AuthError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// OIDC Discovery Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfiguration {
    /// Issuer URL
    pub issuer: String,

    /// Authorization endpoint URL
    pub authorization_endpoint: String,

    /// Token endpoint URL
    pub token_endpoint: String,

    /// UserInfo endpoint URL
    pub userinfo_endpoint: Option<String>,

    /// JWKS URI
    pub jwks_uri: String,

    /// Supported scopes
    pub scopes_supported: Option<Vec<String>>,

    /// Supported response types
    pub response_types_supported: Option<Vec<String>>,
}

/// Client for fetching OIDC configuration
#[derive(Clone)]
pub struct DiscoveryClient {
    client: Client,
    cache: Arc<RwLock<Option<(OidcConfiguration, Instant)>>>,
    cache_ttl: Duration,
}

impl DiscoveryClient {
    /// Create a new discovery client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Get OIDC configuration for an issuer
    pub async fn get_configuration(&self, issuer: &str) -> Result<OidcConfiguration, AuthError> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((config, timestamp)) = &*cache {
                if timestamp.elapsed() < self.cache_ttl && config.issuer == issuer {
                    return Ok(config.clone());
                }
            }
        }

        // Fetch
        let url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(AuthError::Network(response.error_for_status().unwrap_err()));
        }

        let config: OidcConfiguration = response.json().await?;

        // Validate issuer (allow trailing slash mismatch)
        if config.issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
            return Err(AuthError::Config(format!(
                "Issuer mismatch: expected {}, got {}",
                issuer, config.issuer
            )));
        }

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some((config.clone(), Instant::now()));
        }

        Ok(config)
    }
}

impl Default for DiscoveryClient {
    fn default() -> Self {
        Self::new()
    }
}
