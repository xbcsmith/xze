//! A/B testing framework for keyword extraction methods
//!
//! This module provides functionality to split documents between control and treatment
//! groups for A/B testing different keyword extraction methods. It uses deterministic
//! assignment based on document path hashing to ensure consistent group membership.
//!
//! # Architecture
//!
//! The A/B test framework:
//! 1. Assigns documents to control or treatment groups deterministically
//! 2. Tracks assignments for analysis
//! 3. Collects metrics per group
//! 4. Enables statistical comparison of extraction methods
//!
//! # Examples
//!
//! ```
//! use xze_core::ab_testing::{ABTest, ExtractionMethod};
//!
//! let ab_test = ABTest::new(
//!     ExtractionMethod::Frequency,
//!     ExtractionMethod::Llm,
//!     0.5, // 50/50 split
//! );
//!
//! let method = ab_test.assign_group("path/to/document.md");
//! println!("Document assigned to: {:?}", method);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Extraction method for A/B testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionMethod {
    /// Frequency-based extraction (control)
    Frequency,
    /// LLM-based extraction (treatment)
    Llm,
}

impl std::fmt::Display for ExtractionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionMethod::Frequency => write!(f, "frequency"),
            ExtractionMethod::Llm => write!(f, "llm"),
        }
    }
}

/// A/B test configuration and assignment tracker
///
/// Manages assignment of documents to control and treatment groups for
/// comparing keyword extraction methods.
///
/// # Examples
///
/// ```
/// use xze_core::ab_testing::{ABTest, ExtractionMethod};
///
/// let mut ab_test = ABTest::new(
///     ExtractionMethod::Frequency,
///     ExtractionMethod::Llm,
///     0.5,
/// );
///
/// let method1 = ab_test.assign_group("doc1.md");
/// let method2 = ab_test.assign_group("doc2.md");
///
/// // Same document always gets same assignment
/// let method1_again = ab_test.assign_group("doc1.md");
/// assert_eq!(method1, method1_again);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    /// Control group extraction method
    control_method: ExtractionMethod,

    /// Treatment group extraction method
    treatment_method: ExtractionMethod,

    /// Percentage of documents in treatment group (0.0-1.0)
    split_ratio: f64,

    /// Document path to assigned method mapping
    assignments: HashMap<String, ExtractionMethod>,

    /// Statistics per group
    stats: ABTestStats,
}

/// Statistics for A/B test groups
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ABTestStats {
    /// Number of documents in control group
    pub control_count: usize,

    /// Number of documents in treatment group
    pub treatment_count: usize,

    /// Total processing time for control group (ms)
    pub control_time_ms: u64,

    /// Total processing time for treatment group (ms)
    pub treatment_time_ms: u64,

    /// Total keywords extracted by control group
    pub control_keywords_count: usize,

    /// Total keywords extracted by treatment group
    pub treatment_keywords_count: usize,
}

impl ABTest {
    /// Creates a new A/B test configuration
    ///
    /// # Arguments
    ///
    /// * `control_method` - Extraction method for control group
    /// * `treatment_method` - Extraction method for treatment group
    /// * `split_ratio` - Percentage in treatment group (0.0 = all control, 1.0 = all treatment)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::{ABTest, ExtractionMethod};
    ///
    /// let ab_test = ABTest::new(
    ///     ExtractionMethod::Frequency,
    ///     ExtractionMethod::Llm,
    ///     0.5,
    /// );
    /// ```
    pub fn new(
        control_method: ExtractionMethod,
        treatment_method: ExtractionMethod,
        split_ratio: f64,
    ) -> Self {
        Self {
            control_method,
            treatment_method,
            split_ratio: split_ratio.clamp(0.0, 1.0),
            assignments: HashMap::new(),
            stats: ABTestStats::default(),
        }
    }

