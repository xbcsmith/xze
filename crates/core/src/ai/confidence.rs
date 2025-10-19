//! Confidence scoring for AI-generated responses

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Scores the confidence/quality of AI-generated responses
#[derive(Debug, Clone)]
pub struct ConfidenceScorer {
    weights: ScoringWeights,
}

/// Weights for different scoring factors
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    /// Weight for length appropriateness (0.0-1.0)
    pub length: f32,
    /// Weight for structure quality (0.0-1.0)
    pub structure: f32,
    /// Weight for completeness (0.0-1.0)
    pub completeness: f32,
    /// Weight for code quality (0.0-1.0)
    pub code_quality: f32,
    /// Weight for clarity (0.0-1.0)
    pub clarity: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            length: 0.15,
            structure: 0.25,
            completeness: 0.30,
            code_quality: 0.20,
            clarity: 0.10,
        }
    }
}

impl ConfidenceScorer {
    /// Create a new confidence scorer with default weights
    pub fn new() -> Self {
        Self {
            weights: ScoringWeights::default(),
        }
    }

    /// Create a scorer with custom weights
    pub fn with_weights(weights: ScoringWeights) -> Self {
        Self { weights }
    }

    /// Score a response and return confidence metrics
    pub fn score(&self, response: &str, context: &ScoringContext) -> Result<ConfidenceScore> {
        let length_score = self.score_length(response, context);
        let structure_score = self.score_structure(response);
        let completeness_score = self.score_completeness(response, context);
        let code_quality_score = self.score_code_quality(response);
        let clarity_score = self.score_clarity(response);

        // Calculate weighted overall score
        let overall = (length_score * self.weights.length)
            + (structure_score * self.weights.structure)
            + (completeness_score * self.weights.completeness)
            + (code_quality_score * self.weights.code_quality)
            + (clarity_score * self.weights.clarity);

        Ok(ConfidenceScore {
            overall,
            length: length_score,
            structure: structure_score,
            completeness: completeness_score,
            code_quality: code_quality_score,
            clarity: clarity_score,
            factors: self.analyze_factors(response, context),
        })
    }

    /// Score the length appropriateness
    fn score_length(&self, response: &str, context: &ScoringContext) -> f32 {
        let len = response.len();

        // Ideal length depends on document type
        let (ideal_min, ideal_max) = match context.doc_type {
            DocumentType::Tutorial => (1000, 5000),
            DocumentType::HowTo => (500, 3000),
            DocumentType::Reference => (800, 8000),
            DocumentType::Explanation => (1000, 6000),
            DocumentType::Summary => (200, 1000),
            DocumentType::ApiDocs => (1500, 10000),
        };

        if len < ideal_min {
            // Too short - score decreases linearly
            (len as f32 / ideal_min as f32).min(1.0)
        } else if len > ideal_max {
            // Too long - penalty increases with length
            let excess = len - ideal_max;
            let penalty = (excess as f32 / ideal_max as f32) * 0.5;
            (1.0 - penalty).max(0.3)
        } else {
            // Within ideal range
            1.0
        }
    }

    /// Score the structure quality
    fn score_structure(&self, response: &str) -> f32 {
        let mut score = 0.0;
        let mut factors = 0;

        // Check for headers
        let header_count = response.lines().filter(|l| l.starts_with('#')).count();
        if header_count > 0 {
            score += if header_count >= 3 {
                1.0
            } else {
                0.5 + (header_count as f32 * 0.25)
            };
            factors += 1;
        }

        // Check for code blocks
        let code_block_count = response.matches("```").count() / 2;
        if code_block_count > 0 {
            score += if code_block_count >= 2 { 1.0 } else { 0.7 };
            factors += 1;
        }

        // Check for lists
        let list_items = response
            .lines()
            .filter(|l| l.trim().starts_with('-') || l.trim().starts_with('*'))
            .count();
        if list_items > 0 {
            score += if list_items >= 5 { 1.0 } else { 0.7 };
            factors += 1;
        }

        // Check for paragraphs (empty lines indicate separation)
        let paragraph_count = response
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .count();
        if paragraph_count >= 3 {
            score += 1.0;
            factors += 1;
        } else if paragraph_count > 0 {
            score += 0.5;
            factors += 1;
        }

        if factors == 0 {
            0.3 // Minimal structure
        } else {
            score / factors as f32
        }
    }

    /// Score completeness based on expected sections
    fn score_completeness(&self, response: &str, context: &ScoringContext) -> f32 {
        let required_sections = match context.doc_type {
            DocumentType::Tutorial => {
                vec!["Introduction", "Step", "Example", "What", "How", "Next"]
            }
            DocumentType::HowTo => vec!["Goal", "Step", "Example", "Prerequisites"],
            DocumentType::Reference => vec!["Function", "Parameter", "Return", "Example"],
            DocumentType::Explanation => vec!["What", "Why", "How", "Overview"],
            DocumentType::Summary => vec!["Overview", "Feature", "Key"],
            DocumentType::ApiDocs => vec!["Function", "Parameter", "Return", "Description"],
        };

        let mut found_count = 0;
        let total_sections = required_sections.len();

        for section in required_sections {
            if response.to_lowercase().contains(&section.to_lowercase()) {
                found_count += 1;
            }
        }

        if total_sections == 0 {
            1.0
        } else {
            found_count as f32 / total_sections as f32
        }
    }

