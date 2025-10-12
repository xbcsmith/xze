//! Configuration types for XZe core library

use crate::{types::RepositoryId, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use url::Url;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XzeConfig {
    /// Version of the configuration schema
    pub version: String,
    /// Documentation repository configuration
    pub documentation_repo: DocumentationRepoConfig,
    /// Source repositories to analyze
    pub repositories: Vec<RepositoryConfig>,
    /// Ollama configuration
    pub ollama: OllamaConfig,
    /// Generation settings
    pub generation: GenerationConfig,
    /// Pull request settings
    pub pr: PullRequestConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// File system settings
    #[serde(default)]
    pub filesystem: FileSystemConfig,
    /// Git settings
    #[serde(default)]
    pub git: GitConfig,
}

impl Default for XzeConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            documentation_repo: DocumentationRepoConfig::default(),
            repositories: Vec::new(),
            ollama: OllamaConfig::default(),
            generation: GenerationConfig::default(),
            pr: PullRequestConfig::default(),
            logging: LoggingConfig::default(),
            filesystem: FileSystemConfig::default(),
            git: GitConfig::default(),
        }
    }
}

/// Documentation repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationRepoConfig {
    /// Repository URL
    pub url: Url,
    /// Branch to work with
    pub branch: String,
    /// Local path for cloning (optional)
    pub local_path: Option<PathBuf>,
    /// Credentials for accessing the repository
    pub credentials: Option<GitCredentials>,
}

impl Default for DocumentationRepoConfig {
    fn default() -> Self {
        Self {
            url: Url::parse("https://github.com/example/pipeline-documentation").unwrap(),
            branch: "main".to_string(),
            local_path: None,
            credentials: None,
        }
    }
}

/// Repository configuration for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// Repository name/identifier
    pub name: String,
    /// Repository URL
    pub url: Url,
    /// Primary programming language
    pub language: Option<String>,
    /// Branches to watch for changes
    pub watch_branches: Vec<String>,
    /// Local path for cloning (optional)
    pub local_path: Option<PathBuf>,
    /// Credentials for accessing the repository
    pub credentials: Option<GitCredentials>,
    /// Custom configuration for this repository
    #[serde(default)]
    pub custom: HashMap<String, String>,
    /// Whether to enable auto-documentation updates
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// Directories to exclude from analysis
    #[serde(default)]
    pub exclude_dirs: Vec<String>,
    /// File patterns to exclude
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

impl RepositoryConfig {
    /// Get repository ID
    pub fn id(&self) -> RepositoryId {
        RepositoryId::from(self.name.as_str())
    }

    /// Check if a path should be excluded
    pub fn should_exclude_path(&self, path: &str) -> bool {
        // Check exclude directories
        for exclude_dir in &self.exclude_dirs {
            if path.starts_with(exclude_dir) {
                return true;
            }
        }

        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(path))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }
}

/// Git credentials configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCredentials {
    /// Authentication type
    #[serde(flatten)]
    pub auth: GitAuth,
}

/// Git authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GitAuth {
    /// Username and password/token
    #[serde(rename = "userpass")]
    UserPass { username: String, password: String },
    /// SSH key authentication
    #[serde(rename = "ssh_key")]
    SshKey {
        username: String,
        private_key_path: PathBuf,
        passphrase: Option<String>,
    },
    /// SSH agent authentication
    #[serde(rename = "ssh_agent")]
    SshAgent { username: String },
}

/// Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server URL
    pub url: Url,
    /// Model configuration
    pub models: ModelConfig,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: Url::parse("http://localhost:11434").unwrap(),
            models: ModelConfig::default(),
            timeout_seconds: default_timeout(),
            max_concurrent_requests: default_max_concurrent(),
            retry: RetryConfig::default(),
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Primary model to use
    pub primary: String,
    /// Fallback models if primary fails
    #[serde(default)]
    pub fallback: Vec<String>,
    /// Context window size
    #[serde(default = "default_context_window")]
    pub context_window: usize,
    /// Generation temperature
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            primary: "llama2".to_string(),
            fallback: vec!["codellama".to_string()],
            context_window: default_context_window(),
            temperature: default_temperature(),
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// Initial retry delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,
    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Whether to enable streaming responses
    #[serde(default = "default_false")]
    pub streaming: bool,
    /// Custom prompt templates
    #[serde(default)]
    pub prompt_templates: HashMap<String, String>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            streaming: false,
            prompt_templates: HashMap::new(),
        }
    }
}

