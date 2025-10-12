//! Documentation generator for creating Diátaxis-compliant documentation

use crate::{
    ai::AIAnalysisService,
    error::{Result, XzeError},
    repository::Repository,
    types::DiátaxisCategory,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tracing::{debug, info, warn};

/// Documentation generator trait
#[async_trait]
pub trait DocumentationGenerator: Send + Sync {
    /// Generate reference documentation
    async fn generate_reference(&self, repo: &Repository) -> Result<Document>;

    /// Generate how-to documentation
    async fn generate_howto(&self, repo: &Repository, task: &str) -> Result<Document>;

    /// Generate tutorial documentation
    async fn generate_tutorial(&self, repo: &Repository, topic: &str) -> Result<Document>;

    /// Generate explanation documentation
    async fn generate_explanation(&self, repo: &Repository, concept: &str) -> Result<Document>;

    /// Generate all documentation types
    async fn generate_all(&self, repo: &Repository) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // Generate reference
        match self.generate_reference(repo).await {
            Ok(doc) => documents.push(doc),
            Err(e) => warn!("Failed to generate reference documentation: {}", e),
        }

        // Generate common how-to guides
        let howto_tasks = vec![
            "Getting Started",
            "Installation",
            "Configuration",
            "Common Tasks",
            "Troubleshooting",
        ];

        for task in howto_tasks {
            match self.generate_howto(repo, task).await {
                Ok(doc) => documents.push(doc),
                Err(e) => warn!("Failed to generate how-to '{}': {}", task, e),
            }
        }

        // Generate tutorials
        let tutorial_topics = vec!["Quick Start", "Basic Usage", "Advanced Features"];

        for topic in tutorial_topics {
            match self.generate_tutorial(repo, topic).await {
                Ok(doc) => documents.push(doc),
                Err(e) => warn!("Failed to generate tutorial '{}': {}", topic, e),
            }
        }

        // Generate explanations
        let explanation_concepts = vec!["Architecture", "Design Principles", "Core Concepts"];

        for concept in explanation_concepts {
            match self.generate_explanation(repo, concept).await {
                Ok(doc) => documents.push(doc),
                Err(e) => warn!("Failed to generate explanation '{}': {}", concept, e),
            }
        }

        Ok(documents)
    }
}

/// Document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document category (Diátaxis)
    pub category: DiátaxisCategory,
    /// Document title
    pub title: String,
    /// Document content (Markdown)
    pub content: String,
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// File path where document should be saved
    pub file_path: PathBuf,
}

impl Document {
    /// Create a new document
    pub fn new(
        category: DiátaxisCategory,
        title: String,
        content: String,
        file_path: PathBuf,
    ) -> Self {
        Self {
            category,
            title: title.clone(),
            content,
            metadata: DocumentMetadata::new(title),
            file_path,
        }
    }

    /// Get document word count
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    /// Get document line count
    pub fn line_count(&self) -> usize {
        self.content.lines().count()
    }

    /// Check if document is empty
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.custom.insert(key, value);
    }

    /// Get file name from path
    pub fn file_name(&self) -> Option<String> {
        self.file_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    pub title: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Author
    pub author: String,
    /// Document version
    pub version: String,
    /// Tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl DocumentMetadata {
    /// Create new metadata
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            title,
            created_at: now,
            updated_at: now,
            author: "XZe Documentation Generator".to_string(),
            version: "1.0".to_string(),
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }

    /// Update the timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}

/// AI-powered documentation generator
pub struct AIDocumentationGenerator {
    ai_service: Arc<AIAnalysisService>,
    config: GeneratorConfig,
}

impl AIDocumentationGenerator {
    /// Create a new AI documentation generator
    pub fn new(ai_service: Arc<AIAnalysisService>, config: GeneratorConfig) -> Self {
        Self { ai_service, config }
    }

