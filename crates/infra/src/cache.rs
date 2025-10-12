//! Caching infrastructure for XZe
//!
//! This module provides file-based caching utilities for storing
//! and retrieving data to improve performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::fs;
use xze_core::{Result, XzeError};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Base directory for cache storage
    pub cache_dir: PathBuf,
    /// Maximum size of cache in bytes
    pub max_size_bytes: u64,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Whether to enable cache compression
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: std::env::temp_dir().join("xze-cache"),
            max_size_bytes: 1024 * 1024 * 1024,     // 1GB
            default_ttl: Duration::from_secs(3600), // 1 hour
            enable_compression: false,
        }
    }
}

/// File-based cache manager
#[derive(Debug)]
pub struct CacheManager {
    config: CacheConfig,
    index: HashMap<String, CacheEntry>,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(config: CacheConfig) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&config.cache_dir).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to create cache directory: {}", e))
        })?;

        let mut manager = Self {
            config,
            index: HashMap::new(),
        };

        // Load existing index
        manager.load_index().await?;

        Ok(manager)
    }

    /// Store data in cache with key
    pub async fn set<T: Serialize>(&mut self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.config.default_ttl).await
    }

    /// Store data in cache with custom TTL
    pub async fn set_with_ttl<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| XzeError::validation(format!("Failed to serialize cache value: {}", e)))?;

        let cache_file = self.get_cache_file_path(key);
        let expires_at = SystemTime::now() + ttl;

        // Write data to file
        fs::write(&cache_file, &serialized)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write cache file: {}", e)))?;

        // Update index
        let entry = CacheEntry {
            key: key.to_string(),
            file_path: cache_file,
            size: serialized.len() as u64,
            created_at: SystemTime::now(),
            expires_at,
            access_count: 0,
            last_accessed: SystemTime::now(),
        };

        self.index.insert(key.to_string(), entry);

        // Save updated index
        self.save_index().await?;

        // Check if we need to clean up
        self.cleanup_if_needed().await?;

        Ok(())
    }

    /// Retrieve data from cache
    pub async fn get<T: for<'de> Deserialize<'de>>(&mut self, key: &str) -> Result<Option<T>> {
        let entry = match self.index.get_mut(key) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // Check if expired
        if SystemTime::now() > entry.expires_at {
            self.remove(key).await?;
            return Ok(None);
        }

        // Update access statistics
        entry.access_count += 1;
        entry.last_accessed = SystemTime::now();

        // Read data from file
        let data = fs::read(&entry.file_path)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read cache file: {}", e)))?;

        let value: T = serde_json::from_slice(&data).map_err(|e| {
            XzeError::validation(format!("Failed to deserialize cache value: {}", e))
        })?;

        // Save updated index (for access statistics)
        self.save_index().await?;

        Ok(Some(value))
    }

    /// Remove entry from cache
    pub async fn remove(&mut self, key: &str) -> Result<bool> {
        if let Some(entry) = self.index.remove(key) {
            // Remove file if it exists
            if entry.file_path.exists() {
                fs::remove_file(&entry.file_path).await.map_err(|e| {
                    XzeError::filesystem(format!("Failed to remove cache file: {}", e))
                })?;
            }

            self.save_index().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if key exists in cache and is not expired
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(entry) = self.index.get(key) {
            SystemTime::now() <= entry.expires_at
        } else {
            false
        }
    }

    /// Clear all cache entries
    pub async fn clear(&mut self) -> Result<()> {
        for entry in self.index.values() {
            if entry.file_path.exists() {
                let _ = fs::remove_file(&entry.file_path).await;
            }
        }

        self.index.clear();
        self.save_index().await?;

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let now = SystemTime::now();
        let mut total_size = 0;
        let mut expired_count = 0;
        let mut total_access_count = 0;

        for entry in self.index.values() {
            total_size += entry.size;
            total_access_count += entry.access_count;

            if now > entry.expires_at {
                expired_count += 1;
            }
        }

        CacheStats {
            total_entries: self.index.len(),
            total_size_bytes: total_size,
            expired_entries: expired_count,
            total_access_count,
        }
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&mut self) -> Result<usize> {
        let now = SystemTime::now();
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.index {
            if now > entry.expires_at {
                expired_keys.push(key.clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.remove(&key).await?;
        }

        Ok(count)
    }

    /// Get cache file path for a key
    fn get_cache_file_path(&self, key: &str) -> PathBuf {
        let hash = format!("{:x}", md5::compute(key.as_bytes()));
        self.config.cache_dir.join(format!("{}.cache", hash))
    }

    /// Get index file path
    fn get_index_file_path(&self) -> PathBuf {
        self.config.cache_dir.join("index.json")
    }

    /// Load cache index from disk
    async fn load_index(&mut self) -> Result<()> {
        let index_path = self.get_index_file_path();

        if !index_path.exists() {
            return Ok(());
        }

        let data = fs::read(&index_path)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read cache index: {}", e)))?;

        let entries: Vec<CacheEntry> = serde_json::from_slice(&data).map_err(|e| {
            XzeError::validation(format!("Failed to deserialize cache index: {}", e))
        })?;

        self.index.clear();
        for entry in entries {
            self.index.insert(entry.key.clone(), entry);
        }

        Ok(())
    }

    /// Save cache index to disk
    async fn save_index(&self) -> Result<()> {
        let index_path = self.get_index_file_path();
        let entries: Vec<&CacheEntry> = self.index.values().collect();

        let data = serde_json::to_vec_pretty(&entries)
            .map_err(|e| XzeError::validation(format!("Failed to serialize cache index: {}", e)))?;

        fs::write(&index_path, &data)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write cache index: {}", e)))?;

        Ok(())
    }

    /// Cleanup cache if it exceeds size limits
    async fn cleanup_if_needed(&mut self) -> Result<()> {
        let stats = self.stats();

        if stats.total_size_bytes > self.config.max_size_bytes {
            // First remove expired entries
            self.cleanup_expired().await?;

            // If still too large, remove least recently used entries
            let updated_stats = self.stats();
            if updated_stats.total_size_bytes > self.config.max_size_bytes {
                self.cleanup_lru().await?;
            }
        }

        Ok(())
    }

    /// Remove least recently used entries until under size limit
    async fn cleanup_lru(&mut self) -> Result<()> {
        let mut entries: Vec<_> = self.index.values().cloned().collect();
        entries.sort_by_key(|e| e.last_accessed);

        let mut current_size = self.stats().total_size_bytes;

        for entry in entries {
            if current_size <= self.config.max_size_bytes {
                break;
            }

            current_size -= entry.size;
            self.remove(&entry.key).await?;
        }

        Ok(())
    }
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    key: String,
    file_path: PathBuf,
    size: u64,
    created_at: SystemTime,
    expires_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub expired_entries: usize,
    pub total_access_count: u64,
}

impl CacheStats {
    /// Get cache hit ratio (if available)
    pub fn hit_ratio(&self) -> Option<f64> {
        if self.total_access_count > 0 {
            Some(self.total_entries as f64 / self.total_access_count as f64)
        } else {
            None
        }
    }

    /// Get average entry size
    pub fn average_entry_size(&self) -> u64 {
        if self.total_entries > 0 {
            self.total_size_bytes / self.total_entries as u64
        } else {
            0
        }
    }

    /// Get size in megabytes
    pub fn size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let cache = CacheManager::new(config).await;
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = CacheManager::new(config).await.unwrap();

        let test_data = "Hello, cache!";
        cache.set("test_key", &test_data).await.unwrap();

        let retrieved: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data.to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            default_ttl: Duration::from_millis(100),
            ..Default::default()
        };

        let mut cache = CacheManager::new(config).await.unwrap();

        let test_data = "Expiring data";
        cache.set("expire_key", &test_data).await.unwrap();

        // Should exist initially
        assert!(cache.contains_key("expire_key"));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        let retrieved: Option<String> = cache.get("expire_key").await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_cache_removal() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = CacheManager::new(config).await.unwrap();

        let test_data = "To be removed";
        cache.set("remove_key", &test_data).await.unwrap();

        assert!(cache.contains_key("remove_key"));

        let removed = cache.remove("remove_key").await.unwrap();
        assert!(removed);
        assert!(!cache.contains_key("remove_key"));
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = CacheManager::new(config).await.unwrap();

        cache.set("key1", &"value1").await.unwrap();
        cache.set("key2", &"value2").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);

        cache.clear().await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut cache = CacheManager::new(config).await.unwrap();

        cache.set("stats_key", &"statistics test").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_size_bytes, 1024 * 1024 * 1024);
        assert_eq!(config.default_ttl, Duration::from_secs(3600));
        assert!(!config.enable_compression);
    }
}
