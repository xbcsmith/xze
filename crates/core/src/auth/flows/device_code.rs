use crate::auth::config::ProviderConfig;
use crate::auth::errors::AuthError;
use crate::auth::token::Token;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

/// Device Code Response
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    /// Device verification code
    pub device_code: String,

    /// User verification code
    pub user_code: String,

    /// Verification URL
    pub verification_uri: String,

    /// Complete verification URL (with code)
    pub verification_uri_complete: Option<String>,

    /// Expiration time in seconds
    pub expires_in: u64,

    /// Polling interval in seconds
    pub interval: u64,
}

/// Initiate Device Code Flow
pub async fn initiate_device_flow(
    client: &Client,
    config: &ProviderConfig,
    device_auth_url: &str,
) -> Result<DeviceCodeResponse, AuthError> {
    let mut params = vec![("client_id", config.client_id.clone())];

    if let Some(scopes) = &config.scopes {
        params.push(("scope", scopes.join(" ")));
    }

    let response = client.post(device_auth_url).form(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AuthError::OAuth(format!(
            "Device flow initiation failed: {}",
            error_text
        )));
    }

    let code: DeviceCodeResponse = response.json().await?;
    Ok(code)
}

/// Poll for token after user authorization
pub async fn poll_device_token(
    client: &Client,
    config: &ProviderConfig,
    device_code: &str,
    interval: u64,
    timeout: u64,
) -> Result<Token, AuthError> {
    let token_url = config
        .token_url
        .as_ref()
        .ok_or_else(|| AuthError::Config("Missing token_url".into()))?;
    let start = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout);

    loop {
        if start.elapsed() > timeout_duration {
            return Err(AuthError::OAuth("Device flow timed out".into()));
        }

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
            ("client_id", &config.client_id),
        ];

        let response = client.post(token_url).form(&params).send().await?;

        if response.status().is_success() {
            return Ok(response.json().await?);
        }

        // Check for pending error
        let error_text = response.text().await.unwrap_or_default();
        if error_text.contains("authorization_pending") {
            sleep(Duration::from_secs(interval)).await;
            continue;
        }

        if error_text.contains("slow_down") {
            sleep(Duration::from_secs(interval + 5)).await;
            continue;
        }

        return Err(AuthError::OAuth(format!(
            "Device flow failed: {}",
            error_text
        )));
    }
}
