//! Core type definitions for XZe

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Programming language detected in repository
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Rust,
    Go,
    Python,
    JavaScript,
    TypeScript,
    Java,
    CSharp,
    Cpp,
    C,
    Shell,
    Yaml,
    Toml,
    Json,
    Markdown,
    Dockerfile,
    Unknown(String),
}

impl From<&str> for ProgrammingLanguage {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "go" => Self::Go,
            "python" | "py" => Self::Python,
            "javascript" | "js" => Self::JavaScript,
            "typescript" | "ts" => Self::TypeScript,
            "java" => Self::Java,
            "csharp" | "cs" => Self::CSharp,
            "cpp" | "c++" => Self::Cpp,
            "c" => Self::C,
            "shell" | "bash" | "sh" => Self::Shell,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "markdown" | "md" => Self::Markdown,
            "dockerfile" => Self::Dockerfile,
            _ => Self::Unknown(s.to_string()),
        }
    }
}

impl std::fmt::Display for ProgrammingLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Go => write!(f, "Go"),
            Self::Python => write!(f, "Python"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Java => write!(f, "Java"),
            Self::CSharp => write!(f, "C#"),
            Self::Cpp => write!(f, "C++"),
            Self::C => write!(f, "C"),
            Self::Shell => write!(f, "Shell"),
            Self::Yaml => write!(f, "YAML"),
            Self::Toml => write!(f, "TOML"),
            Self::Json => write!(f, "JSON"),
            Self::Markdown => write!(f, "Markdown"),
            Self::Dockerfile => write!(f, "Dockerfile"),
            Self::Unknown(s) => write!(f, "{}", s),
        }
    }
}

/// Diátaxis documentation categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiátaxisCategory {
    /// Learning-oriented tutorials
    Tutorial,
    /// Goal-oriented how-to guides
    HowTo,
    /// Information-oriented reference
    Reference,
    /// Understanding-oriented explanation
    Explanation,
}

impl std::fmt::Display for DiátaxisCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tutorial => write!(f, "Tutorial"),
            Self::HowTo => write!(f, "How-To"),
            Self::Reference => write!(f, "Reference"),
            Self::Explanation => write!(f, "Explanation"),
        }
    }
}

/// Job execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued and waiting to start
    Queued,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with error
    Failed(String),
    /// Job was cancelled
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed(err) => write!(f, "Failed: {}", err),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Significance level of code changes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChangeSeverity {
    /// No significant changes
    None,
    /// Minor changes (comments, formatting, small fixes)
    Patch,
    /// Minor feature additions or moderate changes
    Minor,
    /// Major changes (breaking changes, new features, architecture changes)
    Major,
}

impl std::fmt::Display for ChangeSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Patch => write!(f, "Patch"),
            Self::Minor => write!(f, "Minor"),
            Self::Major => write!(f, "Major"),
        }
    }
}

/// Operation mode for the tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationMode {
    /// Local mode with explicit repository paths
    Local { paths: Vec<String> },
    /// Auto mode with configuration file
    Auto { config_path: String },
}

/// Repository identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryId(pub String);

impl From<String> for RepositoryId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for RepositoryId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for RepositoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Job identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub Uuid);

impl JobId {
    /// Generate a new random job ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// When the document was last updated
    pub updated_at: DateTime<Utc>,
    /// Source repository this document was generated from
    pub source_repository: RepositoryId,
    /// Git commit hash the document is based on
    pub source_commit: String,
    /// Category of documentation
    pub category: DiátaxisCategory,
    /// Tags for the document
    pub tags: Vec<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl DocumentMetadata {
    /// Create new metadata for a document
    pub fn new(
        source_repository: RepositoryId,
        source_commit: String,
        category: DiátaxisCategory,
    ) -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            source_repository,
            source_commit,
            category,
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Set custom metadata
    pub fn set_custom(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }
}

/// File change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path to the changed file
    pub path: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Number of lines added
    pub lines_added: usize,
    /// Number of lines removed
    pub lines_removed: usize,
}

/// Type of file change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed { from: String, to: String },
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "Added"),
            Self::Modified => write!(f, "Modified"),
            Self::Deleted => write!(f, "Deleted"),
            Self::Renamed { from, to } => write!(f, "Renamed: {} -> {}", from, to),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_programming_language_from_str() {
        assert_eq!(ProgrammingLanguage::from("rust"), ProgrammingLanguage::Rust);
        assert_eq!(ProgrammingLanguage::from("rs"), ProgrammingLanguage::Rust);
        assert_eq!(ProgrammingLanguage::from("go"), ProgrammingLanguage::Go);
        assert_eq!(
            ProgrammingLanguage::from("python"),
            ProgrammingLanguage::Python
        );
        assert_eq!(
            ProgrammingLanguage::from("unknown"),
            ProgrammingLanguage::Unknown("unknown".to_string())
        );
    }

    #[test]
    fn test_job_id_generation() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_document_metadata() {
        let mut metadata = DocumentMetadata::new(
            RepositoryId::from("test-repo"),
            "abc123".to_string(),
            DiátaxisCategory::Tutorial,
        );

        assert_eq!(metadata.source_repository.0, "test-repo");
        assert_eq!(metadata.source_commit, "abc123");
        assert_eq!(metadata.category, DiátaxisCategory::Tutorial);

        metadata.add_tag("rust".to_string());
        assert!(metadata.tags.contains(&"rust".to_string()));

        metadata.set_custom("author".to_string(), "xze".to_string());
        assert_eq!(metadata.custom.get("author"), Some(&"xze".to_string()));
    }

    #[test]
    fn test_change_severity_ordering() {
        assert!(ChangeSeverity::None < ChangeSeverity::Patch);
        assert!(ChangeSeverity::Patch < ChangeSeverity::Minor);
        assert!(ChangeSeverity::Minor < ChangeSeverity::Major);
    }

    #[test]
    fn test_serialization() {
        let lang = ProgrammingLanguage::Rust;
        let json = serde_json::to_string(&lang).unwrap();
        let deserialized: ProgrammingLanguage = serde_json::from_str(&json).unwrap();
        assert_eq!(lang, deserialized);
    }
}
