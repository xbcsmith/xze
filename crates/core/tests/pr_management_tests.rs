//! Pull Request Management Integration Tests
//!
//! This module contains comprehensive tests for PR/MR management across
//! different Git platforms (GitHub, GitLab).

use std::collections::HashMap;
use xze_core::git::{
    CreatePrRequest, GitHubPrManager, GitLabPrManager, GitPlatform, PrState, PrTemplateBuilder,
    PrTemplateData, PrUpdate, PullRequestManager,
};

mod common;

// Note: Most of these tests are ignored by default as they require:
// 1. Network access
// 2. Valid API tokens
// 3. Actual test repositories
//
// To run them, set the appropriate environment variables and use:
// cargo test --test pr_management_tests -- --ignored

#[test]
fn test_platform_detection() {
    // GitHub URLs
    assert_eq!(
        GitPlatform::detect("https://github.com/owner/repo"),
        GitPlatform::GitHub
    );
    assert_eq!(
        GitPlatform::detect("git@github.com:owner/repo.git"),
        GitPlatform::GitHub
    );
    assert_eq!(
        GitPlatform::detect("https://github.com/owner/repo.git"),
        GitPlatform::GitHub
    );

    // GitLab URLs
    assert_eq!(
        GitPlatform::detect("https://gitlab.com/owner/repo"),
        GitPlatform::GitLab
    );
    assert_eq!(
        GitPlatform::detect("git@gitlab.com:owner/repo.git"),
        GitPlatform::GitLab
    );
    assert_eq!(
        GitPlatform::detect("https://gitlab.example.com/owner/repo"),
        GitPlatform::GitLab
    );

    // Unknown platforms
    assert_eq!(
        GitPlatform::detect("https://bitbucket.org/owner/repo"),
        GitPlatform::Unknown
    );
}

#[test]
fn test_pr_template_builder_default() {
    let builder = PrTemplateBuilder::new();

    let data = PrTemplateData {
        title: "Add new feature".to_string(),
        source_branch: "feature/new-feature".to_string(),
        target_branch: "main".to_string(),
        changed_files: vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
        ],
        additions: 250,
        deletions: 45,
        commits: vec![
            "feat(core): add new feature implementation".to_string(),
            "test(core): add tests for new feature".to_string(),
            "docs(readme): update documentation".to_string(),
        ],
        jira_issue: Some("PROJ-1234".to_string()),
        context: {
            let mut map = HashMap::new();
            map.insert("Type".to_string(), "Feature".to_string());
            map.insert("Priority".to_string(), "High".to_string());
            map
        },
    };

    let description = builder.build(&data, None).unwrap();

    // Verify all components are present
    assert!(description.contains("Add new feature"));
    assert!(description.contains("feature/new-feature"));
    assert!(description.contains("main"));
    assert!(description.contains("src/main.rs"));
    assert!(description.contains("src/lib.rs"));
    assert!(description.contains("README.md"));
    assert!(description.contains("250"));
    assert!(description.contains("45"));
    assert!(description.contains("feat(core): add new feature implementation"));
    assert!(description.contains("PROJ-1234"));
    assert!(description.contains("Type"));
    assert!(description.contains("Feature"));
}

