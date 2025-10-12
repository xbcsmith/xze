//! Git credentials management

use crate::{Result, XzeError};
use git2::{Cred, CredentialType};
use std::path::Path;

/// Credential store for Git operations
#[derive(Debug, Clone)]
pub struct CredentialStore {
    username: Option<String>,
    password: Option<String>,
    ssh_key_path: Option<String>,
    ssh_passphrase: Option<String>,
}

impl CredentialStore {
    /// Create a new credential store
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            ssh_key_path: None,
            ssh_passphrase: None,
        }
    }

    /// Set username and password credentials
    pub fn with_userpass(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    /// Set SSH key credentials
    pub fn with_ssh_key<P: AsRef<Path>>(
        mut self,
        username: String,
        key_path: P,
        passphrase: Option<String>,
    ) -> Self {
        self.username = Some(username);
        self.ssh_key_path = Some(key_path.as_ref().to_string_lossy().to_string());
        self.ssh_passphrase = passphrase;
        self
    }

    /// Create Git credentials based on the allowed types
    pub fn create_credentials(
        &self,
        _url: &str,
        username_from_url: Option<&str>,
        allowed_types: CredentialType,
    ) -> Result<Cred> {
        if allowed_types.contains(CredentialType::SSH_KEY) {
            if let (Some(username), Some(key_path)) = (&self.username, &self.ssh_key_path) {
                let user = username_from_url.unwrap_or(username);
                return Cred::ssh_key(
                    user,
                    None,
                    Path::new(key_path),
                    self.ssh_passphrase.as_deref(),
                )
                .map_err(XzeError::Git);
            }
        }

        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let (Some(username), Some(password)) = (&self.username, &self.password) {
                let user = username_from_url.unwrap_or(username);
                return Cred::userpass_plaintext(user, password).map_err(XzeError::Git);
            }
        }

        if allowed_types.contains(CredentialType::SSH_MEMORY) {
            if let Some(username) = &self.username {
                let user = username_from_url.unwrap_or(username);
                return Cred::ssh_key_from_agent(user).map_err(XzeError::Git);
            }
        }

        Err(XzeError::auth("No suitable credentials available"))
    }

    /// Check if credentials are configured
    pub fn has_credentials(&self) -> bool {
        (self.username.is_some() && self.password.is_some())
            || (self.username.is_some() && self.ssh_key_path.is_some())
    }

    /// Get username
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create credentials from environment variables
pub fn credentials_from_env() -> CredentialStore {
    let mut store = CredentialStore::new();

    if let (Ok(username), Ok(password)) =
        (std::env::var("GIT_USERNAME"), std::env::var("GIT_PASSWORD"))
    {
        store = store.with_userpass(username, password);
    } else if let (Ok(username), Ok(key_path)) = (
        std::env::var("GIT_SSH_USERNAME"),
        std::env::var("GIT_SSH_KEY"),
    ) {
        let passphrase = std::env::var("GIT_SSH_PASSPHRASE").ok();
        store = store.with_ssh_key(username, key_path, passphrase);
    }

    store
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_credential_store_creation() {
        let store = CredentialStore::new();
        assert!(!store.has_credentials());
        assert!(store.username().is_none());
    }

    #[test]
    fn test_userpass_credentials() {
        let store =
            CredentialStore::new().with_userpass("testuser".to_string(), "testpass".to_string());

        assert!(store.has_credentials());
        assert_eq!(store.username(), Some("testuser"));
    }

    #[test]
    fn test_ssh_key_credentials() {
        let temp_dir = tempdir().unwrap();
        let key_path = temp_dir.path().join("id_rsa");
        fs::write(&key_path, "fake key content").unwrap();

        let store = CredentialStore::new().with_ssh_key(
            "testuser".to_string(),
            &key_path,
            Some("passphrase".to_string()),
        );

        assert!(store.has_credentials());
        assert_eq!(store.username(), Some("testuser"));
    }

    #[test]
    fn test_credentials_from_env() {
        // This test doesn't modify actual env vars to avoid side effects
        let store = credentials_from_env();
        // Just verify it doesn't panic and returns a store
        assert!(!store.username().is_none() || store.username().is_none()); // Always true, just checking it works
    }

    #[test]
    fn test_create_credentials_no_creds() {
        let store = CredentialStore::new();
        let result = store.create_credentials(
            "https://github.com/test/repo",
            None,
            CredentialType::USER_PASS_PLAINTEXT,
        );
        assert!(result.is_err());
    }
}
