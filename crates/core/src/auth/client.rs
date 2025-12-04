use crate::auth::config::ProviderConfig;
use crate::auth::errors::AuthError;
use crate::auth::storage::TokenStorage;
use crate::auth::token::Token;
use reqwest::Client;
use std::sync::Arc;

/// OAuth 2.0 Client for managing tokens
pub struct OAuthClient {
    http_client: Client,
    storage: Arc<dyn TokenStorage>,
}

impl OAuthClient {
    /// Create a new OAuth client
    pub fn new(storage: Arc<dyn TokenStorage>) -> Self {
        Self {
            http_client: Client::new(),
            storage,
        }
    }

    /// Get a valid token for a provider, refreshing if necessary
    pub async fn get_token(
        &self,
        provider: &str,
        config: &ProviderConfig,
    ) -> Result<Token, AuthError> {
        // 1. Try to load from storage
        if let Some(token) = self.storage.load_token(provider).await? {
            if !token.is_expired() {
                return Ok(token);
            }

            // 2. Try to refresh if expired
            if let Some(refresh_token) = &token.refresh_token {
                match self.refresh_token(provider, config, refresh_token).await {
                    Ok(new_token) => return Ok(new_token),
                    Err(e) => tracing::warn!("Failed to refresh token: {}", e),
                }
            }
        }

        // 3. If no token or refresh failed, return error (user needs to re-authenticate)
        Err(AuthError::TokenExpired)
    }

    /// Refresh an access token using a refresh token
    async fn refresh_token(
        &self,
        provider: &str,
        config: &ProviderConfig,
        refresh_token: &str,
    ) -> Result<Token, AuthError> {
        let token_url = config
            .token_url
            .as_ref()
            .ok_or_else(|| AuthError::Config("Missing token_url".into()))?;

        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
        ];

        if let Some(secret) = &config.client_secret {
            params.push(("client_secret", secret));
        }

        let response = self
            .http_client
            .post(token_url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AuthError::OAuth(format!("Refresh failed: {}", error_text)));
        }

        let token: Token = response.json().await?;
        self.storage.save_token(provider, &token).await?;

        Ok(token)
    }
}
