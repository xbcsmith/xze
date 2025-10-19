//! GitLab merge request management
//!
//! This module provides GitLab-specific implementation of the PullRequestManager trait.
//! GitLab uses "merge requests" (MRs) instead of pull requests, but we use the same
//! abstraction for consistency across platforms.

use crate::{Result, XzeError};

use super::pr::{
    Author, CreatePrRequest, MergeMethod, PrState, PrUpdate, PullRequest, PullRequestManager,
};

/// GitLab merge request manager implementation
#[derive(Debug, Clone)]
pub struct GitLabPrManager {
    client: reqwest::Client,
    token: String,
    base_url: String,
}

impl GitLabPrManager {
    /// Create a new GitLab MR manager with default GitLab.com URL
    pub fn new(token: String) -> Self {
        Self::new_with_url(token, "https://gitlab.com".to_string())
    }

    /// Create a new GitLab MR manager with custom GitLab instance URL
    pub fn new_with_url(token: String, base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("xze-bot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            token,
            base_url,
        }
    }

    /// Extract project ID from GitLab URL
    /// Returns the URL-encoded project path (owner/repo)
    fn parse_gitlab_url(&self, repo_url: &str) -> Result<String> {
        // Handle various GitLab URL formats
        let path = if let Some(stripped) = repo_url.strip_prefix("https://gitlab.com/") {
            stripped
        } else if let Some(stripped) = repo_url.strip_prefix("git@gitlab.com:") {
            stripped
        } else if let Some(stripped) = repo_url.strip_prefix(&format!("{}/", self.base_url)) {
            stripped
        } else if let Some(stripped) =
            repo_url.strip_prefix(&format!("git@{}:", self.base_url.replace("https://", "")))
        {
            stripped
        } else {
            return Err(XzeError::validation("Invalid GitLab URL format"));
        };

        // Remove .git suffix if present
        let path = path.strip_suffix(".git").unwrap_or(path);

        // URL encode the project path (replace / with %2F)
        Ok(urlencoding::encode(path).to_string())
    }

    /// Build GitLab API URL
    fn api_url(&self, project_id: &str, endpoint: &str) -> String {
        format!(
            "{}/api/v4/projects/{}/{}",
            self.base_url, project_id, endpoint
        )
    }

