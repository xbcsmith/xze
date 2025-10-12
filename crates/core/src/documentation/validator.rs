//! Documentation validator for checking quality and compliance

use crate::{
    error::{Result, XzeError},
    repository::Repository,
    types::DiátaxisCategory,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{debug, info, warn};

/// Documentation validator trait
#[async_trait]
pub trait DocumentationValidator: Send + Sync {
    /// Validate a single document
    async fn validate_document(&self, path: &Path, content: &str) -> Result<ValidationResult>;

    /// Validate all documentation in a directory
    async fn validate_directory(&self, dir: &Path) -> Result<Vec<ValidationResult>>;

    /// Validate documentation against repository
    async fn validate_against_repository(
        &self,
        docs_dir: &Path,
        repo: &Repository,
    ) -> Result<RepositoryValidationResult>;
}

/// Validation result for a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// File path that was validated
    pub file_path: PathBuf,
    /// Document category (if detected)
    pub category: Option<DiátaxisCategory>,
    /// Overall validation score (0.0 to 1.0)
    pub score: f32,
    /// Issues found during validation
    pub issues: Vec<ValidationIssue>,
    /// Validation metrics
    pub metrics: ValidationMetrics,
    /// Validation timestamp
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl ValidationResult {
    /// Check if validation passed (no critical issues)
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    /// Get issues by severity
    pub fn issues_by_severity(&self, severity: IssueSeverity) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == severity)
            .collect()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.issues_by_severity(IssueSeverity::Error).len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.issues_by_severity(IssueSeverity::Warning).len()
    }

    /// Get info count
    pub fn info_count(&self) -> usize {
        self.issues_by_severity(IssueSeverity::Info).len()
    }
}

/// Repository-level validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryValidationResult {
    /// Repository name
    pub repository_name: String,
    /// Individual document results
    pub document_results: Vec<ValidationResult>,
    /// Overall repository score
    pub overall_score: f32,
    /// Missing documentation categories
    pub missing_categories: Vec<DiátaxisCategory>,
    /// Coverage metrics
    pub coverage: CoverageMetrics,
    /// Repository-level issues
    pub repository_issues: Vec<ValidationIssue>,
}

impl RepositoryValidationResult {
    /// Check if repository documentation is valid
    pub fn is_valid(&self) -> bool {
        self.document_results.iter().all(|result| result.is_valid())
            && !self
                .repository_issues
                .iter()
                .any(|issue| issue.severity == IssueSeverity::Error)
    }

    /// Get total error count across all documents
    pub fn total_error_count(&self) -> usize {
        self.document_results
            .iter()
            .map(|r| r.error_count())
            .sum::<usize>()
            + self
                .repository_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .count()
    }

    /// Get total warning count across all documents
    pub fn total_warning_count(&self) -> usize {
        self.document_results
            .iter()
            .map(|r| r.warning_count())
            .sum::<usize>()
            + self
                .repository_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Warning)
                .count()
    }
}

/// Validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue type
    pub issue_type: IssueType,
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue description
    pub message: String,
    /// Line number (if applicable)
    pub line_number: Option<usize>,
    /// Column number (if applicable)
    pub column_number: Option<usize>,
    /// Suggestion for fixing the issue
    pub suggestion: Option<String>,
}

/// Types of validation issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    /// Structure and formatting issues
    Structure,
    /// Content quality issues
    Content,
    /// Diátaxis compliance issues
    Diataxis,
    /// Link and reference issues
    Links,
    /// Spelling and grammar
    Language,
    /// Markdown formatting
    Markdown,
    /// Missing content
    Missing,
    /// Outdated content
    Outdated,
}

/// Issue severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Critical errors that must be fixed
    Error,
    /// Warnings that should be addressed
    Warning,
    /// Informational suggestions
    Info,
}

/// Validation metrics for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Word count
    pub word_count: usize,
    /// Line count
    pub line_count: usize,
    /// Number of headings
    pub heading_count: usize,
    /// Number of links
    pub link_count: usize,
    /// Number of code blocks
    pub code_block_count: usize,
    /// Readability score (0.0 to 1.0)
    pub readability_score: f32,
    /// Structure score (0.0 to 1.0)
    pub structure_score: f32,
}

