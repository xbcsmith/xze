use crate::auth::config::ProviderConfig;
use crate::auth::errors::AuthError;
use crate::auth::token::Token;
use reqwest::Client;

/// Execute Client Credentials Flow
pub async fn client_credentials_flow(
    client: &Client,
    config: &ProviderConfig,
) -> Result<Token, AuthError> {
    let token_url = config
        .token_url
        .as_ref()
        .ok_or_else(|| AuthError::Config("Missing token_url".into()))?;
    let client_secret = config
        .client_secret
        .as_ref()
        .ok_or_else(|| AuthError::Config("Missing client_secret".into()))?;

    let mut params = vec![
        ("grant_type", "client_credentials".to_string()),
        ("client_id", config.client_id.clone()),
        ("client_secret", client_secret.clone()),
    ];

    if let Some(scopes) = &config.scopes {
        params.push(("scope", scopes.join(" ")));
    }

    let response = client.post(token_url).form(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AuthError::OAuth(format!(
            "Client credentials flow failed: {}",
            error_text
        )));
    }

    let token: Token = response.json().await?;
    Ok(token)
}