/// Pull request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestConfig {
    /// Automatically assign reviewers
    #[serde(default = "default_true")]
    pub auto_assign_reviewers: bool,
    /// Default reviewers to assign
    #[serde(default)]
    pub default_reviewers: Vec<String>,
    /// Labels to add to PRs
    #[serde(default)]
    pub labels: Vec<String>,
    /// PR title template
    #[serde(default = "default_pr_title")]
    pub title_template: String,
    /// PR body template
    #[serde(default = "default_pr_body")]
    pub body_template: String,
    /// Auto-merge configuration
    #[serde(default)]
    pub auto_merge: AutoMergeConfig,
}

impl Default for PullRequestConfig {
    fn default() -> Self {
        Self {
            auto_assign_reviewers: true,
            default_reviewers: Vec::new(),
            labels: vec!["documentation".to_string(), "auto-generated".to_string()],
            title_template: default_pr_title(),
            body_template: default_pr_body(),
            auto_merge: AutoMergeConfig::default(),
        }
    }
}

/// Auto-merge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMergeConfig {
    /// Enable auto-merge
    #[serde(default = "default_false")]
    pub enabled: bool,
    /// Require approvals before auto-merge
    #[serde(default = "default_true")]
    pub require_approvals: bool,
    /// Minimum number of approvals required
    #[serde(default = "default_min_approvals")]
    pub min_approvals: usize,
    /// Wait time before auto-merge (in hours)
    #[serde(default = "default_auto_merge_delay")]
    pub delay_hours: u64,
}

impl Default for AutoMergeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_approvals: true,
            min_approvals: default_min_approvals(),
            delay_hours: default_auto_merge_delay(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Output format (json, text)
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Log file path (optional)
    pub file: Option<PathBuf>,
    /// Whether to log to stdout
    #[serde(default = "default_true")]
    pub stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file: None,
            stdout: true,
        }
    }
}

/// File system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemConfig {
    /// Cache directory
    pub cache_dir: Option<PathBuf>,
    /// Temporary directory
    pub temp_dir: Option<PathBuf>,
    /// Maximum cache size in MB
    #[serde(default = "default_cache_size")]
    pub max_cache_size_mb: usize,
    /// Cache TTL in hours
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_hours: u64,
}

impl Default for FileSystemConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            temp_dir: None,
            max_cache_size_mb: default_cache_size(),
            cache_ttl_hours: default_cache_ttl(),
        }
    }
}

/// Git configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Default author name
    pub author_name: Option<String>,
    /// Default author email
    pub author_email: Option<String>,
    /// Default commit message template
    #[serde(default = "default_commit_message")]
    pub commit_message_template: String,
    /// Whether to sign commits
    #[serde(default = "default_false")]
    pub sign_commits: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            author_name: None,
            author_email: None,
            commit_message_template: default_commit_message(),
            sign_commits: false,
        }
    }
}

impl XzeConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // Try YAML first, then JSON
        match serde_yaml::from_str(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = serde_json::from_str(&content)?;
                Ok(config)
            }
        }
    }

    /// Save configuration to a file
    pub fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get repository by name
    pub fn get_repository(&self, name: &str) -> Option<&RepositoryConfig> {
        self.repositories.iter().find(|r| r.name == name)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate repositories
        if self.repositories.is_empty() {
            return Err(crate::XzeError::validation("No repositories configured"));
        }

        // Check for duplicate repository names
        let mut names = std::collections::HashSet::new();
        for repo in &self.repositories {
            if !names.insert(&repo.name) {
                return Err(crate::XzeError::validation(format!(
                    "Duplicate repository name: {}",
                    repo.name
                )));
            }
        }

        // Validate Ollama URL
        if self.ollama.url.scheme() != "http" && self.ollama.url.scheme() != "https" {
            return Err(crate::XzeError::validation(
                "Ollama URL must use http or https scheme",
            ));
        }

        // Validate model configuration
        if self.ollama.models.primary.is_empty() {
            return Err(crate::XzeError::validation("Primary model cannot be empty"));
        }

        Ok(())
    }
}

