//! Quality validation and continuous monitoring for keyword extraction
//!
//! This module provides quality scoring, feedback collection, and validation
//! frameworks for ensuring long-term keyword extraction quality.
//!
//! # Examples
//!
//! ```
//! use xze_core::quality_validator::{QualityValidator, QualityScore};
//!
//! let validator = QualityValidator::new();
//! let score = validator.calculate_quality_score(10, 8, 2, 0.85, 150.0);
//! assert!(score.is_good());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quality score for keyword extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    /// Overall quality score (0.0 to 1.0)
    pub overall: f64,

    /// Relevance score (0.0 to 1.0)
    pub relevance: f64,

    /// Coverage score (0.0 to 1.0)
    pub coverage: f64,

    /// Precision score (0.0 to 1.0)
    pub precision: f64,

    /// Performance score (0.0 to 1.0)
    pub performance: f64,

    /// Quality grade (Excellent, Good, Fair, Poor)
    pub grade: QualityGrade,
}

impl QualityScore {
    /// Check if quality is good (overall score >= 0.7)
    pub fn is_good(&self) -> bool {
        self.overall >= 0.7
    }

    /// Check if quality is excellent (overall score >= 0.9)
    pub fn is_excellent(&self) -> bool {
        self.overall >= 0.9
    }

    /// Check if quality needs improvement (overall score < 0.6)
    pub fn needs_improvement(&self) -> bool {
        self.overall < 0.6
    }
}

/// Quality grade classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityGrade {
    /// Excellent quality (score >= 0.9)
    Excellent,
    /// Good quality (score >= 0.7)
    Good,
    /// Fair quality (score >= 0.5)
    Fair,
    /// Poor quality (score < 0.5)
    Poor,
}

impl QualityGrade {
    /// Create grade from score
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            QualityGrade::Excellent
        } else if score >= 0.7 {
            QualityGrade::Good
        } else if score >= 0.5 {
            QualityGrade::Fair
        } else {
            QualityGrade::Poor
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityGrade::Excellent => "excellent",
            QualityGrade::Good => "good",
            QualityGrade::Fair => "fair",
            QualityGrade::Poor => "poor",
        }
    }
}

/// User feedback for keyword quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordFeedback {
    /// Document identifier
    pub document_id: String,

    /// Keywords that were extracted
    pub extracted_keywords: Vec<String>,

    /// Keywords that were useful (positive feedback)
    pub useful_keywords: Vec<String>,

    /// Keywords that were not useful (negative feedback)
    pub not_useful_keywords: Vec<String>,

    /// Keywords that were missing (should have been extracted)
    pub missing_keywords: Vec<String>,

    /// Overall rating (1-5)
    pub rating: Option<u8>,

    /// Timestamp of feedback
    pub timestamp: String,

    /// Additional comments
    pub comments: Option<String>,
}

impl KeywordFeedback {
    /// Create new feedback
    pub fn new(document_id: String, extracted_keywords: Vec<String>) -> Self {
        Self {
            document_id,
            extracted_keywords,
            useful_keywords: Vec::new(),
            not_useful_keywords: Vec::new(),
            missing_keywords: Vec::new(),
            rating: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            comments: None,
        }
    }

    /// Calculate precision from feedback
    pub fn precision(&self) -> f64 {
        let total_extracted = self.extracted_keywords.len();
        if total_extracted == 0 {
            return 0.0;
        }

        let useful = self.useful_keywords.len();
        useful as f64 / total_extracted as f64
    }

    /// Calculate recall from feedback
    pub fn recall(&self) -> f64 {
        let total_relevant = self.useful_keywords.len() + self.missing_keywords.len();
        if total_relevant == 0 {
            return 0.0;
        }

        let useful = self.useful_keywords.len();
        useful as f64 / total_relevant as f64
    }

    /// Calculate F1 score
    pub fn f1_score(&self) -> f64 {
        let precision = self.precision();
        let recall = self.recall();

        if precision + recall == 0.0 {
            return 0.0;
        }

        2.0 * (precision * recall) / (precision + recall)
    }
}

/// Validation criteria for production readiness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriteria {
    /// Minimum required error rate (percentage)
    pub max_error_rate: f64,

    /// Minimum required cache hit rate (percentage)
    pub min_cache_hit_rate: f64,

    /// Maximum acceptable average extraction time (milliseconds)
    pub max_avg_extraction_time_ms: f64,

    /// Minimum required quality score (0.0 to 1.0)
    pub min_quality_score: f64,

    /// Minimum improvement over baseline (percentage)
    pub min_improvement_percentage: f64,
}