    /// Score code quality in code blocks
    fn score_code_quality(&self, response: &str) -> f32 {
        let code_blocks: Vec<&str> = response
            .split("```")
            .enumerate()
            .filter_map(|(i, block)| if i % 2 == 1 { Some(block) } else { None })
            .collect();

        if code_blocks.is_empty() {
            return 0.7; // Not all responses need code
        }

        let mut total_score = 0.0;

        for block in &code_blocks {
            let mut block_score = 0.0;
            let mut factors = 0;

            // Check for comments
            if block.contains("//") || block.contains("/*") || block.contains('#') {
                block_score += 1.0;
                factors += 1;
            }

            // Check for reasonable length (not just a stub)
            let lines = block.lines().count();
            if lines >= 5 {
                block_score += 1.0;
                factors += 1;
            } else if lines > 0 {
                block_score += 0.5;
                factors += 1;
            }

            // Check for proper structure (not just placeholders)
            if !block.contains("TODO") && !block.contains("FIXME") && !block.contains("...") {
                block_score += 1.0;
                factors += 1;
            }

            // Check for indentation (suggests structured code)
            let indented_lines = block
                .lines()
                .filter(|l| l.starts_with(' ') || l.starts_with('\t'))
                .count();
            if indented_lines > 0 {
                block_score += 1.0;
                factors += 1;
            }

            if factors > 0 {
                total_score += block_score / factors as f32;
            }
        }

        if code_blocks.is_empty() {
            0.7
        } else {
            total_score / code_blocks.len() as f32
        }
    }

    /// Score clarity and readability
    fn score_clarity(&self, response: &str) -> f32 {
        let mut score = 0.0;
        let mut factors = 0;

        // Check average sentence length (too long = less clear)
        let sentences: Vec<&str> = response
            .split('.')
            .filter(|s| !s.trim().is_empty())
            .collect();

        if !sentences.is_empty() {
            let avg_words_per_sentence: f32 = sentences
                .iter()
                .map(|s| s.split_whitespace().count() as f32)
                .sum::<f32>()
                / sentences.len() as f32;

            // Ideal: 15-25 words per sentence
            let sentence_score = if avg_words_per_sentence < 10.0 {
                0.7 // Too terse
            } else if avg_words_per_sentence > 35.0 {
                0.5 // Too verbose
            } else if (15.0..=25.0).contains(&avg_words_per_sentence) {
                1.0 // Ideal
            } else {
                0.8 // Acceptable
            };

            score += sentence_score;
            factors += 1;
        }

        // Check for jargon overload (many technical terms without explanation)
        let words: Vec<&str> = response.split_whitespace().collect();
        let complex_words = words.iter().filter(|w| w.len() > 12).count();

        if !words.is_empty() {
            let complexity_ratio = complex_words as f32 / words.len() as f32;
            let complexity_score = if complexity_ratio > 0.15 {
                0.6 // Too complex
            } else if complexity_ratio < 0.05 {
                0.9 // Good balance
            } else {
                1.0 // Excellent
            };

            score += complexity_score;
            factors += 1;
        }

        // Check for transition words (indicates flow)
        let transitions = [
            "however",
            "therefore",
            "additionally",
            "furthermore",
            "moreover",
            "consequently",
            "meanwhile",
            "similarly",
            "likewise",
            "instead",
        ];

        let has_transitions = transitions
            .iter()
            .any(|t| response.to_lowercase().contains(t));

        if has_transitions {
            score += 1.0;
        } else {
            score += 0.6;
        }
        factors += 1;

        if factors == 0 {
            0.5
        } else {
            score / factors as f32
        }
    }

    /// Analyze specific factors affecting the score
    fn analyze_factors(&self, response: &str, context: &ScoringContext) -> Vec<ScoreFactor> {
        let mut factors = Vec::new();

        // Length analysis
        let len = response.len();
        factors.push(ScoreFactor {
            name: "Length".to_string(),
            score: self.score_length(response, context),
            description: format!("{} characters", len),
        });

        // Structure analysis
        let header_count = response.lines().filter(|l| l.starts_with('#')).count();
        factors.push(ScoreFactor {
            name: "Structure".to_string(),
            score: self.score_structure(response),
            description: format!("{} sections", header_count),
        });

        // Code blocks
        let code_count = response.matches("```").count() / 2;
        factors.push(ScoreFactor {
            name: "Code Examples".to_string(),
            score: self.score_code_quality(response),
            description: format!("{} code blocks", code_count),
        });

        factors
    }
}

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for scoring
#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub doc_type: DocumentType,
    pub expected_length: Option<usize>,
    pub requires_code: bool,
}

