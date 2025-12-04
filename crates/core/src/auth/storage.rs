use crate::auth::errors::AuthError;
use crate::auth::token::Token;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for storing and retrieving tokens
#[async_trait]
pub trait TokenStorage: Send + Sync {
    /// Save a token for a provider
    async fn save_token(&self, provider: &str, token: &Token) -> Result<(), AuthError>;

    /// Load a token for a provider
    async fn load_token(&self, provider: &str) -> Result<Option<Token>, AuthError>;

    /// Delete a token for a provider
    async fn delete_token(&self, provider: &str) -> Result<(), AuthError>;
}

/// In-memory token storage (for testing or ephemeral use)
#[derive(Debug, Clone)]
pub struct MemoryTokenStorage {
    tokens: Arc<RwLock<HashMap<String, Token>>>,
}

impl MemoryTokenStorage {
    /// Create a new memory token storage
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenStorage for MemoryTokenStorage {
    async fn save_token(&self, provider: &str, token: &Token) -> Result<(), AuthError> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(provider.to_string(), token.clone());
        Ok(())
    }

    async fn load_token(&self, provider: &str) -> Result<Option<Token>, AuthError> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(provider).cloned())
    }

    async fn delete_token(&self, provider: &str) -> Result<(), AuthError> {
        let mut tokens = self.tokens.write().await;
        tokens.remove(provider);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryTokenStorage::new();
        let token = Token {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now()),
            scope: None,
        };

        storage.save_token("test_provider", &token).await.unwrap();

        let loaded = storage.load_token("test_provider").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().access_token, "test");

        storage.delete_token("test_provider").await.unwrap();
        let loaded = storage.load_token("test_provider").await.unwrap();
        assert!(loaded.is_none());
    }
}