    /// Assigns a document to control or treatment group
    ///
    /// Uses deterministic hashing to ensure the same document always gets
    /// the same assignment across runs.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the document
    ///
    /// # Returns
    ///
    /// Returns the extraction method assigned to this document
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::{ABTest, ExtractionMethod};
    ///
    /// let mut ab_test = ABTest::new(
    ///     ExtractionMethod::Frequency,
    ///     ExtractionMethod::Llm,
    ///     0.5,
    /// );
    ///
    /// let method = ab_test.assign_group("path/to/doc.md");
    /// ```
    pub fn assign_group(&mut self, file_path: &str) -> ExtractionMethod {
        if let Some(&method) = self.assignments.get(file_path) {
            return method;
        }

        // Use deterministic hash for consistent assignment
        let mut hasher = DefaultHasher::new();
        file_path.hash(&mut hasher);
        let hash = hasher.finish();

        // Assign to treatment if hash modulo 100 is less than split_ratio * 100
        let in_treatment = ((hash % 100) as f64) < (self.split_ratio * 100.0);
        let method = if in_treatment {
            self.treatment_method
        } else {
            self.control_method
        };

        self.assignments.insert(file_path.to_string(), method);
        method
    }

    /// Gets the assigned method for a document path without modifying state
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the document
    ///
    /// # Returns
    ///
    /// Returns the assigned method if document has been assigned, None otherwise
    pub fn get_assignment(&self, file_path: &str) -> Option<ExtractionMethod> {
        self.assignments.get(file_path).copied()
    }

    /// Records metrics for a document enrichment
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the document
    /// * `processing_time_ms` - Time taken to extract keywords
    /// * `keyword_count` - Number of keywords extracted
    pub fn record_metrics(
        &mut self,
        file_path: &str,
        processing_time_ms: u64,
        keyword_count: usize,
    ) {
        if let Some(&method) = self.assignments.get(file_path) {
            match method {
                ExtractionMethod::Frequency => {
                    self.stats.control_count += 1;
                    self.stats.control_time_ms += processing_time_ms;
                    self.stats.control_keywords_count += keyword_count;
                }
                ExtractionMethod::Llm => {
                    self.stats.treatment_count += 1;
                    self.stats.treatment_time_ms += processing_time_ms;
                    self.stats.treatment_keywords_count += keyword_count;
                }
            }
        }
    }

    /// Gets current A/B test statistics
    ///
    /// # Returns
    ///
    /// Returns clone of current statistics
    pub fn stats(&self) -> ABTestStats {
        self.stats.clone()
    }

    /// Exports A/B test assignments to JSON
    ///
    /// # Returns
    ///
    /// Returns JSON representation of assignments and statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::{ABTest, ExtractionMethod};
    ///
    /// let ab_test = ABTest::new(
    ///     ExtractionMethod::Frequency,
    ///     ExtractionMethod::Llm,
    ///     0.5,
    /// );
    ///
    /// let json = ab_test.to_json();
    /// assert!(json.is_object());
    /// ```
    pub fn to_json(&self) -> serde_json::Value {
        let assignments_by_method: HashMap<String, Vec<String>> =
            self.assignments
                .iter()
                .fold(HashMap::new(), |mut acc, (path, method)| {
                    acc.entry(method.to_string())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                    acc
                });

        serde_json::json!({
            "config": {
                "control_method": self.control_method.to_string(),
                "treatment_method": self.treatment_method.to_string(),
                "split_ratio": self.split_ratio,
            },
            "assignments": {
                "total": self.assignments.len(),
                "control": assignments_by_method.get("frequency").map(|v| v.len()).unwrap_or(0),
                "treatment": assignments_by_method.get("llm").map(|v| v.len()).unwrap_or(0),
            },
            "statistics": {
                "control": {
                    "count": self.stats.control_count,
                    "total_time_ms": self.stats.control_time_ms,
                    "avg_time_ms": if self.stats.control_count > 0 {
                        self.stats.control_time_ms as f64 / self.stats.control_count as f64
                    } else {
                        0.0
                    },
                    "total_keywords": self.stats.control_keywords_count,
                    "avg_keywords": if self.stats.control_count > 0 {
                        self.stats.control_keywords_count as f64 / self.stats.control_count as f64
                    } else {
                        0.0
                    },
                },
                "treatment": {
                    "count": self.stats.treatment_count,
                    "total_time_ms": self.stats.treatment_time_ms,
                    "avg_time_ms": if self.stats.treatment_count > 0 {
                        self.stats.treatment_time_ms as f64 / self.stats.treatment_count as f64
                    } else {
                        0.0
                    },
                    "total_keywords": self.stats.treatment_keywords_count,
                    "avg_keywords": if self.stats.treatment_count > 0 {
                        self.stats.treatment_keywords_count as f64 / self.stats.treatment_count as f64
                    } else {
                        0.0
                    },
                },
            },
        })
    }

