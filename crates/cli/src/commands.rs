//! CLI commands module

use xze_core::Result;

pub mod analyze;
pub mod init;
pub mod load;
pub mod serve;
pub mod validate;

pub use analyze::*;
pub use init::*;
pub use load::*;
pub use serve::*;
pub use validate::*;

/// Base trait for CLI commands
#[allow(async_fn_in_trait)]
pub trait CliCommand {
    /// Execute the command
    async fn execute(&self) -> Result<()>;

    /// Get command name for logging
    fn name(&self) -> &'static str;

    /// Validate command arguments
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Common command execution wrapper
pub async fn execute_command<T: CliCommand>(command: T) -> Result<()> {
    tracing::info!("Executing command: {}", command.name());

    // Validate command
    command.validate()?;

    // Execute command
    command.execute().await?;

    tracing::info!("Command {} completed successfully", command.name());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCommand;

    impl CliCommand for TestCommand {
        async fn execute(&self) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[tokio::test]
    async fn test_execute_command() {
        let cmd = TestCommand;
        assert!(execute_command(cmd).await.is_ok());
    }
}
