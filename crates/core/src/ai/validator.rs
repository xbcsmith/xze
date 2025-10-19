//! Response validation for AI-generated content

use crate::error::{Result, XzeError};
use serde::{Deserialize, Serialize};

/// Validates AI-generated responses for quality and correctness
#[derive(Debug, Clone)]
pub struct ResponseValidator {
    min_length: usize,
    max_length: usize,
    require_code_blocks: bool,
    require_sections: Vec<String>,
}

impl ResponseValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self {
            min_length: 50,
            max_length: 50000,
            require_code_blocks: false,
            require_sections: Vec::new(),
        }
    }

    /// Set minimum response length
    pub fn with_min_length(mut self, length: usize) -> Self {
        self.min_length = length;
        self
    }

    /// Set maximum response length
    pub fn with_max_length(mut self, length: usize) -> Self {
        self.max_length = length;
        self
    }

    /// Require code blocks in response
    pub fn require_code_blocks(mut self) -> Self {
        self.require_code_blocks = true;
        self
    }

    /// Add required section headers
    pub fn with_required_sections(mut self, sections: Vec<String>) -> Self {
        self.require_sections = sections;
        self
    }

    /// Validate a response
    pub fn validate(&self, response: &str) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check length constraints
        if response.len() < self.min_length {
            issues.push(format!(
                "Response too short: {} chars (minimum: {})",
                response.len(),
                self.min_length
            ));
        }

        if response.len() > self.max_length {
            warnings.push(format!(
                "Response very long: {} chars (maximum: {})",
                response.len(),
                self.max_length
            ));
        }

        // Check for empty or whitespace-only response
        if response.trim().is_empty() {
            issues.push("Response is empty or whitespace-only".to_string());
        }

        // Check for code blocks if required
        if self.require_code_blocks && !self.has_code_blocks(response) {
            warnings.push("No code blocks found in response".to_string());
        }

        // Check for required sections
        for section in &self.require_sections {
            if !self.has_section(response, section) {
                warnings.push(format!("Missing required section: {}", section));
            }
        }

        // Check for common AI hallucination patterns
        self.check_hallucinations(response, &mut warnings);

        // Check for incomplete responses
        if self.is_incomplete(response) {
            issues.push("Response appears to be incomplete".to_string());
        }

        // Determine if validation passed
        let valid = issues.is_empty();

        Ok(ValidationResult {
            valid,
            issues,
            warnings,
            length: response.len(),
            has_code_blocks: self.has_code_blocks(response),
            section_count: self.count_sections(response),
        })
    }

    /// Check if response contains code blocks
    fn has_code_blocks(&self, response: &str) -> bool {
        response.contains("```")
    }

    /// Check if response has a specific section
    fn has_section(&self, response: &str, section: &str) -> bool {
        let patterns = [
            format!("# {}", section),
            format!("## {}", section),
            format!("### {}", section),
        ];

        patterns.iter().any(|p| response.contains(p))
    }

    /// Count the number of sections in response
    fn count_sections(&self, response: &str) -> usize {
        response
            .lines()
            .filter(|line| line.starts_with('#'))
            .count()
    }

    /// Check for common hallucination patterns
    fn check_hallucinations(&self, response: &str, warnings: &mut Vec<String>) {
        // Check for placeholder text
        let placeholders = [
            "TODO",
            "FIXME",
            "placeholder",
            "example.com",
            "foo",
            "bar",
            "baz",
            "[Insert",
            "[Add",
        ];

        for placeholder in &placeholders {
            if response.contains(placeholder) {
                warnings.push(format!("Contains placeholder text: {}", placeholder));
                break;
            }
        }

        // Check for contradictions or uncertainty phrases
        let uncertainty_phrases = [
            "I'm not sure",
            "I don't know",
            "maybe",
            "might be",
            "could be",
            "I think",
            "probably",
        ];

        for phrase in &uncertainty_phrases {
            if response.to_lowercase().contains(phrase) {
                warnings.push(format!("Contains uncertainty phrase: {}", phrase));
                break;
            }
        }

        // Check for repeated content (possible loop/hallucination)
        if self.has_excessive_repetition(response) {
            warnings.push("Detected excessive repetition in response".to_string());
        }
    }

    /// Check if response appears incomplete
    fn is_incomplete(&self, response: &str) -> bool {
        let last_line = response.lines().last().unwrap_or("");

        // Check for incomplete sentences
        let ends_with_incomplete = !last_line.trim().ends_with('.')
            && !last_line.trim().ends_with('!')
            && !last_line.trim().ends_with('?')
            && !last_line.trim().ends_with("```")
            && !last_line.trim().is_empty();

        // Check for cut-off mid-sentence
        let cut_off_indicators = ["...", "and", "but", "or", "the", "a", "an"];
        let appears_cut_off = cut_off_indicators
            .iter()
            .any(|&indicator| last_line.trim().to_lowercase().ends_with(indicator));

        ends_with_incomplete || appears_cut_off
    }

    /// Check for excessive repetition
    fn has_excessive_repetition(&self, response: &str) -> bool {
        let lines: Vec<&str> = response.lines().collect();
        if lines.len() < 5 {
            return false;
        }

        // Check if same line appears multiple times consecutively
        let mut consecutive_count = 1;
        for window in lines.windows(2) {
            if window[0] == window[1] {
                consecutive_count += 1;
                if consecutive_count >= 3 {
                    return true;
                }
            } else {
                consecutive_count = 1;
            }
        }

        false
    }
}

