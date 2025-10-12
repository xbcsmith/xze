//! Git pull request management

use crate::{Result, XzeError};
use serde::{Deserialize, Serialize};

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// PR number/ID
    pub number: u64,
    /// PR title
    pub title: String,
    /// PR description/body
    pub body: String,
    /// Source branch
    pub head_branch: String,
    /// Target branch
    pub base_branch: String,
    /// PR state (open, closed, merged)
    pub state: PrState,
    /// Author information
    pub author: Author,
    /// Labels attached to the PR
    pub labels: Vec<String>,
    /// Reviewers assigned
    pub reviewers: Vec<String>,
    /// PR URL
    pub url: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Pull request state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Closed,
    Merged,
    Draft,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// Username
    pub username: String,
    /// Display name
    pub name: Option<String>,
    /// Email address
    pub email: Option<String>,
}

/// Pull request creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePrRequest {
    /// PR title
    pub title: String,
    /// PR description/body
    pub body: String,
    /// Source branch
    pub head: String,
    /// Target branch
    pub base: String,
    /// Draft PR flag
    #[serde(default)]
    pub draft: bool,
    /// Labels to assign
    #[serde(default)]
    pub labels: Vec<String>,
    /// Reviewers to assign
    #[serde(default)]
    pub reviewers: Vec<String>,
    /// Assignees
    #[serde(default)]
    pub assignees: Vec<String>,
}

/// Pull request manager trait
#[allow(async_fn_in_trait)]
pub trait PullRequestManager: Send + Sync {
    /// Create a new pull request
    async fn create_pr(&self, repo_url: &str, request: CreatePrRequest) -> Result<PullRequest>;

    /// Get pull request by number
    async fn get_pr(&self, repo_url: &str, pr_number: u64) -> Result<PullRequest>;

    /// List pull requests
    async fn list_prs(&self, repo_url: &str, state: Option<PrState>) -> Result<Vec<PullRequest>>;

    /// Update pull request
    async fn update_pr(
        &self,
        repo_url: &str,
        pr_number: u64,
        updates: PrUpdate,
    ) -> Result<PullRequest>;

    /// Close pull request
    async fn close_pr(&self, repo_url: &str, pr_number: u64) -> Result<()>;

    /// Merge pull request
    async fn merge_pr(
        &self,
        repo_url: &str,
        pr_number: u64,
        merge_method: MergeMethod,
    ) -> Result<()>;

    /// Add comment to pull request
    async fn add_comment(&self, repo_url: &str, pr_number: u64, comment: &str) -> Result<()>;

    /// Request review from users
    async fn request_review(
        &self,
        repo_url: &str,
        pr_number: u64,
        reviewers: Vec<String>,
    ) -> Result<()>;
}

/// Pull request update request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrUpdate {
    /// New title
    pub title: Option<String>,
    /// New body
    pub body: Option<String>,
    /// New state
    pub state: Option<PrState>,
    /// Labels to add/remove
    pub labels: Option<Vec<String>>,
    /// Reviewers to add/remove
    pub reviewers: Option<Vec<String>>,
}

/// Merge method for pull requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// Create a merge commit
    Merge,
    /// Squash and merge
    Squash,
    /// Rebase and merge
    Rebase,
}

/// GitHub pull request manager implementation
#[derive(Debug, Clone)]
pub struct GitHubPrManager {
    client: reqwest::Client,
    token: String,
}

