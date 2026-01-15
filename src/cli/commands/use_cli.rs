//! Use command implementation.
//!
//! This module implements the `afk use` command for switching AI CLIs.

use crate::bootstrap::{list_ai_clis, switch_ai_cli};

/// Result type for use command operations.
pub type UseCommandResult = Result<(), UseCommandError>;

/// Error type for use command operations.
#[derive(Debug, thiserror::Error)]
pub enum UseCommandError {
    /// Failed to switch to the specified AI CLI.
    #[error("Failed to switch AI CLI")]
    SwitchFailed,
}

/// Execute the use command.
pub fn use_ai_cli(cli: Option<&str>, list: bool) -> UseCommandResult {
    // List mode
    if list {
        list_ai_clis();
        return Ok(());
    }

    // Switch to specified or prompt interactively
    if switch_ai_cli(cli).is_none() {
        return Err(UseCommandError::SwitchFailed);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_command_error_display() {
        let err = UseCommandError::SwitchFailed;
        assert_eq!(err.to_string(), "Failed to switch AI CLI");
    }
}
