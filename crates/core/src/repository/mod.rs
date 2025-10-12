//! Repository management functionality

use crate::{
    error::{Result, XzeError},
    types::{ChangeSeverity, ChangeType, FileChange, ProgrammingLanguage, RepositoryId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

pub mod analyzer;
pub mod manager;
pub mod parser;

// Re-export commonly used types
pub use analyzer::LanguageAnalyzer;
pub use manager::RepositoryManager;
pub use parser::CodeParser;

// Import struct definitions
mod r#struct;
pub use r#struct::*;

/// Repository representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier for the repository
    pub id: RepositoryId,
    /// Repository URL
    pub url: String,
    /// Local path where repository is cloned
    pub local_path: PathBuf,
    /// Primary programming language
    pub language: ProgrammingLanguage,
    /// Repository structure analysis
    pub structure: CodeStructure,
    /// Repository metadata
    pub metadata: RepositoryMetadata,
}

impl Repository {
    /// Create a new repository instance
    pub fn new(
        id: RepositoryId,
        url: String,
        local_path: PathBuf,
        language: ProgrammingLanguage,
    ) -> Self {
        Self {
            id: id.clone(),
            url,
            local_path,
            language,
            structure: CodeStructure::new(),
            metadata: RepositoryMetadata::new(id),
        }
    }

    /// Check if repository exists locally
    pub fn exists(&self) -> bool {
        self.local_path.exists() && self.local_path.is_dir()
    }

    /// Get the repository name from the ID
    pub fn name(&self) -> &str {
        &self.id.0
    }

    /// Update the structure with new analysis
    pub fn update_structure(&mut self, structure: CodeStructure) {
        self.structure = structure;
        self.metadata.last_analyzed = Some(Utc::now());
        self.update_doc_coverage();
    }

    /// Calculate documentation coverage based on structure
    fn update_doc_coverage(&mut self) {
        let total_items = self.structure.item_count();
        if total_items == 0 {
            self.metadata.doc_coverage = 0.0;
            return;
        }

        let documented_items = self.count_documented_items();
        self.metadata.doc_coverage = documented_items as f32 / total_items as f32;
    }

    /// Count items that have documentation
    fn count_documented_items(&self) -> usize {
        let mut count = 0;

        count += self
            .structure
            .modules
            .iter()
            .filter(|m| m.documentation.is_some())
            .count();

        count += self
            .structure
            .functions
            .iter()
            .filter(|f| f.documentation.is_some())
            .count();

        count += self
            .structure
            .types
            .iter()
            .filter(|t| t.documentation.is_some())
            .count();

        count
    }

    /// Get files matching specific patterns
    pub fn find_files(&self, patterns: &[&str]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.local_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.is_file() {
                let path_str = path.to_string_lossy();
                for pattern in patterns {
                    if glob::Pattern::new(pattern)
                        .map_err(|e| {
                            XzeError::validation(format!("Invalid pattern '{}': {}", pattern, e))
                        })?
                        .matches(&path_str)
                    {
                        files.push(path.to_path_buf());
                        break;
                    }
                }
            }
        }

        Ok(files)
    }

    /// Get all source files for the detected language
    pub fn get_source_files(&self) -> Result<Vec<PathBuf>> {
        let patterns = match self.language {
            ProgrammingLanguage::Rust => vec!["**/*.rs"],
            ProgrammingLanguage::Go => vec!["**/*.go"],
            ProgrammingLanguage::Python => vec!["**/*.py"],
            ProgrammingLanguage::JavaScript => vec!["**/*.js"],
            ProgrammingLanguage::TypeScript => vec!["**/*.ts", "**/*.tsx"],
            ProgrammingLanguage::Java => vec!["**/*.java"],
            ProgrammingLanguage::CSharp => vec!["**/*.cs"],
            ProgrammingLanguage::Cpp => vec!["**/*.cpp", "**/*.cc", "**/*.cxx"],
            ProgrammingLanguage::C => vec!["**/*.c"],
            _ => vec!["**/*"],
        };

        self.find_files(&patterns)
    }

    /// Get configuration files
    pub fn get_config_files(&self) -> Result<Vec<PathBuf>> {
        let patterns = vec![
            "**/*.yaml",
            "**/*.yml",
            "**/*.toml",
            "**/*.json",
            "**/.env*",
            "**/Cargo.toml",
            "**/Cargo.lock",
            "**/package.json",
            "**/package-lock.json",
            "**/go.mod",
            "**/go.sum",
            "**/requirements.txt",
            "**/pyproject.toml",
            "**/Dockerfile*",
            "**/*.config.js",
            "**/*.config.ts",
        ];

        self.find_files(&patterns)
    }

    /// Detect the primary language of the repository
    pub fn detect_language(&self) -> Result<ProgrammingLanguage> {
        let mut language_counts: HashMap<ProgrammingLanguage, usize> = HashMap::new();

        for entry in WalkDir::new(&self.local_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                let lang = ProgrammingLanguage::from(extension);
                if !matches!(lang, ProgrammingLanguage::Unknown(_)) {
                    *language_counts.entry(lang).or_insert(0) += 1;
                }
            }
        }

        // Return the most common language
        Ok(language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or(ProgrammingLanguage::Unknown("mixed".to_string())))
    }

    /// Get relative path from repository root
    pub fn relative_path(&self, absolute_path: &Path) -> Option<PathBuf> {
        absolute_path
            .strip_prefix(&self.local_path)
            .ok()
            .map(|p| p.to_path_buf())
    }

    /// Check if a path should be ignored
    pub fn should_ignore_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Common directories to ignore
        let ignore_dirs = [
            "target",
            "build",
            "dist",
            "node_modules",
            ".git",
            ".svn",
            "__pycache__",
            ".pytest_cache",
            "coverage",
            ".coverage",
            "vendor",
            "deps",
            "third_party",
        ];

        for ignore_dir in &ignore_dirs {
            if path_str.contains(ignore_dir) {
                return true;
            }
        }

        // Common file patterns to ignore
        let ignore_patterns = [
            "*.log",
            "*.tmp",
            "*.temp",
            "*.bak",
            "*.swp",
            "*.swo",
            ".DS_Store",
            "Thumbs.db",
            "*.min.js",
            "*.min.css",
        ];

        for pattern in &ignore_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(&path_str) {
                    return true;
                }
            }
        }

        false
    }
}

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// Repository name/identifier
    pub name: RepositoryId,
    /// Primary programming language
    pub language: Option<ProgrammingLanguage>,
    /// Hash of the last analyzed commit
    pub last_commit: Option<String>,
    /// When the repository was last analyzed
    pub last_analyzed: Option<DateTime<Utc>>,
    /// Documentation coverage percentage (0.0 to 1.0)
    pub doc_coverage: f32,
    /// Number of source files
    pub file_count: usize,
    /// Total lines of code
    pub lines_of_code: usize,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl RepositoryMetadata {
    /// Create new metadata for a repository
    pub fn new(name: RepositoryId) -> Self {
        Self {
            name,
            language: None,
            last_commit: None,
            last_analyzed: None,
            doc_coverage: 0.0,
            file_count: 0,
            lines_of_code: 0,
            custom: HashMap::new(),
        }
    }

    /// Update metadata with current stats
    pub fn update_stats(&mut self, file_count: usize, lines_of_code: usize) {
        self.file_count = file_count;
        self.lines_of_code = lines_of_code;
    }

    /// Set custom metadata
    pub fn set_custom(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }

    /// Get custom metadata
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }
}

