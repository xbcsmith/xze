// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reorganization planner using LLM

use crate::ai::providers::{Message, Provider};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;

/// Planner errors
#[derive(Error, Debug)]
pub enum PlannerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Provider error: {0}")]
    Provider(#[from] crate::ai::providers::ProviderError),

    #[error("Failed to parse plan: {0}")]
    ParseError(String),
}

/// A proposed file move
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMove {
    /// Original file path
    pub original_path: PathBuf,
    /// Proposed new path
    pub new_path: PathBuf,
    /// Reason for the move (Diataxis classification)
    pub reason: String,
}

/// A reorganization plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgPlan {
    /// List of file moves
    pub moves: Vec<FileMove>,
}

/// Planner for reorganizing documentation
pub struct ReorgPlanner {
    provider: Arc<dyn Provider>,
}

impl ReorgPlanner {
    /// Create a new planner
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Generate a reorganization plan for the given files
    ///
    /// # Arguments
    ///
    /// * `files` - List of files to analyze
    ///
    /// # Returns
    ///
    /// Returns a proposed reorganization plan
    pub async fn generate_plan(&self, files: Vec<PathBuf>) -> Result<ReorgPlan, PlannerError> {
        if files.is_empty() {
            return Ok(ReorgPlan { moves: Vec::new() });
        }

        // 1. Prepare file summaries
        let mut file_summaries = Vec::new();
        for path in &files {
            let content = match fs::read_to_string(path).await {
                Ok(c) => c,
                Err(_) => continue, // Skip unreadable files
            };

            // Take first 500 chars as preview
            let preview = if content.len() > 500 {
                &content[..500]
            } else {
                &content
            };

            // Escape newlines for prompt safety (basic)
            let preview_clean = preview.replace('\n', " ");

            file_summaries.push(format!(
                "Path: {}\nContent Preview: {}...",
                path.display(),
                preview_clean
            ));
        }

        // 2. Build prompt
        let prompt = self.build_plan_prompt(&file_summaries);
        let messages = vec![Message::user(prompt)];

        // 3. Call AI provider
        let response = self
            .provider
            .complete(&messages, &[], &Default::default())
            .await?;

        // 4. Parse response
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| PlannerError::ParseError("No content in response".to_string()))?;

        let moves = self.parse_plan_response(content)?;

        Ok(ReorgPlan { moves })
    }

    fn build_plan_prompt(&self, summaries: &[String]) -> String {
        format!(
            r#"You are a documentation architect using the Diataxis framework.
Analyze the following files and propose a reorganization plan.

The Diataxis framework categorizes documentation into four types:
1. Tutorials (learning-oriented): "docs/tutorials/"
2. How-To Guides (problem-oriented): "docs/how-to/"
3. Reference (information-oriented): "docs/reference/"
4. Explanation (understanding-oriented): "docs/explanation/"

Files to analyze:
{}

Task:
For each file, determine its best location based on its content.
If a file is already in the correct location, keep it there (new_path = original_path).
If a file is a root README or special file (LICENSE, CONTRIBUTING), keep it in root or appropriate location.

Respond ONLY with a valid JSON array of objects in this format:
[
  {{
    "original_path": "path/to/file.md",
    "new_path": "docs/tutorials/getting-started.md",
    "reason": "It teaches the user how to start, so it is a tutorial."
  }}
]
JSON Response:"#,
            summaries.join("\n\n")
        )
    }

    fn parse_plan_response(&self, response: &str) -> Result<Vec<FileMove>, PlannerError> {
        // Extract JSON
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let moves: Vec<FileMove> = serde_json::from_str(json_str).map_err(|e| {
            PlannerError::ParseError(format!(
                "Failed to parse JSON: {}. Response: {}",
                e, response
            ))
        })?;

        Ok(moves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::providers::{
        Choice, CompletionResponse, Message, ModelConfig, ProviderError, ProviderMetadata,
    };
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[crate::ai::providers::Tool],
            _config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            let content = r#"[
                {
                    "original_path": "old/guide.md",
                    "new_path": "docs/tutorials/guide.md",
                    "reason": "Tutorial content"
                }
            ]"#;

            Ok(CompletionResponse {
                id: "test".to_string(),
                object: "chat.completion".to_string(),
                created: 0,
                model: "test".to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant(content),
                    finish_reason: Some("stop".to_string()),
                }],
                usage: None,
            })
        }

        async fn complete_fast(
            &self,
            messages: &[Message],
            tools: &[crate::ai::providers::Tool],
            config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            self.complete(messages, tools, config).await
        }

        fn get_metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                name: "mock".to_string(),
                supports_streaming: false,
                supports_tools: false,
                supports_vision: false,
                max_tokens: 4096,
            }
        }
    }

    #[tokio::test]
    async fn test_generate_plan() {
        // We need a real file to read, so create one
        use std::fs::File;
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("old/guide.md");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "This is a guide.").unwrap();

        let provider = Arc::new(MockProvider);
        let planner = ReorgPlanner::new(provider);

        // We pass the absolute path, but the mock returns relative paths in JSON.
        // In real usage, we'd probably map them back or use relative paths throughout.
        // For this test, we just check if it parses the mock response.

        // Note: The planner reads the file from disk.
        let plan = planner.generate_plan(vec![file_path]).await.unwrap();

        assert_eq!(plan.moves.len(), 1);
        assert_eq!(plan.moves[0].original_path, PathBuf::from("old/guide.md"));
        assert_eq!(
            plan.moves[0].new_path,
            PathBuf::from("docs/tutorials/guide.md")
        );
    }
}