impl Default for ValidationCriteria {
    fn default() -> Self {
        Self {
            max_error_rate: 1.0,
            min_cache_hit_rate: 90.0,
            max_avg_extraction_time_ms: 2000.0,
            min_quality_score: 0.7,
            min_improvement_percentage: 15.0,
        }
    }
}

impl ValidationCriteria {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            max_error_rate: std::env::var("VALIDATION_MAX_ERROR_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            min_cache_hit_rate: std::env::var("VALIDATION_MIN_CACHE_HIT_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(90.0),
            max_avg_extraction_time_ms: std::env::var("VALIDATION_MAX_AVG_TIME_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000.0),
            min_quality_score: std::env::var("VALIDATION_MIN_QUALITY_SCORE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.7),
            min_improvement_percentage: std::env::var("VALIDATION_MIN_IMPROVEMENT_PCT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15.0),
        }
    }
}

/// Validation result for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Name of the metric being validated
    pub metric_name: String,

    /// Whether the validation passed
    pub passed: bool,

    /// Actual value observed
    pub actual_value: f64,

    /// Expected value or threshold
    pub expected_value: f64,

    /// Optional message
    pub message: Option<String>,
}

/// Complete validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Timestamp of validation
    pub timestamp: String,

    /// Individual validation results
    pub results: Vec<ValidationResult>,

    /// Overall validation status
    pub passed: bool,

    /// Summary message
    pub summary: String,
}

impl ValidationReport {
    /// Get failed validations
    pub fn failed_validations(&self) -> Vec<&ValidationResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.timestamp,
            "passed": self.passed,
            "summary": self.summary,
            "results": self.results.iter().map(|r| {
                serde_json::json!({
                    "metric": r.metric_name,
                    "passed": r.passed,
                    "actual": r.actual_value,
                    "expected": r.expected_value,
                    "message": r.message,
                })
            }).collect::<Vec<_>>()
        })
    }
}

/// Quality validator for keyword extraction
#[derive(Debug)]
pub struct QualityValidator {
    criteria: ValidationCriteria,
    feedback_history: Vec<KeywordFeedback>,
}

impl QualityValidator {
    /// Create a new quality validator with default criteria
    pub fn new() -> Self {
        Self {
            criteria: ValidationCriteria::default(),
            feedback_history: Vec::new(),
        }
    }

    /// Create a new quality validator with custom criteria
    pub fn with_criteria(criteria: ValidationCriteria) -> Self {
        Self {
            criteria,
            feedback_history: Vec::new(),
        }
    }

    /// Calculate quality score from metrics
    ///
    /// # Arguments
    ///
    /// * `total_keywords` - Total number of keywords extracted
    /// * `useful_keywords` - Number of useful keywords (from feedback)
    /// * `missing_keywords` - Number of keywords that should have been extracted
    /// * `cache_hit_rate` - Cache hit rate (0.0 to 1.0)
    /// * `avg_extraction_time_ms` - Average extraction time in milliseconds
    ///
    /// # Returns
    ///
    /// Returns a QualityScore with component and overall scores
    pub fn calculate_quality_score(
        &self,
        total_keywords: usize,
        useful_keywords: usize,
        missing_keywords: usize,
        cache_hit_rate: f64,
        avg_extraction_time_ms: f64,
    ) -> QualityScore {
        // Precision: useful keywords / total keywords
        let precision = if total_keywords > 0 {
            useful_keywords as f64 / total_keywords as f64
        } else {
            0.0
        };

        // Recall: useful keywords / (useful + missing)
        let total_relevant = useful_keywords + missing_keywords;
        let recall = if total_relevant > 0 {
            useful_keywords as f64 / total_relevant as f64
        } else {
            0.0
        };

        // Coverage: how well we're covering the expected keywords
        let coverage = recall;

        // Relevance: F1 score (harmonic mean of precision and recall)
        let relevance = if precision + recall > 0.0 {
            2.0 * (precision * recall) / (precision + recall)
        } else {
            0.0
        };

        // Performance: based on cache hit rate and extraction time
        let cache_score = cache_hit_rate;
        let time_score = if avg_extraction_time_ms <= 1000.0 {
            1.0
        } else if avg_extraction_time_ms <= 2000.0 {
            0.8
        } else if avg_extraction_time_ms <= 5000.0 {
            0.6
        } else {
            0.3
        };
        let performance = (cache_score + time_score) / 2.0;

        // Overall: weighted average
        let overall =
            (relevance * 0.4) + (coverage * 0.2) + (precision * 0.2) + (performance * 0.2);

        let grade = QualityGrade::from_score(overall);

        QualityScore {
            overall,
            relevance,
            coverage,
            precision,
            performance,
            grade,
        }
    }