/// Documentation analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationAnalysis {
    /// Whether documentation exists for this repository
    pub exists: bool,
    /// Whether the documentation is up to date
    pub up_to_date: bool,
    /// Completeness score (0.0 to 1.0)
    pub completeness_score: f32,
    /// Missing documentation sections
    pub missing_sections: Vec<String>,
    /// Outdated documentation sections
    pub outdated_sections: Vec<String>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

impl DocumentationAnalysis {
    /// Create a new analysis with default values
    pub fn new() -> Self {
        Self {
            exists: false,
            up_to_date: false,
            completeness_score: 0.0,
            missing_sections: Vec::new(),
            outdated_sections: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    /// Check if documentation needs updates
    pub fn needs_update(&self) -> bool {
        !self.up_to_date || self.completeness_score < 0.8 || !self.missing_sections.is_empty()
    }

    /// Add a missing section
    pub fn add_missing_section(&mut self, section: String) {
        if !self.missing_sections.contains(&section) {
            self.missing_sections.push(section);
        }
    }

    /// Add an outdated section
    pub fn add_outdated_section(&mut self, section: String) {
        if !self.outdated_sections.contains(&section) {
            self.outdated_sections.push(section);
        }
    }

    /// Add a recommendation
    pub fn add_recommendation(&mut self, recommendation: String) {
        if !self.recommendations.contains(&recommendation) {
            self.recommendations.push(recommendation);
        }
    }
}

/// Change analysis for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeAnalysis {
    /// Files that were changed
    pub changed_files: Vec<FileChange>,
    /// Overall significance of changes
    pub change_significance: ChangeSeverity,
    /// Documentation sections that might be affected
    pub affected_docs: Vec<String>,
    /// Recommended actions based on changes
    pub recommended_actions: Vec<String>,
}

impl ChangeAnalysis {
    /// Create a new change analysis
    pub fn new() -> Self {
        Self {
            changed_files: Vec::new(),
            change_significance: ChangeSeverity::None,
            affected_docs: Vec::new(),
            recommended_actions: Vec::new(),
        }
    }

    /// Add a file change
    pub fn add_file_change(&mut self, change: FileChange) {
        self.changed_files.push(change);
        self.update_significance();
    }