    /// Generate file path for document
    fn generate_file_path(&self, category: &DiátaxisCategory, title: &str) -> PathBuf {
        let category_dir = match category {
            DiátaxisCategory::Tutorial => "tutorials",
            DiátaxisCategory::HowTo => "how-to",
            DiátaxisCategory::Reference => "reference",
            DiátaxisCategory::Explanation => "explanation",
        };

        let filename = format!(
            "{}.md",
            title
                .to_lowercase()
                .replace([' ', '-'], "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        );

        PathBuf::from("docs").join(category_dir).join(filename)
    }

    /// Post-process generated content
    fn post_process_content(&self, content: &str, category: &DiátaxisCategory) -> String {
        let mut processed = content.to_string();

        // Add frontmatter if configured
        if self.config.add_frontmatter {
            let frontmatter = self.generate_frontmatter(category);
            processed = format!("{}\n\n{}", frontmatter, processed);
        }

        // Clean up common AI generation artifacts
        processed = processed
            .replace("```markdown\n", "")
            .replace("\n```", "")
            .trim()
            .to_string();

        // Ensure proper spacing
        processed = processed
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        // Add final newline
        if !processed.ends_with('\n') {
            processed.push('\n');
        }

        processed
    }

    /// Generate frontmatter for document
    fn generate_frontmatter(&self, category: &DiátaxisCategory) -> String {
        format!(
            "---\ntype: {}\ngenerated_by: XZe\ngenerated_at: {}\n---",
            category.to_string().to_lowercase(),
            chrono::Utc::now().to_rfc3339()
        )
    }
}

#[async_trait]
impl DocumentationGenerator for AIDocumentationGenerator {
    async fn generate_reference(&self, repo: &Repository) -> Result<Document> {
        info!("Generating reference documentation for {}", repo.name());

        let content = self
            .ai_service
            .generate_api_documentation(&repo.structure)
            .await?;

        let processed_content = self.post_process_content(&content, &DiátaxisCategory::Reference);
        let title = format!("{} API Reference", repo.name());
        let file_path = self.generate_file_path(&DiátaxisCategory::Reference, &title);

        let mut document = Document::new(
            DiátaxisCategory::Reference,
            title,
            processed_content,
            file_path,
        );

        document.add_metadata("repository".to_string(), repo.name().to_string());
        document.add_metadata("language".to_string(), repo.language.to_string());
        document.metadata.add_tag("api".to_string());
        document.metadata.add_tag("reference".to_string());

        debug!(
            "Generated reference document with {} words",
            document.word_count()
        );
        Ok(document)
    }

    async fn generate_howto(&self, repo: &Repository, task: &str) -> Result<Document> {
        info!(
            "Generating how-to documentation for {} - {}",
            repo.name(),
            task
        );

        let content = self
            .ai_service
            .generate_howto(&repo.structure, task)
            .await?;

        let processed_content = self.post_process_content(&content, &DiátaxisCategory::HowTo);
        let title = format!("How to: {}", task);
        let file_path = self.generate_file_path(&DiátaxisCategory::HowTo, &title);

        let mut document =
            Document::new(DiátaxisCategory::HowTo, title, processed_content, file_path);

        document.add_metadata("repository".to_string(), repo.name().to_string());
        document.add_metadata("task".to_string(), task.to_string());
        document.metadata.add_tag("howto".to_string());
        document.metadata.add_tag("guide".to_string());

        debug!(
            "Generated how-to document with {} words",
            document.word_count()
        );
        Ok(document)
    }

    async fn generate_tutorial(&self, repo: &Repository, topic: &str) -> Result<Document> {
        info!(
            "Generating tutorial documentation for {} - {}",
            repo.name(),
            topic
        );

        let content = self
            .ai_service
            .generate_tutorial(&repo.structure, topic)
            .await?;

        let processed_content = self.post_process_content(&content, &DiátaxisCategory::Tutorial);
        let title = format!("{} Tutorial", topic);
        let file_path = self.generate_file_path(&DiátaxisCategory::Tutorial, &title);

        let mut document = Document::new(
            DiátaxisCategory::Tutorial,
            title,
            processed_content,
            file_path,
        );

        document.add_metadata("repository".to_string(), repo.name().to_string());
        document.add_metadata("topic".to_string(), topic.to_string());
        document.metadata.add_tag("tutorial".to_string());
        document.metadata.add_tag("learning".to_string());

        debug!(
            "Generated tutorial document with {} words",
            document.word_count()
        );
        Ok(document)
    }

    async fn generate_explanation(&self, repo: &Repository, concept: &str) -> Result<Document> {
        info!(
            "Generating explanation documentation for {} - {}",
            repo.name(),
            concept
        );

        let content = self
            .ai_service
            .generate_explanation(&repo.structure, concept)
            .await?;

        let processed_content = self.post_process_content(&content, &DiátaxisCategory::Explanation);
        let title = format!("{} Explanation", concept);
        let file_path = self.generate_file_path(&DiátaxisCategory::Explanation, &title);

        let mut document = Document::new(
            DiátaxisCategory::Explanation,
            title,
            processed_content,
            file_path,
        );

        document.add_metadata("repository".to_string(), repo.name().to_string());
        document.add_metadata("concept".to_string(), concept.to_string());
        document.metadata.add_tag("explanation".to_string());
        document.metadata.add_tag("understanding".to_string());

        debug!(
            "Generated explanation document with {} words",
            document.word_count()
        );
        Ok(document)
    }
}

/// Generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Whether to add YAML frontmatter to documents
    pub add_frontmatter: bool,
    /// Output directory for generated docs
    pub output_dir: PathBuf,
    /// Template directory (optional)
    pub template_dir: Option<PathBuf>,
    /// Whether to overwrite existing files
    pub overwrite_existing: bool,
    /// Maximum content length per document
    pub max_content_length: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            add_frontmatter: true,
            output_dir: PathBuf::from("docs"),
            template_dir: None,
            overwrite_existing: false,
            max_content_length: 50000, // ~50KB
        }
    }
}

