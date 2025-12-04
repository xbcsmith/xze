use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Token storage configuration
    pub storage: StorageConfig,

    /// OIDC configuration
    #[serde(default)]
    pub oidc: Option<OidcConfig>,

    /// OAuth provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

/// Token storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type (file, memory, keychain)
    pub r#type: String,

    /// Path to storage file (if applicable)
    pub path: Option<String>,
}

/// OIDC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Whether OIDC is enabled
    pub enabled: bool,

    /// OIDC issuer URL
    pub issuer: String,

    /// Expected audience
    pub audience: String,

    /// Clock skew tolerance in seconds
    pub clock_skew_seconds: Option<u64>,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Client ID
    pub client_id: String,

    /// Client Secret (optional for public clients)
    pub client_secret: Option<String>,

    /// Requested scopes
    pub scopes: Option<Vec<String>>,

    /// Authorization endpoint URL (optional if using discovery)
    pub auth_url: Option<String>,

    /// Token endpoint URL (optional if using discovery)
    pub token_url: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig {
                r#type: "memory".to_string(),
                path: None,
            },
            oidc: None,
            providers: HashMap::new(),
        }
    }
}
