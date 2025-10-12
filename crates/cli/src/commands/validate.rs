//! Validate command implementation

use clap::Args;

use std::path::{Path, PathBuf};
use xze_core::{Result, XzeError};

use crate::commands::CliCommand;

/// Validate XZe configuration and repository structure
#[derive(Debug, Args)]
pub struct ValidateCommand {
    /// Path to validate (defaults to current directory)
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Configuration file to validate
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Check Ollama connectivity
    #[arg(long)]
    pub check_ollama: bool,

    /// Ollama API URL to test
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// Check Git repository status
    #[arg(long)]
    pub check_git: bool,

    /// Validate file patterns and exclusions
    #[arg(long)]
    pub check_patterns: bool,

    /// Check required dependencies
    #[arg(long)]
    pub check_deps: bool,

    /// Output format (text, json, yaml)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Fail fast on first error
    #[arg(long)]
    pub fail_fast: bool,

    /// Only show errors (suppress warnings)
    #[arg(long)]
    pub errors_only: bool,
}

impl CliCommand for ValidateCommand {
    async fn execute(&self) -> Result<()> {
        let target_path = self
            .path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        if self.verbose {
            tracing::info!("Starting validation for: {}", target_path.display());
        }

        let mut validation_result = ValidationResult::new();

        // Run validation checks
        if let Err(e) = self.validate_path(&target_path).await {
            validation_result.add_error("path", e.to_string());
            if self.fail_fast {
                return Err(e);
            }
        }

        if let Some(config_path) = &self.config {
            if let Err(e) = self.validate_config_file(config_path).await {
                validation_result.add_error("config", e.to_string());
                if self.fail_fast {
                    return Err(e);
                }
            }
        } else {
            // Look for default config file
            let default_config = target_path.join(".xze.yaml");
            if default_config.exists() {
                if let Err(e) = self.validate_config_file(&default_config).await {
                    validation_result.add_error("config", e.to_string());
                    if self.fail_fast {
                        return Err(e);
                    }
                }
            } else {
                validation_result.add_warning(
                    "config",
                    "No configuration file found (.xze.yaml)".to_string(),
                );
            }
        }

        if self.check_git {
            if let Err(e) = self.validate_git_repository(&target_path).await {
                validation_result.add_error("git", e.to_string());
                if self.fail_fast {
                    return Err(e);
                }
            }
        }

        if self.check_ollama {
            if let Err(e) = self.validate_ollama_connectivity().await {
                validation_result.add_error("ollama", e.to_string());
                if self.fail_fast {
                    return Err(e);
                }
            }
        }

        if self.check_patterns {
            if let Err(e) = self.validate_file_patterns(&target_path).await {
                validation_result.add_error("patterns", e.to_string());
                if self.fail_fast {
                    return Err(e);
                }
            }
        }

        if self.check_deps {
            if let Err(e) = self.validate_dependencies().await {
                validation_result.add_error("dependencies", e.to_string());
                if self.fail_fast {
                    return Err(e);
                }
            }
        }

        // Output results
        self.output_results(&validation_result)?;

        // Return error if any validation failed
        if validation_result.has_errors() {
            return Err(XzeError::validation(
                "Validation failed with errors".to_string(),
            ));
        }

        if self.verbose {
            println!("✅ All validations passed!");
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "validate"
    }

    fn validate(&self) -> Result<()> {
        // Validate output format
        let valid_formats = ["text", "json", "yaml"];
        if !valid_formats.contains(&self.format.as_str()) {
            return Err(XzeError::validation(format!(
                "Invalid output format: {}. Valid formats: {}",
                self.format,
                valid_formats.join(", ")
            )));
        }

        Ok(())
    }
}

impl ValidateCommand {
    /// Validate the target path exists and is accessible
    async fn validate_path(&self, path: &PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(XzeError::filesystem(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        if !path.is_dir() {
            return Err(XzeError::validation(format!(
                "Path is not a directory: {}",
                path.display()
            )));
        }

        // Check if directory is readable
        match tokio::fs::read_dir(path).await {
            Ok(_) => Ok(()),
            Err(e) => Err(XzeError::filesystem(format!(
                "Cannot read directory {}: {}",
                path.display(),
                e
            ))),
        }
    }

    /// Validate XZe configuration file
    async fn validate_config_file(&self, config_path: &PathBuf) -> Result<()> {
        if !config_path.exists() {
            return Err(XzeError::filesystem(format!(
                "Configuration file not found: {}",
                config_path.display()
            )));
        }

        let content = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read config file: {}", e)))?;

        // Try to parse as YAML
        let config: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| XzeError::validation(format!("Invalid YAML configuration: {}", e)))?;

        // Validate required fields
        self.validate_config_structure(&config)?;

        if self.verbose {
            println!("✅ Configuration file is valid");
        }

        Ok(())
    }

    /// Validate configuration structure
    fn validate_config_structure(&self, config: &serde_yaml::Value) -> Result<()> {
        let required_sections = vec!["project", "analysis", "documentation", "ai"];

        for section in required_sections {
            if config.get(section).is_none() {
                return Err(XzeError::validation(format!(
                    "Missing required configuration section: {}",
                    section
                )));
            }
        }

        // Validate project section
        if let Some(project) = config.get("project") {
            let required_fields = vec!["name", "language"];
            for field in required_fields {
                if project.get(field).is_none() {
                    return Err(XzeError::validation(format!(
                        "Missing required field in project section: {}",
                        field
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate Git repository status
    async fn validate_git_repository(&self, path: &Path) -> Result<()> {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(XzeError::validation(format!(
                "Not a Git repository: {}",
                path.display()
            )));
        }

        // TODO: Add more Git validations
        // - Check if repository is clean
        // - Validate remote URLs
        // - Check branch status

        if self.verbose {
            println!("✅ Git repository is valid");
        }

        Ok(())
    }

    /// Validate Ollama connectivity
    async fn validate_ollama_connectivity(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/api/tags", self.ollama_url);

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                if self.verbose {
                    println!("✅ Ollama is accessible at {}", self.ollama_url);
                }
                Ok(())
            }
            Ok(response) => Err(XzeError::network(format!(
                "Ollama responded with status: {}",
                response.status()
            ))),
            Err(e) => Err(XzeError::network(format!(
                "Failed to connect to Ollama at {}: {}",
                self.ollama_url, e
            ))),
        }
    }

    /// Validate file patterns and exclusions
    async fn validate_file_patterns(&self, path: &PathBuf) -> Result<()> {
        use walkdir::WalkDir;

        let mut file_count = 0;
        let mut excluded_count = 0;

        for entry in WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                file_count += 1;

                // Check common exclusion patterns
                let path_str = entry.path().to_string_lossy();
                if path_str.contains("/target/")
                    || path_str.contains("/.git/")
                    || path_str.contains("/node_modules/")
                    || path_str.ends_with(".log")
                {
                    excluded_count += 1;
                }
            }
        }

        if file_count == 0 {
            return Err(XzeError::validation(
                "No files found in directory".to_string(),
            ));
        }

        if self.verbose {
            println!(
                "✅ Found {} files ({} would be excluded)",
                file_count, excluded_count
            );
        }

        Ok(())
    }

    /// Validate required dependencies
    async fn validate_dependencies(&self) -> Result<()> {
        // Check for Rust toolchain
        if (tokio::process::Command::new("rustc")
            .arg("--version")
            .output()
            .await)
            .is_err()
        {
            return Err(XzeError::validation("Rust compiler not found".to_string()));
        }

        // Check for Git
        if (tokio::process::Command::new("git")
            .arg("--version")
            .output()
            .await)
            .is_err()
        {
            return Err(XzeError::validation("Git not found".to_string()));
        }

        if self.verbose {
            println!("✅ Required dependencies are available");
        }

        Ok(())
    }

    /// Output validation results in the specified format
    fn output_results(&self, result: &ValidationResult) -> Result<()> {
        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(result).map_err(|e| {
                    XzeError::validation(format!("Failed to serialize JSON: {}", e))
                })?;
                println!("{}", json);
            }
            "yaml" => {
                let yaml = serde_yaml::to_string(result).map_err(|e| {
                    XzeError::validation(format!("Failed to serialize YAML: {}", e))
                })?;
                println!("{}", yaml);
            }
            "text" => {
                self.output_text_results(result);
            }
            _ => {
                return Err(XzeError::validation(format!(
                    "Unsupported format: {}",
                    self.format
                )))
            }
        }

        Ok(())
    }

    /// Output results in human-readable text format
    fn output_text_results(&self, result: &ValidationResult) {
        if result.has_errors() {
            println!(
                "❌ Validation failed with {} error(s):",
                result.errors.len()
            );
            for (category, errors) in &result.errors {
                for error in errors {
                    println!("  [{}] {}", category, error);
                }
            }
        }

        if !self.errors_only && !result.warnings.is_empty() {
            println!("⚠️  Warnings ({}):", result.warnings.len());
            for (category, warnings) in &result.warnings {
                for warning in warnings {
                    println!("  [{}] {}", category, warning);
                }
            }
        }

        if !result.has_errors() && result.warnings.is_empty() {
            println!("✅ All validations passed!");
        }
    }
}

/// Validation result structure
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub errors: std::collections::HashMap<String, Vec<String>>,
    pub warnings: std::collections::HashMap<String, Vec<String>>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            errors: std::collections::HashMap::new(),
            warnings: std::collections::HashMap::new(),
        }
    }

    pub fn add_error(&mut self, category: &str, message: String) {
        self.errors
            .entry(category.to_string())
            .or_default()
            .push(message);
    }

    pub fn add_warning(&mut self, category: &str, message: String) {
        self.warnings
            .entry(category.to_string())
            .or_default()
            .push(message);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_command_format() {
        let mut cmd = ValidateCommand {
            path: None,
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "json".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        assert!(cmd.validate().is_ok());

        cmd.format = "invalid".to_string();
        assert!(cmd.validate().is_err());
    }

    #[tokio::test]
    async fn test_validate_existing_path() {
        let temp_dir = TempDir::new().unwrap();
        let cmd = ValidateCommand {
            path: Some(temp_dir.path().to_path_buf()),
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "text".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        assert!(cmd
            .validate_path(&temp_dir.path().to_path_buf())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_validate_nonexistent_path() {
        let cmd = ValidateCommand {
            path: None,
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "text".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        let result = cmd.validate_path(&PathBuf::from("/nonexistent/path")).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(!result.has_errors());

        result.add_error("test", "Test error".to_string());
        assert!(result.has_errors());

        result.add_warning("test", "Test warning".to_string());
        assert_eq!(result.warnings.get("test").unwrap().len(), 1);
    }

    #[test]
    fn test_validate_config_structure() {
        let cmd = ValidateCommand {
            path: None,
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "text".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        // Valid config
        let valid_config: serde_yaml::Value = serde_yaml::from_str(
            r#"
project:
  name: "test"
  language: "rust"
analysis:
  include_patterns: ["**/*.rs"]
documentation:
  output_dir: "docs"
ai:
  provider: "ollama"
"#,
        )
        .unwrap();

        assert!(cmd.validate_config_structure(&valid_config).is_ok());

        // Invalid config (missing project section)
        let invalid_config: serde_yaml::Value = serde_yaml::from_str(
            r#"
analysis:
  include_patterns: ["**/*.rs"]
"#,
        )
        .unwrap();

        assert!(cmd.validate_config_structure(&invalid_config).is_err());
    }

    #[tokio::test]
    async fn test_validate_file_patterns() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "fn main() {}").await.unwrap();

        let cmd = ValidateCommand {
            path: None,
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "text".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        let result = cmd
            .validate_file_patterns(&temp_dir.path().to_path_buf())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cmd = ValidateCommand {
            path: None,
            config: None,
            check_ollama: false,
            ollama_url: "http://localhost:11434".to_string(),
            check_git: false,
            check_patterns: false,
            check_deps: false,
            format: "text".to_string(),
            verbose: false,
            fail_fast: false,
            errors_only: false,
        };

        let result = cmd
            .validate_file_patterns(&temp_dir.path().to_path_buf())
            .await;
        assert!(result.is_err());
    }
}