#[test]
fn test_pr_template_builder_custom() {
    let mut builder = PrTemplateBuilder::new();

    let custom_template = r#"
PR Title: {{title}}
Source: {{source_branch}} → Target: {{target_branch}}

Files Changed: {{#each changed_files}}{{this}}, {{/each}}

Commits:
{{#each commits}}
* {{this}}
{{/each}}
"#;

    builder
        .register_template("custom", custom_template)
        .unwrap();

    let data = PrTemplateData {
        title: "Custom Template Test".to_string(),
        source_branch: "test-branch".to_string(),
        target_branch: "develop".to_string(),
        changed_files: vec!["file1.rs".to_string(), "file2.rs".to_string()],
        additions: 10,
        deletions: 5,
        commits: vec!["commit1".to_string(), "commit2".to_string()],
        jira_issue: None,
        context: HashMap::new(),
    };

    let description = builder.build(&data, Some("custom")).unwrap();

    assert!(description.contains("Custom Template Test"));
    assert!(description.contains("test-branch"));
    assert!(description.contains("develop"));
    assert!(description.contains("file1.rs"));
    assert!(description.contains("commit1"));
}

#[test]
fn test_pr_template_builder_minimal_data() {
    let builder = PrTemplateBuilder::new();

    let data = PrTemplateData {
        title: "Minimal PR".to_string(),
        source_branch: "fix".to_string(),
        target_branch: "main".to_string(),
        changed_files: vec![],
        additions: 0,
        deletions: 0,
        commits: vec![],
        jira_issue: None,
        context: HashMap::new(),
    };

    let description = builder.build(&data, None).unwrap();

    // Should still render without errors
    assert!(description.contains("Minimal PR"));
    assert!(description.contains("fix"));
    assert!(description.contains("main"));
}

#[test]
fn test_create_pr_request_serialization() {
    let request = CreatePrRequest {
        title: "Test PR".to_string(),
        body: "This is a test pull request".to_string(),
        head: "feature-branch".to_string(),
        base: "main".to_string(),
        draft: true,
        labels: vec!["enhancement".to_string(), "documentation".to_string()],
        reviewers: vec!["reviewer1".to_string(), "reviewer2".to_string()],
        assignees: vec!["assignee1".to_string()],
    };

    let json = serde_json::to_string(&request).unwrap();

    assert!(json.contains("Test PR"));
    assert!(json.contains("feature-branch"));
    assert!(json.contains("enhancement"));
    assert!(json.contains("reviewer1"));
    assert!(json.contains("assignee1"));
}

#[test]
fn test_pr_update_serialization() {
    let update = PrUpdate {
        title: Some("Updated Title".to_string()),
        body: Some("Updated body".to_string()),
        state: Some(PrState::Closed),
        labels: Some(vec!["bug".to_string()]),
        reviewers: Some(vec!["new-reviewer".to_string()]),
    };

    let json = serde_json::to_string(&update).unwrap();

    assert!(json.contains("Updated Title"));
    assert!(json.contains("Updated body"));
    assert!(json.contains("closed"));
    assert!(json.contains("bug"));
}

#[test]
fn test_github_manager_creation() {
    let token = "test-token".to_string();
    let manager = GitHubPrManager::new(token);

    // Manager should be created successfully
    // Just verify it doesn't panic
    drop(manager);
}

#[test]
fn test_gitlab_manager_creation() {
    let token = "test-token".to_string();
    let manager = GitLabPrManager::new(token);

    // Manager should be created successfully
    drop(manager);
}

#[test]
fn test_gitlab_manager_custom_url() {
    let token = "test-token".to_string();
    let custom_url = "https://gitlab.example.com".to_string();
    let manager = GitLabPrManager::new_with_url(token, custom_url);

    // Manager should be created with custom URL
    drop(manager);
}

// GitHub Integration Tests (Ignored by default)

#[tokio::test]
#[ignore]
async fn test_github_create_pr() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");

    let manager = GitHubPrManager::new(token);

    let request = CreatePrRequest {
        title: "Test PR from xze".to_string(),
        body: "This is an automated test PR".to_string(),
        head: "test-branch".to_string(),
        base: "main".to_string(),
        draft: true,
        labels: vec!["test".to_string()],
        reviewers: vec![],
        assignees: vec![],
    };

    let result = manager.create_pr(&repo_url, request).await;
    assert!(result.is_ok());

    let pr = result.unwrap();
    assert_eq!(pr.title, "Test PR from xze");
    assert_eq!(pr.state, PrState::Draft);
}

#[tokio::test]
#[ignore]
async fn test_github_get_pr() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");
    let pr_number = std::env::var("GITHUB_TEST_PR_NUMBER")
        .expect("GITHUB_TEST_PR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid PR number");

    let manager = GitHubPrManager::new(token);

    let result = manager.get_pr(&repo_url, pr_number).await;
    assert!(result.is_ok());

    let pr = result.unwrap();
    assert_eq!(pr.number, pr_number);
}

#[tokio::test]
#[ignore]
async fn test_github_list_prs() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");

    let manager = GitHubPrManager::new(token);

    // List open PRs
    let result = manager.list_prs(&repo_url, Some(PrState::Open)).await;
    assert!(result.is_ok());

    let prs = result.unwrap();
    for pr in prs {
        assert!(pr.state == PrState::Open || pr.state == PrState::Draft);
    }
}

#[tokio::test]
#[ignore]
async fn test_github_update_pr() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");
    let pr_number = std::env::var("GITHUB_TEST_PR_NUMBER")
        .expect("GITHUB_TEST_PR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid PR number");

    let manager = GitHubPrManager::new(token);

    let update = PrUpdate {
        title: Some("Updated Title from Test".to_string()),
        body: Some("Updated description".to_string()),
        state: None,
        labels: None,
        reviewers: None,
    };

    let result = manager.update_pr(&repo_url, pr_number, update).await;
    assert!(result.is_ok());

    let pr = result.unwrap();
    assert_eq!(pr.title, "Updated Title from Test");
}