    /// Saves A/B test assignments and stats to a JSON file
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path to output JSON file
    ///
    /// # Errors
    ///
    /// Returns error if file write fails
    pub async fn save_to_file<P: AsRef<Path>>(&self, output_path: P) -> crate::Result<()> {
        let json = self.to_json();
        let json_string = serde_json::to_string_pretty(&json)?;
        tokio::fs::write(output_path, json_string).await?;
        Ok(())
    }

    /// Resets all assignments and statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::{ABTest, ExtractionMethod};
    ///
    /// let mut ab_test = ABTest::new(
    ///     ExtractionMethod::Frequency,
    ///     ExtractionMethod::Llm,
    ///     0.5,
    /// );
    ///
    /// ab_test.assign_group("doc1.md");
    /// ab_test.reset();
    /// assert_eq!(ab_test.assignment_count(), 0);
    /// ```
    pub fn reset(&mut self) {
        self.assignments.clear();
        self.stats = ABTestStats::default();
    }

    /// Returns the total number of assignments made
    ///
    /// # Returns
    ///
    /// Returns count of documents assigned to groups
    pub fn assignment_count(&self) -> usize {
        self.assignments.len()
    }

    /// Returns the control method
    pub fn control_method(&self) -> ExtractionMethod {
        self.control_method
    }

    /// Returns the treatment method
    pub fn treatment_method(&self) -> ExtractionMethod {
        self.treatment_method
    }

    /// Returns the split ratio
    pub fn split_ratio(&self) -> f64 {
        self.split_ratio
    }
}

impl ABTestStats {
    /// Calculates the average processing time for control group
    ///
    /// # Returns
    ///
    /// Returns average time in milliseconds
    pub fn control_avg_time_ms(&self) -> f64 {
        if self.control_count > 0 {
            self.control_time_ms as f64 / self.control_count as f64
        } else {
            0.0
        }
    }

    /// Calculates the average processing time for treatment group
    ///
    /// # Returns
    ///
    /// Returns average time in milliseconds
    pub fn treatment_avg_time_ms(&self) -> f64 {
        if self.treatment_count > 0 {
            self.treatment_time_ms as f64 / self.treatment_count as f64
        } else {
            0.0
        }
    }

    /// Calculates the average keywords per document for control group
    ///
    /// # Returns
    ///
    /// Returns average keyword count
    pub fn control_avg_keywords(&self) -> f64 {
        if self.control_count > 0 {
            self.control_keywords_count as f64 / self.control_count as f64
        } else {
            0.0
        }
    }

    /// Calculates the average keywords per document for treatment group
    ///
    /// # Returns
    ///
    /// Returns average keyword count
    pub fn treatment_avg_keywords(&self) -> f64 {
        if self.treatment_count > 0 {
            self.treatment_keywords_count as f64 / self.treatment_count as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ab_test_creation() {
        let ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        assert_eq!(ab_test.control_method(), ExtractionMethod::Frequency);
        assert_eq!(ab_test.treatment_method(), ExtractionMethod::Llm);
        assert_eq!(ab_test.split_ratio(), 0.5);
        assert_eq!(ab_test.assignment_count(), 0);
    }

    #[test]
    fn test_deterministic_assignment() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        let path = "test_document.md";
        let method1 = ab_test.assign_group(path);
        let method2 = ab_test.assign_group(path);

        // Same path should get same assignment
        assert_eq!(method1, method2);
        assert_eq!(ab_test.assignment_count(), 1);
    }

    #[test]
    fn test_split_ratio_all_control() {
        let mut ab_test = ABTest::new(
            ExtractionMethod::Frequency,
            ExtractionMethod::Llm,
            0.0, // All control
        );

        for i in 0..100 {
            let path = format!("doc_{}.md", i);
            let method = ab_test.assign_group(&path);
            assert_eq!(method, ExtractionMethod::Frequency);
        }
    }

    #[test]
    fn test_split_ratio_all_treatment() {
        let mut ab_test = ABTest::new(
            ExtractionMethod::Frequency,
            ExtractionMethod::Llm,
            1.0, // All treatment
        );

        for i in 0..100 {
            let path = format!("doc_{}.md", i);
            let method = ab_test.assign_group(&path);
            assert_eq!(method, ExtractionMethod::Llm);
        }
    }

    #[test]
    fn test_split_ratio_approximate() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        let mut treatment_count = 0;
        let total = 1000;

        for i in 0..total {
            let path = format!("doc_{}.md", i);
            let method = ab_test.assign_group(&path);
            if method == ExtractionMethod::Llm {
                treatment_count += 1;
            }
        }

        // Should be approximately 50% (allow 10% variance)
        let ratio = treatment_count as f64 / total as f64;
        assert!(ratio > 0.4 && ratio < 0.6);
    }

