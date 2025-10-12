//! Documentation generation and management

use crate::{
    ai::AIAnalysisService, error::Result, repository::Repository, types::DiátaxisCategory,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

pub mod generator;
pub mod validator;

pub use generator::{
    AIDocumentationGenerator, Document, DocumentMetadata, DocumentationGenerator, GeneratorConfig,
};
pub use validator::{
    DiátaxisValidator, DocumentationValidator, ValidationResult, ValidatorConfig
};

/// Documentation analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationAnalysis {
    pub exists: bool,
    pub up_to_date: bool,
    pub completeness_score: f32,
    pub missing_sections: Vec<DiátaxisCategory>,
    pub outdated_sections: Vec<String>,
}

impl DocumentationAnalysis {
    pub fn new() -> Self {
        Self {
            exists: false,
            up_to_date: false,
            completeness_score: 0.0,
            missing_sections: Vec::new(),
            outdated_sections: Vec::new(),
        }
    }

    /// Check if documentation needs updates
    pub fn needs_update(&self) -> bool {
        !self.up_to_date || self.completeness_score < 0.8 || !self.missing_sections.is_empty()
    }
}

impl Default for DocumentationAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for documentation generation
#[async_trait]
pub trait DocGenerator: Send + Sync {
    async fn generate_reference(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_howto(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_tutorial(&self, repo: &Repository) -> Result<Vec<Document>>;
    async fn generate_explanation(&self, repo: &Repository) -> Result<Vec<Document>>;
}

/// Documentation service
pub struct DocumentationService {
    ai_service: Arc<AIAnalysisService>,
    generator: Box<dyn DocumentationGenerator>,
    validator: Box<dyn DocumentationValidator>,
}

impl DocumentationService {
    pub fn new(
        ai_service: Arc<AIAnalysisService>,
        generator: Box<dyn DocumentationGenerator>,
        validator: Box<dyn DocumentationValidator>,
    ) -> Self {
        Self {
            ai_service,
            generator,
            validator,
        }
    }

    /// Create a new documentation service with default implementations
    pub fn with_defaults(ai_service: Arc<AIAnalysisService>) -> Self {
        let generator_config = GeneratorConfig::default();
        let validator_config = ValidatorConfig::default();

        Self {
            ai_service: ai_service.clone(),
            generator: Box::new(AIDocumentationGenerator::new(ai_service, generator_config)),
            validator: Box::new(DiátaxisValidator::new(validator_config)),
        }
    }

    /// Generate all documentation for a repository
    pub async fn generate_all(&self, repo: &Repository) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // Generate reference documentation
        let reference_doc = self.generator.generate_reference(repo).await?;
        documents.push(reference_doc);

        // Generate how-to guides (example task)
        let howto_doc = self.generator.generate_howto(repo, "setup").await?;
        documents.push(howto_doc);

        // Generate tutorials (example topic)
        let tutorial_doc = self
            .generator
            .generate_tutorial(repo, "getting started")
            .await?;
        documents.push(tutorial_doc);

        // Generate explanations (example concept)
        let explanation_doc = self
            .generator
            .generate_explanation(repo, "architecture")
            .await?;
        documents.push(explanation_doc);

        Ok(documents)
    }

    /// Analyze existing documentation
    pub async fn analyze_documentation(
        &self,
        docs_path: &Path,
        _service_name: &str,
    ) -> Result<DocumentationAnalysis> {
        let results = self.validator.validate_directory(docs_path).await?;

        let mut analysis = DocumentationAnalysis::new();
        analysis.exists = !results.is_empty();

        if !results.is_empty() {
            let total_score: f32 = results.iter().map(|r| r.score).sum();
            analysis.completeness_score = total_score / results.len() as f32;
            analysis.up_to_date = results.iter().all(|r| r.is_valid());
        }

        Ok(analysis)
    }

    /// Update existing documentation
    pub async fn update_documentation(
        &self,
        _repo: &Repository,
        existing_docs: Vec<Document>,
        changes: &str,
    ) -> Result<Vec<Document>> {
        let mut updated_docs = Vec::new();

        for doc in existing_docs {
            let updated_content = self
                .ai_service
                .generate_text(&format!(
                    "Update this documentation:\n\n{}\n\nChanges:\n{}",
                    doc.content, changes
                ))
                .await?;

            let mut updated_doc = doc.clone();
            updated_doc.content = updated_content;
            updated_doc.metadata.updated_at = chrono::Utc::now();

            updated_docs.push(updated_doc);
        }

        Ok(updated_docs)
    }

    /// Validate documentation quality
    pub async fn validate_documentation(&self, document: &Document) -> Result<ValidationResult> {
        // Create a temporary path for validation
        let temp_path = std::path::PathBuf::from(&document.title);
        self.validator
            .validate_document(&temp_path, &document.content)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            DiátaxisCategory::Reference,
            "API Reference".to_string(),
            "# API\n\nContent".to_string(),
            PathBuf::from("reference/api.md"),
        );

        assert_eq!(doc.category, DiátaxisCategory::Reference);
        assert_eq!(doc.title, "API Reference");
    }

    #[test]
    fn test_documentation_analysis() {
        let mut analysis = DocumentationAnalysis::new();
        assert!(analysis.needs_update());

        analysis.up_to_date = true;
        analysis.completeness_score = 0.9;
        assert!(!analysis.needs_update());
    }
}