/// Coverage metrics for repository documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    /// Percentage of public functions documented
    pub function_coverage: f32,
    /// Percentage of public types documented
    pub type_coverage: f32,
    /// Percentage of modules documented
    pub module_coverage: f32,
    /// Overall documentation coverage
    pub overall_coverage: f32,
    /// Diátaxis category coverage
    pub category_coverage: HashMap<DiátaxisCategory, bool>,
}

/// Default documentation validator implementation
pub struct DiátaxisValidator {
    config: ValidatorConfig,
}

impl DiátaxisValidator {
    /// Create a new validator
    pub fn new(config: ValidatorConfig) -> Self {
        Self { config }
    }

    /// Detect document category from content and path
    fn detect_category(&self, path: &Path, content: &str) -> Option<DiátaxisCategory> {
        let path_str = path.to_string_lossy().to_lowercase();
        let content_lower = content.to_lowercase();

        // Check path-based detection first
        if path_str.contains("tutorial") {
            Some(DiátaxisCategory::Tutorial)
        } else if path_str.contains("how-to") || path_str.contains("howto") {
            Some(DiátaxisCategory::HowTo)
        } else if path_str.contains("reference") || path_str.contains("api") {
            Some(DiátaxisCategory::Reference)
        } else if path_str.contains("explanation") || path_str.contains("architecture") {
            Some(DiátaxisCategory::Explanation)
        } else {
            // Content-based detection
            if content_lower.contains("step by step") || content_lower.contains("tutorial") {
                Some(DiátaxisCategory::Tutorial)
            } else if content_lower.contains("how to") || content_lower.contains("guide") {
                Some(DiátaxisCategory::HowTo)
            } else if content_lower.contains("api") || content_lower.contains("reference") {
                Some(DiátaxisCategory::Reference)
            } else if content_lower.contains("why") || content_lower.contains("explanation") {
                Some(DiátaxisCategory::Explanation)
            } else {
                None
            }
        }
    }

    /// Calculate readability score (simplified)
    fn calculate_readability_score(&self, content: &str) -> f32 {
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.is_empty() {
            return 0.0;
        }

        let sentences = content.matches(&['.', '!', '?'][..]).count().max(1);
        let avg_sentence_length = words.len() as f32 / sentences as f32;

        // Simple readability heuristic (inverted - shorter sentences are more readable)
        let base_score = 1.0 - (avg_sentence_length / 50.0).min(1.0);

        // Adjust for markdown formatting (headers, lists make it more readable)
        let format_bonus = if content.contains("##") || content.contains("- ") {
            0.1
        } else {
            0.0
        };

        (base_score + format_bonus).min(1.0)
    }

    /// Calculate structure score
    fn calculate_structure_score(&self, content: &str) -> f32 {
        let mut score = 0.0;
        let max_score = 5.0;

        // Has title (H1)
        if content.contains("# ") {
            score += 1.0;
        }

        // Has subsections (H2, H3)
        if content.matches("## ").count() > 0 {
            score += 1.0;
        }

        // Has code examples
        if content.contains("```") {
            score += 1.0;
        }

        // Has lists
        if content.contains("- ") || content.contains("1. ") {
            score += 1.0;
        }

        // Has links
        if content.contains("[") && content.contains("](") {
            score += 1.0;
        }

        score / max_score
    }

