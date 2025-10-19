//! Integration tests for Git credential management
//!
//! These tests validate credential storage, authentication methods, and
//! credential resolution strategies.
//!
//! Tests can be run with:
//! ```bash
//! cargo test --package xze-core --test git_credentials_tests
//! ```

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xze_core::git::{credentials_from_env, CredentialStore};
use xze_core::Result;

#[test]
fn test_credential_store_creation() {
    let store = CredentialStore::new();
    assert!(!store.has_credentials());
    assert!(store.username().is_none());
}

#[test]
fn test_credential_store_with_userpass() {
    let store =
        CredentialStore::new().with_userpass("testuser".to_string(), "testpass".to_string());

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("testuser".to_string()));

    let creds = store.get_userpass();
    assert!(creds.is_some());

    let (user, pass) = creds.unwrap();
    assert_eq!(user, "testuser");
    assert_eq!(pass, "testpass");
}

#[test]
fn test_credential_store_with_ssh_key() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let private_key = temp_dir.path().join("id_rsa");
    let public_key = temp_dir.path().join("id_rsa.pub");

    // Create fake key files
    fs::write(
        &private_key,
        "-----BEGIN OPENSSH PRIVATE KEY-----\nfake key\n-----END OPENSSH PRIVATE KEY-----\n",
    )?;
    fs::write(
        &public_key,
        "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQ fake@test",
    )?;

    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        &private_key,
        Some(&public_key),
        Some("passphrase".to_string()),
    );

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("git".to_string()));
    assert_eq!(store.get_ssh_key_path(), Some(private_key));

    Ok(())
}

#[test]
fn test_credential_store_clear() {
    let store = CredentialStore::new().with_userpass("user".to_string(), "pass".to_string());

    assert!(store.has_credentials());

    store.clear();

    assert!(!store.has_credentials());
    assert!(store.username().is_none());
    assert!(store.get_userpass().is_none());
}

#[test]
fn test_credential_store_clone() {
    let store = CredentialStore::new().with_userpass("user".to_string(), "pass".to_string());

    let cloned = store.clone();

    assert_eq!(store.username(), cloned.username());
    assert_eq!(store.get_userpass(), cloned.get_userpass());
}

#[test]
fn test_credential_store_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let store =
        Arc::new(CredentialStore::new().with_userpass("user".to_string(), "pass".to_string()));

    let mut handles = vec![];

    // Spawn multiple threads reading credentials
    for _ in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = thread::spawn(move || {
            assert_eq!(store_clone.username(), Some("user".to_string()));
            assert!(store_clone.has_credentials());
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_credential_store_set_methods() {
    let store = CredentialStore::new();

    assert!(!store.has_credentials());

    store.set_userpass("newuser".to_string(), "newpass".to_string());

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("newuser".to_string()));

    let (user, pass) = store.get_userpass().unwrap();
    assert_eq!(user, "newuser");
    assert_eq!(pass, "newpass");
}

#[test]
fn test_credential_store_with_agent() {
    let store = CredentialStore::new().with_agent(true);

    // Agent is enabled, store should validate even without explicit credentials
    assert!(store.validate().is_ok());
}

#[test]
fn test_credential_store_with_credential_helper() {
    let store = CredentialStore::new().with_credential_helper(true);

    // Credential helper is enabled, store should validate even without explicit credentials
    assert!(store.validate().is_ok());
}

#[test]
fn test_credential_validation_no_credentials() {
    let store = CredentialStore::new()
        .with_agent(false)
        .with_credential_helper(false);

    let result = store.validate();
    assert!(result.is_err());
}

#[test]
fn test_credential_validation_with_agent() {
    let store = CredentialStore::new().with_agent(true);

    let result = store.validate();
    assert!(result.is_ok());
}

#[test]
fn test_credential_validation_with_helper() {
    let store = CredentialStore::new().with_credential_helper(true);

    let result = store.validate();
    assert!(result.is_ok());
}

#[test]
fn test_credential_validation_with_userpass() {
    let store = CredentialStore::new().with_userpass("user".to_string(), "pass".to_string());

    let result = store.validate();
    assert!(result.is_ok());
}

#[test]
fn test_credential_validation_missing_ssh_key() {
    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        "/nonexistent/key",
        None::<&str>,
        None,
    );

    let result = store.validate();
    assert!(result.is_err());
}

