//! Documentation generator for creating Diátaxis-compliant documentation

use crate::{
    ai::AIAnalysisService,
    error::{Result, XzeError},
    repository::{CodeStructure, Repository},
    types::DiátaxisCategory,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
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
    #[allow(dead_code)]
    template_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl AIDocumentationGenerator {
    /// Create a new AI documentation generator
    pub fn new(ai_service: Arc<AIAnalysisService>, config: GeneratorConfig) -> Self {
        Self {
            ai_service,
            config,
            template_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load template from cache or file
    #[allow(dead_code)]
    async fn load_template(&self, template_name: &str) -> Result<Option<String>> {
        // Check cache first
        {
            let cache = self.template_cache.read().await;
            if let Some(template) = cache.get(template_name) {
                return Ok(Some(template.clone()));
            }
        }

        // Load from file if template_dir is configured
        if let Some(template_dir) = &self.config.template_dir {
            let template_path = template_dir.join(format!("{}.md", template_name));
            if template_path.exists() {
                let content = tokio::fs::read_to_string(&template_path)
                    .await
                    .map_err(|e| XzeError::filesystem(format!("Failed to read template: {}", e)))?;

                // Cache the template
                let mut cache = self.template_cache.write().await;
                cache.insert(template_name.to_string(), content.clone());

                return Ok(Some(content));
            }
        }

        Ok(None)
    }

    /// Build context for template rendering
    #[allow(dead_code)]
    fn build_context(&self, repo: &Repository, category: &DiátaxisCategory) -> TemplateContext {
        TemplateContext {
            project_name: repo.name().to_string(),
            project_language: repo.language.to_string(),
            category: category.clone(),
            structure: repo.structure.clone(),
            metadata: repo.metadata.clone(),
        }
    }

    /// Generate table of contents for document
    #[allow(dead_code)]
    fn generate_toc(&self, content: &str) -> String {
        let mut toc = String::from("## Table of Contents\n\n");
        let mut in_code_block = false;

        for line in content.lines() {
            // Track code blocks to avoid headers inside them
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                continue;
            }

            // Process markdown headers
            if let Some(header) = line.trim().strip_prefix('#') {
                let level = header.chars().take_while(|c| *c == '#').count() + 1;
                let title = header.trim_start_matches('#').trim();

                if level > 1 && level <= 4 {
                    // Skip the main title (level 1)
                    let indent = "  ".repeat(level - 2);
                    let anchor = title
                        .to_lowercase()
                        .replace(char::is_whitespace, "-")
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '-')
                        .collect::<String>();

                    toc.push_str(&format!("{}- [{}](#{})\n", indent, title, anchor));
                }
            }
        }

        toc.push('\n');
        toc
    }

    /// Insert TOC into content after first header
    #[allow(dead_code)]
    fn insert_toc(&self, content: &str) -> String {
        let toc = self.generate_toc(content);
        let lines: Vec<&str> = content.lines().collect();

        // Find the first header
        let mut insert_pos = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with('#') {
                insert_pos = i + 1;
                break;
            }
        }

        // Skip any empty lines after the header
        while insert_pos < lines.len() && lines[insert_pos].trim().is_empty() {
            insert_pos += 1;
        }

        // Insert TOC
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            result.push_str(line);
            result.push('\n');

            if i == insert_pos - 1 {
                result.push('\n');
                result.push_str(&toc);
            }
        }

        result
    }

    /// Generate cross-reference links
    #[allow(dead_code)]
    fn generate_cross_references(&self, documents: &[Document]) -> HashMap<String, Vec<String>> {
        let mut references: HashMap<String, Vec<String>> = HashMap::new();

        for doc in documents {
            let mut related = Vec::new();

            // Link tutorials to how-tos and references
            match doc.category {
                DiátaxisCategory::Tutorial => {
                    for other in documents {
                        if matches!(
                            other.category,
                            DiátaxisCategory::HowTo | DiátaxisCategory::Reference
                        ) {
                            related.push(format!(
                                "[{}](../{}/{})",
                                other.title,
                                match other.category {
                                    DiátaxisCategory::HowTo => "how-to",
                                    DiátaxisCategory::Reference => "reference",
                                    _ => "",
                                },
                                other
                                    .file_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                            ));
                        }
                    }
                }
                DiátaxisCategory::HowTo => {
                    for other in documents {
                        if matches!(
                            other.category,
                            DiátaxisCategory::Tutorial | DiátaxisCategory::Reference
                        ) {
                            related.push(format!(
                                "[{}](../{}/{})",
                                other.title,
                                match other.category {
                                    DiátaxisCategory::Tutorial => "tutorials",
                                    DiátaxisCategory::Reference => "reference",
                                    _ => "",
                                },
                                other
                                    .file_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                            ));
                        }
                    }
                }
                DiátaxisCategory::Reference => {
                    for other in documents {
                        if matches!(other.category, DiátaxisCategory::Explanation) {
                            related.push(format!(
                                "[{}](../explanation/{})",
                                other.title,
                                other
                                    .file_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                            ));
                        }
                    }
                }
                DiátaxisCategory::Explanation => {
                    for other in documents {
                        if matches!(
                            other.category,
                            DiátaxisCategory::Tutorial | DiátaxisCategory::Reference
                        ) {
                            related.push(format!(
                                "[{}](../{}/{})",
                                other.title,
                                match other.category {
                                    DiátaxisCategory::Tutorial => "tutorials",
                                    DiátaxisCategory::Reference => "reference",
                                    _ => "",
                                },
                                other
                                    .file_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                            ));
                        }
                    }
                }
            }

            references.insert(doc.title.clone(), related);
        }

        references
    }

    /// Add related documentation section to content
    #[allow(dead_code)]
    fn add_related_docs(&self, content: &str, related: &[String]) -> String {
        if related.is_empty() {
            return content.to_string();
        }

        let mut result = content.to_string();

        if !result.ends_with("\n\n") {
            if result.ends_with('\n') {
                result.push('\n');
            } else {
                result.push_str("\n\n");
            }
        }

        result.push_str("## Related Documentation\n\n");
        for link in related {
            result.push_str(&format!("- {}\n", link));
        }
        result.push('\n');

        result
    }

    /// Generate file path for document
    fn generate_file_path(&self, category: &DiátaxisCategory, title: &str) -> PathBuf {
        let category_dir = match category {
            DiátaxisCategory::Tutorial => "tutorials",
            DiátaxisCategory::HowTo => "how_to",
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

        self.config.output_dir.join(category_dir).join(filename)
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

    /// Generate index file for a category
    pub async fn generate_index(
        &self,
        category: &DiátaxisCategory,
        documents: &[Document],
    ) -> Result<PathBuf> {
        let category_dir = match category {
            DiátaxisCategory::Tutorial => "tutorials",
            DiátaxisCategory::HowTo => "how_to",
            DiátaxisCategory::Reference => "reference",
            DiátaxisCategory::Explanation => "explanation",
        };

        let index_path = self.config.output_dir.join(category_dir).join("README.md");

        // Create parent directories
        if let Some(parent) = index_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| XzeError::filesystem(format!("Failed to create directory: {}", e)))?;
        }

        // Filter documents for this category
        let category_docs: Vec<&Document> = documents
            .iter()
            .filter(|d| d.category == *category)
            .collect();

        // Generate index content
        let mut content = format!("# {}\n\n", category);

        content.push_str(self.get_category_description(category));
        content.push_str("\n\n## Documents\n\n");

        if category_docs.is_empty() {
            content.push_str("No documents available yet.\n");
        } else {
            for doc in category_docs {
                let filename = doc
                    .file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                content.push_str(&format!("- [{}]({})\n", doc.title, filename));
            }
        }

        content.push('\n');

        // Write index file
        tokio::fs::write(&index_path, content)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write index: {}", e)))?;

        info!("Generated index for {:?} at {:?}", category, index_path);
        Ok(index_path)
    }

    /// Generate all index files
    pub async fn generate_all_indexes(&self, documents: &[Document]) -> Result<Vec<PathBuf>> {
        let mut index_paths = Vec::new();

        for category in [
            DiátaxisCategory::Tutorial,
            DiátaxisCategory::HowTo,
            DiátaxisCategory::Reference,
            DiátaxisCategory::Explanation,
        ] {
            match self.generate_index(&category, documents).await {
                Ok(path) => index_paths.push(path),
                Err(e) => warn!("Failed to generate index for {:?}: {}", category, e),
            }
        }

        Ok(index_paths)
    }

    /// Get description for a category
    fn get_category_description(&self, category: &DiátaxisCategory) -> &str {
        match category {
            DiátaxisCategory::Tutorial => {
                "Tutorials are learning-oriented lessons that guide you through \
                learning a specific topic step by step."
            }
            DiátaxisCategory::HowTo => {
                "How-to guides are goal-oriented recipes that help you solve \
                specific problems and accomplish tasks."
            }
            DiátaxisCategory::Reference => {
                "Reference documentation provides technical descriptions of the \
                system, its APIs, and components."
            }
            DiátaxisCategory::Explanation => {
                "Explanations are understanding-oriented discussions that clarify \
                and illuminate particular topics."
            }
        }
    }
}

