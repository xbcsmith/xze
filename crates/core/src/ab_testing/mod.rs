//! A/B testing framework for keyword extraction methods
//!
//! This module provides functionality to split documents into control and treatment
//! groups for comparing different keyword extraction methods. It uses consistent
//! hashing to ensure the same document always gets assigned to the same group.
//!
//! # Examples
//!
//! ```
//! use xze_core::ab_testing::{ABTest, ExtractionGroup};
//!
//! # fn example() -> xze_core::Result<()> {
//! let test = ABTest::new(50.0); // 50% split
//!
//! let group = test.assign_group("path/to/document.md");
//! match group {
//!     ExtractionGroup::Control => println!("Use frequency extraction"),
//!     ExtractionGroup::Treatment => println!("Use LLM extraction"),
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Group assignment for A/B testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionGroup {
    /// Control group (baseline method)
    Control,
    /// Treatment group (experimental method)
    Treatment,
}

impl std::fmt::Display for ExtractionGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Control => write!(f, "control"),
            Self::Treatment => write!(f, "treatment"),
        }
    }
}

/// A/B test configuration for keyword extraction
#[derive(Debug, Clone)]
pub struct ABTest {
    /// Percentage of documents assigned to treatment group (0.0-100.0)
    treatment_percentage: f64,
    /// Cached group assignments
    assignments: HashMap<String, ExtractionGroup>,
}

impl ABTest {
    /// Create a new A/B test with the specified treatment percentage
    ///
    /// # Arguments
    ///
    /// * `treatment_percentage` - Percentage of documents in treatment group (0.0-100.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::ABTest;
    ///
    /// let test = ABTest::new(50.0); // 50/50 split
    /// ```
    pub fn new(treatment_percentage: f64) -> Self {
        let treatment_percentage = treatment_percentage.clamp(0.0, 100.0);
        Self {
            treatment_percentage,
            assignments: HashMap::new(),
        }
    }

    /// Assign a document to control or treatment group
    ///
    /// Uses consistent hashing based on the document path to ensure
    /// the same document always gets assigned to the same group.
    ///
    /// # Arguments
    ///
    /// * `document_path` - Path to the document
    ///
    /// # Returns
    ///
    /// Returns the group assignment for this document
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::ab_testing::{ABTest, ExtractionGroup};
    ///
    /// let test = ABTest::new(50.0);
    /// let group = test.assign_group("docs/guide.md");
    /// assert!(matches!(group, ExtractionGroup::Control | ExtractionGroup::Treatment));
    /// ```
    pub fn assign_group(&mut self, document_path: &str) -> ExtractionGroup {
        // Return cached assignment if exists
        if let Some(&group) = self.assignments.get(document_path) {
            return group;
        }

        // Calculate hash for consistent assignment
        let mut hasher = DefaultHasher::new();
        document_path.hash(&mut hasher);
        let hash = hasher.finish();

        // Assign based on hash modulo
        let in_treatment = (hash % 100) < self.treatment_percentage as u64;
        let group = if in_treatment {
            ExtractionGroup::Treatment
        } else {
            ExtractionGroup::Control
        };

        // Cache the assignment
        self.assignments.insert(document_path.to_string(), group);

        group
    }

    /// Get the treatment percentage
    pub fn treatment_percentage(&self) -> f64 {
        self.treatment_percentage
    }

    /// Get the number of documents assigned to each group
    ///
    /// # Returns
    ///
    /// Returns (control_count, treatment_count)
    pub fn group_counts(&self) -> (usize, usize) {
        let control = self
            .assignments
            .values()
            .filter(|&&g| g == ExtractionGroup::Control)
            .count();
        let treatment = self.assignments.len() - control;
        (control, treatment)
    }

    /// Get all assignments
    pub fn assignments(&self) -> &HashMap<String, ExtractionGroup> {
        &self.assignments
    }

    /// Clear all cached assignments
    pub fn clear(&mut self) {
        self.assignments.clear();
    }

    /// Save assignments to JSON file
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path to output file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be written
    pub async fn save_assignments(
        &self,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct AssignmentData {
            treatment_percentage: f64,
            assignments: HashMap<String, String>,
            summary: Summary,
        }

        #[derive(Serialize)]
        struct Summary {
            control_count: usize,
            treatment_count: usize,
            total: usize,
        }

        let (control_count, treatment_count) = self.group_counts();

        let data = AssignmentData {
            treatment_percentage: self.treatment_percentage,
            assignments: self
                .assignments
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
            summary: Summary {
                control_count,
                treatment_count,
                total: self.assignments.len(),
            },
        };

        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(output_path, json).await?;

        Ok(())
    }

    /// Load assignments from JSON file
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to input file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or parsed
    pub async fn load_assignments(input_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct AssignmentData {
            treatment_percentage: f64,
            assignments: HashMap<String, String>,
        }

        let contents = tokio::fs::read_to_string(input_path).await?;
        let data: AssignmentData = serde_json::from_str(&contents)?;

        let assignments: HashMap<String, ExtractionGroup> = data
            .assignments
            .into_iter()
            .filter_map(|(k, v)| {
                let group = match v.as_str() {
                    "control" => ExtractionGroup::Control,
                    "treatment" => ExtractionGroup::Treatment,
                    _ => return None,
                };
                Some((k, group))
            })
            .collect();

        Ok(Self {
            treatment_percentage: data.treatment_percentage,
            assignments,
        })
    }
}

