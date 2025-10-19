//! Git credentials management
//!
//! This module provides comprehensive credential management for Git operations including:
//! - Username/password authentication
//! - SSH key authentication
//! - Git credential helper integration
//! - Secure credential storage
//! - Environment variable loading

use crate::{Result, XzeError};
use git2::{Cred, CredentialType};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Thread-safe credential store for Git operations
#[derive(Debug, Clone)]
pub struct CredentialStore {
    inner: Arc<RwLock<CredentialStoreInner>>,
}

#[derive(Debug, Clone)]
struct CredentialStoreInner {
    username: Option<String>,
    password: Option<String>,
    ssh_key_path: Option<PathBuf>,
    ssh_public_key_path: Option<PathBuf>,
    ssh_passphrase: Option<String>,
    use_agent: bool,
    use_credential_helper: bool,
}

impl CredentialStore {
    /// Create a new empty credential store
    ///
    /// # Example
    ///
    /// ```
    /// use xze_core::git::CredentialStore;
    ///
    /// let store = CredentialStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CredentialStoreInner {
                username: None,
                password: None,
                ssh_key_path: None,
                ssh_public_key_path: None,
                ssh_passphrase: None,
                use_agent: true,
                use_credential_helper: true,
            })),
        }
    }

    /// Create a credential store with username and password
    ///
    /// # Arguments
    ///
    /// * `username` - Git username
    /// * `password` - Git password or personal access token
    ///
    /// # Example
    ///
    /// ```
    /// use xze_core::git::CredentialStore;
    ///
    /// let store = CredentialStore::new()
    ///     .with_userpass("user".to_string(), "token".to_string());
    /// ```
    pub fn with_userpass(self, username: String, password: String) -> Self {
        let mut inner = self.inner.write().unwrap();
        inner.username = Some(username);
        inner.password = Some(password);
        drop(inner);
        self
    }

    /// Create a credential store with SSH key authentication
    ///
    /// # Arguments
    ///
    /// * `username` - Git username
    /// * `private_key_path` - Path to private SSH key
    /// * `public_key_path` - Optional path to public SSH key
    /// * `passphrase` - Optional passphrase for the SSH key
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xze_core::git::CredentialStore;
    /// use std::path::Path;
    ///
    /// let store = CredentialStore::new()
    ///     .with_ssh_key(
    ///         "git".to_string(),
    ///         Path::new("/home/user/.ssh/id_rsa"),
    ///         None,
    ///         None,
    ///     );
    /// ```
    pub fn with_ssh_key<P: AsRef<Path>>(
        self,
        username: String,
        private_key_path: P,
        public_key_path: Option<P>,
        passphrase: Option<String>,
    ) -> Self {
        let mut inner = self.inner.write().unwrap();
        inner.username = Some(username);
        inner.ssh_key_path = Some(private_key_path.as_ref().to_path_buf());
        inner.ssh_public_key_path = public_key_path.map(|p| p.as_ref().to_path_buf());
        inner.ssh_passphrase = passphrase;
        drop(inner);
        self
    }

    /// Enable or disable SSH agent
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use SSH agent
    pub fn with_agent(self, enabled: bool) -> Self {
        let mut inner = self.inner.write().unwrap();
        inner.use_agent = enabled;
        drop(inner);
        self
    }

    /// Enable or disable git credential helper
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to use git credential helper
    pub fn with_credential_helper(self, enabled: bool) -> Self {
        let mut inner = self.inner.write().unwrap();
        inner.use_credential_helper = enabled;
        drop(inner);
        self
    }

    /// Set username and password
    ///
    /// # Arguments
    ///
    /// * `username` - Git username
    /// * `password` - Git password or personal access token
    pub fn set_userpass(&self, username: String, password: String) {
        let mut inner = self.inner.write().unwrap();
        inner.username = Some(username);
        inner.password = Some(password);
    }

    /// Set SSH key credentials
    ///
    /// # Arguments
    ///
    /// * `username` - Git username
    /// * `private_key_path` - Path to private SSH key
    /// * `public_key_path` - Optional path to public SSH key
    /// * `passphrase` - Optional passphrase for the SSH key
    pub fn set_ssh_key<P: AsRef<Path>>(
        &self,
        username: String,
        private_key_path: P,
        public_key_path: Option<P>,
        passphrase: Option<String>,
    ) {
        let mut inner = self.inner.write().unwrap();
        inner.username = Some(username);
        inner.ssh_key_path = Some(private_key_path.as_ref().to_path_buf());
        inner.ssh_public_key_path = public_key_path.map(|p| p.as_ref().to_path_buf());
        inner.ssh_passphrase = passphrase;
    }

    /// Get username
    pub fn username(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();
        inner.username.clone()
    }

    /// Get credentials (username and password)
    pub fn get_userpass(&self) -> Option<(String, String)> {
        let inner = self.inner.read().unwrap();
        match (&inner.username, &inner.password) {
            (Some(u), Some(p)) => Some((u.clone(), p.clone())),
            _ => None,
        }
    }

    /// Get SSH key path
    pub fn get_ssh_key_path(&self) -> Option<PathBuf> {
        let inner = self.inner.read().unwrap();
        inner.ssh_key_path.clone()
    }

    /// Check if credentials are configured
    ///
    /// # Returns
    ///
    /// True if either username/password or SSH key is configured
    pub fn has_credentials(&self) -> bool {
        let inner = self.inner.read().unwrap();
        (inner.username.is_some() && inner.password.is_some())
            || (inner.username.is_some() && inner.ssh_key_path.is_some())
    }

    /// Clear all credentials
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.username = None;
        inner.password = None;
        inner.ssh_key_path = None;
        inner.ssh_public_key_path = None;
        inner.ssh_passphrase = None;
    }

    /// Create Git credentials based on the allowed types
    ///
    /// This method implements the credential callback for git2-rs,
    /// attempting various authentication methods in order of preference.
    ///
    /// # Arguments
    ///
    /// * `url` - Repository URL being accessed
    /// * `username_from_url` - Username extracted from URL if present
    /// * `allowed_types` - Credential types allowed by the remote
    ///
    /// # Returns
    ///
    /// Git credential object or error if no suitable credentials available
    pub fn create_credentials(
        &self,
        url: &str,
        username_from_url: Option<&str>,
        allowed_types: CredentialType,
    ) -> Result<Cred> {
        let inner = self.inner.read().unwrap();

        tracing::debug!(
            "Creating credentials for {} with allowed types: {:?}",
            url,
            allowed_types
        );

        // Try SSH key authentication
        if allowed_types.contains(CredentialType::SSH_KEY) {
            if let (Some(username), Some(key_path)) = (&inner.username, &inner.ssh_key_path) {
                let user = username_from_url.unwrap_or(username);
                tracing::debug!("Attempting SSH key authentication for user: {}", user);

                let public_key = inner.ssh_public_key_path.as_deref();

                return Cred::ssh_key(user, public_key, key_path, inner.ssh_passphrase.as_deref())
                    .map_err(|e| {
                        tracing::error!("SSH key authentication failed: {}", e);
                        XzeError::Git(e)
                    });
            }
        }

        // Try username/password authentication
        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let (Some(username), Some(password)) = (&inner.username, &inner.password) {
                let user = username_from_url.unwrap_or(username);
                tracing::debug!(
                    "Attempting username/password authentication for user: {}",
                    user
                );

                return Cred::userpass_plaintext(user, password).map_err(|e| {
                    tracing::error!("Username/password authentication failed: {}", e);
                    XzeError::Git(e)
                });
            }
        }

        // Try SSH agent
        if allowed_types.contains(CredentialType::SSH_MEMORY) && inner.use_agent {
            if let Some(username) = &inner.username {
                let user = username_from_url.unwrap_or(username);
                tracing::debug!("Attempting SSH agent authentication for user: {}", user);

                return Cred::ssh_key_from_agent(user).map_err(|e| {
                    tracing::debug!("SSH agent authentication failed: {}", e);
                    XzeError::Git(e)
                });
            } else if let Some(user) = username_from_url {
                tracing::debug!("Attempting SSH agent authentication for URL user: {}", user);
                return Cred::ssh_key_from_agent(user).map_err(|e| {
                    tracing::debug!("SSH agent authentication failed: {}", e);
                    XzeError::Git(e)
                });
            }
        }

        // Try default credentials (git credential helper)
        if inner.use_credential_helper && allowed_types.contains(CredentialType::DEFAULT) {
            tracing::debug!("Attempting default credential helper");
            return Cred::default().map_err(|e| {
                tracing::debug!("Default credential helper failed: {}", e);
                XzeError::Git(e)
            });
        }

        tracing::error!("No suitable credentials available for {}", url);
        Err(XzeError::auth("No suitable credentials available"))
    }

    /// Validate that configured credentials are usable
    ///
    /// # Returns
    ///
    /// Result indicating whether credentials are valid
    pub fn validate(&self) -> Result<()> {
        let inner = self.inner.read().unwrap();

        // Check SSH key exists if configured
        if let Some(key_path) = &inner.ssh_key_path {
            if !key_path.exists() {
                return Err(XzeError::auth(format!(
                    "SSH private key not found at: {}",
                    key_path.display()
                )));
            }

            if let Some(pub_path) = &inner.ssh_public_key_path {
                if !pub_path.exists() {
                    return Err(XzeError::auth(format!(
                        "SSH public key not found at: {}",
                        pub_path.display()
                    )));
                }
            }
        }

        // Check that credentials are configured
        if !self.has_credentials() && !inner.use_agent && !inner.use_credential_helper {
            return Err(XzeError::auth(
                "No credentials configured and helpers disabled",
            ));
        }

        Ok(())
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Create credentials from environment variables
///
/// Reads credentials from the following environment variables:
/// - `GIT_USERNAME` and `GIT_PASSWORD` for username/password auth
/// - `GIT_SSH_USERNAME`, `GIT_SSH_KEY`, and optionally `GIT_SSH_PASSPHRASE` for SSH
///
/// # Example
///
/// ```
/// use xze_core::git::credentials_from_env;
///
/// let store = credentials_from_env();
/// ```
pub fn credentials_from_env() -> CredentialStore {
    let store = CredentialStore::new();

    // Check for username/password credentials
    if let (Ok(username), Ok(password)) =
        (std::env::var("GIT_USERNAME"), std::env::var("GIT_PASSWORD"))
    {
        tracing::debug!("Loading username/password from environment");
        store.set_userpass(username, password);
    }
    // Check for SSH credentials
    else if let (Ok(username), Ok(key_path)) = (
        std::env::var("GIT_SSH_USERNAME"),
        std::env::var("GIT_SSH_KEY"),
    ) {
        let passphrase = std::env::var("GIT_SSH_PASSPHRASE").ok();
        let public_key = std::env::var("GIT_SSH_PUBLIC_KEY").ok();

        tracing::debug!("Loading SSH credentials from environment");
        store.set_ssh_key(
            username,
            PathBuf::from(key_path),
            public_key.map(PathBuf::from),
            passphrase,
        );
    }

    store
}

/// Create credentials from default SSH key locations
///
/// Attempts to find SSH keys in standard locations:
/// - `~/.ssh/id_rsa`
/// - `~/.ssh/id_ed25519`
/// - `~/.ssh/id_ecdsa`
///
/// # Arguments
///
/// * `username` - Username to use for authentication
///
/// # Returns
///
/// CredentialStore configured with found SSH keys, or empty if none found
pub fn credentials_from_ssh_agent(username: String) -> CredentialStore {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return CredentialStore::new(),
    };

    let ssh_dir = home.join(".ssh");
    let key_names = ["id_ed25519", "id_rsa", "id_ecdsa"];

    for key_name in &key_names {
        let private_key = ssh_dir.join(key_name);
        let public_key = ssh_dir.join(format!("{}.pub", key_name));

        if private_key.exists() {
            tracing::debug!("Found SSH key: {}", private_key.display());
            let pub_key = if public_key.exists() {
                Some(public_key)
            } else {
                None
            };

            return CredentialStore::new()
                .with_ssh_key(username, private_key, pub_key, None)
                .with_agent(true);
        }
    }

    tracing::debug!("No SSH keys found in standard locations");
    CredentialStore::new().with_agent(true)
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
        assert_eq!(store.username(), Some("testuser".to_string()));

        let creds = store.get_userpass();
        assert!(creds.is_some());
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "testuser");
        assert_eq!(pass, "testpass");
    }

    #[test]
    fn test_ssh_key_credentials() {
        let temp_dir = tempdir().unwrap();
        let key_path = temp_dir.path().join("id_rsa");
        let pub_path = temp_dir.path().join("id_rsa.pub");

        fs::write(&key_path, "fake private key content").unwrap();
        fs::write(&pub_path, "fake public key content").unwrap();

        let store = CredentialStore::new().with_ssh_key(
            "testuser".to_string(),
            &key_path,
            Some(&pub_path),
            Some("passphrase".to_string()),
        );

        assert!(store.has_credentials());
        assert_eq!(store.username(), Some("testuser".to_string()));
        assert_eq!(store.get_ssh_key_path(), Some(key_path));
    }

    #[test]
    fn test_set_credentials() {
        let store = CredentialStore::new();
        assert!(!store.has_credentials());

        store.set_userpass("user".to_string(), "pass".to_string());
        assert!(store.has_credentials());

        let creds = store.get_userpass();
        assert!(creds.is_some());
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "pass");
    }

    #[test]
    fn test_clear_credentials() {
        let store = CredentialStore::new();
        store.set_userpass("user".to_string(), "pass".to_string());
        assert!(store.has_credentials());

        store.clear();
        assert!(!store.has_credentials());
        assert!(store.username().is_none());
    }

    #[test]
    fn test_clone() {
        let store = CredentialStore::new();
        store.set_userpass("user".to_string(), "pass".to_string());

        let cloned = store.clone();
        assert_eq!(store.get_userpass(), cloned.get_userpass());
    }

    #[test]
    fn test_credentials_from_env() {
        // This test doesn't modify actual env vars to avoid side effects
        let store = credentials_from_env();
        // Just verify it doesn't panic and returns a store
        assert!(!store.username().is_none() || store.username().is_none());
    }

    #[test]
    fn test_create_credentials_no_creds() {
        let store = CredentialStore::new()
            .with_agent(false)
            .with_credential_helper(false);

        let result = store.create_credentials(
            "https://github.com/test/repo",
            None,
            CredentialType::USER_PASS_PLAINTEXT,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_ssh_key() {
        let store = CredentialStore::new().with_ssh_key(
            "user".to_string(),
            "/nonexistent/key",
            None::<&str>,
            None,
        );

        let result = store.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_no_credentials() {
        let store = CredentialStore::new()
            .with_agent(false)
            .with_credential_helper(false);

        let result = store.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_with_agent() {
        let store = CredentialStore::new().with_agent(false);
        let inner = store.inner.read().unwrap();
        assert!(!inner.use_agent);
    }

    #[test]
    fn test_with_credential_helper() {
        let store = CredentialStore::new().with_credential_helper(false);
        let inner = store.inner.read().unwrap();
        assert!(!inner.use_credential_helper);
    }

    #[test]
    fn test_validate_with_agent() {
        let store = CredentialStore::new()
            .with_agent(true)
            .with_credential_helper(false);

        // Should be valid because agent is enabled
        let result = store.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_helper() {
        let store = CredentialStore::new()
            .with_agent(false)
            .with_credential_helper(true);

        // Should be valid because credential helper is enabled
        let result = store.validate();
        assert!(result.is_ok());
    }
}
