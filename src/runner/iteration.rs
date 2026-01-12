//! Single iteration execution.
//!
//! This module handles spawning AI CLI, streaming output, and detecting completion signals.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::config::AfkConfig;
use crate::prompt::{PromptError, generate_prompt_with_root};

use super::output_handler::OutputHandler;

/// Result of a single iteration.
#[derive(Debug)]
pub struct IterationResult {
    /// Whether the iteration succeeded.
    pub success: bool,
    /// Task ID if available.
    pub task_id: Option<String>,
    /// Error message if any.
    pub error: Option<String>,
    /// Output from the AI CLI.
    pub output: String,
}

impl IterationResult {
    /// Create a successful result.
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            task_id: None,
            error: None,
            output,
        }
    }

    /// Create a failed result with error.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            task_id: None,
            error: Some(error.into()),
            output: String::new(),
        }
    }

    /// Create a failed result with error and output.
    pub fn failure_with_output(error: impl Into<String>, output: String) -> Self {
        Self {
            success: false,
            task_id: None,
            error: Some(error.into()),
            output,
        }
    }
}

/// Error type for iteration operations.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum IterationError {
    #[error("Failed to generate prompt: {0}")]
    PromptError(#[from] PromptError),
    #[error("Failed to spawn AI CLI: {0}")]
    SpawnError(#[from] std::io::Error),
    #[error("AI CLI not found: {0}")]
    NotFound(String),
    #[error("Iteration timed out")]
    Timeout,
}

/// Handles spawning AI CLI and streaming output.
pub struct IterationRunner {
    config: AfkConfig,
    output: OutputHandler,
    current_iteration: u32,
    max_iterations: u32,
    current_task_id: Option<String>,
    current_task_description: Option<String>,
}

impl IterationRunner {
    /// Create a new IterationRunner.
    pub fn new(config: AfkConfig) -> Self {
        Self {
            config,
            output: OutputHandler::new(),
            current_iteration: 0,
            max_iterations: 0,
            current_task_id: None,
            current_task_description: None,
        }
    }

    /// Create with custom OutputHandler.
    pub fn with_output_handler(config: AfkConfig, output: OutputHandler) -> Self {
        Self {
            config,
            output,
            current_iteration: 0,
            max_iterations: 0,
            current_task_id: None,
            current_task_description: None,
        }
    }

    /// Set context for the current iteration.
    pub fn set_iteration_context(
        &mut self,
        iteration: u32,
        max_iterations: u32,
        task_id: Option<String>,
        task_description: Option<String>,
    ) {
        self.current_iteration = iteration;
        self.max_iterations = max_iterations;
        self.current_task_id = task_id;
        self.current_task_description = task_description;
    }

    /// Run a single iteration.
    ///
    /// # Arguments
    ///
    /// * `iteration` - Current iteration number
    /// * `prompt` - Optional prompt content (generates if None)
    ///
    /// # Returns
    ///
    /// IterationResult with success status and any output.
    pub fn run(&mut self, iteration: u32, prompt: Option<String>) -> IterationResult {
        // Update iteration context if not already set
        if self.current_iteration == 0 {
            self.current_iteration = iteration;
        }

        // Generate prompt if not provided
        let prompt = match prompt {
            Some(p) => p,
            None => match generate_prompt_with_root(&self.config, true, None, None) {
                Ok(result) => result.prompt,
                Err(e) => {
                    return IterationResult::failure(format!("Failed to generate prompt: {e}"));
                }
            },
        };

        // Check for stop signals in prompt
        if prompt.contains("AFK_COMPLETE") {
            return IterationResult {
                success: true,
                task_id: None,
                error: Some("AFK_COMPLETE".to_string()),
                output: String::new(),
            };
        }
        if prompt.contains("AFK_LIMIT_REACHED") {
            return IterationResult {
                success: false,
                task_id: None,
                error: Some("AFK_LIMIT_REACHED".to_string()),
                output: String::new(),
            };
        }

        // Build command
        let mut cmd_parts = vec![self.config.ai_cli.command.clone()];
        cmd_parts.extend(self.config.ai_cli.args.clone());

        self.output.iteration_header(iteration, self.max_iterations);
        self.output.command_info(&cmd_parts);

        self.execute_command(&cmd_parts, &prompt)
    }

    /// Execute AI CLI command and return result.
    fn execute_command(&mut self, cmd_parts: &[String], prompt: &str) -> IterationResult {
        if cmd_parts.is_empty() {
            return IterationResult::failure("No command specified");
        }

        let command = &cmd_parts[0];
        let args: Vec<&str> = cmd_parts[1..].iter().map(|s| s.as_str()).collect();

        // Build full command with prompt as final argument
        let mut cmd = Command::new(command);
        cmd.args(&args)
            .arg(prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Start feedback display (shows live status)
        self.output.set_iteration_context(
            self.current_iteration,
            self.max_iterations,
            self.current_task_id.clone(),
            self.current_task_description.clone(),
        );
        self.output.start_feedback(None);

        // Spawn process
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                self.output.stop_feedback();
                if e.kind() == std::io::ErrorKind::NotFound {
                    return IterationResult::failure(format!(
                        "AI CLI not found: {}. Is it installed and in your PATH?",
                        command
                    ));
                }
                return IterationResult::failure(format!("Failed to spawn AI CLI: {e}"));
            }
        };

        // Stream stdout
        let mut output_buffer = Vec::new();
        let mut completion_detected = false;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let line_with_newline = format!("{line}\n");
                        self.output.stream_line(&line_with_newline);
                        output_buffer.push(line_with_newline.clone());

                        // Check for completion signal
                        if self.output.contains_completion_signal(&line) {
                            completion_detected = true;
                            self.output.completion_detected();
                            // Terminate the process
                            let _ = child.kill();
                            break;
                        }
                    }
                    Err(e) => {
                        self.output.warning(&format!("Error reading output: {e}"));
                        break;
                    }
                }
            }
        }

        // Show iteration summary with stats
        self.output.iteration_summary();

        // Stop feedback display
        self.output.stop_feedback();

        let output = output_buffer.concat();

        if completion_detected {
            return IterationResult::success(output);
        }

        // Wait for process to finish
        match child.wait() {
            Ok(status) => {
                if !status.success() {
                    let exit_code = status.code().unwrap_or(-1);
                    return IterationResult::failure_with_output(
                        format!("AI CLI exited with code {exit_code}"),
                        output,
                    );
                }
                IterationResult::success(output)
            }
            Err(e) => IterationResult::failure_with_output(
                format!("Failed to wait for AI CLI: {e}"),
                output,
            ),
        }
    }

    /// Get a reference to the output handler.
    pub fn output_handler(&self) -> &OutputHandler {
        &self.output
    }

    /// Get a mutable reference to the output handler.
    pub fn output_handler_mut(&mut self) -> &mut OutputHandler {
        &mut self.output
    }
}