/// Document writer for saving generated documents
pub struct DocumentWriter {
    config: GeneratorConfig,
}

impl DocumentWriter {
    /// Create a new document writer
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Write a document to file
    pub async fn write_document(&self, document: &Document) -> Result<PathBuf> {
        let full_path = self.config.output_dir.join(&document.file_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| XzeError::filesystem(format!("Failed to create directory: {}", e)))?;
        }

        // Check if file exists and if we should overwrite
        if full_path.exists() && !self.config.overwrite_existing {
            return Err(XzeError::validation(format!(
                "File already exists and overwrite is disabled: {:?}",
                full_path
            )));
        }

        // Write content
        tokio::fs::write(&full_path, &document.content)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write file: {}", e)))?;

        info!("Wrote document to {:?}", full_path);
        Ok(full_path)
    }

    /// Write multiple documents
    pub async fn write_documents(&self, documents: &[Document]) -> Result<Vec<PathBuf>> {
        let mut written_files = Vec::new();

        for document in documents {
            match self.write_document(document).await {
                Ok(path) => written_files.push(path),
                Err(e) => {
                    warn!("Failed to write document '{}': {}", document.title, e);
                }
            }
        }

        info!("Wrote {} documents", written_files.len());
        Ok(written_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::ModelConfig, types::ProgrammingLanguage};
    use tempfile::TempDir;

    fn create_test_repository() -> Repository {
        let temp_dir = TempDir::new().unwrap();
        Repository::new(
            crate::types::RepositoryId::from("test-repo"),
            "https://github.com/test/repo".to_string(),
            temp_dir.path().to_path_buf(),
            ProgrammingLanguage::Rust,
        )
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            DiátaxisCategory::Tutorial,
            "Test Tutorial".to_string(),
            "# Test Tutorial\n\nThis is a test.".to_string(),
            PathBuf::from("test.md"),
        );

        assert_eq!(doc.category, DiátaxisCategory::Tutorial);
        assert_eq!(doc.title, "Test Tutorial");
        assert_eq!(doc.word_count(), 6);
        assert_eq!(doc.line_count(), 3);
        assert!(!doc.is_empty());
    }

    #[test]
    fn test_document_metadata() {
        let mut metadata = DocumentMetadata::new("Test Doc".to_string());
        assert_eq!(metadata.title, "Test Doc");
        assert_eq!(metadata.version, "1.0");

        metadata.add_tag("test".to_string());
        assert!(metadata.tags.contains(&"test".to_string()));

        // Adding same tag again should not duplicate
        metadata.add_tag("test".to_string());
        assert_eq!(metadata.tags.len(), 1);
    }

    #[test]
    fn test_generator_config_default() {
        let config = GeneratorConfig::default();
        assert!(config.add_frontmatter);
        assert_eq!(config.output_dir, PathBuf::from("docs"));
        assert!(!config.overwrite_existing);
        assert_eq!(config.max_content_length, 50000);
    }

    #[test]
    fn test_file_path_generation() {
        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));
        let generator = AIDocumentationGenerator::new(ai_service, GeneratorConfig::default());

        let path = generator.generate_file_path(&DiátaxisCategory::Tutorial, "Getting Started");
        assert_eq!(path, PathBuf::from("docs/tutorials/getting_started.md"));

        let path = generator.generate_file_path(&DiátaxisCategory::HowTo, "How to Configure");
        assert_eq!(path, PathBuf::from("docs/how-to/how_to_configure.md"));
    }

    #[test]
    fn test_content_post_processing() {
        let ai_service = Arc::new(AIAnalysisService::new(
            "http://localhost:11434".to_string(),
            ModelConfig::default(),
        ));
        let generator = AIDocumentationGenerator::new(ai_service, GeneratorConfig::default());

        let raw_content = "```markdown\n# Test\nContent\n```";
        let processed = generator.post_process_content(raw_content, &DiátaxisCategory::Tutorial);

        assert!(processed.starts_with("---\n"));
        assert!(processed.contains("# Test"));
        assert!(processed.contains("Content"));
        assert!(!processed.contains("```markdown"));
    }

    #[tokio::test]
    async fn test_document_writer() {
        let temp_dir = TempDir::new().unwrap();
        let config = GeneratorConfig {
            output_dir: temp_dir.path().to_path_buf(),
            overwrite_existing: true,
            ..Default::default()
        };

        let writer = DocumentWriter::new(config);

        let document = Document::new(
            DiátaxisCategory::Tutorial,
            "Test Tutorial".to_string(),
            "# Test Tutorial\n\nContent here.".to_string(),
            PathBuf::from("tutorials/test.md"),
        );

        let written_path = writer.write_document(&document).await.unwrap();
        assert!(written_path.exists());

        let content = std::fs::read_to_string(&written_path).unwrap();
        assert!(content.contains("# Test Tutorial"));
    }
}