    /// Validate markdown content
    fn validate_markdown(&self, content: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;

            // Check for broken links
            if line.contains("](") {
                // Simple check for broken markdown links
                if line.matches('[').count() != line.matches(']').count() {
                    issues.push(ValidationIssue {
                        issue_type: IssueType::Links,
                        severity: IssueSeverity::Error,
                        message: "Malformed link syntax".to_string(),
                        line_number: Some(line_number),
                        column_number: None,
                        suggestion: Some("Check that all brackets are properly closed".to_string()),
                    });
                }
            }

            // Check for empty headings
            if line.starts_with('#') && line.trim_start_matches('#').trim().is_empty() {
                issues.push(ValidationIssue {
                    issue_type: IssueType::Structure,
                    severity: IssueSeverity::Warning,
                    message: "Empty heading found".to_string(),
                    line_number: Some(line_number),
                    column_number: None,
                    suggestion: Some("Add content to the heading or remove it".to_string()),
                });
            }

            // Check for very long lines
            if line.len() > self.config.max_line_length {
                issues.push(ValidationIssue {
                    issue_type: IssueType::Markdown,
                    severity: IssueSeverity::Info,
                    message: format!("Line exceeds {} characters", self.config.max_line_length),
                    line_number: Some(line_number),
                    column_number: None,
                    suggestion: Some(
                        "Consider breaking long lines for better readability".to_string(),
                    ),
                });
            }
        }

        issues
    }

    /// Validate Diátaxis compliance
    fn validate_diataxis_compliance(
        &self,
        content: &str,
        category: &DiátaxisCategory,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let content_lower = content.to_lowercase();

        match category {
            DiátaxisCategory::Tutorial => {
                // Tutorials should be learning-oriented and hands-on
                if !content_lower.contains("step") && !content_lower.contains("example") {
                    issues.push(ValidationIssue {
                        issue_type: IssueType::Diataxis,
                        severity: IssueSeverity::Warning,
                        message: "Tutorial should include step-by-step instructions or examples"
                            .to_string(),
                        line_number: None,
                        column_number: None,
                        suggestion: Some(
                            "Add practical examples and step-by-step guidance".to_string(),
                        ),
                    });
                }
            }
            DiátaxisCategory::HowTo => {
                // How-to guides should be goal-oriented
                if !content_lower.contains("how") && !content_lower.contains("to") {
                    issues.push(ValidationIssue {
                        issue_type: IssueType::Diataxis,
                        severity: IssueSeverity::Info,
                        message: "How-to guide should clearly state the goal".to_string(),
                        line_number: None,
                        column_number: None,
                        suggestion: Some(
                            "Start with a clear statement of what will be accomplished".to_string(),
                        ),
                    });
                }
            }
            DiátaxisCategory::Reference => {
                // Reference should be information-oriented
                if !content.contains("```") && !content_lower.contains("api") {
                    issues.push(ValidationIssue {
                        issue_type: IssueType::Diataxis,
                        severity: IssueSeverity::Warning,
                        message:
                            "Reference documentation should include code examples or API details"
                                .to_string(),
                        line_number: None,
                        column_number: None,
                        suggestion: Some(
                            "Add code examples, function signatures, or API specifications"
                                .to_string(),
                        ),
                    });
                }
            }
            DiátaxisCategory::Explanation => {
                // Explanation should be understanding-oriented
                if !content_lower.contains("why") && !content_lower.contains("because") {
                    issues.push(ValidationIssue {
                        issue_type: IssueType::Diataxis,
                        severity: IssueSeverity::Info,
                        message:
                            "Explanation documentation should focus on understanding and context"
                                .to_string(),
                        line_number: None,
                        column_number: None,
                        suggestion: Some(
                            "Explain the 'why' behind concepts and design decisions".to_string(),
                        ),
                    });
                }
            }
        }

        issues
    }
}

