//! Intent classification types for Diataxis documentation framework
//!
//! This module provides types for classifying documentation intent according to the
//! Diataxis framework, which categorizes documentation into four types:
//! - Tutorial: Learning-oriented, step-by-step lessons
//! - HowTo: Task-oriented, problem-solving guides
//! - Reference: Information-oriented, technical specifications
//! - Explanation: Understanding-oriented, conceptual discussions

use crate::error::XzeError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Diataxis documentation intent classification
///
/// Represents the four types of documentation in the Diataxis framework.
///
/// # Examples
///
/// ```
/// use xze_core::ai::intent_types::DiataxisIntent;
///
/// let intent = DiataxisIntent::Tutorial;
/// assert_eq!(intent.as_str(), "tutorial");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiataxisIntent {
    /// Learning-oriented documentation: step-by-step tutorials for beginners
    Tutorial,
    /// Task-oriented documentation: practical guides for solving specific problems
    HowTo,
    /// Information-oriented documentation: technical specifications and API references
    Reference,
    /// Understanding-oriented documentation: conceptual explanations and architecture
    Explanation,
}

impl DiataxisIntent {
    /// Convert intent to string representation
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ai::intent_types::DiataxisIntent;
    ///
    /// assert_eq!(DiataxisIntent::Tutorial.as_str(), "tutorial");
    /// assert_eq!(DiataxisIntent::HowTo.as_str(), "howto");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tutorial => "tutorial",
            Self::HowTo => "howto",
            Self::Reference => "reference",
            Self::Explanation => "explanation",
        }
    }

    /// Parse intent from string (fallible)
    ///
    /// This is a convenience method that returns `Option`. For the standard
    /// `FromStr` trait implementation, use `str::parse()`.
    ///
    /// # Arguments
    ///
    /// * `s` - String representation of the intent
    ///
    /// # Returns
    ///
    /// Returns `Some(DiataxisIntent)` if valid, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ai::intent_types::DiataxisIntent;
    ///
    /// assert_eq!(DiataxisIntent::parse("tutorial"), Some(DiataxisIntent::Tutorial));
    /// assert_eq!(DiataxisIntent::parse("how-to"), Some(DiataxisIntent::HowTo));
    /// assert_eq!(DiataxisIntent::parse("invalid"), None);
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tutorial" | "tutorials" => Some(Self::Tutorial),
            "howto" | "how-to" | "how_to" | "guide" => Some(Self::HowTo),
            "reference" | "api" | "api-reference" => Some(Self::Reference),
            "explanation" | "explanations" | "concept" | "concepts" => Some(Self::Explanation),
            _ => None,
        }
    }

    /// Get all possible intents
    pub fn all() -> &'static [DiataxisIntent] {
        &[
            Self::Tutorial,
            Self::HowTo,
            Self::Reference,
            Self::Explanation,
        ]
    }

    /// Get a description of this intent type
    pub fn description(&self) -> &'static str {
        match self {
            Self::Tutorial => "Learning-oriented: step-by-step tutorials for beginners",
            Self::HowTo => "Task-oriented: practical guides for solving specific problems",
            Self::Reference => "Information-oriented: technical specifications and API references",
            Self::Explanation => "Understanding-oriented: conceptual explanations and architecture",
        }
    }
}

impl fmt::Display for DiataxisIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DiataxisIntent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid intent: '{}'", s))
    }
}

/// Confidence score for classification results
///
/// Represents a confidence value between 0.0 and 1.0.
/// Values are automatically clamped to this range.
///
/// # Examples
///
/// ```
/// use xze_core::ai::intent_types::Confidence;
///
/// let conf = Confidence::new(0.85);
/// assert_eq!(conf.value(), 0.85);
/// assert_eq!(conf.percentage(), 85.0);
///
/// // Values are clamped to 0.0-1.0 range
/// let clamped = Confidence::new(1.5);
/// assert_eq!(clamped.value(), 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Confidence(f32);

impl Confidence {
    /// Create a new confidence score
    ///
    /// # Arguments
    ///
    /// * `value` - Confidence value (automatically clamped to 0.0-1.0)
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the confidence value as f32
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Get confidence as percentage (0-100)
    pub fn percentage(&self) -> f32 {
        self.0 * 100.0
    }

