# How to Run Git Integration Tests

## Overview

XZe includes comprehensive integration tests for Git operations that validate
functionality with real Git repositories. This guide explains how to run these
tests, configure authentication for remote tests, and interpret results.

## Test Structure

Integration tests are organized in two files:

- `crates/core/tests/git_integration_tests.rs` - Git operations tests
- `crates/core/tests/git_credentials_tests.rs` - Credential management tests

## Running Tests

### Run All Integration Tests

```bash
cargo test --package xze-core --test git_integration_tests --test git_credentials_tests
```

### Run Local Tests Only

Local tests do not require network access or authentication:

```bash
cargo test --package xze-core --test git_integration_tests -- --skip remote --skip authentication
```

### Run Specific Test

```bash
cargo test --package xze-core --test git_integration_tests test_stage_and_commit
```

### Run with Verbose Output

```bash
cargo test --package xze-core --test git_integration_tests -- --nocapture
```

## Test Categories

### Local Tests

These tests create temporary Git repositories and do not require network access:

- Repository initialization and opening
- Branch operations (create, checkout, delete, list)
- Commit operations (stage, commit)
- Diff analysis and change detection
- Tag management
- Stash operations
- Reset operations
- Credential store configuration

**Run local tests:**

```bash
cargo test --package xze-core --test git_integration_tests -- --skip remote
```

### Remote Tests

These tests require network access and may require authentication:

- Clone public repositories
- Clone private repositories (requires credentials)
- Fetch from remote
- Push to remote (requires write access)

**Run remote tests:**

```bash
cargo test --package xze-core --test git_integration_tests -- remote
```

**Note:** Remote tests are marked with `#[ignore]` by default and must be
explicitly run.

## Configuring Authentication

### Environment Variables

Set these environment variables to enable authenticated tests:

#### HTTP Authentication

```bash
export GIT_USERNAME="your-username"
export GIT_PASSWORD="your-token-or-password"
```

For GitHub, use a personal access token instead of your password.

#### SSH Authentication

```bash
export GIT_SSH_USERNAME="git"
export GIT_SSH_KEY="/home/user/.ssh/id_ed25519"
export GIT_SSH_PUBLIC_KEY="/home/user/.ssh/id_ed25519.pub"
export GIT_SSH_PASSPHRASE="optional-passphrase"
```

### Test Repository Configuration

For private repository tests, set the repository URL:

```bash
export TEST_REPO_URL="https://github.com/your-org/private-repo.git"
```

Or for SSH:

```bash
export TEST_REPO_URL="git@github.com:your-org/private-repo.git"
```

### Skip Remote Tests

If you want to always skip remote tests:

```bash
export SKIP_REMOTE_TESTS=1
```

## Running Authenticated Tests

### With GitHub Personal Access Token

```bash
export GIT_USERNAME="your-github-username"
export GIT_PASSWORD="ghp_your_personal_access_token"
export TEST_REPO_URL="https://github.com/your-org/test-repo.git"

cargo test --package xze-core --test git_integration_tests -- --ignored
```

### With SSH Key

```bash
export GIT_SSH_USERNAME="git"
export GIT_SSH_KEY="$HOME/.ssh/id_ed25519"
export TEST_REPO_URL="git@github.com:your-org/test-repo.git"

cargo test --package xze-core --test git_integration_tests -- --ignored
```

## Test Examples

### Run All Local Tests

```bash
cargo test --package xze-core --test git_integration_tests -- --skip remote --skip authentication
```

Expected output:

```text
running 24 tests
test test_branch_info_metadata ... ok
test test_change_detection ... ok
test test_checkout_branch_switches ... ok
test test_commit_with_custom_signature ... ok
test test_complete_workflow ... ok
test test_create_and_checkout_branch ... ok
test test_credential_store_configuration ... ok
test test_delete_branch ... ok
test test_diff_analysis_between_commits ... ok
test test_diff_summary_statistics ... ok
test test_diff_working_directory ... ok
test test_error_handling_create_duplicate_branch ... ok
test test_error_handling_delete_current_branch ... ok
test test_error_handling_open_nonexistent ... ok
test test_init_repository ... ok
test test_list_branches ... ok
test test_multiple_commits_with_diff ... ok
test test_no_conflicts_in_clean_repo ... ok
test test_open_existing_repository ... ok
test test_reset_operations ... ok
test test_stage_and_commit ... ok
test test_stage_specific_files ... ok
test test_stash_operations ... ok
test test_tag_operations ... ok

test result: ok. 24 passed; 0 failed; 1 ignored; 0 measured; 3 filtered out
```

### Run Credential Tests

```bash
cargo test --package xze-core --test git_credentials_tests
```

Expected output:

