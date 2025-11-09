# Git Operations in XZe

## Overview

XZe provides comprehensive Git operations functionality for automating
documentation workflows. This guide explains how to use Git operations for
repository management, credential configuration, change detection, and automated
workflows.

## Core Concepts

### GitOperations

The `GitOperations` struct is the main interface for Git operations. It manages
repository interactions, authentication, and change tracking.

### CredentialStore

Thread-safe credential storage supporting multiple authentication methods:

- Username/password (personal access tokens)
- SSH key authentication
- SSH agent integration
- Git credential helper support

### Change Detection

Advanced diff analysis tracks file changes, categorizes modifications, and
provides detailed statistics for incremental documentation updates.

## Getting Started

### Basic Setup

```rust
use xze_core::git::{GitOperations, CredentialStore, credentials_from_env};

// Load credentials from environment
let store = credentials_from_env();
let git_ops = GitOperations::new(store);
```

### Manual Credential Configuration

```rust
use xze_core::git::CredentialStore;

// Username and password/token
let store = CredentialStore::new()
    .with_userpass("username".to_string(), "token".to_string());

// SSH key authentication
let store = CredentialStore::new()
    .with_ssh_key(
        "git".to_string(),
        "/home/user/.ssh/id_ed25519",
        Some("/home/user/.ssh/id_ed25519.pub"),
        None, // passphrase
    )
    .with_agent(true);
```

## Repository Operations

### Cloning Repositories

```rust
use std::path::Path;

// Clone a remote repository
let repo = git_ops.clone(
    "https://github.com/user/repo.git",
    Path::new("./local-repo")
).await?;
```

### Opening Existing Repositories

```rust
// Open an existing local repository
let repo = git_ops.open("./existing-repo")?;
```

### Initializing New Repositories

```rust
// Initialize a new Git repository
let repo = git_ops.init("./new-repo")?;
```

## Branch Management

### Creating Branches

```rust
// Create a new branch without checking it out
git_ops.create_branch(&repo, "feature-branch", false)?;

// Create and checkout a new branch
git_ops.create_branch(&repo, "docs-update", true)?;
```

### Switching Branches

```rust
// Checkout an existing branch
git_ops.checkout_branch(&repo, "main")?;
```

### Listing Branches

```rust
// List all local branches
let branches = git_ops.list_branches(&repo, true)?;

for branch in branches {
    println!("Branch: {}", branch.name);
    println!("  Current: {}", branch.is_current);
    println!("  Upstream: {:?}", branch.upstream);
    println!("  Commit: {}", branch.commit_hash);
}
```

### Getting Current Branch

```rust
let current = git_ops.current_branch(&repo)?;
println!("On branch: {}", current);
```

### Deleting Branches

```rust
// Delete a local branch
git_ops.delete_branch(&repo, "old-branch")?;
```

## Committing Changes

### Staging Files

```rust
// Stage all changes
git_ops.stage_all(&repo)?;

// Stage specific files
let files = vec!["README.md", "docs/api.md"];
git_ops.stage_files(&repo, &files)?;
```

### Creating Commits

```rust
// Create a commit with automatic signature
git_ops.commit(&repo, "docs: update API documentation")?;

// Create a commit with custom signature
use git2::Signature;

let author = Signature::now("John Doe", "john@example.com")?;
let committer = Signature::now("CI Bot", "ci@example.com")?;

git_ops.commit_with_signature(
    &repo,
    "docs: automated documentation update",
    &author,
    &committer
)?;
```

## Remote Operations

### Pushing Changes

```rust
// Push current branch to remote
git_ops.push(&repo, "main")?;

// Push feature branch
git_ops.push(&repo, "feature-branch")?;
```

### Fetching Updates

```rust
// Fetch all updates from remote
git_ops.fetch(&repo)?;
```

### Pulling Changes

```rust
// Fetch and fast-forward merge
git_ops.pull(&repo)?;
```

### Getting Remote URL

```rust
let url = git_ops.get_remote_url(&repo, "origin")?;
println!("Remote URL: {}", url);
```

## Change Detection

### Checking for Changes

```rust
// Quick check for uncommitted changes
if git_ops.has_changes(&repo)? {
    println!("Repository has uncommitted changes");
}

// Check if repository is completely clean
if git_ops.is_clean(&repo)? {
    println!("Repository is clean (no changes or conflicts)");
}
```

### Getting File Status

```rust
use git2::Status;

let status = git_ops.get_status(&repo)?;

for (path, status) in status {
    if status.is_wt_new() {
        println!("New file: {}", path.display());
    } else if status.is_wt_modified() {
        println!("Modified: {}", path.display());
    } else if status.is_wt_deleted() {
        println!("Deleted: {}", path.display());
    }
}
```

### Diff Analysis