    #[test]
    fn test_record_metrics() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        let path1 = "doc1.md";
        let path2 = "doc2.md";

        let method1 = ab_test.assign_group(path1);
        let method2 = ab_test.assign_group(path2);

        ab_test.record_metrics(path1, 100, 10);
        ab_test.record_metrics(path2, 200, 15);

        let stats = ab_test.stats();

        // Check that metrics were recorded
        match method1 {
            ExtractionMethod::Frequency => {
                assert_eq!(stats.control_count, 1);
                assert_eq!(stats.control_time_ms, 100);
                assert_eq!(stats.control_keywords_count, 10);
            }
            ExtractionMethod::Llm => {
                assert_eq!(stats.treatment_count, 1);
                assert_eq!(stats.treatment_time_ms, 100);
                assert_eq!(stats.treatment_keywords_count, 10);
            }
        }
    }

    #[test]
    fn test_stats_averages() {
        let mut stats = ABTestStats::default();
        stats.control_count = 2;
        stats.control_time_ms = 300;
        stats.control_keywords_count = 20;

        stats.treatment_count = 3;
        stats.treatment_time_ms = 600;
        stats.treatment_keywords_count = 45;

        assert_eq!(stats.control_avg_time_ms(), 150.0);
        assert_eq!(stats.control_avg_keywords(), 10.0);
        assert_eq!(stats.treatment_avg_time_ms(), 200.0);
        assert_eq!(stats.treatment_avg_keywords(), 15.0);
    }

    #[test]
    fn test_to_json() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        ab_test.assign_group("doc1.md");
        ab_test.assign_group("doc2.md");

        let json = ab_test.to_json();
        assert!(json.is_object());
        assert!(json["config"].is_object());
        assert!(json["assignments"].is_object());
        assert!(json["statistics"].is_object());
    }

    #[test]
    fn test_reset() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        ab_test.assign_group("doc1.md");
        ab_test.assign_group("doc2.md");
        ab_test.record_metrics("doc1.md", 100, 10);

        assert_eq!(ab_test.assignment_count(), 2);

        ab_test.reset();

        assert_eq!(ab_test.assignment_count(), 0);
        let stats = ab_test.stats();
        assert_eq!(stats.control_count, 0);
        assert_eq!(stats.treatment_count, 0);
    }

    #[test]
    fn test_get_assignment() {
        let mut ab_test = ABTest::new(ExtractionMethod::Frequency, ExtractionMethod::Llm, 0.5);

        assert_eq!(ab_test.get_assignment("doc1.md"), None);

        let method = ab_test.assign_group("doc1.md");
        assert_eq!(ab_test.get_assignment("doc1.md"), Some(method));
    }

    #[test]
    fn test_split_ratio_clamp() {
        let ab_test = ABTest::new(
            ExtractionMethod::Frequency,
            ExtractionMethod::Llm,
            1.5, // Should be clamped to 1.0
        );

        assert_eq!(ab_test.split_ratio(), 1.0);

        let ab_test2 = ABTest::new(
            ExtractionMethod::Frequency,
            ExtractionMethod::Llm,
            -0.5, // Should be clamped to 0.0
        );

        assert_eq!(ab_test2.split_ratio(), 0.0);
    }
}
