//! File system abstractions and utilities for XZe infrastructure

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use xze_core::{Result, XzeError};

/// File system manager for handling file operations
#[derive(Debug, Clone)]
pub struct FileSystemManager {
    base_path: PathBuf,
}

impl FileSystemManager {
    /// Create a new file system manager
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Ensure a directory exists, creating it if necessary
    pub async fn ensure_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let full_path = self.base_path.join(path);
        if !full_path.exists() {
            fs::create_dir_all(&full_path).await.map_err(|e| {
                XzeError::filesystem(format!("Failed to create directory {:?}: {}", full_path, e))
            })?;
        }
        Ok(())
    }

    /// Read a file as string
    pub async fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let full_path = self.base_path.join(path);
        fs::read_to_string(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to read file {:?}: {}", full_path, e))
        })
    }

    /// Write string content to a file
    pub async fn write_string<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<()> {
        let full_path = self.base_path.join(path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to create parent directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        fs::write(&full_path, content).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to write file {:?}: {}", full_path, e))
        })
    }

    /// Read bytes from a file
    pub async fn read_bytes<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let full_path = self.base_path.join(path);
        fs::read(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!(
                "Failed to read bytes from file {:?}: {}",
                full_path, e
            ))
        })
    }

    /// Write bytes to a file
    pub async fn write_bytes<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> Result<()> {
        let full_path = self.base_path.join(path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to create parent directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        fs::write(&full_path, content).await.map_err(|e| {
            XzeError::filesystem(format!(
                "Failed to write bytes to file {:?}: {}",
                full_path, e
            ))
        })
    }

    /// Check if a file exists
    pub async fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        let full_path = self.base_path.join(path);
        full_path.exists()
    }

    /// Delete a file
    pub async fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let full_path = self.base_path.join(path);
        fs::remove_file(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to remove file {:?}: {}", full_path, e))
        })
    }

    /// Delete a directory and all its contents
    pub async fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let full_path = self.base_path.join(path);
        fs::remove_dir_all(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to remove directory {:?}: {}", full_path, e))
        })
    }

    /// List files in a directory
    pub async fn list_files<P: AsRef<Path>>(&self, path: P) -> Result<Vec<PathBuf>> {
        let full_path = self.base_path.join(path);
        let mut entries = fs::read_dir(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to read directory {:?}: {}", full_path, e))
        })?;

        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            XzeError::filesystem(format!(
                "Failed to read directory entry in {:?}: {}",
                full_path, e
            ))
        })? {
            if entry
                .file_type()
                .await
                .map_err(|e| {
                    XzeError::filesystem(format!(
                        "Failed to get file type for {:?}: {}",
                        entry.path(),
                        e
                    ))
                })?
                .is_file()
            {
                files.push(entry.path());
            }
        }

        Ok(files)
    }

    /// Copy a file from source to destination
    pub async fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> Result<()> {
        let from_path = self.base_path.join(from);
        let to_path = self.base_path.join(to);

        // Ensure parent directory exists for destination
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to create parent directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        fs::copy(&from_path, &to_path).await.map_err(|e| {
            XzeError::filesystem(format!(
                "Failed to copy file from {:?} to {:?}: {}",
                from_path, to_path, e
            ))
        })?;

        Ok(())
    }

    /// Get file metadata
    pub async fn metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata> {
        let full_path = self.base_path.join(path);
        let metadata = fs::metadata(&full_path).await.map_err(|e| {
            XzeError::filesystem(format!("Failed to get metadata for {:?}: {}", full_path, e))
        })?;

        Ok(FileMetadata {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            }),
        })
    }
}

/// File metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: Option<u64>, // Unix timestamp
}

/// File system utilities
pub struct FileSystemUtils;