/// Template context for rendering
#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub project_name: String,
    pub project_language: String,
    pub category: DiátaxisCategory,
    pub structure: CodeStructure,
    pub metadata: crate::repository::RepositoryMetadata,
}

/// Index generator for creating category indexes
pub struct IndexGenerator {
    config: GeneratorConfig,
}

impl IndexGenerator {
    /// Create a new index generator
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate main documentation index
    pub async fn generate_main_index(&self, documents: &[Document]) -> Result<PathBuf> {
        let index_path = self.config.output_dir.join("README.md");

        let mut content = String::from("# Documentation\n\n");
        content.push_str(
            "This documentation follows the Diátaxis framework, organizing content \
            into four categories based on your needs:\n\n",
        );

        // Group by category
        let mut by_category: HashMap<DiátaxisCategory, Vec<&Document>> = HashMap::new();
        for doc in documents {
            by_category
                .entry(doc.category.clone())
                .or_default()
                .push(doc);
        }

        // Generate sections for each category
        for category in [
            DiátaxisCategory::Tutorial,
            DiátaxisCategory::HowTo,
            DiátaxisCategory::Reference,
            DiátaxisCategory::Explanation,
        ] {
            content.push_str(&format!("## {}\n\n", category));

            let description = match category {
                DiátaxisCategory::Tutorial => {
                    "**Learning-oriented**: Step-by-step lessons to learn new concepts"
                }
                DiátaxisCategory::HowTo => {
                    "**Goal-oriented**: Practical guides to solve specific problems"
                }
                DiátaxisCategory::Reference => {
                    "**Information-oriented**: Technical specifications and API details"
                }
                DiátaxisCategory::Explanation => {
                    "**Understanding-oriented**: Background and conceptual discussion"
                }
            };

            content.push_str(&format!("{}\n\n", description));

            if let Some(docs) = by_category.get(&category) {
                for doc in docs {
                    let relative_path = doc.file_path.to_str().unwrap_or("");
                    content.push_str(&format!("- [{}]({})\n", doc.title, relative_path));
                }
            } else {
                content.push_str("*No documents available yet*\n");
            }

            content.push('\n');
        }

        content.push_str("---\n\n*This documentation was automatically generated by XZe.*\n");

        // Write main index
        tokio::fs::write(&index_path, content)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to write main index: {}", e)))?;

        info!("Generated main documentation index at {:?}", index_path);
        Ok(index_path)
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
