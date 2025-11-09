# pull request management guide

## overview

XZe provides comprehensive pull request (PR) and merge request (MR) management capabilities across multiple Git hosting platforms. This guide explains how to use these features effectively.

## supported platforms

### github

Full support for GitHub pull requests with all standard features:

- Create, read, update, and delete PRs
- Draft PR support
- Label management
- Reviewer assignment
- Comments and discussions
- Status checks monitoring
- Multiple merge strategies

### gitlab

Full support for GitLab merge requests (MRs):

- Create, read, update, and delete MRs
- Support for GitLab.com and self-hosted instances
- Draft MR support via title prefixing
- Label management
- Reviewer assignment
- Notes (comments) on MRs
- Multiple merge strategies

### future platforms

The architecture supports easy addition of:

- Bitbucket Cloud and Server
- Gitea
- Gogs
- Azure DevOps

## core concepts

### platform abstraction

XZe uses a trait-based abstraction that provides a consistent API across platforms:

```rust
pub trait PullRequestManager: Send + Sync {
    async fn create_pr(&self, repo_url: &str, request: CreatePrRequest) -> Result<PullRequest>;
    async fn get_pr(&self, repo_url: &str, pr_number: u64) -> Result<PullRequest>;
    async fn list_prs(&self, repo_url: &str, state: Option<PrState>) -> Result<Vec<PullRequest>>;
    async fn update_pr(&self, repo_url: &str, pr_number: u64, updates: PrUpdate) -> Result<PullRequest>;
    async fn close_pr(&self, repo_url: &str, pr_number: u64) -> Result<()>;
    async fn merge_pr(&self, repo_url: &str, pr_number: u64, merge_method: MergeMethod) -> Result<()>;
    async fn add_comment(&self, repo_url: &str, pr_number: u64, comment: &str) -> Result<()>;
    async fn request_review(&self, repo_url: &str, pr_number: u64, reviewers: Vec<String>) -> Result<()>;
}
```

### automatic platform detection

XZe automatically detects the platform from repository URLs:

```rust
use xze_core::git::GitPlatform;

let platform = GitPlatform::detect("https://github.com/owner/repo");
// Returns GitPlatform::GitHub

let platform = GitPlatform::detect("https://gitlab.com/owner/repo");
// Returns GitPlatform::GitLab
```

## authentication

### github authentication

Use a GitHub Personal Access Token with appropriate permissions:

**Required Scopes**:
- `repo` - Full control of private repositories
- `write:discussion` - Read and write team discussions

**Setup**:

```bash
export GITHUB_TOKEN="ghp_your_token_here"
```

**Usage**:

```rust
let token = std::env::var("GITHUB_TOKEN")?;
let manager = GitHubPrManager::new(token);
```

### gitlab authentication

Use a GitLab Personal Access Token with appropriate permissions:

**Required Scopes**:
- `api` - Access the authenticated user's API
- `write_repository` - Allows write access to the repository

**Setup**:

```bash
export GITLAB_TOKEN="glpat_your_token_here"
```

**Usage**:

```rust
// For GitLab.com
let token = std::env::var("GITLAB_TOKEN")?;
let manager = GitLabPrManager::new(token);

// For self-hosted GitLab
let manager = GitLabPrManager::new_with_url(
    token,
    "https://gitlab.example.com".to_string()
);
```

## creating pull requests

### basic pr creation

```rust
use xze_core::git::{GitHubPrManager, CreatePrRequest, PullRequestManager};

#[tokio::main]
async fn main() -> Result<()> {
    let manager = GitHubPrManager::new(
        std::env::var("GITHUB_TOKEN")?
    );

    let request = CreatePrRequest {
        title: "feat(core): add new feature".to_string(),
        body: "This PR adds a new feature...".to_string(),
        head: "feature/new-feature".to_string(),
        base: "main".to_string(),
        draft: false,
        labels: vec!["enhancement".to_string()],
        reviewers: vec!["reviewer1".to_string()],
        assignees: vec![],
    };

    let pr = manager.create_pr(
        "https://github.com/owner/repo",
        request
    ).await?;

    println!("Created PR #{}: {}", pr.number, pr.url);
    Ok(())
}
```

