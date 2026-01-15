//! Verify command implementation.
//!
//! This module implements the `afk verify` command for running quality gates.

use crate::config::AfkConfig;
use crate::runner::{has_configured_gates, run_quality_gates};

/// Result type for verify command operations.
pub type VerifyCommandResult = Result<VerifyOutcome, VerifyCommandError>;

/// Outcome of the verify command.
pub struct VerifyOutcome {
    /// Whether all gates passed.
    pub all_passed: bool,
}

/// Error type for verify command operations.
#[derive(Debug, thiserror::Error)]
pub enum VerifyCommandError {
    /// Error loading the configuration file.
    #[error("Failed to load config: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

/// Execute the verify command.
pub fn verify(verbose: bool) -> VerifyCommandResult {
    // Load config
    let config = AfkConfig::load(None)?;

    // Check if any gates are configured
    if !has_configured_gates(&config.feedback_loops) {
        println!("\x1b[33mNo quality gates configured.\x1b[0m");
        println!();
        println!("Configure gates in .afk/config.json:");
        println!("  {{");
        println!("    \"feedbackLoops\": {{");
        println!("      \"lint\": \"cargo clippy\",");
        println!("      \"test\": \"cargo test\"");
        println!("    }}");
        println!("  }}");
        return Ok(VerifyOutcome { all_passed: true });
    }

    // Run quality gates
    let result = run_quality_gates(&config.feedback_loops, verbose);

    Ok(VerifyOutcome {
        all_passed: result.all_passed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_command_error_display() {
        let err = VerifyCommandError::ConfigError(crate::config::ConfigError::ReadError(
            std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        ));
        assert!(err.to_string().contains("Failed to load config"));
    }
}