impl Default for ResponseValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of response validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the response is valid
    pub valid: bool,
    /// Critical issues that make the response invalid
    pub issues: Vec<String>,
    /// Non-critical warnings
    pub warnings: Vec<String>,
    /// Response length in characters
    pub length: usize,
    /// Whether response contains code blocks
    pub has_code_blocks: bool,
    /// Number of sections in response
    pub section_count: usize,
}

impl ValidationResult {
    /// Check if validation passed without issues
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get a summary of validation
    pub fn summary(&self) -> String {
        if self.valid && self.warnings.is_empty() {
            "Validation passed".to_string()
        } else if self.valid {
            format!("Validation passed with {} warnings", self.warnings.len())
        } else {
            format!("Validation failed with {} issues", self.issues.len())
        }
    }

    /// Convert to Result type
    pub fn into_result(self) -> Result<()> {
        if self.valid {
            Ok(())
        } else {
            Err(XzeError::validation(format!(
                "Response validation failed: {}",
                self.issues.join(", ")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = ResponseValidator::new();
        assert_eq!(validator.min_length, 50);
        assert_eq!(validator.max_length, 50000);
    }

    #[test]
    fn test_valid_response() {
        let validator = ResponseValidator::new();
        let response = "This is a valid response with sufficient length to pass validation.";
        let result = validator.validate(response).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_short_response() {
        let validator = ResponseValidator::new();
        let response = "Too short";
        let result = validator.validate(response).unwrap();
        assert!(!result.is_valid());
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_empty_response() {
        let validator = ResponseValidator::new();
        let response = "   ";
        let result = validator.validate(response).unwrap();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_code_blocks_detection() {
        let validator = ResponseValidator::new().require_code_blocks();
        let response_with_code = "Here is some code:\n```rust\nfn main() {}\n```\nEnd.";
        let response_without_code =
            "This is a response without any code blocks but with sufficient length.";

        let result_with = validator.validate(response_with_code).unwrap();
        assert!(result_with.has_code_blocks);

        let result_without = validator.validate(response_without_code).unwrap();
        assert!(!result_without.has_code_blocks);
        assert!(result_without.has_warnings());
    }

    #[test]
    fn test_required_sections() {
        let validator = ResponseValidator::new()
            .with_required_sections(vec!["Overview".to_string(), "Examples".to_string()]);

        let response_complete = r#"
# Overview
This is the overview section.

## Examples
Here are some examples.
        "#;

        let response_incomplete = r#"
# Overview
This is the overview section.
        "#;

        let result_complete = validator.validate(response_complete).unwrap();
        assert!(result_complete.is_valid());

        let result_incomplete = validator.validate(response_incomplete).unwrap();
        assert!(result_incomplete.has_warnings());
    }

    #[test]
    fn test_placeholder_detection() {
        let validator = ResponseValidator::new();
        let response = "This response contains a TODO item that should be flagged.";
        let result = validator.validate(response).unwrap();
        assert!(result.has_warnings());
    }

    #[test]
    fn test_incomplete_response() {
        let validator = ResponseValidator::new();
        let incomplete = "This is an incomplete response that ends with the word and";
        let result = validator.validate(incomplete).unwrap();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_section_counting() {
        let validator = ResponseValidator::new();
        let response = r#"
# Section 1
Content here.

## Section 2
More content.

### Section 3
Even more content.
        "#;

        let result = validator.validate(response).unwrap();
        assert_eq!(result.section_count, 3);
    }

    #[test]
    fn test_excessive_repetition() {
        let validator = ResponseValidator::new();
        let repeated = "Line 1\nLine 1\nLine 1\nThis should be flagged as repetition.";
        let result = validator.validate(repeated).unwrap();
        assert!(result.has_warnings());
    }
}