    /// Check if confidence meets a threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ai::intent_types::Confidence;
    ///
    /// let conf = Confidence::new(0.85);
    /// assert!(conf.meets_threshold(0.7));
    /// assert!(!conf.meets_threshold(0.9));
    /// ```
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.0 >= threshold
    }

    /// Get confidence level category
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ai::intent_types::{Confidence, ConfidenceLevel};
    ///
    /// assert_eq!(Confidence::new(0.9).level(), ConfidenceLevel::High);
    /// assert_eq!(Confidence::new(0.6).level(), ConfidenceLevel::Medium);
    /// assert_eq!(Confidence::new(0.3).level(), ConfidenceLevel::Low);
    /// ```
    pub fn level(&self) -> ConfidenceLevel {
        match self.0 {
            x if x >= 0.8 => ConfidenceLevel::High,
            x if x >= 0.6 => ConfidenceLevel::Medium,
            _ => ConfidenceLevel::Low,
        }
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self(0.0)
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.percentage())
    }
}

/// Confidence level categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// Low confidence (< 0.6)
    Low,
    /// Medium confidence (0.6 - 0.8)
    Medium,
    /// High confidence (>= 0.8)
    High,
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// Result of intent classification
///
/// Contains the primary intent, optional secondary intents,
/// confidence scores, reasoning, and metadata.
///
/// # Examples
///
/// ```
/// use xze_core::ai::intent_types::{ClassificationResult, DiataxisIntent, Confidence, ClassificationMetadata};
///
/// let result = ClassificationResult {
///     primary_intent: DiataxisIntent::Tutorial,
///     secondary_intents: vec![],
///     confidence: Confidence::new(0.85),
///     reasoning: "Contains step-by-step instructions".to_string(),
///     metadata: ClassificationMetadata::default(),
/// };
///
/// assert_eq!(result.primary_intent, DiataxisIntent::Tutorial);
/// assert!(result.is_high_confidence());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Primary classified intent
    pub primary_intent: DiataxisIntent,

    /// Secondary intents with their confidence scores (for multi-intent detection)
    #[serde(default)]
    pub secondary_intents: Vec<(DiataxisIntent, Confidence)>,

    /// Confidence score for the primary classification
    pub confidence: Confidence,

    /// Reasoning for the classification decision
    pub reasoning: String,

    /// Classification metadata
    pub metadata: ClassificationMetadata,
}

impl ClassificationResult {
    /// Create a new classification result
    pub fn new(primary_intent: DiataxisIntent, confidence: Confidence, reasoning: String) -> Self {
        Self {
            primary_intent,
            secondary_intents: Vec::new(),
            confidence,
            reasoning,
            metadata: ClassificationMetadata::default(),
        }
    }

    /// Add a secondary intent
    pub fn with_secondary(mut self, intent: DiataxisIntent, confidence: Confidence) -> Self {
        self.secondary_intents.push((intent, confidence));
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: ClassificationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if classification has high confidence
    pub fn is_high_confidence(&self) -> bool {
        matches!(self.confidence.level(), ConfidenceLevel::High)
    }

    /// Check if classification has multiple intents
    pub fn is_multi_intent(&self) -> bool {
        !self.secondary_intents.is_empty()
    }

    /// Get all intents (primary + secondary)
    pub fn all_intents(&self) -> Vec<(DiataxisIntent, Confidence)> {
        let mut intents = vec![(self.primary_intent, self.confidence)];
        intents.extend(self.secondary_intents.iter().copied());
        intents
    }
}

/// Metadata about the classification process
///
/// Contains information about caching, timing, model used, and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    /// Whether result was retrieved from cache
    #[serde(default)]
    pub cached: bool,

    /// Classification duration in milliseconds
    pub duration_ms: u64,

    /// Model used for classification
    pub model: String,