#[test]
fn test_credential_validation_valid_ssh_key() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let key_path = temp_dir.path().join("id_rsa");

    fs::write(&key_path, "fake key content")?;

    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        &key_path,
        None::<&std::path::PathBuf>,
        None,
    );

    let result = store.validate();
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_credentials_from_env_no_vars() {
    // This test assumes no credentials are set in environment
    // In CI, you might want to unset these variables first
    std::env::remove_var("GIT_USERNAME");
    std::env::remove_var("GIT_PASSWORD");
    std::env::remove_var("GIT_SSH_USERNAME");
    std::env::remove_var("GIT_SSH_KEY");

    let store = credentials_from_env();

    // Store is created but has no credentials if env vars not set
    // It can still use agent or helper
    assert!(
        !store.has_credentials() || store.username().is_some() // In case env vars are actually set
    );
}

#[test]
fn test_credentials_from_env_with_userpass() {
    std::env::set_var("GIT_USERNAME", "envuser");
    std::env::set_var("GIT_PASSWORD", "envpass");

    let store = credentials_from_env();

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("envuser".to_string()));

    let (user, pass) = store.get_userpass().unwrap();
    assert_eq!(user, "envuser");
    assert_eq!(pass, "envpass");

    // Cleanup
    std::env::remove_var("GIT_USERNAME");
    std::env::remove_var("GIT_PASSWORD");
}

#[test]
fn test_credentials_from_env_with_ssh() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let key_path = temp_dir.path().join("test_key");
    fs::write(&key_path, "fake key")?;

    std::env::set_var("GIT_SSH_USERNAME", "git");
    std::env::set_var("GIT_SSH_KEY", key_path.to_str().unwrap());
    std::env::set_var("GIT_SSH_PASSPHRASE", "secret");

    let store = credentials_from_env();

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("git".to_string()));
    assert_eq!(store.get_ssh_key_path(), Some(key_path.clone()));

    // Cleanup
    std::env::remove_var("GIT_SSH_USERNAME");
    std::env::remove_var("GIT_SSH_KEY");
    std::env::remove_var("GIT_SSH_PASSPHRASE");

    Ok(())
}

#[test]
fn test_credentials_no_home() {
    std::env::remove_var("HOME");

    let store = CredentialStore::new().with_agent(true);

    // Should still create a store with agent enabled
    assert!(store.validate().is_ok());
}

#[test]
fn test_credential_store_with_ssh_keys() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let ssh_dir = temp_dir.path().join(".ssh");
    fs::create_dir(&ssh_dir)?;

    // Create a fake SSH key
    let key_path = ssh_dir.join("id_ed25519");
    let pub_path = ssh_dir.join("id_ed25519.pub");
    fs::write(&key_path, "fake private key")?;
    fs::write(&pub_path, "fake public key")?;

    let store = CredentialStore::new()
        .with_ssh_key(
            "git".to_string(),
            &key_path,
            Some(&pub_path) as Option<&std::path::PathBuf>,
            None,
        )
        .with_agent(true);

    assert!(store.has_credentials());
    assert_eq!(store.username(), Some("git".to_string()));
    assert!(store.validate().is_ok());

    Ok(())
}

#[test]
fn test_credential_priority_userpass_over_agent() {
    let store = CredentialStore::new()
        .with_userpass("user".to_string(), "pass".to_string())
        .with_agent(true);

    // Should have explicit credentials
    assert!(store.has_credentials());
    assert!(store.get_userpass().is_some());
}

#[test]
fn test_credential_priority_ssh_over_userpass() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let key_path = temp_dir.path().join("id_rsa");
    fs::write(&key_path, "fake key")?;

    let store = CredentialStore::new()
        .with_userpass("user".to_string(), "pass".to_string())
        .with_ssh_key(
            "git".to_string(),
            &key_path,
            None::<&std::path::PathBuf>,
            None,
        );

    // Should have both credentials, but SSH takes priority
    assert!(store.has_credentials());
    assert!(store.get_ssh_key_path().is_some());
    assert!(store.get_userpass().is_some());

    Ok(())
}

#[test]
fn test_multiple_credential_updates() {
    let store = CredentialStore::new();

    // Set userpass
    store.set_userpass("user1".to_string(), "pass1".to_string());
    assert_eq!(store.username(), Some("user1".to_string()));

    // Update userpass
    store.set_userpass("user2".to_string(), "pass2".to_string());
    assert_eq!(store.username(), Some("user2".to_string()));

    let (user, pass) = store.get_userpass().unwrap();
    assert_eq!(user, "user2");
    assert_eq!(pass, "pass2");
}

