// SPDX-License-Identifier: MIT OR Apache-2.0
//! Plan executor for applying reorganization changes

use super::planner::ReorgPlan;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;

/// Execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Source file not found: {0}")]
    SourceNotFound(String),

    #[error("Destination already exists: {0}")]
    DestinationExists(String),
}

/// Result of a single file move execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveResult {
    /// Original path
    pub original_path: PathBuf,
    /// New path
    pub new_path: PathBuf,
    /// Whether the move was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Executor for reorganization plans
pub struct ReorgExecutor {
    dry_run: bool,
}

impl ReorgExecutor {
    /// Create a new executor
    ///
    /// # Arguments
    ///
    /// * `dry_run` - If true, only simulate moves
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Execute a reorganization plan
    ///
    /// # Arguments
    ///
    /// * `plan` - The plan to execute
    ///
    /// # Returns
    ///
    /// Returns a list of results for each move
    pub async fn execute(&self, plan: ReorgPlan) -> Vec<MoveResult> {
        let mut results = Vec::new();

        for file_move in plan.moves {
            let result = self
                .execute_move(&file_move.original_path, &file_move.new_path)
                .await;
            results.push(result);
        }

        results
    }

    async fn execute_move(&self, from: &PathBuf, to: &PathBuf) -> MoveResult {
        // Skip if paths are identical
        if from == to {
            return MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: true,
                error: None,
            };
        }

        // Check source existence
        if !from.exists() {
            return MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: false,
                error: Some(format!("Source file not found: {}", from.display())),
            };
        }

        // Check destination existence (to avoid overwrite)
        if to.exists() {
            return MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: false,
                error: Some(format!("Destination file already exists: {}", to.display())),
            };
        }

        if self.dry_run {
            return MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: true,
                error: None,
            };
        }

        // Create parent directories
        if let Some(parent) = to.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return MoveResult {
                    original_path: from.clone(),
                    new_path: to.clone(),
                    success: false,
                    error: Some(format!("Failed to create directory: {}", e)),
                };
            }
        }

        // Move file
        match fs::rename(from, to).await {
            Ok(_) => MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: true,
                error: None,
            },
            Err(e) => MoveResult {
                original_path: from.clone(),
                new_path: to.clone(),
                success: false,
                error: Some(format!("Failed to move file: {}", e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reorg::planner::FileMove;
    use std::fs::File;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_execute_move() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let src = root.join("old.md");
        let dst = root.join("new/folder/new.md");

        File::create(&src).unwrap();

        let plan = ReorgPlan {
            moves: vec![FileMove {
                original_path: src.clone(),
                new_path: dst.clone(),
                reason: "test".to_string(),
            }],
        };

        let executor = ReorgExecutor::new(false);
        let results = executor.execute(plan).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(!src.exists());
        assert!(dst.exists());
    }

    #[tokio::test]
    async fn test_execute_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let src = root.join("old.md");
        let dst = root.join("new.md");

        File::create(&src).unwrap();

        let plan = ReorgPlan {
            moves: vec![FileMove {
                original_path: src.clone(),
                new_path: dst.clone(),
                reason: "test".to_string(),
            }],
        };

        let executor = ReorgExecutor::new(true);
        let results = executor.execute(plan).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(src.exists()); // Should still exist
        assert!(!dst.exists()); // Should not exist
    }
}
