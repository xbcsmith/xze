//! CLI configuration module

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use xze_core::{config::XzeConfig, Result, XzeError};

/// CLI-specific configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Default output format
    pub default_output_format: OutputFormat,
    /// Default log level
    pub default_log_level: LogLevel,
    /// Whether to use colors in output
    pub use_colors: bool,
    /// Default configuration file path
    pub default_config_path: Option<PathBuf>,
    /// Cache directory for CLI operations
    pub cache_dir: Option<PathBuf>,
    /// Maximum number of concurrent operations
    pub max_concurrent: usize,
    /// Command history settings
    pub history: HistoryConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_output_format: OutputFormat::Pretty,
            default_log_level: LogLevel::Info,
            use_colors: true,
            default_config_path: None,
            cache_dir: None,
            max_concurrent: 4,
            history: HistoryConfig::default(),
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Yaml,
    Pretty,
    Compact,
    Table,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Yaml => write!(f, "yaml"),
            Self::Pretty => write!(f, "pretty"),
            Self::Compact => write!(f, "compact"),
            Self::Table => write!(f, "table"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = XzeError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "pretty" => Ok(Self::Pretty),
            "compact" => Ok(Self::Compact),
            "table" => Ok(Self::Table),
            _ => Err(XzeError::validation(format!(
                "Invalid output format: {}",
                s
            ))),
        }
    }
}

/// Log level options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = XzeError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(XzeError::validation(format!("Invalid log level: {}", s))),
        }
    }
}

/// Command history configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Whether to save command history
    pub enabled: bool,
    /// Maximum number of commands to keep in history
    pub max_entries: usize,
    /// History file path
    pub file_path: Option<PathBuf>,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            file_path: None,
        }
    }
}

/// CLI configuration manager
pub struct ConfigManager {
    cli_config: CliConfig,
    xze_config: Option<XzeConfig>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            cli_config: CliConfig::default(),
            xze_config: None,
        }
    }

    /// Load CLI configuration from file
    pub fn load_cli_config<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let content = std::fs::read_to_string(path)?;

        // Try YAML first, then JSON
        self.cli_config = match serde_yaml::from_str(&content) {
            Ok(config) => config,
            Err(_) => serde_json::from_str(&content)?,
        };

        Ok(())
    }

    /// Save CLI configuration to file
    pub fn save_cli_config<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(&self.cli_config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load XZe configuration from file
    pub fn load_xze_config<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        self.xze_config = Some(XzeConfig::from_file(path)?);
        Ok(())
    }

    /// Get CLI configuration
    pub fn cli_config(&self) -> &CliConfig {
        &self.cli_config
    }

    /// Get XZe configuration
    pub fn xze_config(&self) -> Option<&XzeConfig> {
        self.xze_config.as_ref()
    }

    /// Get or create XZe configuration
    pub fn xze_config_or_default(&self) -> XzeConfig {
        self.xze_config.clone().unwrap_or_default()
    }

    /// Set CLI configuration
    pub fn set_cli_config(&mut self, config: CliConfig) {
        self.cli_config = config;
    }

    /// Set XZe configuration
    pub fn set_xze_config(&mut self, config: XzeConfig) {
        self.xze_config = Some(config);
    }

    /// Get the default CLI config directory
    pub fn default_config_dir() -> Result<PathBuf> {
        let config_dir = if let Some(dir) = dirs::config_dir() {
            dir.join("xze")
        } else {
            std::env::current_dir()?.join(".xze")
        };

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir)
    }

    /// Get the default CLI config file path
    pub fn default_cli_config_path() -> Result<PathBuf> {
        Ok(Self::default_config_dir()?.join("cli-config.yaml"))
    }

    /// Get the default cache directory
    pub fn default_cache_dir() -> Result<PathBuf> {
        let cache_dir = if let Some(dir) = dirs::cache_dir() {
            dir.join("xze")
        } else {
            std::env::temp_dir().join("xze-cache")
        };

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }

        Ok(cache_dir)
    }

    /// Initialize default configuration files if they don't exist
    pub fn init_default_configs() -> Result<()> {
        let cli_config_path = Self::default_cli_config_path()?;

        if !cli_config_path.exists() {
            let default_config = CliConfig::default();
            let content = serde_yaml::to_string(&default_config)?;
            std::fs::write(&cli_config_path, content)?;
            tracing::info!("Created default CLI configuration at {:?}", cli_config_path);
        }

        Ok(())
    }

    /// Merge CLI arguments with configuration
    pub fn merge_with_args(&mut self, args: &CliArgs) -> Result<()> {
        if let Some(output_format) = &args.output_format {
            self.cli_config.default_output_format = output_format.parse()?;
        }

        if let Some(log_level) = &args.log_level {
            self.cli_config.default_log_level = log_level.parse()?;
        }

        if let Some(use_colors) = args.use_colors {
            self.cli_config.use_colors = use_colors;
        }

        if let Some(cache_dir) = &args.cache_dir {
            self.cli_config.cache_dir = Some(cache_dir.clone());
        }

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// CLI arguments that can override configuration
#[derive(Debug, Default)]
pub struct CliArgs {
    pub output_format: Option<String>,
    pub log_level: Option<String>,
    pub use_colors: Option<bool>,
    pub cache_dir: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
}

/// Check if output supports colors
pub fn supports_color() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }

    // Check if we're connected to a terminal
    atty::is(atty::Stream::Stdout)
}

/// Get the user's preferred shell
pub fn get_user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("yaml".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
        assert_eq!(
            "pretty".parse::<OutputFormat>().unwrap(),
            OutputFormat::Pretty
        );

        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);

        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_cli_config_serialization() {
        let config = CliConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: CliConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(
            config.default_output_format,
            deserialized.default_output_format
        );
        assert_eq!(config.default_log_level, deserialized.default_log_level);
    }

    #[test]
    fn test_config_manager() {
        let mut manager = ConfigManager::new();

        assert_eq!(
            manager.cli_config().default_output_format,
            OutputFormat::Pretty
        );
        assert!(manager.xze_config().is_none());

        let new_cli_config = CliConfig {
            default_output_format: OutputFormat::Json,
            ..Default::default()
        };

        manager.set_cli_config(new_cli_config);
        assert_eq!(
            manager.cli_config().default_output_format,
            OutputFormat::Json
        );
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-config.yaml");

        let mut manager = ConfigManager::new();

        // Save config
        manager.save_cli_config(&config_path).unwrap();
        assert!(config_path.exists());

        // Modify config and save
        let mut new_config = CliConfig::default();
        new_config.default_output_format = OutputFormat::Json;
        manager.set_cli_config(new_config);
        manager.save_cli_config(&config_path).unwrap();

        // Load config back
        let mut new_manager = ConfigManager::new();
        new_manager.load_cli_config(&config_path).unwrap();

        assert_eq!(
            new_manager.cli_config().default_output_format,
            OutputFormat::Json
        );
    }

    #[test]
    fn test_merge_with_args() {
        let mut manager = ConfigManager::new();
        let args = CliArgs {
            output_format: Some("json".to_string()),
            log_level: Some("debug".to_string()),
            use_colors: Some(false),
            ..Default::default()
        };

        manager.merge_with_args(&args).unwrap();

        assert_eq!(
            manager.cli_config().default_output_format,
            OutputFormat::Json
        );
        assert_eq!(manager.cli_config().default_log_level, LogLevel::Debug);
        assert!(!manager.cli_config().use_colors);
    }
}
