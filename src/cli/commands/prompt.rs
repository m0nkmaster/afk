//! Prompt command implementation.
//!
//! This module implements the `afk prompt` command for previewing prompts.

use crate::cli::output::output_prompt;
use crate::config::{AfkConfig, OutputMode};
use crate::prompt::generate_prompt;

/// Result type for prompt command operations.
pub type PromptCommandResult = Result<(), PromptCommandError>;

/// Error type for prompt command operations.
#[derive(Debug, thiserror::Error)]
pub enum PromptCommandError {
    /// Error generating the prompt from template.
    #[error("Failed to generate prompt: {0}")]
    GenerateError(#[from] crate::prompt::PromptError),
    /// Error outputting the prompt.
    #[error("Output error: {0}")]
    OutputError(#[from] crate::cli::output::OutputError),
}

/// Options for the prompt command.
pub struct PromptOptions {
    /// Copy to clipboard.
    pub copy: bool,
    /// Write to file.
    pub file: bool,
    /// Print to stdout.
    pub stdout: bool,
    /// Include afk command instructions for AI.
    pub bootstrap: bool,
    /// Override max iterations.
    pub limit: Option<u32>,
}

/// Execute the prompt command.
pub fn prompt(options: PromptOptions) -> PromptCommandResult {
    // Load config
    let config = AfkConfig::load(None).unwrap_or_default();

    // Generate the prompt
    let result = generate_prompt(&config, options.bootstrap, options.limit)?;

    // Determine output mode
    let mode = if options.stdout {
        OutputMode::Stdout
    } else if options.file {
        OutputMode::File
    } else if options.copy {
        OutputMode::Clipboard
    } else {
        config.output.default.clone()
    };

    // Output the prompt
    let is_stdout = mode == OutputMode::Stdout;
    let _ = output_prompt(&result.prompt, mode, &config);

    // Show info unless going to stdout
    if !options.stdout && !is_stdout {
        println!("\x1b[2mIteration {}\x1b[0m", result.iteration);
        if result.all_complete {
            println!("\x1b[32m✓ All tasks complete!\x1b[0m");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_command_error_display() {
        let err = PromptCommandError::OutputError(crate::cli::output::OutputError::ClipboardError(
            "test".to_string(),
        ));
        assert!(err.to_string().contains("Output error"));
    }
}