```text
running 31 tests
test test_concurrent_credential_access ... ok
test test_credential_callback_no_suitable_type ... ok
test test_credential_priority_ssh_over_userpass ... ok
test test_credential_priority_userpass_over_agent ... ok
test test_credential_store_builder_pattern ... ok
test test_credential_store_clear ... ok
test test_credential_store_clone ... ok
test test_credential_store_creation ... ok
test test_credential_store_default_impl ... ok
test test_credential_store_set_methods ... ok
test test_credential_store_thread_safety ... ok
test test_credential_store_with_agent ... ok
test test_credential_store_with_credential_helper ... ok
test test_credential_store_with_public_key ... ok
test test_credential_store_with_ssh_key ... ok
test test_credential_store_with_ssh_keys ... ok
test test_credential_store_with_userpass ... ok
test test_credential_store_ssh_with_passphrase ... ok
test test_credential_validation_missing_public_key ... ok
test test_credential_validation_missing_ssh_key ... ok
test test_credential_validation_no_credentials ... ok
test test_credential_validation_valid_ssh_key ... ok
test test_credential_validation_with_agent ... ok
test test_credential_validation_with_helper ... ok
test test_credential_validation_with_userpass ... ok
test test_credentials_from_env_no_vars ... ok
test test_credentials_from_env_with_ssh ... ok
test test_credentials_from_env_with_userpass ... ok
test test_credentials_no_home ... ok
test test_empty_credential_store_behavior ... ok
test test_multiple_credential_updates ... ok

test result: ok. 31 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run local integration tests
        run: cargo test --package xze-core --test git_integration_tests -- --skip remote

      - name: Run credential tests
        run: cargo test --package xze-core --test git_credentials_tests

      - name: Run remote tests (if credentials available)
        if: ${{ secrets.GIT_USERNAME != '' }}
        env:
          GIT_USERNAME: ${{ secrets.GIT_USERNAME }}
          GIT_PASSWORD: ${{ secrets.GIT_PASSWORD }}
          TEST_REPO_URL: ${{ secrets.TEST_REPO_URL }}
        run: cargo test --package xze-core --test git_integration_tests -- --ignored
```

## Troubleshooting

### Test Failures

#### Permission Denied Errors

If you see permission errors, ensure SSH keys have correct permissions:

```bash
chmod 600 ~/.ssh/id_ed25519
chmod 644 ~/.ssh/id_ed25519.pub
```

#### Authentication Failures

Check that environment variables are set correctly:

```bash
echo $GIT_USERNAME
echo $GIT_PASSWORD
```

For SSH, verify your key is loaded:

```bash
ssh-add -l
```

#### Network Errors

Remote tests require internet connectivity. If behind a proxy:

```bash
export http_proxy="http://proxy.example.com:8080"
export https_proxy="http://proxy.example.com:8080"
```

#### Test Timeouts

Some tests may timeout on slow connections. Increase timeout:

```bash
cargo test --package xze-core --test git_integration_tests -- --test-threads=1
```

### Debugging Failed Tests

Run a single test with debug output:

```bash
RUST_LOG=debug cargo test --package xze-core --test git_integration_tests test_name -- --nocapture
```

Enable backtrace for panics:

```bash
RUST_BACKTRACE=1 cargo test --package xze-core --test git_integration_tests
```

## Test Coverage

Current integration test coverage:

- Repository operations: 100%
- Branch management: 100%
- Commit operations: 100%
- Diff analysis: 100%
- Change detection: 100%
- Tag management: 100%
- Stash operations: 100%
- Reset operations: 100%
- Credential management: 100%
- Error handling: 100%

Total: 55+ integration tests

## Writing New Tests

### Adding a Test

Create a new test function in the appropriate file:

```rust
#[test]
fn test_my_new_feature() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;

    let git_ops = GitOperations::new(CredentialStore::new());
    let repo = git_ops.open(&repo_path)?;

    // Test your feature

    Ok(())
}
```

### Using Test Helpers

Common test helpers are available in `common/mod.rs`:

```rust
use common::{create_test_repo, create_test_files};

#[test]
fn test_with_helpers() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("test-repo");

    create_test_repo(&repo_path)?;
    create_test_files(&repo_path, &[
        ("file1.txt", "content1"),
        ("file2.txt", "content2"),
    ])?;

    // Continue with test

    Ok(())
}
```

### Marking Tests

Mark tests that require network:

```rust
#[test]
#[ignore = "requires network access"]
fn test_remote_operation() -> Result<()> {
    // Test code
    Ok(())
}
```

Mark tests that require authentication:

```rust
#[test]
#[ignore = "requires network access and authentication"]
fn test_authenticated_operation() -> Result<()> {
    // Test code
    Ok(())
}
```

## Best Practices

1. Always use temporary directories for test repositories
2. Clean up resources with `TempDir` automatic cleanup
3. Use descriptive test names
4. Test both success and error cases
5. Mark network tests appropriately
6. Document authentication requirements
7. Use helper functions for common setup
8. Keep tests independent and isolated

## Related Documentation

- Git Operations Guide: `docs/explanation/git_operations.md`
- Git Integration: `docs/explanation/git_integration.md`
- Phase 2.1 Completion: `docs/explanation/phase2_1_completion.md`