impl GitHubPrManager {
    /// Create a new GitHub PR manager
    pub fn new(token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("xze-bot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, token }
    }

    /// Extract owner and repo from GitHub URL
    fn parse_github_url(&self, repo_url: &str) -> Result<(String, String)> {
        let url = if repo_url.starts_with("https://github.com/") {
            repo_url.strip_prefix("https://github.com/").unwrap()
        } else if repo_url.starts_with("git@github.com:") {
            repo_url.strip_prefix("git@github.com:").unwrap()
        } else {
            return Err(XzeError::validation("Invalid GitHub URL format"));
        };

        let url = url.strip_suffix(".git").unwrap_or(url);
        let parts: Vec<&str> = url.split('/').collect();

        if parts.len() != 2 {
            return Err(XzeError::validation("Invalid GitHub repository format"));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Build GitHub API URL
    fn api_url(&self, owner: &str, repo: &str, endpoint: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/{}",
            owner, repo, endpoint
        )
    }
}

impl PullRequestManager for GitHubPrManager {
    async fn create_pr(&self, repo_url: &str, request: CreatePrRequest) -> Result<PullRequest> {
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(&owner, &repo, "pulls");

        let github_request = serde_json::json!({
            "title": request.title,
            "body": request.body,
            "head": request.head,
            "base": request.base,
            "draft": request.draft
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&github_request)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to create PR: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(XzeError::ai(format!("GitHub API error: {}", error_text)));
        }

        let pr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse PR response: {}", e)))?;

        self.parse_github_pr(&pr_data)
    }

    async fn get_pr(&self, repo_url: &str, pr_number: u64) -> Result<PullRequest> {
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(&owner, &repo, &format!("pulls/{}", pr_number));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to get PR: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::not_found(format!("PR #{} not found", pr_number)));
        }

        let pr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse PR response: {}", e)))?;

        self.parse_github_pr(&pr_data)
    }

    async fn list_prs(&self, repo_url: &str, state: Option<PrState>) -> Result<Vec<PullRequest>> {
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let mut url = self.api_url(&owner, &repo, "pulls");

        if let Some(state) = state {
            let state_param = match state {
                PrState::Open => "open",
                PrState::Closed => "closed",
                PrState::Merged => "closed", // GitHub API doesn't have separate merged state
                PrState::Draft => "open",
            };
            url.push_str(&format!("?state={}", state_param));
        }

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to list PRs: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to list pull requests"));
        }

        let prs_data: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse PRs response: {}", e)))?;

        let mut prs = Vec::new();
        for pr_data in prs_data {
            if let Ok(pr) = self.parse_github_pr(&pr_data) {
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
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(&owner, &repo, &format!("pulls/{}", pr_number));

        let mut update_data = serde_json::Map::new();

        if let Some(title) = updates.title {
            update_data.insert("title".to_string(), serde_json::Value::String(title));
        }

        if let Some(body) = updates.body {
            update_data.insert("body".to_string(), serde_json::Value::String(body));
        }

        if let Some(state) = updates.state {
            let state_str = match state {
                PrState::Open => "open",
                PrState::Closed => "closed",
                _ => return Err(XzeError::validation("Invalid state for update")),
            };
            update_data.insert(
                "state".to_string(),
                serde_json::Value::String(state_str.to_string()),
            );
        }

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&update_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to update PR: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to update pull request"));
        }

        let pr_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| XzeError::ai(format!("Failed to parse PR response: {}", e)))?;

        self.parse_github_pr(&pr_data)
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
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(&owner, &repo, &format!("pulls/{}/merge", pr_number));

