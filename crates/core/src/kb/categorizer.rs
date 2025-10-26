//! File categorization for incremental loading operations
//!
//! This module categorizes files into different categories based on their
//! state in the knowledge base: Skip (unchanged), Update (modified), Add (new),
//! and Delete (removed from filesystem but still in database).

use crate::kb::error::{KbError, Result};
use crate::kb::hash::calculate_file_hash;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};
use walkdir::WalkDir;

/// File category for incremental loading decisions
///
/// This enum determines what action to take for each file
/// during the loading process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    /// File exists and hash matches - skip processing
    Skip,
    /// File exists but hash differs - needs update
    Update,
    /// File is new - needs to be added
    Add,
    /// File was deleted - needs cleanup
    Delete,
}

/// Categorized files by action needed
///
/// Groups discovered files by the action that needs to be taken.
#[derive(Debug, Default, Clone)]
pub struct CategorizedFiles {
    /// Files to skip (unchanged)
    pub skip: Vec<String>,
    /// Files to update (modified)
    pub update: Vec<String>,
    /// Files to add (new)
    pub add: Vec<String>,
    /// Files to delete (removed)
    pub delete: Vec<String>,
}

impl CategorizedFiles {
    /// Create a new empty categorization
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of files across all categories
    ///
    /// # Returns
    ///
    /// Total count of all files in all categories
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::kb::categorizer::CategorizedFiles;
    ///
    /// let mut files = CategorizedFiles::new();
    /// files.skip.push("file1.txt".to_string());
    /// files.add.push("file2.txt".to_string());
    /// assert_eq!(files.total_files(), 2);
    /// ```
    pub fn total_files(&self) -> usize {
        self.skip.len() + self.update.len() + self.add.len() + self.delete.len()
    }

    /// Get number of files that need processing (not skipped)
    ///
    /// # Returns
    ///
    /// Count of files that need to be added, updated, or deleted
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::kb::categorizer::CategorizedFiles;
    ///
    /// let mut files = CategorizedFiles::new();
    /// files.skip.push("unchanged.txt".to_string());
    /// files.update.push("modified.txt".to_string());
    /// files.add.push("new.txt".to_string());
    /// assert_eq!(files.files_to_process(), 2);
    /// ```
    pub fn files_to_process(&self) -> usize {
        self.update.len() + self.add.len() + self.delete.len()
    }
}

/// File categorizer for determining which files need processing
///
/// This struct compares current filesystem state with database state
/// to categorize files appropriately.
pub struct FileCategorizer {
    current_files: HashMap<String, String>,
    existing_files: HashMap<String, String>,
}

impl FileCategorizer {
    /// Discover files in directories and calculate their hashes
    ///
    /// Walks the specified directories recursively, calculating SHA-256 hashes
    /// for all discovered files.
    ///
    /// # Arguments
    ///
    /// * `paths` - Vector of directory or file paths to discover
    ///
    /// # Returns
    ///
    /// Returns a HashMap mapping file paths to their SHA-256 hashes
    ///
    /// # Errors
    ///
    /// Returns `KbError::InvalidPath` if a path does not exist
    /// Returns `KbError::Traversal` if directory walking fails
    /// Returns `KbError::HashCalculation` if hash calculation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xze_core::kb::categorizer::FileCategorizer;
    ///
    /// # async fn example() -> xze_core::kb::error::Result<()> {
    /// let paths = vec!["./docs".to_string()];
    /// let files = FileCategorizer::discover_files_with_hashes(&paths).await?;
    /// println!("Discovered {} files", files.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_files_with_hashes(paths: &[String]) -> Result<HashMap<String, String>> {
        let mut discovered_files = HashMap::new();