### draft pull requests

Create a draft PR for work in progress:

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

**Platform Differences**:
- **GitHub**: Native draft PR support
- **GitLab**: Draft prefix added to title (e.g., "Draft: ...")

### pr with labels and reviewers

```rust
let request = CreatePrRequest {
    title: "fix(auth): resolve login issue".to_string(),
    body: "Fixes #123\n\nThis PR resolves...".to_string(),
    head: "fix/login".to_string(),
    base: "develop".to_string(),
    draft: false,
    labels: vec![
        "bug".to_string(),
        "high-priority".to_string(),
        "security".to_string(),
    ],
    reviewers: vec![
        "security-team".to_string(),
        "lead-developer".to_string(),
    ],
    assignees: vec!["developer1".to_string()],
};

let pr = manager.create_pr(repo_url, request).await?;
```

## using templates

### default template

XZe includes a comprehensive default template:

```rust
use xze_core::git::{PrTemplateBuilder, PrTemplateData};
use std::collections::HashMap;

let builder = PrTemplateBuilder::new();

let data = PrTemplateData {
    title: "Add authentication module".to_string(),
    source_branch: "feature/auth".to_string(),
    target_branch: "main".to_string(),
    changed_files: vec![
        "src/auth/mod.rs".to_string(),
        "src/auth/jwt.rs".to_string(),
        "tests/auth_tests.rs".to_string(),
    ],
    additions: 450,
    deletions: 30,
    commits: vec![
        "feat(auth): add JWT token generation".to_string(),
        "feat(auth): add authentication middleware".to_string(),
        "test(auth): add comprehensive tests".to_string(),
        "docs(auth): add API documentation".to_string(),
    ],
    jira_issue: Some("AUTH-123".to_string()),
    context: HashMap::new(),
};

let description = builder.build(&data, None)?;
```

### custom templates

Register and use custom templates:

```rust
let mut builder = PrTemplateBuilder::new();

let custom_template = r#"
## {{title}}

**Changes**: `{{source_branch}}` → `{{target_branch}}`

### Files Modified
{{#each changed_files}}
- {{this}}
{{/each}}

### Statistics
- Added: {{additions}} lines
- Removed: {{deletions}} lines

{{#if jira_issue}}
**Ticket**: {{jira_issue}}
{{/if}}

### Commits
{{#each commits}}
1. {{this}}
{{/each}}
"#;

builder.register_template("custom", custom_template)?;

let description = builder.build(&data, Some("custom"))?;
```

### template with context

Add custom context data to templates:

```rust
let mut context = HashMap::new();
context.insert("Breaking Changes".to_string(), "None".to_string());
context.insert("Migration Required".to_string(), "No".to_string());
context.insert("Performance Impact".to_string(), "Minimal".to_string());

let data = PrTemplateData {
    title: "Refactor database layer".to_string(),
    source_branch: "refactor/db".to_string(),
    target_branch: "main".to_string(),
    changed_files: vec!["src/db/mod.rs".to_string()],
    additions: 200,
    deletions: 150,
    commits: vec!["refactor(db): optimize queries".to_string()],
    jira_issue: Some("DB-456".to_string()),
    context,  // Custom context
};

let description = builder.build(&data, None)?;
```

## managing pull requests

### listing pull requests

List all PRs with optional state filtering:

```rust
use xze_core::git::PrState;

// List all open PRs
let open_prs = manager.list_prs(
    repo_url,
    Some(PrState::Open)
).await?;

for pr in open_prs {
    println!("PR #{}: {}", pr.number, pr.title);
}

// List all PRs (any state)
let all_prs = manager.list_prs(repo_url, None).await?;

// List merged PRs
let merged_prs = manager.list_prs(
    repo_url,
    Some(PrState::Merged)
).await?;
```