```rust
use xze_core::git::ChangeType;

// Analyze changes between two commits
let diff = git_ops.diff_analysis(&repo, Some("HEAD~1"), Some("HEAD"))?;

println!("Files changed: {}", diff.files_changed);
println!("Total insertions: {}", diff.insertions);
println!("Total deletions: {}", diff.deletions);

// Iterate through all changes
for change in &diff.changes {
    println!("File: {}", change.path.display());
    println!("  Type: {:?}", change.change_type);
    println!("  +{} -{}", change.additions, change.deletions);
}

// Get changes by type
let added_files = diff.changes_by_type(ChangeType::Added);
let modified_files = diff.changes_by_type(ChangeType::Modified);
let deleted_files = diff.changes_by_type(ChangeType::Deleted);

println!("Added: {}", added_files.len());
println!("Modified: {}", modified_files.len());
println!("Deleted: {}", deleted_files.len());
```

### Working Directory Diff

```rust
// Compare working directory to HEAD
let diff = git_ops.diff_analysis(&repo, Some("HEAD"), None)?;

if diff.has_changes() {
    println!("Working directory has {} uncommitted changes", diff.files_changed);
}
```

## Conflict Management

### Detecting Conflicts

```rust
// Check for merge conflicts
if git_ops.has_conflicts(&repo)? {
    println!("Repository has merge conflicts");

    // Get list of conflicted files
    let conflicts = git_ops.get_conflicts(&repo)?;
    for path in conflicts {
        println!("Conflict in: {}", path.display());
    }
}
```

## Tag Management

### Creating Tags

```rust
// Create a lightweight tag
git_ops.create_tag(&repo, "v1.0.0", None, false)?;

// Create an annotated tag
git_ops.create_tag(
    &repo,
    "v1.0.0",
    Some("Release version 1.0.0"),
    false
)?;

// Force overwrite existing tag
git_ops.create_tag(&repo, "v1.0.0", Some("Updated release"), true)?;
```

### Listing Tags

```rust
let tags = git_ops.list_tags(&repo)?;
for tag in tags {
    println!("Tag: {}", tag);
}
```

### Deleting Tags

```rust
git_ops.delete_tag(&repo, "v0.9.0")?;
```

## Stash Operations

### Stashing Changes

```rust
// Stash with default message
git_ops.stash(&mut repo, None)?;

// Stash with custom message
git_ops.stash(&mut repo, Some("WIP: refactoring docs module"))?;
```

### Applying Stashed Changes

```rust
// Pop most recent stash
git_ops.stash_pop(&mut repo, 0)?;

// Pop specific stash
git_ops.stash_pop(&mut repo, 2)?;
```

## Advanced Operations

### Reset Operations

```rust
use git2::ResetType;

// Soft reset (keep changes staged)
git_ops.reset(&repo, "HEAD~1", ResetType::Soft)?;

// Mixed reset (unstage changes)
git_ops.reset(&repo, "HEAD~1", ResetType::Mixed)?;

// Hard reset (discard all changes)
git_ops.reset(&repo, "HEAD~1", ResetType::Hard)?;
```

### Getting Commit Information

```rust
// Get last commit hash
let hash = git_ops.get_last_commit_hash(&repo)?;
println!("Last commit: {}", hash);
```

## Credential Configuration

### Environment Variables

Set these environment variables for automatic credential loading:

**HTTP Authentication**

```bash
export GIT_USERNAME="your-username"
export GIT_PASSWORD="your-token-or-password"
```

**SSH Authentication**

```bash
export GIT_SSH_USERNAME="git"
export GIT_SSH_KEY="/home/user/.ssh/id_ed25519"
export GIT_SSH_PUBLIC_KEY="/home/user/.ssh/id_ed25519.pub"
export GIT_SSH_PASSPHRASE="optional-passphrase"
```

### Programmatic Configuration

```rust
use xze_core::git::CredentialStore;

let store = CredentialStore::new();

// Set username/password
store.set_userpass("user".to_string(), "token".to_string());

// Set SSH key
store.set_ssh_key(
    "git".to_string(),
    "/path/to/private/key",
    Some("/path/to/public/key"),
    Some("passphrase".to_string())
);

// Validate credentials
store.validate()?;
```

### Using SSH Agent

```rust
let store = CredentialStore::new()
    .with_agent(true)
    .with_credential_helper(false);
```

### Using Git Credential Helper

```rust
let store = CredentialStore::new()
    .with_agent(false)
    .with_credential_helper(true);
```

## Common Workflows

### Automated Documentation Update

```rust
use xze_core::git::{GitOperations, credentials_from_env};

async fn update_docs() -> Result<()> {
    let git_ops = GitOperations::new(credentials_from_env());

    // Clone or open repository
    let repo = if Path::new("./repo").exists() {
        git_ops.open("./repo")?
    } else {
        git_ops.clone("https://github.com/org/repo.git", Path::new("./repo")).await?
    };

    // Create feature branch
    git_ops.create_branch(&repo, "docs-auto-update", true)?;

    // Generate documentation (your logic here)
    // ...

    // Stage and commit changes
    git_ops.stage_all(&repo)?;
    git_ops.commit(&repo, "docs: automated documentation update")?;

    // Push to remote
    git_ops.push(&repo, "docs-auto-update")?;

    Ok(())
}
```

