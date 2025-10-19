//! Context window and token management for AI models

use crate::error::{Result, XzeError};
use serde::{Deserialize, Serialize};

/// Manages context windows and token budgets for AI models
#[derive(Debug, Clone)]
pub struct ContextManager {
    max_tokens: usize,
    reserved_tokens: usize,
    encoding_overhead: f32,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            reserved_tokens: 512,   // Reserve tokens for response
            encoding_overhead: 1.3, // Conservative estimate for encoding overhead
        }
    }

    /// Create manager with custom settings
    pub fn with_settings(
        max_tokens: usize,
        reserved_tokens: usize,
        encoding_overhead: f32,
    ) -> Self {
        Self {
            max_tokens,
            reserved_tokens,
            encoding_overhead,
        }
    }

    /// Get the maximum available tokens for prompt
    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_tokens)
    }

    /// Estimate token count for text (rough approximation)
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Rough approximation: 1 token ≈ 4 characters for English text
        // Apply encoding overhead
        let base_count = (text.len() as f32 / 4.0) * self.encoding_overhead;
        base_count.ceil() as usize
    }

    /// Check if text fits within context window
    pub fn fits_in_context(&self, text: &str) -> bool {
        let tokens = self.estimate_tokens(text);
        tokens <= self.available_tokens()
    }

    /// Truncate text to fit within context window
    pub fn truncate_to_fit(&self, text: &str) -> Result<String> {
        if self.fits_in_context(text) {
            return Ok(text.to_string());
        }

        let available = self.available_tokens();
        let estimated_chars = (available as f32 * 4.0 / self.encoding_overhead) as usize;

        if estimated_chars == 0 {
            return Err(XzeError::ai("Context window too small"));
        }

        // Truncate at sentence boundary if possible
        let truncated = &text[..estimated_chars.min(text.len())];

        // Try to find last sentence boundary
        if let Some(pos) = truncated.rfind('.') {
            if pos > estimated_chars / 2 {
                // Good sentence boundary found
                return Ok(truncated[..=pos].to_string());
            }
        }

        // Fallback to word boundary
        if let Some(pos) = truncated.rfind(' ') {
            return Ok(truncated[..pos].to_string());
        }

        Ok(truncated.to_string())
    }

    /// Split text into chunks that fit in context window
    pub fn chunk_text(&self, text: &str, overlap: usize) -> Result<Vec<String>> {
        if self.fits_in_context(text) {
            return Ok(vec![text.to_string()]);
        }

        let available = self.available_tokens();
        let estimated_chars = (available as f32 * 4.0 / self.encoding_overhead) as usize;

        if estimated_chars == 0 {
            return Err(XzeError::ai("Context window too small for chunking"));
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = (start + estimated_chars).min(text.len());
            let chunk = &text[start..end];

            // Try to break at paragraph boundary
            let chunk_str = if end < text.len() {
                if let Some(pos) = chunk.rfind("\n\n") {
                    if pos > estimated_chars / 2 {
                        &chunk[..pos]
                    } else if let Some(pos) = chunk.rfind('.') {
                        &chunk[..=pos]
                    } else {
                        chunk
                    }
                } else if let Some(pos) = chunk.rfind('.') {
                    &chunk[..=pos]
                } else {
                    chunk
                }
            } else {
                chunk
            };

            chunks.push(chunk_str.to_string());

            // Move start position with overlap
            start += chunk_str.len().saturating_sub(overlap);

            // Prevent infinite loop
            if start >= text.len() || chunk_str.is_empty() {
                break;
            }
        }

        Ok(chunks)
    }

    /// Create a context summary for long texts
    pub fn summarize_context(&self, text: &str, sections: Vec<&str>) -> Result<String> {
        let mut summary = String::new();

        for section in sections {
            if let Some(content) = self.extract_section(text, section) {
                summary.push_str(&format!("## {}\n\n", section));

                // Take first paragraph or reasonable excerpt
                let excerpt = if content.len() > 500 {
                    let truncated = &content[..500];
                    if let Some(pos) = truncated.rfind('.') {
                        &content[..=pos]
                    } else {
                        truncated
                    }
                } else {
                    content.as_str()
                };

                summary.push_str(excerpt);
                summary.push_str("\n\n");
            }
        }

        if summary.is_empty() {
            // Fallback: use beginning of text
            return self.truncate_to_fit(text);
        }

        Ok(summary)
    }

    /// Extract a section from text
    fn extract_section<'a>(&self, text: &'a str, section: &str) -> Option<String> {
        let patterns = [
            format!("# {}", section),
            format!("## {}", section),
            format!("### {}", section),
        ];

        for pattern in &patterns {
            if let Some(start) = text.find(pattern) {
                let content_start = start + pattern.len();
                let content = &text[content_start..];

                // Find next section or end
                let end = content.find("\n#").unwrap_or(content.len());

                return Some(content[..end].trim().to_string());
            }
        }

        None
    }

    /// Calculate token budget for a prompt with components
    pub fn calculate_budget(&self, components: &[PromptComponent]) -> Result<TokenBudget> {
        let mut total_tokens = 0;
        let mut component_tokens = Vec::new();

        for component in components {
            let tokens = self.estimate_tokens(&component.text);
            component_tokens.push((component.name.clone(), tokens));
            total_tokens += tokens;
        }

        let available = self.available_tokens();
        let remaining = available.saturating_sub(total_tokens);

        Ok(TokenBudget {
            total: self.max_tokens,
            available,
            used: total_tokens,
            remaining,
            components: component_tokens,
            within_limit: total_tokens <= available,
        })
    }

    /// Optimize prompt to fit within context window
    pub fn optimize_prompt(&self, components: Vec<PromptComponent>) -> Result<String> {
        let budget = self.calculate_budget(&components)?;

        if budget.within_limit {
            // All components fit, concatenate them
            return Ok(components
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"));
        }

        // Need to reduce size - prioritize by importance
        let mut sorted_components = components;
        sorted_components.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut result = String::new();
        let mut used_tokens = 0;
        let available = self.available_tokens();

        for component in sorted_components {
            let tokens = self.estimate_tokens(&component.text);

            if used_tokens + tokens <= available {
                if !result.is_empty() {
                    result.push_str("\n\n");
                }
                result.push_str(&component.text);
                used_tokens += tokens;
            } else {
                // Try to fit a truncated version
                let remaining = available.saturating_sub(used_tokens);
                if remaining > 100 && component.allow_truncation {
                    let estimated_chars =
                        (remaining as f32 * 4.0 / self.encoding_overhead) as usize;
                    if estimated_chars > 50 {
                        let truncated = self.truncate_to_fit(&component.text)?;
                        if !result.is_empty() {
                            result.push_str("\n\n");
                        }
                        result.push_str(&truncated);
                        break; // No more room
                    }
                }
                break;
            }
        }

        if result.is_empty() {
            return Err(XzeError::ai(
                "Unable to fit any components in context window",
            ));
        }

        Ok(result)
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(4096) // Default 4K context window
    }
}

