// SPDX-License-Identifier: MIT OR Apache-2.0
//! Repository scanner for documentation files

use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Scanner errors
#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WalkDir error: {0}")]
    WalkDir(#[from] walkdir::Error),
}

/// Result of a scan
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// List of found markdown files
    pub files: Vec<PathBuf>,
}

/// Scanner for finding documentation files in a repository
pub struct RepositoryScanner {
    root_path: PathBuf,
    ignore_dirs: Vec<String>,
}

impl RepositoryScanner {
    /// Create a new repository scanner
    ///
    /// # Arguments
    ///
    /// * `root_path` - Root directory to scan
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            ignore_dirs: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".idea".to_string(),
                ".vscode".to_string(),
            ],
        }
    }

    /// Add a directory to ignore
    pub fn with_ignore(mut self, dir: &str) -> Self {
        self.ignore_dirs.push(dir.to_string());
        self
    }

    /// Scan the repository for markdown files
    ///
    /// # Returns
    ///
    /// Returns a list of paths to markdown files
    pub async fn scan(&self) -> Result<Vec<PathBuf>, ScannerError> {
        let mut files = Vec::new();

        let ignore_dirs = self.ignore_dirs.clone();
        let walker = WalkDir::new(&self.root_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(move |e| {
                let name = e.file_name().to_str().unwrap_or("");
                !ignore_dirs.iter().any(|d| d == name)
            });

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            // Check for markdown files
            if entry.file_type().is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_markdown_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create some files
        File::create(root.join("README.md")).unwrap();
        File::create(root.join("guide.markdown")).unwrap();
        File::create(root.join("script.rs")).unwrap();

        // Create ignored dir
        let git_dir = root.join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        File::create(git_dir.join("config.md")).unwrap(); // Should be ignored

        let scanner = RepositoryScanner::new(root);
        let files = scanner.scan().await.unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.file_name().unwrap() == "README.md"));
        assert!(files
            .iter()
            .any(|p| p.file_name().unwrap() == "guide.markdown"));
        assert!(!files.iter().any(|p| p.file_name().unwrap() == "config.md"));
    }
}