#[tokio::test]
#[ignore]
async fn test_github_add_comment() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");
    let pr_number = std::env::var("GITHUB_TEST_PR_NUMBER")
        .expect("GITHUB_TEST_PR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid PR number");

    let manager = GitHubPrManager::new(token);

    let comment = "This is an automated test comment from xze";
    let result = manager.add_comment(&repo_url, pr_number, comment).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_github_request_review() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let repo_url = std::env::var("GITHUB_TEST_REPO").expect("GITHUB_TEST_REPO not set");
    let pr_number = std::env::var("GITHUB_TEST_PR_NUMBER")
        .expect("GITHUB_TEST_PR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid PR number");
    let reviewer = std::env::var("GITHUB_TEST_REVIEWER").expect("GITHUB_TEST_REVIEWER not set");

    let manager = GitHubPrManager::new(token);

    let result = manager
        .request_review(&repo_url, pr_number, vec![reviewer])
        .await;
    assert!(result.is_ok());
}

// GitLab Integration Tests (Ignored by default)

#[tokio::test]
#[ignore]
async fn test_gitlab_create_mr() {
    let token = std::env::var("GITLAB_TOKEN").expect("GITLAB_TOKEN not set");
    let repo_url = std::env::var("GITLAB_TEST_REPO").expect("GITLAB_TEST_REPO not set");

    let manager = GitLabPrManager::new(token);

    let request = CreatePrRequest {
        title: "Test MR from xze".to_string(),
        body: "This is an automated test MR".to_string(),
        head: "test-branch".to_string(),
        base: "main".to_string(),
        draft: true,
        labels: vec!["test".to_string()],
        reviewers: vec![],
        assignees: vec![],
    };

    let result = manager.create_pr(&repo_url, request).await;
    assert!(result.is_ok());

    let mr = result.unwrap();
    assert!(mr.title.contains("Test MR from xze") || mr.title.contains("Draft:"));
}

#[tokio::test]
#[ignore]
async fn test_gitlab_get_mr() {
    let token = std::env::var("GITLAB_TOKEN").expect("GITLAB_TOKEN not set");
    let repo_url = std::env::var("GITLAB_TEST_REPO").expect("GITLAB_TEST_REPO not set");
    let mr_number = std::env::var("GITLAB_TEST_MR_NUMBER")
        .expect("GITLAB_TEST_MR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid MR number");

    let manager = GitLabPrManager::new(token);

    let result = manager.get_pr(&repo_url, mr_number).await;
    assert!(result.is_ok());

    let mr = result.unwrap();
    assert_eq!(mr.number, mr_number);
}

#[tokio::test]
#[ignore]
async fn test_gitlab_list_mrs() {
    let token = std::env::var("GITLAB_TOKEN").expect("GITLAB_TOKEN not set");
    let repo_url = std::env::var("GITLAB_TEST_REPO").expect("GITLAB_TEST_REPO not set");

    let manager = GitLabPrManager::new(token);

    // List open MRs
    let result = manager.list_prs(&repo_url, Some(PrState::Open)).await;
    assert!(result.is_ok());

    let mrs = result.unwrap();
    for mr in mrs {
        assert!(mr.state == PrState::Open || mr.state == PrState::Draft);
    }
}