### Incremental Processing Based on Changes

```rust
async fn process_changed_files() -> Result<()> {
    let git_ops = GitOperations::new(credentials_from_env());
    let repo = git_ops.open("./repo")?;

    // Get changes since last processing
    let diff = git_ops.diff_analysis(&repo, Some("last-processed"), Some("HEAD"))?;

    // Process only changed files
    for change in &diff.changes {
        match change.change_type {
            ChangeType::Added | ChangeType::Modified => {
                println!("Processing: {}", change.path.display());
                // Process file
            }
            ChangeType::Deleted => {
                println!("Removing documentation for: {}", change.path.display());
                // Clean up generated docs
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Branch Cleanup

```rust
fn cleanup_old_branches() -> Result<()> {
    let git_ops = GitOperations::new(credentials_from_env());
    let repo = git_ops.open("./repo")?;

    let branches = git_ops.list_branches(&repo, true)?;

    for branch in branches {
        if !branch.is_current && branch.name.starts_with("docs-") {
            println!("Deleting branch: {}", branch.name);
            git_ops.delete_branch(&repo, &branch.name)?;
        }
    }

    Ok(())
}
```

## Error Handling

### Common Errors

```rust
use xze_core::XzeError;

match git_ops.push(&repo, "main") {
    Ok(_) => println!("Push successful"),
    Err(XzeError::Auth { message }) => {
        eprintln!("Authentication failed: {}", message);
    }
    Err(XzeError::Git(e)) => {
        eprintln!("Git error: {}", e);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

### Validation

```rust
// Validate credentials before use
let store = credentials_from_env();
if let Err(e) = store.validate() {
    eprintln!("Invalid credentials: {}", e);
    return;
}

let git_ops = GitOperations::new(store);
```

## Best Practices

### Credential Security

1. Never hardcode credentials in source code
2. Use environment variables or secure credential stores
3. Enable SSH agent when available
4. Use personal access tokens instead of passwords
5. Rotate credentials regularly

### Branch Naming

Use descriptive branch names with prefixes:

- `docs/feature-name` - Documentation updates
- `auto/task-description` - Automated updates
- `fix/issue-number` - Bug fixes

### Commit Messages

Follow conventional commit format:

- `docs: update API documentation`
- `feat: add new git operations`
- `fix: correct diff analysis for renames`

### Error Recovery

Always check for uncommitted changes before operations:

```rust
if git_ops.has_changes(&repo)? {
    println!("Stashing uncommitted changes");
    git_ops.stash(&mut repo, Some("Auto-stash before operation"))?;
}

// Perform operation
git_ops.pull(&repo)?;

// Restore stashed changes if needed
git_ops.stash_pop(&mut repo, 0)?;
```

## Performance Tips

1. Reuse `GitOperations` instance for multiple operations
2. Clone repositories to local disk for repeated access
3. Use shallow clones for large repositories when full history not needed
4. Enable SSH agent to avoid repeated key authentication

## Troubleshooting

### Authentication Failures

If authentication fails, check:

1. Credentials are correctly configured
2. SSH keys have correct permissions (600 for private key)
3. SSH agent is running if using agent authentication
4. Personal access token has required scopes
5. Firewall allows Git protocol connections

### Merge Conflicts

Current limitation: XZe does not auto-resolve conflicts. To handle:

```rust
if git_ops.has_conflicts(&repo)? {
    let conflicts = git_ops.get_conflicts(&repo)?;
    eprintln!("Manual conflict resolution required for:");
    for path in conflicts {
        eprintln!("  - {}", path.display());
    }
    return Err(XzeError::repository("Merge conflicts detected"));
}
```

### Push Failures

Common causes:

1. Branch is behind remote (need to pull first)
2. Authentication failed
3. Branch protection rules prevent push
4. Network connectivity issues

## Limitations

Current known limitations:

1. Pull only supports fast-forward merges
2. No automatic conflict resolution
3. No rebase or cherry-pick support
4. Limited submodule support
5. No GPG signature support

## Future Features

Planned enhancements:

- Merge conflict resolution
- Interactive rebase support
- Cherry-pick operations
- Submodule management
- GPG commit signing
- Worktree support
- Partial clone support

## Related Documentation

- Credential Management: `credentials.rs`
- Error Handling: `error.rs`
- Implementation Details: `phase2_1_completion.md`
- API Reference: Rustdoc generated documentation

## Support

For issues or questions:

1. Check rustdoc API documentation
2. Review phase completion reports
3. Examine unit tests for examples
4. Consult git2-rs documentation for low-level details