### getting pr details

Retrieve detailed information about a specific PR:

```rust
let pr = manager.get_pr(repo_url, 42).await?;

println!("Title: {}", pr.title);
println!("State: {:?}", pr.state);
println!("Author: {}", pr.author.username);
println!("Branch: {} → {}", pr.head_branch, pr.base_branch);
println!("URL: {}", pr.url);
println!("Labels: {:?}", pr.labels);
println!("Reviewers: {:?}", pr.reviewers);
```

### updating pull requests

Update PR properties:

```rust
use xze_core::git::PrUpdate;

let update = PrUpdate {
    title: Some("Updated Title".to_string()),
    body: Some("Updated description with more details...".to_string()),
    state: None,  // Don't change state
    labels: Some(vec!["updated".to_string(), "reviewed".to_string()]),
    reviewers: Some(vec!["additional-reviewer".to_string()]),
};

let updated_pr = manager.update_pr(repo_url, 42, update).await?;
```

### closing pull requests

Close a PR without merging:

```rust
manager.close_pr(repo_url, 42).await?;
println!("PR closed");
```

### merging pull requests

Merge a PR with your preferred method:

```rust
use xze_core::git::MergeMethod;

// Traditional merge commit
manager.merge_pr(
    repo_url,
    42,
    MergeMethod::Merge
).await?;

// Squash and merge
manager.merge_pr(
    repo_url,
    43,
    MergeMethod::Squash
).await?;

// Rebase and merge
manager.merge_pr(
    repo_url,
    44,
    MergeMethod::Rebase
).await?;
```

### adding comments

Add comments to PRs for communication:

```rust
let comment = "LGTM! Great work on this feature.";
manager.add_comment(repo_url, 42, comment).await?;

// Add formatted comment
let comment = format!(
    "Build Status: ✓ Passed\n\nAll checks completed successfully at {}",
    chrono::Utc::now()
);
manager.add_comment(repo_url, 42, &comment).await?;
```

### requesting reviews

Request reviews from team members:

```rust
let reviewers = vec![
    "team-lead".to_string(),
    "senior-dev".to_string(),
];

manager.request_review(repo_url, 42, reviewers).await?;
```

## advanced features

### checking pr status

Check if a PR is ready to merge:

```rust
// GitHub-specific feature
let github_manager = GitHubPrManager::new(token);

let is_ready = github_manager.is_mergeable(repo_url, 42).await?;

if is_ready {
    println!("PR is ready to merge");
} else {
    println!("PR has conflicts or failing checks");
}
```

### monitoring ci/cd status

Get status checks from CI/CD systems:

```rust
// GitHub-specific feature
let checks = github_manager.get_status_checks(repo_url, 42).await?;

for check in checks {
    println!("Check: {}", check.context);
    println!("State: {}", check.state);
    if let Some(desc) = check.description {
        println!("Description: {}", desc);
    }
}
```

### managing labels

Add or remove labels dynamically:

```rust
// GitHub-specific feature
let labels = vec!["approved".to_string(), "ready-to-merge".to_string()];
github_manager.add_labels(repo_url, 42, labels).await?;

// Remove a label
github_manager.remove_label(repo_url, 42, "wip").await?;
```

## platform-specific considerations

### github specific

**API Rate Limits**:
- 5,000 requests/hour for authenticated requests
- 60 requests/hour for unauthenticated

**Best Practices**:
- Use conditional requests with ETags when possible
- Batch operations where supported
- Monitor `X-RateLimit-Remaining` header

**Status Checks**:
- GitHub Actions, CircleCI, Travis CI all supported
- Check status before merging
- Use status API for external checks

### gitlab specific

**API Rate Limits**:
- 300 requests/minute for GitLab.com
- Configurable for self-hosted instances