#[async_trait]
impl DocumentationValidator for DiátaxisValidator {
    async fn validate_document(&self, path: &Path, content: &str) -> Result<ValidationResult> {
        debug!("Validating document: {:?}", path);

        let category = self.detect_category(path, content);
        let mut issues = Vec::new();

        // Basic validation
        if content.trim().is_empty() {
            issues.push(ValidationIssue {
                issue_type: IssueType::Content,
                severity: IssueSeverity::Error,
                message: "Document is empty".to_string(),
                line_number: None,
                column_number: None,
                suggestion: Some("Add content to the document".to_string()),
            });
        }

        // Markdown validation
        issues.extend(self.validate_markdown(content));

        // Diátaxis compliance validation
        if let Some(ref cat) = category {
            issues.extend(self.validate_diataxis_compliance(content, cat));
        }

        // Calculate metrics
        let word_count = content.split_whitespace().count();
        let line_count = content.lines().count();
        let heading_count = content.matches('#').count();
        let link_count = content.matches("](").count();
        let code_block_count = content.matches("```").count() / 2; // Pairs of ```

        let readability_score = self.calculate_readability_score(content);
        let structure_score = self.calculate_structure_score(content);

        // Calculate overall score
        let error_penalty = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .count() as f32
            * 0.3;
        let warning_penalty = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .count() as f32
            * 0.1;
        let base_score = (readability_score + structure_score) / 2.0;
        let score = (base_score - error_penalty - warning_penalty).max(0.0);

        Ok(ValidationResult {
            file_path: path.to_path_buf(),
            category,
            score,
            issues,
            metrics: ValidationMetrics {
                word_count,
                line_count,
                heading_count,
                link_count,
                code_block_count,
                readability_score,
                structure_score,
            },
            validated_at: chrono::Utc::now(),
        })
    }

    async fn validate_directory(&self, dir: &Path) -> Result<Vec<ValidationResult>> {
        info!("Validating documentation directory: {:?}", dir);

        let mut results = Vec::new();
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| XzeError::filesystem(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                match tokio::fs::read_to_string(&path).await {
                    Ok(content) => match self.validate_document(&path, &content).await {
                        Ok(result) => results.push(result),
                        Err(e) => warn!("Failed to validate {:?}: {}", path, e),
                    },
                    Err(e) => warn!("Failed to read file {:?}: {}", path, e),
                }
            } else if path.is_dir() {
                // Recursively validate subdirectories
                match self.validate_directory(&path).await {
                    Ok(mut sub_results) => results.append(&mut sub_results),
                    Err(e) => warn!("Failed to validate subdirectory {:?}: {}", path, e),
                }
            }
        }