    /// Add user feedback to history
    pub fn add_feedback(&mut self, feedback: KeywordFeedback) {
        self.feedback_history.push(feedback);
    }

    /// Get all feedback history
    pub fn feedback_history(&self) -> &[KeywordFeedback] {
        &self.feedback_history
    }

    /// Calculate aggregate metrics from feedback history
    pub fn aggregate_feedback_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        if self.feedback_history.is_empty() {
            return metrics;
        }

        let mut total_precision = 0.0;
        let mut total_recall = 0.0;
        let mut total_f1 = 0.0;
        let mut total_rating = 0.0;
        let mut rating_count = 0;

        for feedback in &self.feedback_history {
            total_precision += feedback.precision();
            total_recall += feedback.recall();
            total_f1 += feedback.f1_score();

            if let Some(rating) = feedback.rating {
                total_rating += rating as f64;
                rating_count += 1;
            }
        }

        let count = self.feedback_history.len() as f64;

        metrics.insert("avg_precision".to_string(), total_precision / count);
        metrics.insert("avg_recall".to_string(), total_recall / count);
        metrics.insert("avg_f1_score".to_string(), total_f1 / count);

        if rating_count > 0 {
            metrics.insert("avg_rating".to_string(), total_rating / rating_count as f64);
        }

        metrics
    }

    /// Validate production readiness
    ///
    /// # Arguments
    ///
    /// * `error_rate` - Current error rate (percentage)
    /// * `cache_hit_rate` - Current cache hit rate (percentage)
    /// * `avg_extraction_time_ms` - Average extraction time in milliseconds
    /// * `quality_score` - Current quality score (0.0 to 1.0)
    /// * `improvement_percentage` - Improvement over baseline (percentage)
    ///
    /// # Returns
    ///
    /// Returns a ValidationReport with detailed results
    pub fn validate_production_readiness(
        &self,
        error_rate: f64,
        cache_hit_rate: f64,
        avg_extraction_time_ms: f64,
        quality_score: f64,
        improvement_percentage: f64,
    ) -> ValidationReport {
        let mut results = Vec::new();

        // Validate error rate
        let error_passed = error_rate <= self.criteria.max_error_rate;
        results.push(ValidationResult {
            metric_name: "Error Rate".to_string(),
            passed: error_passed,
            actual_value: error_rate,
            expected_value: self.criteria.max_error_rate,
            message: if error_passed {
                Some(format!(
                    "Error rate {:.2}% is below threshold {:.2}%",
                    error_rate, self.criteria.max_error_rate
                ))
            } else {
                Some(format!(
                    "Error rate {:.2}% exceeds threshold {:.2}%",
                    error_rate, self.criteria.max_error_rate
                ))
            },
        });

        // Validate cache hit rate
        let cache_passed = cache_hit_rate >= self.criteria.min_cache_hit_rate;
        results.push(ValidationResult {
            metric_name: "Cache Hit Rate".to_string(),
            passed: cache_passed,
            actual_value: cache_hit_rate,
            expected_value: self.criteria.min_cache_hit_rate,
            message: if cache_passed {
                Some(format!(
                    "Cache hit rate {:.2}% meets threshold {:.2}%",
                    cache_hit_rate, self.criteria.min_cache_hit_rate
                ))
            } else {
                Some(format!(
                    "Cache hit rate {:.2}% below threshold {:.2}%",
                    cache_hit_rate, self.criteria.min_cache_hit_rate
                ))
            },
        });

        // Validate extraction time
        let time_passed = avg_extraction_time_ms <= self.criteria.max_avg_extraction_time_ms;
        results.push(ValidationResult {
            metric_name: "Average Extraction Time".to_string(),
            passed: time_passed,
            actual_value: avg_extraction_time_ms,
            expected_value: self.criteria.max_avg_extraction_time_ms,
            message: if time_passed {
                Some(format!(
                    "Avg time {:.2}ms is below threshold {:.2}ms",
                    avg_extraction_time_ms, self.criteria.max_avg_extraction_time_ms
                ))
            } else {
                Some(format!(
                    "Avg time {:.2}ms exceeds threshold {:.2}ms",
                    avg_extraction_time_ms, self.criteria.max_avg_extraction_time_ms
                ))
            },
        });

        // Validate quality score
        let quality_passed = quality_score >= self.criteria.min_quality_score;
        results.push(ValidationResult {
            metric_name: "Quality Score".to_string(),
            passed: quality_passed,
            actual_value: quality_score,
            expected_value: self.criteria.min_quality_score,
            message: if quality_passed {
                Some(format!(
                    "Quality score {:.2} meets threshold {:.2}",
                    quality_score, self.criteria.min_quality_score
                ))
            } else {
                Some(format!(
                    "Quality score {:.2} below threshold {:.2}",
                    quality_score, self.criteria.min_quality_score
                ))
            },
        });

        // Validate improvement
        let improvement_passed = improvement_percentage >= self.criteria.min_improvement_percentage;
        results.push(ValidationResult {
            metric_name: "Search Quality Improvement".to_string(),
            passed: improvement_passed,
            actual_value: improvement_percentage,
            expected_value: self.criteria.min_improvement_percentage,
            message: if improvement_passed {
                Some(format!(
                    "Improvement {:.2}% meets target {:.2}%",
                    improvement_percentage, self.criteria.min_improvement_percentage
                ))
            } else {
                Some(format!(
                    "Improvement {:.2}% below target {:.2}%",
                    improvement_percentage, self.criteria.min_improvement_percentage
                ))
            },
        });

        let passed = results.iter().all(|r| r.passed);
        let passed_count = results.iter().filter(|r| r.passed).count();
        let total_count = results.len();

        let summary = if passed {
            format!(
                "All {} validation criteria passed. System is production ready.",
                total_count
            )
        } else {
            format!(
                "{} of {} validation criteria passed. System needs improvement.",
                passed_count, total_count
            )
        };

        ValidationReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            results,
            passed,
            summary,
        }
    }

    /// Generate periodic quality report
    pub fn generate_quality_report(&self) -> serde_json::Value {
        let aggregate_metrics = self.aggregate_feedback_metrics();

        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "feedback_count": self.feedback_history.len(),
            "aggregate_metrics": aggregate_metrics,
            "validation_criteria": {
                "max_error_rate": self.criteria.max_error_rate,
                "min_cache_hit_rate": self.criteria.min_cache_hit_rate,
                "max_avg_extraction_time_ms": self.criteria.max_avg_extraction_time_ms,
                "min_quality_score": self.criteria.min_quality_score,
                "min_improvement_percentage": self.criteria.min_improvement_percentage,
            }
        })
    }
}