#[tokio::test]
#[ignore]
async fn test_gitlab_update_mr() {
    let token = std::env::var("GITLAB_TOKEN").expect("GITLAB_TOKEN not set");
    let repo_url = std::env::var("GITLAB_TEST_REPO").expect("GITLAB_TEST_REPO not set");
    let mr_number = std::env::var("GITLAB_TEST_MR_NUMBER")
        .expect("GITLAB_TEST_MR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid MR number");

    let manager = GitLabPrManager::new(token);

    let update = PrUpdate {
        title: Some("Updated MR Title from Test".to_string()),
        body: Some("Updated description".to_string()),
        state: None,
        labels: Some(vec!["updated".to_string()]),
        reviewers: None,
    };

    let result = manager.update_pr(&repo_url, mr_number, update).await;
    assert!(result.is_ok());

    let mr = result.unwrap();
    assert_eq!(mr.title, "Updated MR Title from Test");
}

#[tokio::test]
#[ignore]
async fn test_gitlab_add_note() {
    let token = std::env::var("GITLAB_TOKEN").expect("GITLAB_TOKEN not set");
    let repo_url = std::env::var("GITLAB_TEST_REPO").expect("GITLAB_TEST_REPO not set");
    let mr_number = std::env::var("GITLAB_TEST_MR_NUMBER")
        .expect("GITLAB_TEST_MR_NUMBER not set")
        .parse::<u64>()
        .expect("Invalid MR number");

    let manager = GitLabPrManager::new(token);

    let comment = "This is an automated test note from xze";
    let result = manager.add_comment(&repo_url, mr_number, comment).await;
    assert!(result.is_ok());
}

// Cross-platform tests

#[test]
fn test_pr_manager_concrete_types() {
    let github_token = "test-github-token".to_string();
    let gitlab_token = "test-gitlab-token".to_string();

    let github_manager = GitHubPrManager::new(github_token);
    let gitlab_manager = GitLabPrManager::new(gitlab_token);

    // Verify managers can be created successfully
    drop(github_manager);
    drop(gitlab_manager);
}

#[test]
fn test_comprehensive_pr_template() {
    let builder = PrTemplateBuilder::new();

    let mut context = HashMap::new();
    context.insert("Breaking Changes".to_string(), "None".to_string());
    context.insert("Migration Required".to_string(), "No".to_string());
    context.insert("Documentation Updated".to_string(), "Yes".to_string());

    let data = PrTemplateData {
        title: "Comprehensive Feature Implementation".to_string(),
        source_branch: "feature/comprehensive-feature".to_string(),
        target_branch: "develop".to_string(),
        changed_files: vec![
            "src/core/mod.rs".to_string(),
            "src/api/handler.rs".to_string(),
            "tests/integration_tests.rs".to_string(),
            "docs/api.md".to_string(),
        ],
        additions: 450,
        deletions: 120,
        commits: vec![
            "feat(core): implement new feature module".to_string(),
            "feat(api): add API endpoints for feature".to_string(),
            "test(integration): add comprehensive tests".to_string(),
            "docs(api): document new endpoints".to_string(),
            "refactor(core): improve error handling".to_string(),
        ],
        jira_issue: Some("PROJ-5678".to_string()),
        context,
    };

    let description = builder.build(&data, None).unwrap();

    // Verify comprehensive content
    assert!(description.contains("Comprehensive Feature Implementation"));
    assert!(description.contains("feature/comprehensive-feature"));
    assert!(description.contains("develop"));
    assert!(description.contains("450"));
    assert!(description.contains("120"));
    assert!(description.contains("src/core/mod.rs"));
    assert!(description.contains("tests/integration_tests.rs"));
    assert!(description.contains("PROJ-5678"));
    assert!(description.contains("Breaking Changes"));
    assert!(description.contains("feat(core): implement new feature module"));
}