    /// Timestamp of classification
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ClassificationMetadata {
    fn default() -> Self {
        Self {
            cached: false,
            duration_ms: 0,
            model: "unknown".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl ClassificationMetadata {
    /// Create new metadata with model name
    pub fn new(model: String) -> Self {
        Self {
            model,
            ..Default::default()
        }
    }

    /// Set cached flag
    pub fn set_cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    /// Set duration
    pub fn set_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Errors that can occur during intent classification
#[derive(Error, Debug)]
pub enum ClassificationError {
    /// Empty query provided
    #[error("Cannot classify empty query")]
    EmptyQuery,

    /// AI service unavailable
    #[error("AI service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Failed to parse AI response
    #[error("Failed to parse classification response: {0}")]
    ParseError(String),

    /// Invalid intent in response
    #[error("Invalid intent '{0}' in classification response")]
    InvalidIntent(String),

    /// Confidence score below threshold
    #[error("Low confidence score: {actual:.2} (threshold: {threshold:.2})")]
    LowConfidence { actual: f32, threshold: f32 },
}

impl From<ClassificationError> for XzeError {
    fn from(err: ClassificationError) -> Self {
        match err {
            ClassificationError::EmptyQuery => XzeError::validation("Cannot classify empty query"),
            ClassificationError::ServiceUnavailable(msg) => {
                XzeError::ai(format!("Classification service unavailable: {}", msg))
            }
            ClassificationError::ParseError(msg) => {
                XzeError::ai(format!("Failed to parse classification: {}", msg))
            }
            ClassificationError::InvalidIntent(intent) => {
                XzeError::validation(format!("Invalid intent: {}", intent))
            }
            ClassificationError::LowConfidence { actual, threshold } => {
                XzeError::validation(format!("Low confidence: {:.2} < {:.2}", actual, threshold))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diataxis_intent_as_str() {
        assert_eq!(DiataxisIntent::Tutorial.as_str(), "tutorial");
        assert_eq!(DiataxisIntent::HowTo.as_str(), "howto");
        assert_eq!(DiataxisIntent::Reference.as_str(), "reference");
        assert_eq!(DiataxisIntent::Explanation.as_str(), "explanation");
    }

    #[test]
    fn test_diataxis_intent_parse() {
        assert_eq!(
            DiataxisIntent::parse("tutorial"),
            Some(DiataxisIntent::Tutorial)
        );
        assert_eq!(
            DiataxisIntent::parse("Tutorial"),
            Some(DiataxisIntent::Tutorial)
        );
        assert_eq!(DiataxisIntent::parse("how-to"), Some(DiataxisIntent::HowTo));
        assert_eq!(DiataxisIntent::parse("howto"), Some(DiataxisIntent::HowTo));
        assert_eq!(DiataxisIntent::parse("guide"), Some(DiataxisIntent::HowTo));
        assert_eq!(
            DiataxisIntent::parse("reference"),
            Some(DiataxisIntent::Reference)
        );
        assert_eq!(
            DiataxisIntent::parse("api"),
            Some(DiataxisIntent::Reference)
        );
        assert_eq!(
            DiataxisIntent::parse("explanation"),
            Some(DiataxisIntent::Explanation)
        );
        assert_eq!(
            DiataxisIntent::parse("concept"),
            Some(DiataxisIntent::Explanation)
        );
        assert_eq!(DiataxisIntent::parse("invalid"), None);
    }

    #[test]
    fn test_diataxis_intent_from_str() {
        assert_eq!(
            "tutorial".parse::<DiataxisIntent>().unwrap(),
            DiataxisIntent::Tutorial
        );
        assert_eq!(
            "howto".parse::<DiataxisIntent>().unwrap(),
            DiataxisIntent::HowTo
        );
        assert_eq!(
            "reference".parse::<DiataxisIntent>().unwrap(),
            DiataxisIntent::Reference
        );
        assert_eq!(
            "explanation".parse::<DiataxisIntent>().unwrap(),
            DiataxisIntent::Explanation
        );
        assert!("invalid".parse::<DiataxisIntent>().is_err());
    }

    #[test]
    fn test_diataxis_intent_display() {
        assert_eq!(format!("{}", DiataxisIntent::Tutorial), "tutorial");
        assert_eq!(format!("{}", DiataxisIntent::HowTo), "howto");
    }

    #[test]
    fn test_confidence_clamping() {
        assert_eq!(Confidence::new(0.5).value(), 0.5);
        assert_eq!(Confidence::new(-0.1).value(), 0.0);
        assert_eq!(Confidence::new(1.5).value(), 1.0);
        assert_eq!(Confidence::new(0.0).value(), 0.0);
        assert_eq!(Confidence::new(1.0).value(), 1.0);
    }

    #[test]
    fn test_confidence_percentage() {
        assert_eq!(Confidence::new(0.85).percentage(), 85.0);
        assert_eq!(Confidence::new(0.0).percentage(), 0.0);
        assert_eq!(Confidence::new(1.0).percentage(), 100.0);
    }

    #[test]
    fn test_confidence_threshold() {
        let conf = Confidence::new(0.75);
        assert!(conf.meets_threshold(0.5));
        assert!(conf.meets_threshold(0.75));
        assert!(!conf.meets_threshold(0.8));
    }

    #[test]
    fn test_confidence_level() {
        assert_eq!(Confidence::new(0.9).level(), ConfidenceLevel::High);
        assert_eq!(Confidence::new(0.8).level(), ConfidenceLevel::High);
        assert_eq!(Confidence::new(0.7).level(), ConfidenceLevel::Medium);
        assert_eq!(Confidence::new(0.6).level(), ConfidenceLevel::Medium);
        assert_eq!(Confidence::new(0.5).level(), ConfidenceLevel::Low);
        assert_eq!(Confidence::new(0.0).level(), ConfidenceLevel::Low);
    }

    #[test]
    fn test_confidence_display() {
        assert_eq!(format!("{}", Confidence::new(0.85)), "85.0%");
        assert_eq!(format!("{}", Confidence::new(0.0)), "0.0%");
        assert_eq!(format!("{}", Confidence::new(1.0)), "100.0%");
    }

    #[test]
    fn test_classification_result_creation() {
        let result = ClassificationResult::new(
            DiataxisIntent::Tutorial,
            Confidence::new(0.85),
            "Test reasoning".to_string(),
        );

        assert_eq!(result.primary_intent, DiataxisIntent::Tutorial);
        assert_eq!(result.confidence.value(), 0.85);
        assert_eq!(result.reasoning, "Test reasoning");
        assert!(result.secondary_intents.is_empty());
    }

    #[test]
    fn test_classification_result_with_secondary() {
        let result = ClassificationResult::new(
            DiataxisIntent::Tutorial,
            Confidence::new(0.85),
            "Test".to_string(),
        )
        .with_secondary(DiataxisIntent::HowTo, Confidence::new(0.6));

        assert_eq!(result.secondary_intents.len(), 1);
        assert!(result.is_multi_intent());
    }

    #[test]
    fn test_classification_result_high_confidence() {
        let high = ClassificationResult::new(
            DiataxisIntent::Tutorial,
            Confidence::new(0.85),
            "Test".to_string(),
        );
        assert!(high.is_high_confidence());

        let medium = ClassificationResult::new(
            DiataxisIntent::Tutorial,
            Confidence::new(0.65),
            "Test".to_string(),
        );
        assert!(!medium.is_high_confidence());
    }

    #[test]
    fn test_classification_result_all_intents() {
        let result = ClassificationResult::new(
            DiataxisIntent::Tutorial,
            Confidence::new(0.85),
            "Test".to_string(),
        )
        .with_secondary(DiataxisIntent::HowTo, Confidence::new(0.6))
        .with_secondary(DiataxisIntent::Reference, Confidence::new(0.4));

        let all = result.all_intents();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].0, DiataxisIntent::Tutorial);
        assert_eq!(all[1].0, DiataxisIntent::HowTo);
        assert_eq!(all[2].0, DiataxisIntent::Reference);
    }

    #[test]
    fn test_classification_error_conversion() {
        let err: XzeError = ClassificationError::EmptyQuery.into();
        assert!(matches!(err, XzeError::Validation { .. }));

        let err: XzeError = ClassificationError::ServiceUnavailable("test".to_string()).into();
        assert!(matches!(err, XzeError::AiService { .. }));

        let err: XzeError = ClassificationError::InvalidIntent("bad".to_string()).into();
        assert!(matches!(err, XzeError::Validation { .. }));
    }

    #[test]
    fn test_metadata_default() {
        let meta = ClassificationMetadata::default();
        assert!(!meta.cached);
        assert_eq!(meta.duration_ms, 0);
        assert_eq!(meta.model, "unknown");
    }

    #[test]
    fn test_metadata_builder() {
        let meta = ClassificationMetadata::new("llama2".to_string())
            .set_cached(true)
            .set_duration(150);

        assert_eq!(meta.model, "llama2");
        assert!(meta.cached);
        assert_eq!(meta.duration_ms, 150);
    }
}