**Project Path Encoding**:
- Paths are URL-encoded (e.g., `owner%2Frepo`)
- Handled automatically by GitLabPrManager

**Reviewer Assignment**:
- Requires numeric user IDs
- Username lookup needed for full functionality

**Draft MRs**:
- Uses title prefix "Draft:" or "WIP:"
- Automatically added when draft flag is true

## error handling

### common errors

**Authentication Errors**:

```rust
match manager.create_pr(repo_url, request).await {
    Ok(pr) => println!("Created PR: {}", pr.url),
    Err(e) => {
        if e.to_string().contains("401") {
            eprintln!("Authentication failed. Check your token.");
        } else if e.to_string().contains("403") {
            eprintln!("Permission denied. Check token scopes.");
        } else {
            eprintln!("Error: {}", e);
        }
    }
}
```

**Network Errors**:

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn create_pr_with_retry(
    manager: &GitHubPrManager,
    repo_url: &str,
    request: CreatePrRequest,
) -> Result<PullRequest> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match manager.create_pr(repo_url, request.clone()).await {
            Ok(pr) => return Ok(pr),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                eprintln!("Attempt {} failed: {}. Retrying...", attempts, e);
                sleep(Duration::from_secs(2u64.pow(attempts))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## best practices

### conventional commits integration

Structure PR titles to match conventional commits:

```rust
let request = CreatePrRequest {
    title: "feat(auth): add OAuth2 authentication".to_string(),
    // ... other fields
};
```

### jira integration

Link PRs to JIRA issues:

```rust
let mut context = HashMap::new();
context.insert("JIRA".to_string(), "PROJ-1234".to_string());

let data = PrTemplateData {
    jira_issue: Some("PROJ-1234".to_string()),
    context,
    // ... other fields
};
```

### semantic versioning

Use labels to indicate version impact:

```rust
let request = CreatePrRequest {
    labels: vec![
        "semver:minor".to_string(),  // Version impact
        "changelog:feature".to_string(),  // Changelog category
    ],
    // ... other fields
};
```

### code review workflow

```rust
// 1. Create draft PR
let request = CreatePrRequest {
    draft: true,
    // ... other fields
};
let pr = manager.create_pr(repo_url, request).await?;

// 2. Mark ready for review
let update = PrUpdate {
    state: Some(PrState::Open),
    // ... other fields
};
manager.update_pr(repo_url, pr.number, update).await?;

// 3. Request reviews
manager.request_review(
    repo_url,
    pr.number,
    vec!["reviewer1".to_string()]
).await?;

// 4. Add status comment
manager.add_comment(
    repo_url,
    pr.number,
    "Ready for review. All tests passing."
).await?;

// 5. Merge when approved
manager.merge_pr(
    repo_url,
    pr.number,
    MergeMethod::Squash
).await?;
```

## troubleshooting

### token permissions

If you encounter permission errors:

1. Verify token scopes in platform settings
2. Ensure token has not expired
3. Check repository permissions for the token user

### rate limiting

If you hit rate limits:

1. Implement exponential backoff
2. Cache PR data when possible
3. Use webhooks instead of polling
4. Consider upgrading API plan

### platform detection failures

If platform is not detected:

1. Check repository URL format
2. Use explicit manager construction
3. Verify URL accessibility

## examples

See the `examples/` directory for complete working examples:

- `examples/create_pr.rs` - Basic PR creation
- `examples/pr_workflow.rs` - Complete PR workflow
- `examples/custom_template.rs` - Custom template usage
- `examples/multi_platform.rs` - Cross-platform PR management

## additional resources

- GitHub API Documentation: https://docs.github.com/en/rest
- GitLab API Documentation: https://docs.gitlab.com/ee/api/
- XZe API Reference: `docs/reference/pr_api.md`
- Template Reference: `crates/core/templates/pr_template.hbs`