        let merge_data = serde_json::json!({
            "merge_method": merge_method
        });

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&merge_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to merge PR: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to merge pull request"));
        }

        Ok(())
    }

    async fn add_comment(&self, repo_url: &str, pr_number: u64, comment: &str) -> Result<()> {
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(&owner, &repo, &format!("issues/{}/comments", pr_number));

        let comment_data = serde_json::json!({
            "body": comment
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&comment_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to add comment: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to add comment to pull request"));
        }

        Ok(())
    }

    async fn request_review(
        &self,
        repo_url: &str,
        pr_number: u64,
        reviewers: Vec<String>,
    ) -> Result<()> {
        let (owner, repo) = self.parse_github_url(repo_url)?;
        let url = self.api_url(
            &owner,
            &repo,
            &format!("pulls/{}/requested_reviewers", pr_number),
        );

        let review_data = serde_json::json!({
            "reviewers": reviewers
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&review_data)
            .send()
            .await
            .map_err(|e| XzeError::network(format!("Failed to request review: {}", e)))?;

        if !response.status().is_success() {
            return Err(XzeError::ai("Failed to request review"));
        }

        Ok(())
    }
}

impl GitHubPrManager {
    /// Parse GitHub API PR response into our PR struct
    fn parse_github_pr(&self, data: &serde_json::Value) -> Result<PullRequest> {
        let number = data["number"]
            .as_u64()
            .ok_or_else(|| XzeError::validation("Missing PR number"))?;

        let title = data["title"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing PR title"))?
            .to_string();

        let body = data["body"].as_str().unwrap_or("").to_string();

        let head_branch = data["head"]["ref"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing head branch"))?
            .to_string();

        let base_branch = data["base"]["ref"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing base branch"))?
            .to_string();

        let state_str = data["state"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing PR state"))?;

        let is_merged = data["merged"].as_bool().unwrap_or(false);
        let is_draft = data["draft"].as_bool().unwrap_or(false);

        let state = match (state_str, is_merged, is_draft) {
            (_, true, _) => PrState::Merged,
            ("closed", false, _) => PrState::Closed,
            ("open", false, true) => PrState::Draft,
            ("open", false, false) => PrState::Open,
            _ => PrState::Open,
        };

        let author = Author {
            username: data["user"]["login"]
                .as_str()
                .ok_or_else(|| XzeError::validation("Missing author username"))?
                .to_string(),
            name: data["user"]["name"].as_str().map(|s| s.to_string()),
            email: data["user"]["email"].as_str().map(|s| s.to_string()),
        };

        let labels = data["labels"]
            .as_array()
            .map(|labels| {
                labels
                    .iter()
                    .filter_map(|label| label["name"].as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let url = data["html_url"]
            .as_str()
            .ok_or_else(|| XzeError::validation("Missing PR URL"))?
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
            reviewers: Vec::new(), // Would need separate API call to get reviewers
            url,
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_state_serialization() {
        assert_eq!(serde_json::to_string(&PrState::Open).unwrap(), "\"open\"");
        assert_eq!(
            serde_json::to_string(&PrState::Closed).unwrap(),
            "\"closed\""
        );
        assert_eq!(
            serde_json::to_string(&PrState::Merged).unwrap(),
            "\"merged\""
        );
    }

    #[test]
    fn test_merge_method_serialization() {
        assert_eq!(
            serde_json::to_string(&MergeMethod::Merge).unwrap(),
            "\"merge\""
        );
        assert_eq!(
            serde_json::to_string(&MergeMethod::Squash).unwrap(),
            "\"squash\""
        );
        assert_eq!(
            serde_json::to_string(&MergeMethod::Rebase).unwrap(),
            "\"rebase\""
        );
    }

    #[test]
    fn test_create_pr_request() {
        let request = CreatePrRequest {
            title: "Test PR".to_string(),
            body: "This is a test".to_string(),
            head: "feature-branch".to_string(),
            base: "main".to_string(),
            draft: false,
            labels: vec!["enhancement".to_string()],
            reviewers: vec!["reviewer1".to_string()],
            assignees: vec!["assignee1".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test PR"));
        assert!(json.contains("feature-branch"));
    }

    #[test]
    fn test_github_url_parsing() {
        let manager = GitHubPrManager::new("fake-token".to_string());

        let (owner, repo) = manager
            .parse_github_url("https://github.com/owner/repo")
            .unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        let (owner, repo) = manager
            .parse_github_url("git@github.com:owner/repo.git")
            .unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        assert!(manager.parse_github_url("invalid-url").is_err());
    }

    #[test]
    fn test_api_url_building() {
        let manager = GitHubPrManager::new("fake-token".to_string());
        let url = manager.api_url("owner", "repo", "pulls");
        assert_eq!(url, "https://api.github.com/repos/owner/repo/pulls");
    }
}