impl FileSystemUtils {
    /// Calculate the size of a directory recursively
    pub async fn directory_size<P: AsRef<Path>>(path: P) -> Result<u64> {
        let mut total_size = 0u64;
        let mut stack = vec![path.as_ref().to_path_buf()];

        while let Some(current_path) = stack.pop() {
            let mut entries = fs::read_dir(&current_path).await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to read directory {:?}: {}",
                    current_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to read directory entry in {:?}: {}",
                    current_path, e
                ))
            })? {
                let file_type = entry.file_type().await.map_err(|e| {
                    XzeError::filesystem(format!(
                        "Failed to get file type for {:?}: {}",
                        entry.path(),
                        e
                    ))
                })?;

                if file_type.is_file() {
                    let metadata = entry.metadata().await.map_err(|e| {
                        XzeError::filesystem(format!(
                            "Failed to get metadata for {:?}: {}",
                            entry.path(),
                            e
                        ))
                    })?;
                    total_size += metadata.len();
                } else if file_type.is_dir() {
                    stack.push(entry.path());
                }
            }
        }

        Ok(total_size)
    }

    /// Find files matching a pattern in a directory
    pub async fn find_files<P: AsRef<Path>>(
        path: P,
        pattern: &str,
        recursive: bool,
    ) -> Result<Vec<PathBuf>> {
        let mut found_files = Vec::new();
        let mut stack = vec![path.as_ref().to_path_buf()];

        while let Some(current_path) = stack.pop() {
            let mut entries = fs::read_dir(&current_path).await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to read directory {:?}: {}",
                    current_path, e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                XzeError::filesystem(format!(
                    "Failed to read directory entry in {:?}: {}",
                    current_path, e
                ))
            })? {
                let file_type = entry.file_type().await.map_err(|e| {
                    XzeError::filesystem(format!(
                        "Failed to get file type for {:?}: {}",
                        entry.path(),
                        e
                    ))
                })?;

                if file_type.is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.contains(pattern) {
                            found_files.push(entry.path());
                        }
                    }
                } else if file_type.is_dir() && recursive {
                    stack.push(entry.path());
                }
            }
        }

        Ok(found_files)
    }

    /// Clean up empty directories
    pub async fn cleanup_empty_dirs<P: AsRef<Path>>(path: P) -> Result<usize> {
        let mut cleaned_count = 0;
        let mut stack = vec![path.as_ref().to_path_buf()];

        while let Some(current_path) = stack.pop() {
            let mut entries = match fs::read_dir(&current_path).await {
                Ok(entries) => entries,
                Err(_) => continue, // Skip if we can't read the directory
            };

            let mut has_files = false;
            let mut subdirs = Vec::new();

            while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    has_files = true;
                    break;
                } else if file_type.is_dir() {
                    subdirs.push(entry.path());
                }
            }

            // Process subdirectories first
            for subdir in subdirs {
                stack.push(subdir);
            }

            // If no files and no subdirectories, remove the directory
            if !has_files {
                let is_empty = match fs::read_dir(&current_path).await {
                    Ok(mut entries) => matches!(entries.next_entry().await, Ok(None)),
                    Err(_) => false,
                };

                if is_empty && fs::remove_dir(&current_path).await.is_ok() {
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_filesystem_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());
        assert_eq!(fs_manager.base_path(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_ensure_dir() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());

        let test_dir = "test/nested/directory";
        assert!(fs_manager.ensure_dir(test_dir).await.is_ok());
        assert!(temp_dir.path().join(test_dir).exists());
    }

    #[tokio::test]
    async fn test_read_write_string() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());

        let test_file = "test.txt";
        let test_content = "Hello, world!";

        assert!(fs_manager
            .write_string(test_file, test_content)
            .await
            .is_ok());

        let read_content = fs_manager.read_to_string(test_file).await.unwrap();
        assert_eq!(read_content, test_content);
    }

    #[tokio::test]
    async fn test_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());

        let test_file = "exists_test.txt";
        assert!(!fs_manager.exists(test_file).await);

        fs_manager.write_string(test_file, "content").await.unwrap();
        assert!(fs_manager.exists(test_file).await);
    }

    #[tokio::test]
    async fn test_copy_file() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());

        let source = "source.txt";
        let dest = "dest.txt";
        let content = "test content";

        fs_manager.write_string(source, content).await.unwrap();
        assert!(fs_manager.copy_file(source, dest).await.is_ok());

        let copied_content = fs_manager.read_to_string(dest).await.unwrap();
        assert_eq!(copied_content, content);
    }

    #[tokio::test]
    async fn test_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let fs_manager = FileSystemManager::new(temp_dir.path());

        let test_file = "metadata_test.txt";
        let content = "test content for metadata";

        fs_manager.write_string(test_file, content).await.unwrap();

        let metadata = fs_manager.metadata(test_file).await.unwrap();
        assert!(metadata.is_file);
        assert!(!metadata.is_dir);
        assert_eq!(metadata.size, content.len() as u64);
    }
}