/// A/B test results for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResults {
    /// Test configuration
    pub treatment_percentage: f64,
    /// Control group statistics
    pub control: GroupStats,
    /// Treatment group statistics
    pub treatment: GroupStats,
    /// Statistical comparison
    pub comparison: Comparison,
}

/// Statistics for a test group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupStats {
    /// Number of documents
    pub document_count: usize,
    /// Average extraction time (ms)
    pub avg_extraction_time_ms: f64,
    /// Average keywords per document
    pub avg_keywords_per_doc: f64,
    /// Total errors
    pub error_count: usize,
    /// Error rate (percentage)
    pub error_rate: f64,
}

/// Statistical comparison between groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    /// Difference in extraction time (ms)
    pub time_difference_ms: f64,
    /// Difference in keywords per document
    pub keyword_difference: f64,
    /// Improvement percentage
    pub improvement_pct: f64,
    /// Statistical significance (if calculated)
    pub p_value: Option<f64>,
}

impl ABTestResults {
    /// Create a summary of the A/B test results
    pub fn summary(&self) -> String {
        format!(
            "A/B Test Results:\n\
             Treatment: {:.1}% | Control: {:.1}%\n\
             \n\
             Control Group (n={}):\n\
             - Avg extraction time: {:.2}ms\n\
             - Avg keywords: {:.1}\n\
             - Error rate: {:.1}%\n\
             \n\
             Treatment Group (n={}):\n\
             - Avg extraction time: {:.2}ms\n\
             - Avg keywords: {:.1}\n\
             - Error rate: {:.1}%\n\
             \n\
             Comparison:\n\
             - Time difference: {:.2}ms ({})\n\
             - Keyword difference: {:.2}\n\
             - Overall improvement: {:.1}%",
            self.treatment_percentage,
            100.0 - self.treatment_percentage,
            self.control.document_count,
            self.control.avg_extraction_time_ms,
            self.control.avg_keywords_per_doc,
            self.control.error_rate,
            self.treatment.document_count,
            self.treatment.avg_extraction_time_ms,
            self.treatment.avg_keywords_per_doc,
            self.treatment.error_rate,
            self.comparison.time_difference_ms,
            if self.comparison.time_difference_ms < 0.0 {
                "faster"
            } else {
                "slower"
            },
            self.comparison.keyword_difference,
            self.comparison.improvement_pct,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ab_test_creation() {
        let test = ABTest::new(50.0);
        assert_eq!(test.treatment_percentage(), 50.0);
    }

    #[test]
    fn test_ab_test_clamps_percentage() {
        let test1 = ABTest::new(-10.0);
        assert_eq!(test1.treatment_percentage(), 0.0);

        let test2 = ABTest::new(150.0);
        assert_eq!(test2.treatment_percentage(), 100.0);
    }

    #[test]
    fn test_assign_group_consistent() {
        let mut test = ABTest::new(50.0);

        let path = "test/document.md";
        let group1 = test.assign_group(path);
        let group2 = test.assign_group(path);

        assert_eq!(group1, group2, "Same document should get same group");
    }

    #[test]
    fn test_assign_group_distribution() {
        let mut test = ABTest::new(50.0);

        let mut control_count = 0;
        let mut treatment_count = 0;

        for i in 0..100 {
            let path = format!("test/doc_{}.md", i);
            match test.assign_group(&path) {
                ExtractionGroup::Control => control_count += 1,
                ExtractionGroup::Treatment => treatment_count += 1,
            }
        }

        assert!(control_count > 20, "Control group should have documents");
        assert!(
            treatment_count > 20,
            "Treatment group should have documents"
        );
        assert_eq!(
            control_count + treatment_count,
            100,
            "All documents should be assigned"
        );
    }

    #[test]
    fn test_group_counts() {
        let mut test = ABTest::new(50.0);

        test.assign_group("doc1.md");
        test.assign_group("doc2.md");
        test.assign_group("doc3.md");

        let (control, treatment) = test.group_counts();
        assert_eq!(control + treatment, 3);
    }

    #[test]
    fn test_clear() {
        let mut test = ABTest::new(50.0);

        test.assign_group("doc1.md");
        test.assign_group("doc2.md");

        assert_eq!(test.assignments.len(), 2);

        test.clear();
        assert_eq!(test.assignments.len(), 0);
    }

    #[tokio::test]
    async fn test_save_and_load_assignments() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("assignments.json");

        let mut test1 = ABTest::new(60.0);
        test1.assign_group("doc1.md");
        test1.assign_group("doc2.md");

        test1.save_assignments(&file_path).await.unwrap();

        let test2 = ABTest::load_assignments(&file_path).await.unwrap();

        assert_eq!(test2.treatment_percentage(), 60.0);
        assert_eq!(test2.assignments().len(), 2);
    }

    #[test]
    fn test_extraction_group_display() {
        assert_eq!(format!("{}", ExtractionGroup::Control), "control");
        assert_eq!(format!("{}", ExtractionGroup::Treatment), "treatment");
    }
}