        for path_str in paths {
            let path = Path::new(path_str);
            if !path.exists() {
                return Err(KbError::invalid_path(path_str, "path does not exist"));
            }

            if path.is_file() {
                let hash = calculate_file_hash(path).await?;
                discovered_files.insert(path_str.clone(), hash);
            } else if path.is_dir() {
                for entry in WalkDir::new(path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let file_path = entry.path();
                        let path_string = file_path
                            .to_str()
                            .ok_or_else(|| {
                                KbError::invalid_path(
                                    file_path.display().to_string(),
                                    "path contains invalid UTF-8",
                                )
                            })?
                            .to_string();

                        match calculate_file_hash(file_path).await {
                            Ok(hash) => {
                                discovered_files.insert(path_string.clone(), hash);
                                debug!("Discovered file: {} with hash", path_string);
                            }
                            Err(e) => {
                                debug!("Skipping file {} due to error: {}", path_string, e);
                            }
                        }
                    }
                }
            }
        }

        info!("Discovered {} files in total", discovered_files.len());
        Ok(discovered_files)
    }

    /// Create a new file categorizer
    ///
    /// # Arguments
    ///
    /// * `current_files` - HashMap of current files on disk (path -> hash)
    /// * `existing_files` - HashMap of files in database (path -> hash)
    ///
    /// # Returns
    ///
    /// Returns a new FileCategorizer instance
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use xze_core::kb::categorizer::FileCategorizer;
    ///
    /// let current = HashMap::new();
    /// let existing = HashMap::new();
    /// let categorizer = FileCategorizer::new(current, existing);
    /// ```
    pub fn new(
        current_files: HashMap<String, String>,
        existing_files: HashMap<String, String>,
    ) -> Self {
        Self {
            current_files,
            existing_files,
        }
    }

    /// Categorize files based on current state and database state
    ///
    /// Compares current filesystem state with database state to determine
    /// which files need to be added, updated, skipped, or deleted.
    ///
    /// # Returns
    ///
    /// Returns a `CategorizedFiles` struct with files grouped by action
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use xze_core::kb::categorizer::FileCategorizer;
    ///
    /// let mut current = HashMap::new();
    /// current.insert("file1.txt".to_string(), "hash1".to_string());
    ///
    /// let existing = HashMap::new();
    ///
    /// let categorizer = FileCategorizer::new(current, existing);
    /// let categorized = categorizer.categorize();
    ///
    /// assert_eq!(categorized.add.len(), 1);
    /// ```
    pub fn categorize(&self) -> CategorizedFiles {
        let mut result = CategorizedFiles::new();

        let current_paths: HashSet<_> = self.current_files.keys().collect();
        let existing_paths: HashSet<_> = self.existing_files.keys().collect();

        // Process current files
        for (path, current_hash) in &self.current_files {
            match self.existing_files.get(path) {
                Some(existing_hash) => {
                    if current_hash == existing_hash {
                        // File unchanged - skip
                        result.skip.push(path.clone());
                    } else {
                        // File modified - update
                        result.update.push(path.clone());
                    }
                }
                None => {
                    // File is new - add
                    result.add.push(path.clone());
                }
            }
        }

        // Find deleted files (in database but not on disk)
        for path in existing_paths {
            if !current_paths.contains(path) {
                result.delete.push(path.clone());
            }
        }

        result
    }

    /// Log a summary of categorization results
    ///
    /// Logs the counts of files in each category at info level.
    ///
    /// # Arguments
    ///
    /// * `categorized` - The categorized files to summarize
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::kb::categorizer::{FileCategorizer, CategorizedFiles};
    ///
    /// let categorized = CategorizedFiles::new();
    /// FileCategorizer::log_summary(&categorized);
    /// ```
    pub fn log_summary(categorized: &CategorizedFiles) {
        info!("File categorization summary:");
        info!("  Skip (unchanged): {}", categorized.skip.len());
        info!("  Update (modified): {}", categorized.update.len());
        info!("  Add (new): {}", categorized.add.len());
        info!("  Delete (removed): {}", categorized.delete.len());
        info!("  Total files: {}", categorized.total_files());
        info!("  Files to process: {}", categorized.files_to_process());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_categorized_files_new() {
        let categorized = CategorizedFiles::new();
        assert_eq!(categorized.total_files(), 0);
        assert_eq!(categorized.files_to_process(), 0);
        assert!(categorized.skip.is_empty());
        assert!(categorized.update.is_empty());
        assert!(categorized.add.is_empty());
        assert!(categorized.delete.is_empty());
    }

    #[test]
    fn test_categorized_files_counts() {
        let mut categorized = CategorizedFiles::new();
        categorized.skip.push("file1.txt".to_string());
        categorized.skip.push("file2.txt".to_string());
        categorized.update.push("file3.txt".to_string());
        categorized.add.push("file4.txt".to_string());
        categorized.delete.push("file5.txt".to_string());

        assert_eq!(categorized.total_files(), 5);
        assert_eq!(categorized.files_to_process(), 3);
    }

    #[test]
    fn test_file_category_equality() {
        assert_eq!(FileCategory::Skip, FileCategory::Skip);
        assert_eq!(FileCategory::Add, FileCategory::Add);
        assert_eq!(FileCategory::Update, FileCategory::Update);
        assert_eq!(FileCategory::Delete, FileCategory::Delete);

        assert_ne!(FileCategory::Skip, FileCategory::Add);
        assert_ne!(FileCategory::Update, FileCategory::Delete);
    }

    #[test]
    fn test_categorize_new_files() {
        let mut current = HashMap::new();
        current.insert("new_file.txt".to_string(), "hash123".to_string());

        let existing = HashMap::new();

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.add.len(), 1);
        assert_eq!(result.add[0], "new_file.txt");
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.update.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_unchanged_files() {
        let mut current = HashMap::new();
        current.insert("file.txt".to_string(), "hash123".to_string());

        let mut existing = HashMap::new();
        existing.insert("file.txt".to_string(), "hash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.skip.len(), 1);
        assert_eq!(result.skip[0], "file.txt");
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.update.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_modified_files() {
        let mut current = HashMap::new();
        current.insert("file.txt".to_string(), "newhash456".to_string());

        let mut existing = HashMap::new();
        existing.insert("file.txt".to_string(), "oldhash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.update.len(), 1);
        assert_eq!(result.update[0], "file.txt");
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.delete.len(), 0);
    }

    #[test]
    fn test_categorize_deleted_files() {
        let current = HashMap::new();

        let mut existing = HashMap::new();
        existing.insert("deleted_file.txt".to_string(), "hash123".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.delete.len(), 1);
        assert_eq!(result.delete[0], "deleted_file.txt");
        assert_eq!(result.add.len(), 0);
        assert_eq!(result.skip.len(), 0);
        assert_eq!(result.update.len(), 0);
    }

    #[test]
    fn test_categorize_mixed_scenario() {
        let mut current = HashMap::new();
        current.insert("new.txt".to_string(), "hash1".to_string());
        current.insert("unchanged.txt".to_string(), "hash2".to_string());
        current.insert("modified.txt".to_string(), "newhash3".to_string());

        let mut existing = HashMap::new();
        existing.insert("unchanged.txt".to_string(), "hash2".to_string());
        existing.insert("modified.txt".to_string(), "oldhash3".to_string());
        existing.insert("deleted.txt".to_string(), "hash4".to_string());

        let categorizer = FileCategorizer::new(current, existing);
        let result = categorizer.categorize();

        assert_eq!(result.add.len(), 1);
        assert!(result.add.contains(&"new.txt".to_string()));

        assert_eq!(result.skip.len(), 1);
        assert!(result.skip.contains(&"unchanged.txt".to_string()));

        assert_eq!(result.update.len(), 1);
        assert!(result.update.contains(&"modified.txt".to_string()));

        assert_eq!(result.delete.len(), 1);
        assert!(result.delete.contains(&"deleted.txt".to_string()));

        assert_eq!(result.total_files(), 4);
        assert_eq!(result.files_to_process(), 3);
    }

    #[tokio::test]
    async fn test_discover_files_invalid_path() {
        let paths = vec!["/nonexistent/path/that/does/not/exist".to_string()];
        let result = FileCategorizer::discover_files_with_hashes(&paths).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KbError::InvalidPath { .. }));
    }

    #[test]
    fn test_log_summary_does_not_panic() {
        let mut categorized = CategorizedFiles::new();
        categorized.skip.push("file1.txt".to_string());
        categorized.add.push("file2.txt".to_string());

        // Should not panic
        FileCategorizer::log_summary(&categorized);
    }
}