// Default value functions
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_timeout() -> u64 {
    300
}
fn default_max_concurrent() -> usize {
    4
}
fn default_context_window() -> usize {
    4096
}
fn default_temperature() -> f32 {
    0.3
}
fn default_max_tokens() -> usize {
    2048
}
fn default_max_retries() -> usize {
    3
}
fn default_initial_delay() -> u64 {
    1000
}
fn default_max_delay() -> u64 {
    30000
}
fn default_backoff_multiplier() -> f64 {
    2.0
}
fn default_min_approvals() -> usize {
    1
}
fn default_auto_merge_delay() -> u64 {
    24
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_cache_size() -> usize {
    1024
}
fn default_cache_ttl() -> u64 {
    24
}

fn default_pr_title() -> String {
    "docs: Update documentation for {{repository_name}}".to_string()
}

fn default_pr_body() -> String {
    r#"This PR updates the documentation for {{repository_name}} based on recent changes.

## Changes
{{#each changes}}
- {{this}}
{{/each}}

## AI Analysis Summary
{{ai_summary}}

---
*This PR was automatically generated by XZe*"#
        .to_string()
}

fn default_commit_message() -> String {
    "docs: Update documentation for {{repository_name}}\n\n{{summary}}".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = XzeConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.ollama.models.primary, "llama2");
        assert!(config.pr.auto_assign_reviewers);
    }

    #[test]
    fn test_config_serialization() {
        let config = XzeConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: XzeConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            config.ollama.models.primary,
            deserialized.ollama.models.primary
        );
    }

    #[test]
    fn test_config_validation() {
        let mut config = XzeConfig::default();

        // Should fail with no repositories
        assert!(config.validate().is_err());

        // Add a repository
        config.repositories.push(RepositoryConfig {
            name: "test".to_string(),
            url: Url::parse("https://github.com/test/test").unwrap(),
            language: Some("rust".to_string()),
            watch_branches: vec!["main".to_string()],
            local_path: None,
            credentials: None,
            custom: HashMap::new(),
            auto_update: true,
            exclude_dirs: Vec::new(),
            exclude_patterns: Vec::new(),
        });

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_exclude_patterns() {
        let repo = RepositoryConfig {
            name: "test".to_string(),
            url: Url::parse("https://github.com/test/test").unwrap(),
            language: None,
            watch_branches: vec![],
            local_path: None,
            credentials: None,
            custom: HashMap::new(),
            auto_update: true,
            exclude_dirs: vec!["target".to_string(), "node_modules".to_string()],
            exclude_patterns: vec!["*.log".to_string(), "temp_*".to_string()],
        };

        assert!(repo.should_exclude_path("target/debug/main"));
        assert!(repo.should_exclude_path("node_modules/package/index.js"));
        assert!(repo.should_exclude_path("app.log"));
        assert!(repo.should_exclude_path("temp_file.txt"));
        assert!(!repo.should_exclude_path("src/main.rs"));
    }

    #[test]
    fn test_file_io() {
        let config = XzeConfig::default();
        let temp_file = NamedTempFile::new().unwrap();

        // Save config
        config.to_file(temp_file.path()).unwrap();

        // Load config
        let loaded_config = XzeConfig::from_file(temp_file.path()).unwrap();
        assert_eq!(config.version, loaded_config.version);
    }

    #[test]
    fn test_repository_lookup() {
        let mut config = XzeConfig::default();
        config.repositories.push(RepositoryConfig {
            name: "test-repo".to_string(),
            url: Url::parse("https://github.com/test/repo").unwrap(),
            language: Some("rust".to_string()),
            watch_branches: vec!["main".to_string()],
            local_path: None,
            credentials: None,
            custom: HashMap::new(),
            auto_update: true,
            exclude_dirs: Vec::new(),
            exclude_patterns: Vec::new(),
        });

        assert!(config.get_repository("test-repo").is_some());
        assert!(config.get_repository("nonexistent").is_none());
    }
}