        Ok(results)
    }

    async fn validate_against_repository(
        &self,
        docs_dir: &Path,
        repo: &Repository,
    ) -> Result<RepositoryValidationResult> {
        info!(
            "Validating documentation against repository: {}",
            repo.name()
        );

        let document_results = self.validate_directory(docs_dir).await?;
        let mut repository_issues = Vec::new();

        // Check for missing categories
        let present_categories: std::collections::HashSet<_> = document_results
            .iter()
            .filter_map(|r| r.category.as_ref())
            .collect();

        let all_categories = [
            DiátaxisCategory::Tutorial,
            DiátaxisCategory::HowTo,
            DiátaxisCategory::Reference,
            DiátaxisCategory::Explanation,
        ];

        let missing_categories: Vec<_> = all_categories
            .iter()
            .filter(|cat| !present_categories.contains(cat))
            .cloned()
            .collect();

        for missing_cat in &missing_categories {
            repository_issues.push(ValidationIssue {
                issue_type: IssueType::Missing,
                severity: IssueSeverity::Warning,
                message: format!("Missing {} documentation", missing_cat),
                line_number: None,
                column_number: None,
                suggestion: Some(format!("Consider adding {} documentation", missing_cat)),
            });
        }

        // Calculate coverage metrics
        let public_functions = repo.structure.public_functions().len();
        let total_functions = repo.structure.functions.len();
        let function_coverage = if total_functions > 0 {
            public_functions as f32 / total_functions as f32
        } else {
            1.0
        };

        let coverage = CoverageMetrics {
            function_coverage,
            type_coverage: 0.8,   // Placeholder
            module_coverage: 0.9, // Placeholder
            overall_coverage: function_coverage,
            category_coverage: all_categories
                .iter()
                .map(|cat| (cat.clone(), present_categories.contains(cat)))
                .collect(),
        };

        // Calculate overall score
        let doc_scores: Vec<f32> = document_results.iter().map(|r| r.score).collect();
        let overall_score = if doc_scores.is_empty() {
            0.0
        } else {
            doc_scores.iter().sum::<f32>() / doc_scores.len() as f32
        };

        Ok(RepositoryValidationResult {
            repository_name: repo.name().to_string(),
            document_results,
            overall_score,
            missing_categories,
            coverage,
            repository_issues,
        })
    }
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Maximum line length before warning
    pub max_line_length: usize,
    /// Minimum word count for documents
    pub min_word_count: usize,
    /// Whether to check for broken links
    pub check_links: bool,
    /// Whether to validate Diátaxis compliance
    pub validate_diataxis: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            max_line_length: 120,
            min_word_count: 50,
            check_links: true,
            validate_diataxis: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_document_validation() {
        let validator = DiátaxisValidator::new(ValidatorConfig::default());
        let path = Path::new("test.md");
        let content = "# Test Document\n\nThis is a test document with some content.";

        let result = validator.validate_document(path, content).await.unwrap();
        assert!(result.score > 0.5);
        assert!(result.metrics.word_count > 0);
        assert_eq!(result.metrics.heading_count, 1);
    }

    #[tokio::test]
    async fn test_empty_document_validation() {
        let validator = DiátaxisValidator::new(ValidatorConfig::default());
        let path = Path::new("empty.md");
        let content = "";

        let result = validator.validate_document(path, content).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.error_count() > 0);
    }

    #[test]
    fn test_category_detection() {
        let validator = DiátaxisValidator::new(ValidatorConfig::default());

        let tutorial_path = Path::new("tutorials/getting_started.md");
        let tutorial_content = "This is a step by step tutorial";
        assert_eq!(
            validator.detect_category(tutorial_path, tutorial_content),
            Some(DiátaxisCategory::Tutorial)
        );

        let reference_path = Path::new("reference/api.md");
        let reference_content = "API reference documentation";
        assert_eq!(
            validator.detect_category(reference_path, reference_content),
            Some(DiátaxisCategory::Reference)
        );
    }

    #[test]
    fn test_readability_score() {
        let validator = DiátaxisValidator::new(ValidatorConfig::default());

        let good_content = "This is clear. Easy to read. Has structure.";
        let score = validator.calculate_readability_score(good_content);
        assert!(score > 0.5);

        let bad_content = "This is a very long sentence that goes on and on without any breaks or punctuation which makes it very difficult to read and understand what the author is trying to communicate to the reader.";
        let bad_score = validator.calculate_readability_score(bad_content);
        assert!(bad_score < score);
    }

    #[test]
    fn test_structure_score() {
        let validator = DiátaxisValidator::new(ValidatorConfig::default());

        let well_structured = "# Title\n\n## Section\n\n- List item\n\n```code```\n\n[link](url)";
        let score = validator.calculate_structure_score(well_structured);
        assert_eq!(score, 1.0);

        let poorly_structured = "Just some text without any structure";
        let poor_score = validator.calculate_structure_score(poorly_structured);
        assert!(poor_score < score);
    }

    #[test]
    fn test_validation_result_methods() {
        let mut result = ValidationResult {
            file_path: PathBuf::from("test.md"),
            category: Some(DiátaxisCategory::Tutorial),
            score: 0.8,
            issues: vec![
                ValidationIssue {
                    issue_type: IssueType::Content,
                    severity: IssueSeverity::Error,
                    message: "Test error".to_string(),
                    line_number: Some(1),
                    column_number: None,
                    suggestion: None,
                },
                ValidationIssue {
                    issue_type: IssueType::Structure,
                    severity: IssueSeverity::Warning,
                    message: "Test warning".to_string(),
                    line_number: Some(2),
                    column_number: None,
                    suggestion: None,
                },
            ],
            metrics: ValidationMetrics {
                word_count: 100,
                line_count: 10,
                heading_count: 2,
                link_count: 1,
                code_block_count: 1,
                readability_score: 0.8,
                structure_score: 0.9,
            },
            validated_at: chrono::Utc::now(),
        };

        assert!(!result.is_valid()); // Has error
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 1);
        assert_eq!(result.info_count(), 0);
    }
}