impl Default for QualityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_grade_from_score() {
        assert_eq!(QualityGrade::from_score(0.95), QualityGrade::Excellent);
        assert_eq!(QualityGrade::from_score(0.75), QualityGrade::Good);
        assert_eq!(QualityGrade::from_score(0.55), QualityGrade::Fair);
        assert_eq!(QualityGrade::from_score(0.35), QualityGrade::Poor);
    }

    #[test]
    fn test_quality_grade_as_str() {
        assert_eq!(QualityGrade::Excellent.as_str(), "excellent");
        assert_eq!(QualityGrade::Good.as_str(), "good");
        assert_eq!(QualityGrade::Fair.as_str(), "fair");
        assert_eq!(QualityGrade::Poor.as_str(), "poor");
    }

    #[test]
    fn test_quality_score_is_good() {
        let score = QualityScore {
            overall: 0.75,
            relevance: 0.8,
            coverage: 0.7,
            precision: 0.75,
            performance: 0.8,
            grade: QualityGrade::Good,
        };
        assert!(score.is_good());
        assert!(!score.is_excellent());
        assert!(!score.needs_improvement());
    }

    #[test]
    fn test_keyword_feedback_new() {
        let feedback = KeywordFeedback::new(
            "doc1".to_string(),
            vec!["rust".to_string(), "cargo".to_string()],
        );

        assert_eq!(feedback.document_id, "doc1");
        assert_eq!(feedback.extracted_keywords.len(), 2);
        assert!(feedback.useful_keywords.is_empty());
    }

    #[test]
    fn test_keyword_feedback_precision() {
        let mut feedback = KeywordFeedback::new(
            "doc1".to_string(),
            vec!["rust".to_string(), "cargo".to_string(), "build".to_string()],
        );
        feedback.useful_keywords = vec!["rust".to_string(), "cargo".to_string()];

        let precision = feedback.precision();
        assert!((precision - 0.6667).abs() < 0.001);
    }

    #[test]
    fn test_keyword_feedback_recall() {
        let mut feedback = KeywordFeedback::new(
            "doc1".to_string(),
            vec!["rust".to_string(), "cargo".to_string()],
        );
        feedback.useful_keywords = vec!["rust".to_string(), "cargo".to_string()];
        feedback.missing_keywords = vec!["async".to_string()];

        let recall = feedback.recall();
        assert!((recall - 0.6667).abs() < 0.001);
    }

    #[test]
    fn test_keyword_feedback_f1_score() {
        let mut feedback = KeywordFeedback::new(
            "doc1".to_string(),
            vec!["rust".to_string(), "cargo".to_string()],
        );
        feedback.useful_keywords = vec!["rust".to_string(), "cargo".to_string()];
        feedback.missing_keywords = vec!["async".to_string()];

        let f1 = feedback.f1_score();
        assert!(f1 > 0.0);
        assert!(f1 <= 1.0);
    }

    #[test]
    fn test_validation_criteria_default() {
        let criteria = ValidationCriteria::default();
        assert_eq!(criteria.max_error_rate, 1.0);
        assert_eq!(criteria.min_cache_hit_rate, 90.0);
        assert_eq!(criteria.max_avg_extraction_time_ms, 2000.0);
    }

    #[test]
    fn test_quality_validator_new() {
        let validator = QualityValidator::new();
        assert!(validator.feedback_history.is_empty());
    }

    #[test]
    fn test_quality_validator_calculate_quality_score() {
        let validator = QualityValidator::new();
        let score = validator.calculate_quality_score(10, 8, 2, 0.90, 150.0);

        assert!(score.overall > 0.0);
        assert!(score.overall <= 1.0);
        assert!(score.precision > 0.0);
        assert!(score.relevance > 0.0);
    }

    #[test]
    fn test_quality_validator_add_feedback() {
        let mut validator = QualityValidator::new();
        let feedback = KeywordFeedback::new("doc1".to_string(), vec!["rust".to_string()]);

        validator.add_feedback(feedback);
        assert_eq!(validator.feedback_history().len(), 1);
    }

    #[test]
    fn test_quality_validator_aggregate_feedback_metrics() {
        let mut validator = QualityValidator::new();

        let mut feedback = KeywordFeedback::new(
            "doc1".to_string(),
            vec!["rust".to_string(), "cargo".to_string()],
        );
        feedback.useful_keywords = vec!["rust".to_string()];
        feedback.rating = Some(4);

        validator.add_feedback(feedback);

        let metrics = validator.aggregate_feedback_metrics();
        assert!(metrics.contains_key("avg_precision"));
        assert!(metrics.contains_key("avg_rating"));
    }

    #[test]
    fn test_quality_validator_validate_production_readiness_pass() {
        let validator = QualityValidator::new();
        let report = validator.validate_production_readiness(
            0.5,   // error_rate
            95.0,  // cache_hit_rate
            150.0, // avg_extraction_time_ms
            0.85,  // quality_score
            20.0,  // improvement_percentage
        );

        assert!(report.passed);
        assert_eq!(report.results.len(), 5);
    }

    #[test]
    fn test_quality_validator_validate_production_readiness_fail() {
        let validator = QualityValidator::new();
        let report = validator.validate_production_readiness(
            5.0,    // error_rate (too high)
            50.0,   // cache_hit_rate (too low)
            3000.0, // avg_extraction_time_ms (too high)
            0.5,    // quality_score (too low)
            5.0,    // improvement_percentage (too low)
        );

        assert!(!report.passed);
        assert!(!report.failed_validations().is_empty());
    }

    #[test]
    fn test_validation_report_failed_validations() {
        let report = ValidationReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            results: vec![
                ValidationResult {
                    metric_name: "metric1".to_string(),
                    passed: true,
                    actual_value: 1.0,
                    expected_value: 1.0,
                    message: None,
                },
                ValidationResult {
                    metric_name: "metric2".to_string(),
                    passed: false,
                    actual_value: 0.5,
                    expected_value: 1.0,
                    message: None,
                },
            ],
            passed: false,
            summary: "Test".to_string(),
        };

        let failed = report.failed_validations();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].metric_name, "metric2");
    }

    #[test]
    fn test_validation_report_to_json() {
        let report = ValidationReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            results: vec![],
            passed: true,
            summary: "All passed".to_string(),
        };

        let json = report.to_json();
        assert_eq!(json["passed"], true);
        assert_eq!(json["summary"], "All passed");
    }

    #[test]
    fn test_quality_validator_generate_quality_report() {
        let validator = QualityValidator::new();
        let report = validator.generate_quality_report();

        assert!(report["timestamp"].is_string());
        assert_eq!(report["feedback_count"], 0);
        assert!(report["validation_criteria"].is_object());
    }
}