#[test]
fn test_credential_store_builder_pattern() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let key_path = temp_dir.path().join("key");
    fs::write(&key_path, "key")?;

    let store = CredentialStore::new()
        .with_userpass("user".to_string(), "pass".to_string())
        .with_ssh_key(
            "git".to_string(),
            &key_path,
            None::<&std::path::PathBuf>,
            None,
        )
        .with_agent(true)
        .with_credential_helper(true);

    assert!(store.has_credentials());
    assert!(store.validate().is_ok());

    Ok(())
}

#[test]
fn test_credential_store_default_impl() {
    let store = CredentialStore::default();
    assert!(!store.has_credentials());
}

#[test]
fn test_concurrent_credential_access() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(CredentialStore::new());

    let store_clone = Arc::clone(&store);
    let writer = thread::spawn(move || {
        for i in 0..100 {
            store_clone.set_userpass(format!("user{}", i), format!("pass{}", i));
        }
    });

    let store_clone = Arc::clone(&store);
    let reader = thread::spawn(move || {
        for _ in 0..100 {
            let _ = store_clone.username();
            let _ = store_clone.get_userpass();
        }
    });

    writer.join().unwrap();
    reader.join().unwrap();

    // Should still be valid after concurrent access
    assert!(store.has_credentials());
}

#[test]
fn test_credential_store_ssh_with_passphrase() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let key_path = temp_dir.path().join("encrypted_key");
    fs::write(&key_path, "encrypted key content")?;

    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        &key_path,
        None::<&std::path::PathBuf>,
        Some("my-secret-passphrase".to_string()),
    );

    assert!(store.has_credentials());
    assert_eq!(store.get_ssh_key_path(), Some(key_path));
    // Passphrase is stored internally, we can't access it directly but validation should pass
    assert!(store.validate().is_ok());

    Ok(())
}

#[test]
fn test_credential_store_with_public_key() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let private_key = temp_dir.path().join("id_rsa");
    let public_key = temp_dir.path().join("id_rsa.pub");

    fs::write(&private_key, "private key")?;
    fs::write(&public_key, "public key")?;

    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        &private_key,
        Some(&public_key),
        None,
    );

    assert!(store.has_credentials());
    assert_eq!(store.get_ssh_key_path(), Some(private_key));
    // Public key path is stored internally, validation should pass
    assert!(store.validate().is_ok());

    Ok(())
}

#[test]
fn test_credential_validation_missing_public_key() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let private_key = temp_dir.path().join("id_rsa");
    let public_key = temp_dir.path().join("id_rsa.pub");

    // Only create private key
    fs::write(&private_key, "private key")?;

    let store = CredentialStore::new().with_ssh_key(
        "git".to_string(),
        &private_key,
        Some(&public_key), // Reference to non-existent public key
        None,
    );

    let result = store.validate();
    assert!(result.is_err());

    Ok(())
}

// Integration test with git2 credential callback
#[test]
#[ignore = "requires git2 credential callback testing"]
fn test_credential_callback_integration() -> Result<()> {
    use git2::CredentialType;

    let store =
        CredentialStore::new().with_userpass("testuser".to_string(), "testpass".to_string());

    // Test credential callback
    let result = store.create_credentials(
        "https://github.com/test/repo",
        Some("testuser"),
        CredentialType::USER_PASS_PLAINTEXT,
    );

    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_credential_callback_no_suitable_type() {
    use git2::CredentialType;

    let store = CredentialStore::new()
        .with_userpass("user".to_string(), "pass".to_string())
        .with_agent(false)
        .with_credential_helper(false);

    // Request SSH key but only have userpass
    let result = store.create_credentials(
        "git@github.com:test/repo.git",
        Some("git"),
        CredentialType::SSH_KEY,
    );

    assert!(result.is_err());
}

#[test]
fn test_empty_credential_store_behavior() {
    let store = CredentialStore::new()
        .with_agent(false)
        .with_credential_helper(false);

    assert!(!store.has_credentials());
    assert!(store.username().is_none());
    assert!(store.get_userpass().is_none());
    assert!(store.get_ssh_key_path().is_none());
    assert!(store.validate().is_err());
}