    /// Parse GitLab API MR response into our PR struct
    fn parse_gitlab_mr(&self, data: &serde_json::Value) -> Result<PullRequest> {
        let number = data["iid"]
            .as_u64()
            .ok_or_else(|| XzeError::validation("Missing MR iid"))?;

        let title = data["title"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing MR title"))?
            .to_string();

        let body = data["description"].as_str().unwrap_or("").to_string();

        let head_branch = data["source_branch"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing source branch"))?
            .to_string();

        let base_branch = data["target_branch"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing target branch"))?
            .to_string();

        let state_str = data["state"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing MR state"))?;

        let merge_status = data["merge_status"].as_str().unwrap_or("");
        let is_draft = data["draft"].as_bool().unwrap_or(false)
            || data["work_in_progress"].as_bool().unwrap_or(false);

        let state = match (state_str, merge_status, is_draft) {
            ("merged", _, _) => PrState::Merged,
            ("closed", _, _) => PrState::Closed,
            ("opened", _, true) => PrState::Draft,
            ("opened", _, false) => PrState::Open,
            _ => PrState::Open,
        };

        let author = Author {
            username: data["author"]["username"]
                .as_str()
                .ok_or_else(|| XzeError::validation("Missing author username"))?
                .to_string(),
            name: data["author"]["name"].as_str().map(|s| s.to_string()),
            email: None, // GitLab API doesn't return email in MR response
        };

        let labels = data["labels"]
            .as_array()
            .map(|labels| {
                labels
                    .iter()
                    .filter_map(|label| label.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let reviewers = data["reviewers"]
            .as_array()
            .map(|reviewers| {
                reviewers
                    .iter()
                    .filter_map(|reviewer| reviewer["username"].as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let url = data["web_url"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing MR URL"))?
            .to_string();

        let created_at = data["created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok_or_else(|| XzeError::validation("Invalid created_at timestamp"))?;

        let updated_at = data["updated_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok_or_else(|| XzeError::validation("Invalid updated_at timestamp"))?;

        Ok(PullRequest {
            number,
            title,
            body,
            head_branch,
            base_branch,
            state,
            author,
            labels,
            reviewers,
            url,
            created_at,
            updated_at,
        })
    }
}

#[allow(async_fn_in_trait)]
impl PullRequestManager for GitLabPrManager {
    async fn create_pr(&self, repo_url: &str, request: CreatePrRequest) -> Result<PullRequest> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, "merge_requests");

        let mut gitlab_request = serde_json::json!({
            "source_branch": request.head,
            "target_branch": request.base,
            "title": request.title,
            "description": request.body,
        });

        // GitLab uses title prefix for draft MRs
        if request.draft {
            gitlab_request["title"] =
                serde_json::Value::String(format!("Draft: {}", request.title));
        }

        // Add labels if provided
        if !request.labels.is_empty() {
            gitlab_request["labels"] = serde_json::json!(request.labels.join(","));
        }

        // Add assignees if provided
        if !request.assignees.is_empty() {
            // GitLab uses assignee_ids, but we can use assignee_id for a single assignee
            // For simplicity, we'll just assign the first one
            gitlab_request["assignee_id"] = serde_json::json!(request.assignees[0]);
        }

        // Add reviewer IDs if provided
        if !request.reviewers.is_empty() {
            gitlab_request["reviewer_ids"] = serde_json::json!(request.reviewers);
        }

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&gitlab_request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to create MR: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!("GitLab API error: {}", error_text)));
        }

        let mr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse MR response: {}", e)))?;

        self.parse_gitlab_mr(&mr_data)
    }

    async fn get_pr(&self, repo_url: &str, pr_number: u64) -> Result<PullRequest> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, &format!("merge_requests/{}", pr_number));

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to get MR: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::not_found(format!("MR !{} not found", pr_number)));
        }

        let mr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse MR response: {}", e)))?;

        self.parse_gitlab_mr(&mr_data)
    }

    async fn list_prs(&self, repo_url: &str, state: Option<PrState>) -> Result<Vec<PullRequest>> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let mut url = self.api_url(&project_id, "merge_requests");

        if let Some(state) = state {
            let state_param = match state {
                PrState::Open | PrState::Draft => "opened",
                PrState::Closed => "closed",
                PrState::Merged => "merged",
            };
            url.push_str(&format!("?state={}", state_param));
        }

        let response = self
            .client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to list MRs: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to list merge requests"));
        }

        let mrs_data: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse MRs response: {}", e)))?;

        let mut prs = Vec::new();
        for mr_data in mrs_data {
            if let Ok(pr) = self.parse_gitlab_mr(&mr_data) {
                prs.push(pr);
            }
        }

        Ok(prs)
    }

    async fn update_pr(
        &self,
        repo_url: &str,
        pr_number: u64,
        updates: PrUpdate,
    ) -> Result<PullRequest> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, &format!("merge_requests/{}", pr_number));

        let mut update_data = serde_json::Map::new();

        if let Some(title) = updates.title {
            update_data.insert("title".to_string(), serde_json::Value::String(title));
        }

        if let Some(body) = updates.body {
            update_data.insert("description".to_string(), serde_json::Value::String(body));
        }

        if let Some(state) = updates.state {
            let state_event = match state {
                PrState::Open => "reopen",
                PrState::Closed => "close",
                _ => return Err(XzeError::validation("Invalid state for update")),
            };
            update_data.insert(
                "state_event".to_string(),
                serde_json::Value::String(state_event.to_string()),
            );
        }

        if let Some(labels) = updates.labels {
            update_data.insert(
                "labels".to_string(),
                serde_json::Value::String(labels.join(",")),
            );
        }

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&update_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to update MR: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!(
                "Failed to update merge request: {}",
                error_text
            )));
        }

        let mr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse MR response: {}", e)))?;

        self.parse_gitlab_mr(&mr_data)
    }

    async fn close_pr(&self, repo_url: &str, pr_number: u64) -> Result<()> {
        let updates = PrUpdate {
            state: Some(PrState::Closed),
            ..Default::default()
        };

        self.update_pr(repo_url, pr_number, updates).await?;
        Ok(())
    }

    async fn merge_pr(
        &self,
        repo_url: &str,
        pr_number: u64,
        merge_method: MergeMethod,
    ) -> Result<()> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, &format!("merge_requests/{}/merge", pr_number));

        let merge_commit_message = match merge_method {
            MergeMethod::Merge => None,
            MergeMethod::Squash => Some("merge"),
            MergeMethod::Rebase => Some("merge"),
        };

        let mut merge_data = serde_json::Map::new();

        if let Some(msg) = merge_commit_message {
            merge_data.insert(
                "merge_commit_message".to_string(),
                serde_json::Value::String(msg.to_string()),
            );
        }

        // Set squash flag for squash merges
        if merge_method == MergeMethod::Squash {
            merge_data.insert("squash".to_string(), serde_json::Value::Bool(true));
        }

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&merge_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to merge MR: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!(
                "Failed to merge merge request: {}",
                error_text
            )));
        }

        Ok(())
    }

    async fn add_comment(&self, repo_url: &str, pr_number: u64, comment: &str) -> Result<()> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, &format!("merge_requests/{}/notes", pr_number));

        let comment_data = serde_json::json!({
            "body": comment
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&comment_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to add note: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!(
                "Failed to add note to merge request: {}",
                error_text
            )));
        }

        Ok(())
    }

    async fn request_review(
        &self,
        repo_url: &str,
        pr_number: u64,
        reviewers: Vec<String>,
    ) -> Result<()> {
        let project_id = self.parse_gitlab_url(repo_url)?;
        let url = self.api_url(&project_id, &format!("merge_requests/{}", pr_number));

        // GitLab requires reviewer IDs, not usernames
        // In a real implementation, you'd need to look up user IDs by username
        // For now, we'll try to use the reviewer strings as IDs directly
        let review_data = serde_json::json!({
            "reviewer_ids": reviewers
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&review_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to request review: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!(
                "Failed to request review: {}",
                error_text
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_url_parsing() {
        let manager = GitLabPrManager::new("fake-token".to_string());

        let project_id = manager
            .parse_gitlab_url("https://gitlab.com/owner/repo")
            .unwrap();
        assert_eq!(project_id, "owner%2Frepo");

        let project_id = manager
            .parse_gitlab_url("git@gitlab.com:owner/repo.git")
            .unwrap();
        assert_eq!(project_id, "owner%2Frepo");

        assert!(manager.parse_gitlab_url("invalid-url").is_err());
    }

    #[test]
    fn test_api_url_building() {
        let manager = GitLabPrManager::new("fake-token".to_string());
        let url = manager.api_url("owner%2Frepo", "merge_requests");
        assert_eq!(
            url,
            "https://gitlab.com/api/v4/projects/owner%2Frepo/merge_requests"
        );
    }

    #[test]
    fn test_custom_gitlab_instance() {
        let manager = GitLabPrManager::new_with_url(
            "fake-token".to_string(),
            "https://gitlab.example.com".to_string(),
        );

        let project_id = manager
            .parse_gitlab_url("https://gitlab.example.com/owner/repo")
            .unwrap();
        assert_eq!(project_id, "owner%2Frepo");

        let url = manager.api_url("owner%2Frepo", "merge_requests");
        assert_eq!(
            url,
            "https://gitlab.example.com/api/v4/projects/owner%2Frepo/merge_requests"
        );
    }
}