    /// Update the overall significance based on file changes
    fn update_significance(&mut self) {
        let mut max_significance = ChangeSeverity::None;

        for change in &self.changed_files {
            let significance = self.calculate_file_significance(change);
            if significance > max_significance {
                max_significance = significance;
            }
        }

        self.change_significance = max_significance;
    }

    /// Calculate significance of a single file change
    fn calculate_file_significance(&self, change: &FileChange) -> ChangeSeverity {
        // Major changes
        if change.path.contains("Cargo.toml")
            || change.path.contains("package.json")
            || change.path.contains("go.mod")
            || change.path.ends_with(".proto")
            || change.path.contains("api/")
            || matches!(change.change_type, ChangeType::Added | ChangeType::Deleted)
        {
            return ChangeSeverity::Major;
        }

        // Minor changes
        if change.lines_added + change.lines_removed > 50
            || change.path.ends_with(".rs")
            || change.path.ends_with(".go")
            || change.path.ends_with(".py")
            || change.path.ends_with(".js")
            || change.path.ends_with(".ts")
        {
            return ChangeSeverity::Minor;
        }

        // Patch changes
        if change.lines_added + change.lines_removed > 0 {
            return ChangeSeverity::Patch;
        }

        ChangeSeverity::None
    }

    /// Check if changes affect API
    pub fn affects_api(&self) -> bool {
        self.changed_files.iter().any(|change| {
            change.path.contains("api/")
                || change.path.ends_with(".proto")
                || change.path.contains("interface")
                || change.path.contains("contract")
        })
    }

    /// Check if changes affect configuration
    pub fn affects_config(&self) -> bool {
        self.changed_files.iter().any(|change| {
            change.path.ends_with(".yaml")
                || change.path.ends_with(".yml")
                || change.path.ends_with(".toml")
                || change.path.ends_with(".json")
                || change.path.contains("config")
        })
    }

    /// Get total lines changed
    pub fn total_lines_changed(&self) -> usize {
        self.changed_files
            .iter()
            .map(|change| change.lines_added + change.lines_removed)
            .sum()
    }
}

impl Default for ChangeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DocumentationAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::new(
            RepositoryId::from("test-repo"),
            "https://github.com/test/repo".to_string(),
            temp_dir.path().to_path_buf(),
            ProgrammingLanguage::Rust,
        );

        assert_eq!(repo.name(), "test-repo");
        assert_eq!(repo.language, ProgrammingLanguage::Rust);
        assert!(!repo.exists()); // Directory is empty
    }

    #[test]
    fn test_documentation_analysis() {
        let mut analysis = DocumentationAnalysis::new();
        assert!(!analysis.exists);
        assert!(!analysis.up_to_date);
        assert!(analysis.needs_update());

        analysis.exists = true;
        analysis.up_to_date = true;
        analysis.completeness_score = 0.9;
        assert!(!analysis.needs_update());

        analysis.add_missing_section("API Reference".to_string());
        assert!(analysis.needs_update());
    }

    #[test]
    fn test_change_analysis() {
        let mut analysis = ChangeAnalysis::new();
        assert_eq!(analysis.change_significance, ChangeSeverity::None);

        // Add a major change
        analysis.add_file_change(FileChange {
            path: "Cargo.toml".to_string(),
            change_type: ChangeType::Modified,
            lines_added: 5,
            lines_removed: 2,
        });

        assert_eq!(analysis.change_significance, ChangeSeverity::Major);
        assert!(analysis.affects_config());
    }

    #[test]
    fn test_repository_metadata() {
        let mut metadata = RepositoryMetadata::new(RepositoryId::from("test"));
        assert_eq!(metadata.doc_coverage, 0.0);
        assert_eq!(metadata.file_count, 0);

        metadata.update_stats(10, 500);
        assert_eq!(metadata.file_count, 10);
        assert_eq!(metadata.lines_of_code, 500);

        metadata.set_custom("author".to_string(), "xze".to_string());
        assert_eq!(metadata.get_custom("author"), Some(&"xze".to_string()));
    }

    #[test]
    fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();

        // Create some Rust files
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();

        let repo = Repository::new(
            RepositoryId::from("test"),
            "test".to_string(),
            temp_dir.path().to_path_buf(),
            ProgrammingLanguage::Unknown("".to_string()),
        );

        let detected = repo.detect_language().unwrap();
        // Should detect Rust due to .rs files
        assert!(matches!(
            detected,
            ProgrammingLanguage::Rust | ProgrammingLanguage::Toml
        ));
    }

    #[test]
    fn test_should_ignore_path() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::new(
            RepositoryId::from("test"),
            "test".to_string(),
            temp_dir.path().to_path_buf(),
            ProgrammingLanguage::Rust,
        );

        assert!(repo.should_ignore_path(Path::new("target/debug/main")));
        assert!(repo.should_ignore_path(Path::new("node_modules/package")));
        assert!(repo.should_ignore_path(Path::new("file.log")));
        assert!(!repo.should_ignore_path(Path::new("src/main.rs")));
    }
}