/// Run a single iteration with fresh AI context.
///
/// Convenience function that creates an IterationRunner and runs it.
pub fn run_iteration(config: &AfkConfig, iteration: u32) -> IterationResult {
    let mut runner = IterationRunner::new(config.clone());
    runner.run(iteration, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiCliConfig;

    #[test]
    fn test_iteration_result_success() {
        let result = IterationResult::success("Some output".to_string());
        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.task_id.is_none());
        assert_eq!(result.output, "Some output");
    }

    #[test]
    fn test_iteration_result_failure() {
        let result = IterationResult::failure("Something went wrong");
        assert!(!result.success);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert!(result.output.is_empty());
    }

    #[test]
    fn test_iteration_result_failure_with_output() {
        let result = IterationResult::failure_with_output("Error", "partial output".to_string());
        assert!(!result.success);
        assert_eq!(result.error, Some("Error".to_string()));
        assert_eq!(result.output, "partial output");
    }

    #[test]
    fn test_iteration_runner_new() {
        let config = AfkConfig::default();
        let runner = IterationRunner::new(config);
        assert_eq!(runner.current_iteration, 0);
        assert_eq!(runner.max_iterations, 0);
        assert!(runner.current_task_id.is_none());
    }

    #[test]
    fn test_set_iteration_context() {
        let config = AfkConfig::default();
        let mut runner = IterationRunner::new(config);

        runner.set_iteration_context(
            5,
            10,
            Some("task-1".to_string()),
            Some("Description".to_string()),
        );

        assert_eq!(runner.current_iteration, 5);
        assert_eq!(runner.max_iterations, 10);
        assert_eq!(runner.current_task_id, Some("task-1".to_string()));
        assert_eq!(
            runner.current_task_description,
            Some("Description".to_string())
        );
    }

    #[test]
    fn test_run_with_nonexistent_command() {
        let config = AfkConfig {
            ai_cli: AiCliConfig {
                command: "nonexistent_command_that_does_not_exist_12345".to_string(),
                args: vec![],
            },
            ..Default::default()
        };

        let mut runner = IterationRunner::new(config);
        runner.set_iteration_context(1, 10, None, None);

        let result = runner.run(1, Some("test prompt".to_string()));

        assert!(!result.success);
        assert!(result.error.unwrap().contains("AI CLI not found"));
    }

    #[test]
    fn test_run_with_stop_signal_in_prompt() {
        let config = AfkConfig::default();
        let mut runner = IterationRunner::new(config);

        // Prompt contains AFK_COMPLETE
        let result = runner.run(1, Some("AFK_COMPLETE - All tasks done".to_string()));

        assert!(result.success);
        assert_eq!(result.error, Some("AFK_COMPLETE".to_string()));
    }

    #[test]
    fn test_run_with_limit_signal_in_prompt() {
        let config = AfkConfig::default();
        let mut runner = IterationRunner::new(config);

        // Prompt contains AFK_LIMIT_REACHED
        let result = runner.run(1, Some("AFK_LIMIT_REACHED".to_string()));

        assert!(!result.success);
        assert_eq!(result.error, Some("AFK_LIMIT_REACHED".to_string()));
    }

    #[test]
    fn test_execute_command_empty_parts() {
        let config = AfkConfig::default();
        let mut runner = IterationRunner::new(config);

        let result = runner.execute_command(&[], "prompt");

        assert!(!result.success);
        assert!(result.error.unwrap().contains("No command specified"));
    }

    // Note: Integration tests that actually run commands would need
    // a test fixture with a mock AI CLI.
}