/// Component of a prompt with priority
#[derive(Debug, Clone)]
pub struct PromptComponent {
    pub name: String,
    pub text: String,
    pub priority: u8, // Higher = more important (0-255)
    pub allow_truncation: bool,
}

impl PromptComponent {
    /// Create a new prompt component
    pub fn new(name: String, text: String, priority: u8) -> Self {
        Self {
            name,
            text,
            priority,
            allow_truncation: true,
        }
    }

    /// Create a component that cannot be truncated
    pub fn required(name: String, text: String, priority: u8) -> Self {
        Self {
            name,
            text,
            priority,
            allow_truncation: false,
        }
    }
}

/// Token budget information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Total context window size
    pub total: usize,
    /// Available tokens for prompt
    pub available: usize,
    /// Tokens used by current prompt
    pub used: usize,
    /// Remaining tokens
    pub remaining: usize,
    /// Token usage per component
    pub components: Vec<(String, usize)>,
    /// Whether prompt fits within limit
    pub within_limit: bool,
}

impl TokenBudget {
    /// Get usage as percentage
    pub fn usage_percentage(&self) -> f32 {
        if self.available == 0 {
            100.0
        } else {
            (self.used as f32 / self.available as f32) * 100.0
        }
    }

    /// Get a summary description
    pub fn summary(&self) -> String {
        format!(
            "Using {}/{} tokens ({:.1}%) - {} remaining",
            self.used,
            self.available,
            self.usage_percentage(),
            self.remaining
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_creation() {
        let manager = ContextManager::new(4096);
        assert_eq!(manager.max_tokens, 4096);
        assert!(manager.available_tokens() > 0);
    }

    #[test]
    fn test_token_estimation() {
        let manager = ContextManager::new(4096);
        let text = "Hello, world!";
        let tokens = manager.estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 100); // Should be reasonable
    }

    #[test]
    fn test_fits_in_context() {
        let manager = ContextManager::new(1000);
        let short_text = "This is a short text.";
        let long_text = "A".repeat(10000);

        assert!(manager.fits_in_context(short_text));
        assert!(!manager.fits_in_context(&long_text));
    }

    #[test]
    fn test_truncate_to_fit() {
        let manager = ContextManager::new(1000);
        let long_text = "A".repeat(10000);

        let truncated = manager.truncate_to_fit(&long_text).unwrap();
        assert!(truncated.len() < long_text.len());
        assert!(manager.fits_in_context(&truncated));
    }

    #[test]
    fn test_chunk_text() {
        let manager = ContextManager::new(1000);
        let long_text = "A".repeat(10000);

        let chunks = manager.chunk_text(&long_text, 100).unwrap();
        assert!(chunks.len() > 1);

        for chunk in &chunks {
            assert!(manager.fits_in_context(chunk));
        }
    }

    #[test]
    fn test_extract_section() {
        let manager = ContextManager::new(4096);
        let text = r#"
# Introduction

This is the introduction.

## Details

Here are the details.

# Conclusion

This is the conclusion.
        "#;

        let intro = manager.extract_section(text, "Introduction");
        assert!(intro.is_some());
        assert!(intro.unwrap().contains("introduction"));

        let details = manager.extract_section(text, "Details");
        assert!(details.is_some());
    }

    #[test]
    fn test_token_budget() {
        let manager = ContextManager::new(4096);
        let components = vec![
            PromptComponent::new("intro".to_string(), "Introduction text".to_string(), 10),
            PromptComponent::new("body".to_string(), "Body text".to_string(), 5),
        ];

        let budget = manager.calculate_budget(&components).unwrap();
        assert!(budget.total > 0);
        assert!(budget.used > 0);
        assert!(budget.within_limit);
    }

    #[test]
    fn test_optimize_prompt() {
        let manager = ContextManager::new(1000);
        let components = vec![
            PromptComponent::new("high".to_string(), "High priority content".to_string(), 10),
            PromptComponent::new("low".to_string(), "Low priority content".to_string(), 1),
            PromptComponent::new("huge".to_string(), "A".repeat(50000), 5),
        ];

        let result = manager.optimize_prompt(components).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains("High priority"));
    }

    #[test]
    fn test_usage_percentage() {
        let budget = TokenBudget {
            total: 4096,
            available: 3584,
            used: 1792,
            remaining: 1792,
            components: vec![],
            within_limit: true,
        };

        assert_eq!(budget.usage_percentage(), 50.0);
    }
}
