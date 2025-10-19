# how to create pull requests

This guide provides step-by-step instructions for creating pull requests using XZe.

## prerequisites

Before you begin, ensure you have:

1. XZe installed and configured
2. A valid API token for your Git platform
3. Repository access with appropriate permissions
4. Rust development environment (if using the API directly)

## quick start

### step 1: set up authentication

Create an API token and set it as an environment variable:

**For GitHub**:

```bash
# Create token at: https://github.com/settings/tokens
export GITHUB_TOKEN="ghp_your_token_here"
```

**For GitLab**:

```bash
# Create token at: https://gitlab.com/-/profile/personal_access_tokens
export GITLAB_TOKEN="glpat_your_token_here"
```

### step 2: create a basic pull request

```rust
use xze_core::git::{GitHubPrManager, CreatePrRequest, PullRequestManager};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the manager
    let token = std::env::var("GITHUB_TOKEN")?;
    let manager = GitHubPrManager::new(token);

    // Create the request
    let request = CreatePrRequest {
        title: "feat: add new feature".to_string(),
        body: "This PR adds a new feature".to_string(),
        head: "feature-branch".to_string(),
        base: "main".to_string(),
        draft: false,
        labels: vec![],
        reviewers: vec![],
        assignees: vec![],
    };

    // Create the PR
    let pr = manager.create_pr(
        "https://github.com/owner/repo",
        request
    ).await?;

    println!("Created PR #{}: {}", pr.number, pr.url);
    Ok(())
}
```

### step 3: view the created pr

The output will include the PR number and URL. Open the URL in your browser to view the PR.

## detailed walkthrough

### creating a draft pr

Draft PRs are useful for work-in-progress changes:

```rust
let request = CreatePrRequest {
    title: "WIP: Implementing feature X".to_string(),
    body: "Work in progress...".to_string(),
    head: "feature/x".to_string(),
    base: "main".to_string(),
    draft: true,  // Mark as draft
    labels: vec!["wip".to_string()],
    reviewers: vec![],
    assignees: vec![],
};

let pr = manager.create_pr(repo_url, request).await?;
```

### adding labels and reviewers

Include labels and reviewers when creating the PR:

```rust
let request = CreatePrRequest {
    title: "fix: resolve authentication bug".to_string(),
    body: "Fixes #123".to_string(),
    head: "fix/auth-bug".to_string(),
    base: "develop".to_string(),
    draft: false,
    labels: vec![
        "bug".to_string(),
        "high-priority".to_string(),
    ],
    reviewers: vec![
        "team-lead".to_string(),
        "senior-dev".to_string(),
    ],
    assignees: vec!["current-user".to_string()],
};

let pr = manager.create_pr(repo_url, request).await?;
```

### using templates for pr descriptions

Generate rich PR descriptions using templates:

```rust
use xze_core::git::{PrTemplateBuilder, PrTemplateData};
use std::collections::HashMap;

// Create template builder
let builder = PrTemplateBuilder::new();

// Prepare template data
let data = PrTemplateData {
    title: "Add authentication module".to_string(),
    source_branch: "feature/auth".to_string(),
    target_branch: "main".to_string(),
    changed_files: vec![
        "src/auth/mod.rs".to_string(),
        "src/auth/jwt.rs".to_string(),
    ],
    additions: 320,
    deletions: 15,
    commits: vec![
        "feat(auth): add JWT token generation".to_string(),
        "test(auth): add authentication tests".to_string(),
    ],
    jira_issue: Some("AUTH-123".to_string()),
    context: HashMap::new(),
};

// Generate description
let description = builder.build(&data, None)?;

// Create PR with generated description
let request = CreatePrRequest {
    title: data.title.clone(),
    body: description,
    head: data.source_branch.clone(),
    base: data.target_branch.clone(),
    draft: false,
    labels: vec!["feature".to_string()],
    reviewers: vec![],
    assignees: vec![],
};

let pr = manager.create_pr(repo_url, request).await?;
```

## platform-specific instructions

### github

