use crate::auth::errors::AuthError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

/// ID Token Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject identifier
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Expiration time
    pub exp: usize,

    /// Issued at time
    pub iat: usize,

    /// Nonce
    pub nonce: Option<String>,

    /// Email
    pub email: Option<String>,

    /// Name
    pub name: Option<String>,
}

/// Token Validator
pub struct TokenValidator {
    issuer: String,
    audience: String,
}

impl TokenValidator {
    /// Create a new token validator
    pub fn new(issuer: String, audience: String) -> Self {
        Self { issuer, audience }
    }

    /// Validate an ID token
    ///
    /// Note: This currently only validates claims (iss, aud, exp).
    /// Signature verification requires fetching JWKS which is not yet implemented.
    pub fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken("Invalid token format".to_string()));
        }

        // Decode payload (second part)
        let payload = parts[1];
        // Add padding if needed (JWTs are not padded, but base64 might expect it? URL_SAFE_NO_PAD handles it usually)
        // Actually JWT uses base64url without padding.

        let decoded = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|e| AuthError::InvalidToken(format!("Base64 decode failed: {}", e)))?;

        let claims: Claims = serde_json::from_slice(&decoded)
            .map_err(|e| AuthError::InvalidToken(format!("JSON parse failed: {}", e)))?;

        // Validate Issuer
        if claims.iss != self.issuer {
            return Err(AuthError::InvalidToken(format!(
                "Invalid issuer: expected {}, got {}",
                self.issuer, claims.iss
            )));
        }

        // Validate Audience
        if claims.aud != self.audience {
            return Err(AuthError::InvalidToken(format!(
                "Invalid audience: expected {}, got {}",
                self.audience, claims.aud
            )));
        }

        // Validate Expiration
        let now = chrono::Utc::now().timestamp() as usize;
        if claims.exp < now {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_claims_validation() {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: "user123".to_string(),
            iss: "https://issuer.com".to_string(),
            aud: "my-app".to_string(),
            exp: now + 3600,
            iat: now,
            nonce: None,
            email: Some("user@example.com".to_string()),
            name: None,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        let validator = TokenValidator::new("https://issuer.com".to_string(), "my-app".to_string());
        let result = validator.validate(&token);

        assert!(result.is_ok());
        let validated_claims = result.unwrap();
        assert_eq!(validated_claims.sub, "user123");
    }

    #[test]
    fn test_expired_token() {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: "user123".to_string(),
            iss: "https://issuer.com".to_string(),
            aud: "my-app".to_string(),
            exp: now - 3600, // Expired
            iat: now - 7200,
            nonce: None,
            email: None,
            name: None,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"secret"),
        )
        .unwrap();

        let validator = TokenValidator::new("https://issuer.com".to_string(), "my-app".to_string());
        let result = validator.validate(&token);

        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }
}
