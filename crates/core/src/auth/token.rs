use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OAuth 2.0 Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// The access token string
    pub access_token: String,

    /// The refresh token string (optional)
    pub refresh_token: Option<String>,

    /// The type of token (usually "Bearer")
    pub token_type: String,

    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,

    /// Scopes associated with the token
    pub scope: Option<String>,
}

impl Token {
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Get remaining time until expiration
    pub fn expires_in(&self) -> Option<Duration> {
        self.expires_at.map(|expires_at| {
            let now = Utc::now();
            if expires_at > now {
                (expires_at - now).to_std().unwrap_or(Duration::ZERO)
            } else {
                Duration::ZERO
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_token_expiration() {
        let token = Token {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() - ChronoDuration::seconds(10)),
            scope: None,
        };
        assert!(token.is_expired());
        assert_eq!(token.expires_in(), Some(Duration::ZERO));

        let token = Token {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + ChronoDuration::seconds(3600)),
            scope: None,
        };
        assert!(!token.is_expired());
        assert!(token.expires_in().unwrap().as_secs() > 0);
    }
}
