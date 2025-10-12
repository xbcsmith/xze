//! Git credentials management

use std::sync::{Arc, RwLock};

/// Credential store for Git operations
#[derive(Clone)]
pub struct CredentialStore {
    inner: Arc<RwLock<CredentialStoreInner>>,
}

struct CredentialStoreInner {
    username: Option<String>,
    password: Option<String>,
    ssh_key_path: Option<String>,
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CredentialStoreInner {
                username: None,
                password: None,
                ssh_key_path: None,
            })),
        }
    }

    /// Set username and password
    pub fn set_credentials(&self, username: String, password: String) {
        let mut inner = self.inner.write().unwrap();
        inner.username = Some(username);
        inner.password = Some(password);
    }

    /// Set SSH key path
    pub fn set_ssh_key(&self, path: String) {
        let mut inner = self.inner.write().unwrap();
        inner.ssh_key_path = Some(path);
    }

    /// Get credentials
    pub fn get_credentials(&self) -> Option<(String, String)> {
        let inner = self.inner.read().unwrap();
        match (&inner.username, &inner.password) {
            (Some(u), Some(p)) => Some((u.clone(), p.clone())),
            _ => None,
        }
    }

    /// Get SSH key path
    pub fn get_ssh_key(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();
        inner.ssh_key_path.clone()
    }

    /// Load credentials from environment variables
    pub fn from_env() -> Self {
        let store = Self::new();

        if let (Ok(username), Ok(password)) = (
            std::env::var("GIT_USERNAME"),
            std::env::var("GIT_PASSWORD"),
        ) {
            store.set_credentials(username, password);
        }

        if let Ok(ssh_key) = std::env::var("GIT_SSH_KEY") {
            store.set_ssh_key(ssh_key);
        }

        store
    }

    /// Clear all credentials
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.username = None;
        inner.password = None;
        inner.ssh_key_path = None;
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_store() {
        let store = CredentialStore::new();
        assert!(store.get_credentials().is_none());

        store.set_credentials("user".to_string(), "pass".to_string());
        let creds = store.get_credentials();
        assert!(creds.is_some());
        
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "pass");
    }

    #[test]
    fn test_ssh_key() {
        let store = CredentialStore::new();
        assert!(store.get_ssh_key().is_none());

        store.set_ssh_key("/path/to/key".to_string());
        assert_eq!(store.get_ssh_key(), Some("/path/to/key".to_string()));
    }

    #[test]
    fn test_clear_credentials() {
        let store = CredentialStore::new();
        store.set_credentials("user".to_string(), "pass".to_string());
        
        assert!(store.get_credentials().is_some());
        
        store.clear();
        assert!(store.get_credentials().is_none());
    }

    #[test]
    fn test_clone() {
        let store = CredentialStore::new();
        store.set_credentials("user".to_string(), "pass".to_string());

        let cloned = store.clone();
        assert_eq!(store.get_credentials(), cloned.get_credentials());
    }
}