impl ScoringContext {
    /// Create a new scoring context
    pub fn new(doc_type: DocumentType) -> Self {
        Self {
            doc_type,
            expected_length: None,
            requires_code: false,
        }
    }

    /// Set expected length
    pub fn with_expected_length(mut self, length: usize) -> Self {
        self.expected_length = Some(length);
        self
    }

    /// Mark as requiring code examples
    pub fn requires_code(mut self) -> Self {
        self.requires_code = true;
        self
    }
}

/// Type of documentation being generated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentType {
    Tutorial,
    HowTo,
    Reference,
    Explanation,
    Summary,
    ApiDocs,
}

/// Confidence score for a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Overall confidence score (0.0-1.0)
    pub overall: f32,
    /// Score for length appropriateness
    pub length: f32,
    /// Score for structure quality
    pub structure: f32,
    /// Score for completeness
    pub completeness: f32,
    /// Score for code quality
    pub code_quality: f32,
    /// Score for clarity
    pub clarity: f32,
    /// Detailed factors
    pub factors: Vec<ScoreFactor>,
}

impl ConfidenceScore {
    /// Get confidence level as a category
    pub fn level(&self) -> ConfidenceLevel {
        if self.overall >= 0.8 {
            ConfidenceLevel::High
        } else if self.overall >= 0.6 {
            ConfidenceLevel::Medium
        } else if self.overall >= 0.4 {
            ConfidenceLevel::Low
        } else {
            ConfidenceLevel::VeryLow
        }
    }

    /// Check if score meets minimum threshold
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.overall >= threshold
    }

    /// Get a summary description
    pub fn summary(&self) -> String {
        format!(
            "Confidence: {:.1}% ({})",
            self.overall * 100.0,
            self.level()
        )
    }
}

/// Confidence level categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    VeryLow,
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VeryLow => write!(f, "Very Low"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
        }
    }
}

/// Individual scoring factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreFactor {
    pub name: String,
    pub score: f32,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer_creation() {
        let scorer = ConfidenceScorer::new();
        assert_eq!(scorer.weights.length, 0.15);
    }

    #[test]
    fn test_confidence_level() {
        let high_score = ConfidenceScore {
            overall: 0.85,
            length: 0.9,
            structure: 0.8,
            completeness: 0.85,
            code_quality: 0.8,
            clarity: 0.9,
            factors: vec![],
        };

        assert_eq!(high_score.level(), ConfidenceLevel::High);
        assert!(high_score.meets_threshold(0.8));
    }

    #[test]
    fn test_length_scoring() {
        let scorer = ConfidenceScorer::new();
        let context = ScoringContext::new(DocumentType::Tutorial);

        let short_response = "Too short";
        let ideal_response = "A".repeat(2000);
        let long_response = "A".repeat(10000);

        let short_score = scorer.score_length(short_response, &context);
        let ideal_score = scorer.score_length(&ideal_response, &context);
        let long_score = scorer.score_length(&long_response, &context);

        assert!(short_score < ideal_score);
        assert!(ideal_score > long_score);
        assert_eq!(ideal_score, 1.0);
    }

    #[test]
    fn test_structure_scoring() {
        let scorer = ConfidenceScorer::new();

        let well_structured = r#"
# Title

## Section 1

Some content here.

- Item 1
- Item 2

```rust
fn example() {}
```

## Section 2

More content.
        "#;

        let poorly_structured = "Just a single paragraph with no structure at all.";

        let good_score = scorer.score_structure(well_structured);
        let poor_score = scorer.score_structure(poorly_structured);

        assert!(good_score > poor_score);
        assert!(good_score > 0.7);
    }

    #[test]
    fn test_code_quality_scoring() {
        let scorer = ConfidenceScorer::new();

        let good_code = r#"
Here's an example:

```rust
// This is a well-documented function
fn calculate_sum(a: i32, b: i32) -> i32 {
    // Add the numbers
    a + b
}
```
        "#;

        let poor_code = r#"
```
TODO: Add implementation
```
        "#;

        let good_score = scorer.score_code_quality(good_code);
        let poor_score = scorer.score_code_quality(poor_code);

        assert!(good_score > poor_score);
    }

    #[test]
    fn test_overall_scoring() {
        let scorer = ConfidenceScorer::new();
        let context = ScoringContext::new(DocumentType::Tutorial);

        let quality_response = r#"
# Tutorial: Getting Started

## Introduction

This tutorial will guide you through the basics.

## Step 1: Setup

First, let's set up our environment:

```rust
// Initialize the configuration
let config = Config::default();
```

## Step 2: Usage

Now we can use the API:

```rust
// Create a new instance
let instance = MyStruct::new(config);
instance.process();
```

## Conclusion

You've learned the basics. Next steps include exploring advanced features.
        "#;

        let result = scorer.score(quality_response, &context).unwrap();

        assert!(result.overall > 0.6);
        assert_eq!(result.level(), ConfidenceLevel::Medium);
    }
}
