//! Init command implementation

use clap::Args;
use std::path::{Path, PathBuf};
use xze_core::{config::XzeConfig, Result, XzeError};

use crate::commands::CliCommand;

/// Initialize XZe configuration in a repository
#[derive(Debug, Args)]
pub struct InitCommand {
    /// Path to initialize (defaults to current directory)
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Configuration template to use
    #[arg(short, long, default_value = "default")]
    pub template: String,

    /// Force initialization even if config already exists
    #[arg(short, long)]
    pub force: bool,

    /// Skip interactive prompts
    #[arg(short, long)]
    pub yes: bool,

    /// Project name (defaults to directory name)
    #[arg(long)]
    pub name: Option<String>,

    /// Project description
    #[arg(long)]
    pub description: Option<String>,

    /// Primary programming language
    #[arg(long)]
    pub language: Option<String>,

    /// Documentation output format
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

impl CliCommand for InitCommand {
    async fn execute(&self) -> Result<()> {
        let target_path = self
            .path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        if !target_path.exists() {
            return Err(XzeError::filesystem(format!(
                "Target directory does not exist: {}",
                target_path.display()
            )));
        }

        let config_path = target_path.join(".xze.yaml");

        // Check if config already exists
        if config_path.exists() && !self.force {
            if self.yes {
                return Err(XzeError::validation(
                    "Configuration already exists. Use --force to overwrite.".to_string(),
                ));
            }

            println!(
                "Configuration file already exists at: {}",
                config_path.display()
            );
            println!("Use --force to overwrite or choose a different directory.");
            return Ok(());
        }

        // Create configuration
        let config = self.create_config(&target_path).await?;

        // Write configuration file
        self.write_config_file(&config_path, &config).await?;

        println!("✅ XZe initialized successfully!");
        println!("📁 Configuration written to: {}", config_path.display());
        println!("🚀 Run 'xze analyze' to start analyzing your repository");

        Ok(())
    }

    fn name(&self) -> &'static str {
        "init"
    }

    fn validate(&self) -> Result<()> {
        // Validate template
        let valid_templates = ["default", "minimal", "comprehensive"];
        if !valid_templates.contains(&self.template.as_str()) {
            return Err(XzeError::validation(format!(
                "Invalid template: {}. Available templates: {}",
                self.template,
                valid_templates.join(", ")
            )));
        }

        // Validate format
        let valid_formats = ["markdown", "html", "pdf"];
        if !valid_formats.contains(&self.format.as_str()) {
            return Err(XzeError::validation(format!(
                "Invalid format: {}. Available formats: {}",
                self.format,
                valid_formats.join(", ")
            )));
        }

        Ok(())
    }
}

impl InitCommand {
    /// Create configuration based on project analysis and user input
    async fn create_config(&self, target_path: &Path) -> Result<XzeConfig> {
        let _project_name = self.determine_project_name(target_path)?;
        let _language = self.determine_language(target_path).await?;
        let _description = self
            .description
            .clone()
            .unwrap_or_else(|| format!("Documentation for {}", _project_name));

        let config = XzeConfig::default();

        Ok(config)
    }

    /// Determine project name from directory or user input
    fn determine_project_name(&self, target_path: &Path) -> Result<String> {
        if let Some(name) = &self.name {
            return Ok(name.clone());
        }

        target_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .ok_or_else(|| XzeError::validation("Could not determine project name".to_string()))
    }

    /// Determine primary programming language
    async fn determine_language(&self, target_path: &Path) -> Result<String> {
        if let Some(language) = &self.language {
            return Ok(language.clone());
        }

        // Auto-detect language based on file extensions
        let language = self.detect_language(target_path).await?;
        Ok(language)
    }

    /// Detect programming language by scanning files
    async fn detect_language(&self, path: &Path) -> Result<String> {
        use std::collections::HashMap;
        use walkdir::WalkDir;

        let mut language_counts: HashMap<String, usize> = HashMap::new();

        for entry in WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(extension) = entry.path().extension() {
                if let Some(ext_str) = extension.to_str() {
                    let language = match ext_str {
                        "rs" => "rust",
                        "go" => "go",
                        "py" => "python",
                        "js" | "ts" => "javascript",
                        "java" => "java",
                        "cpp" | "cc" | "cxx" => "cpp",
                        "c" => "c",
                        "cs" => "csharp",
                        "rb" => "ruby",
                        "php" => "php",
                        _ => continue,
                    };

                    *language_counts.entry(language.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Return the most common language, or "unknown" if none found
        let detected_language = language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or_else(|| "unknown".to_string());

        Ok(detected_language)
    }

    /// Write configuration to file
    async fn write_config_file(&self, config_path: &PathBuf, config: &XzeConfig) -> Result<()> {
        let yaml_content = serde_yaml::to_string(config)
            .map_err(|e| XzeError::validation(format!("Failed to serialize config: {}", e)))?;

        tokio::fs::write(config_path, yaml_content)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}

/// XZe configuration structure

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_template() {
        let mut cmd = InitCommand {
            path: None,
            template: "default".to_string(),
            force: false,
            yes: false,
            name: None,
            description: None,
            language: None,
            format: "markdown".to_string(),
        };

        assert!(cmd.validate().is_ok());

        cmd.template = "invalid".to_string();
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_validate_format() {
        let mut cmd = InitCommand {
            path: None,
            template: "default".to_string(),
            force: false,
            yes: false,
            name: None,
            description: None,
            language: None,
            format: "markdown".to_string(),
        };

        assert!(cmd.validate().is_ok());

        cmd.format = "invalid".to_string();
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_determine_project_name() {
        let cmd = InitCommand {
            path: None,
            template: "default".to_string(),
            force: false,
            yes: false,
            name: Some("test-project".to_string()),
            description: None,
            language: None,
            format: "markdown".to_string(),
        };

        let path = PathBuf::from("/test/my-project");
        let name = cmd.determine_project_name(&path).unwrap();
        assert_eq!(name, "test-project");
    }

    #[tokio::test]
    async fn test_detect_language_rust() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("main.rs");
        tokio::fs::write(&rust_file, "fn main() {}").await.unwrap();

        let cmd = InitCommand {
            path: None,
            template: "default".to_string(),
            force: false,
            yes: false,
            name: None,
            description: None,
            language: None,
            format: "markdown".to_string(),
        };

        let language = cmd
            .detect_language(&temp_dir.path().to_path_buf())
            .await
            .unwrap();
        assert_eq!(language, "rust");
    }

    #[test]
    fn test_config_serialization() {
        let config = XzeConfig::minimal(
            "test".to_string(),
            "rust".to_string(),
            "Test project".to_string(),
        );

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("language: rust"));
    }

    #[tokio::test]
    async fn test_init_nonexistent_directory() {
        let cmd = InitCommand {
            path: Some(PathBuf::from("/nonexistent/directory")),
            template: "default".to_string(),
            force: false,
            yes: false,
            name: None,
            description: None,
            language: None,
            format: "markdown".to_string(),
        };

        assert!(cmd.execute().await.is_err());
    }
}
