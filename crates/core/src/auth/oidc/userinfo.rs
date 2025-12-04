use crate::auth::errors::AuthError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Standard OIDC User Profile
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    /// Subject identifier
    pub sub: String,

    /// Full name
    pub name: Option<String>,

    /// Given name
    pub given_name: Option<String>,

    /// Family name
    pub family_name: Option<String>,

    /// Email address
    pub email: Option<String>,

    /// Whether email is verified
    pub email_verified: Option<bool>,

    /// Profile picture URL
    pub picture: Option<String>,
}

/// Client for UserInfo endpoint
pub struct UserInfoClient {
    client: Client,
}

impl UserInfoClient {
    /// Create a new UserInfo client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch user profile from UserInfo endpoint
    pub async fn get_user_info(
        &self,
        endpoint: &str,
        access_token: &str,
    ) -> Result<UserProfile, AuthError> {
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::Network(response.error_for_status().unwrap_err()));
        }

        let profile: UserProfile = response.json().await?;
        Ok(profile)
    }
}

impl Default for UserInfoClient {
    fn default() -> Self {
        Self::new()
    }
}