**Token Requirements**:
- Scope: `repo` (full control of private repositories)
- Scope: `write:discussion` (for comments)

**Creating a PR**:

```rust
let token = std::env::var("GITHUB_TOKEN")?;
let manager = GitHubPrManager::new(token);

let request = CreatePrRequest {
    title: "feat: new feature".to_string(),
    body: "Description here".to_string(),
    head: "feature-branch".to_string(),
    base: "main".to_string(),
    draft: false,
    labels: vec!["enhancement".to_string()],
    reviewers: vec!["reviewer".to_string()],
    assignees: vec![],
};

let pr = manager.create_pr(
    "https://github.com/owner/repo",
    request
).await?;
```

### gitlab

**Token Requirements**:
- Scope: `api` (access the authenticated user's API)
- Scope: `write_repository` (write access to repository)

**Creating an MR on GitLab.com**:

```rust
let token = std::env::var("GITLAB_TOKEN")?;
let manager = GitLabPrManager::new(token);

let request = CreatePrRequest {
    title: "feat: new feature".to_string(),
    body: "Description here".to_string(),
    head: "feature-branch".to_string(),
    base: "main".to_string(),
    draft: false,
    labels: vec!["enhancement".to_string()],
    reviewers: vec![],
    assignees: vec![],
};

let mr = manager.create_pr(
    "https://gitlab.com/owner/repo",
    request
).await?;
```

**Self-hosted GitLab**:

```rust
let manager = GitLabPrManager::new_with_url(
    token,
    "https://gitlab.example.com".to_string()
);

let mr = manager.create_pr(
    "https://gitlab.example.com/owner/repo",
    request
).await?;
```

## custom templates

### creating a custom template

Define your own template format:

```rust
let mut builder = PrTemplateBuilder::new();

let custom_template = r#"
# {{title}}

## Overview
From: `{{source_branch}}` → `{{target_branch}}`

## Changes
{{#each changed_files}}
- {{this}}
{{/each}}

## Statistics
- **Added**: {{additions}} lines
- **Removed**: {{deletions}} lines

{{#if jira_issue}}
## Related Ticket
JIRA: {{jira_issue}}
{{/if}}

## Commits
{{#each commits}}
- {{this}}
{{/each}}
"#;

builder.register_template("custom", custom_template)?;
```

### using the custom template

```rust
let description = builder.build(&data, Some("custom"))?;

let request = CreatePrRequest {
    body: description,
    // ... other fields
};
```

## common workflows

### workflow 1: feature development

```rust
// 1. Create draft PR early
let request = CreatePrRequest {
    title: "feat: implementing new API".to_string(),
    body: "Work in progress".to_string(),
    head: "feature/new-api".to_string(),
    base: "develop".to_string(),
    draft: true,
    labels: vec!["wip".to_string()],
    reviewers: vec![],
    assignees: vec![],
};

let pr = manager.create_pr(repo_url, request).await?;
println!("Draft PR created: {}", pr.url);

// 2. Continue development...

// 3. Mark ready for review (update PR state)
let update = PrUpdate {
    title: Some("feat: add new API endpoints".to_string()),
    body: Some("Complete implementation with tests".to_string()),
    state: Some(PrState::Open),
    labels: Some(vec!["ready-for-review".to_string()]),
    reviewers: Some(vec!["team-lead".to_string()]),
};

manager.update_pr(repo_url, pr.number, update).await?;
```

### workflow 2: bug fix

```rust
// Create PR with template
let data = PrTemplateData {
    title: "fix: resolve memory leak in parser".to_string(),
    source_branch: "fix/memory-leak".to_string(),
    target_branch: "main".to_string(),
    changed_files: vec!["src/parser.rs".to_string()],
    additions: 25,
    deletions: 40,
    commits: vec![
        "fix(parser): resolve memory leak".to_string(),
        "test(parser): add regression test".to_string(),
    ],
    jira_issue: Some("BUG-456".to_string()),
    context: {
        let mut map = HashMap::new();
        map.insert("Severity".to_string(), "High".to_string());
        map.insert("Affected Versions".to_string(), "1.0-1.5".to_string());
        map
    },
};

let builder = PrTemplateBuilder::new();
let description = builder.build(&data, None)?;

let request = CreatePrRequest {
    title: data.title.clone(),
    body: description,
    head: data.source_branch.clone(),
    base: data.target_branch.clone(),
    draft: false,
    labels: vec!["bug".to_string(), "high-priority".to_string()],
    reviewers: vec!["senior-dev".to_string()],
    assignees: vec!["assignee".to_string()],
};

let pr = manager.create_pr(repo_url, request).await?;
```

### workflow 3: automated pr creation

```rust
async fn automated_pr_workflow(
    manager: &impl PullRequestManager,
    repo_url: &str,
    branch: &str,
) -> Result<PullRequest> {
    // Collect git data
    let commits = get_commits_in_branch(branch)?;
    let changed_files = get_changed_files(branch)?;
    let stats = get_change_statistics(branch)?;

    // Build description from template
    let builder = PrTemplateBuilder::new();
    let data = PrTemplateData {
        title: generate_title_from_commits(&commits),
        source_branch: branch.to_string(),
        target_branch: "main".to_string(),
        changed_files,
        additions: stats.additions,
        deletions: stats.deletions,
        commits,
        jira_issue: extract_jira_issue(branch),
        context: HashMap::new(),
    };

    let description = builder.build(&data, None)?;

    // Create PR
    let request = CreatePrRequest {
        title: data.title.clone(),
        body: description,
        head: branch.to_string(),
        base: "main".to_string(),
        draft: false,
        labels: auto_detect_labels(&data)?,
        reviewers: auto_assign_reviewers(&data)?,
        assignees: vec![],
    };

    manager.create_pr(repo_url, request).await
}
```

## troubleshooting

### authentication errors

**Problem**: "401 Unauthorized" error

**Solution**:
1. Verify token is set correctly
2. Check token has not expired
3. Ensure token has required scopes

```bash
# Test token
curl -H "Authorization: token $GITHUB_TOKEN" https://api.github.com/user
```

### permission errors

**Problem**: "403 Forbidden" error

**Solution**:
1. Verify you have write access to the repository
2. Check token scopes include repository access
3. Ensure you're not hitting rate limits

### branch not found

**Problem**: "Branch not found" error

**Solution**:
1. Ensure branch exists on remote
2. Push branch before creating PR
3. Verify branch name is correct

```bash
# Push branch first
git push origin feature-branch
```

### rate limiting

**Problem**: "429 Too Many Requests" error

**Solution**:
1. Wait for rate limit to reset
2. Implement retry logic with backoff
3. Cache data to reduce API calls

```rust
// Add retry logic
async fn create_pr_with_retry(
    manager: &impl PullRequestManager,
    repo_url: &str,
    request: CreatePrRequest,
    max_retries: u32,
) -> Result<PullRequest> {
    let mut attempts = 0;

    loop {
        match manager.create_pr(repo_url, request.clone()).await {
            Ok(pr) => return Ok(pr),
            Err(e) if attempts < max_retries && is_rate_limit_error(&e) => {
                attempts += 1;
                let delay = 2u64.pow(attempts);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## best practices

1. **Use conventional commits** in PR titles for clarity
2. **Include JIRA/ticket numbers** for traceability
3. **Add appropriate labels** for categorization
4. **Request specific reviewers** based on code ownership
5. **Use draft PRs** for work-in-progress
6. **Generate descriptions** using templates for consistency
7. **Add context** in custom template fields
8. **Test locally** before creating PR

## next steps

- Read the [PR Management Guide](../explanations/pr_management.md)
- Check the [API Reference](../reference/pr_api.md)
- Explore template customization options
- Integrate with CI/CD pipelines

## additional resources

- GitHub API: https://docs.github.com/en/rest
- GitLab API: https://docs.gitlab.com/ee/api/
- Handlebars Templates: https://handlebarsjs.com/
