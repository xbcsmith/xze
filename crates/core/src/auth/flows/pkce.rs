use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rng, Rng};
use sha2::{Digest, Sha256};

/// PKCE Code Verifier and Challenge
pub struct Pkce {
    /// The code verifier (secret)
    pub verifier: String,

    /// The code challenge (public)
    pub challenge: String,
}

impl Pkce {
    /// Generate a new PKCE pair
    pub fn new() -> Self {
        let verifier = Self::generate_verifier();
        let challenge = Self::generate_challenge(&verifier);
        Self {
            verifier,
            challenge,
        }
    }

    fn generate_verifier() -> String {
        let mut rng = rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = Pkce::new();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn test_challenge_verification() {
        let verifier = "test_verifier";
        let challenge = Pkce::generate_challenge(verifier);

        // Verify manually
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(hash);

        assert_eq!(challenge, expected);
    }
}
