//! Completions command implementation.
//!
//! This module implements the `afk completions` command for generating shell completions.

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;

/// Result type for completions command operations.
pub type CompletionsCommandResult = Result<(), CompletionsCommandError>;

/// Error type for completions command operations.
#[derive(Debug, thiserror::Error)]
pub enum CompletionsCommandError {
    /// The specified shell is not supported for completions.
    #[error("Unsupported shell: {0}")]
    UnsupportedShell(String),
}

/// Execute the completions command.
pub fn completions(shell: &str) -> CompletionsCommandResult {
    let shell_enum = match shell {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        _ => return Err(CompletionsCommandError::UnsupportedShell(shell.to_string())),
    };

    let mut cmd = Cli::command();
    generate(shell_enum, &mut cmd, "afk", &mut io::stdout());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completions_command_error_display() {
        let err = CompletionsCommandError::UnsupportedShell("powershell".to_string());
        assert!(err.to_string().contains("Unsupported shell"));
        assert!(err.to_string().contains("powershell"));
    }
}
