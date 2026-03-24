//! Single iteration execution.
//!
//! This module handles spawning AI CLI, streaming output, and detecting completion signals.
//! Supports both plain text and NDJSON stream-json output formats.

#[cfg(not(feature = "pty"))]
use std::io::{BufRead, BufReader};
#[cfg(not(feature = "pty"))]
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::config::AfkConfig;
use crate::parser::StreamJsonParser;
use crate::prompt::generate_prompt_with_root;
use crate::tui::TuiEvent;

use super::output_handler::OutputHandler;
use super::output_sink::{stream_subprocess_output, ConsoleSink};

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

/// Handles spawning AI CLI and streaming output.
pub struct IterationRunner {
    config: AfkConfig,
    output: OutputHandler,
    current_iteration: u32,
    max_iterations: u32,
    current_task_id: Option<String>,
    current_task_description: Option<String>,
    /// Optional sender for TUI events.
    tui_sender: Option<Sender<TuiEvent>>,
    /// NDJSON parser for stream-json format.
    stream_parser: Option<StreamJsonParser>,
}

impl IterationRunner {
    /// Create a new IterationRunner.
    pub fn new(config: AfkConfig) -> Self {
        let stream_parser = if config.ai_cli.uses_stream_json() {
            Some(StreamJsonParser::new(config.ai_cli.detect_cli_format()))
        } else {
            None
        };

        let mut output = OutputHandler::new();
        output.set_activity_thresholds(
            config.feedback.active_threshold_secs,
            config.feedback.thinking_threshold_secs,
        );

        Self {
            config,
            output,
            current_iteration: 0,
            max_iterations: 0,
            current_task_id: None,
            current_task_description: None,
            tui_sender: None,
            stream_parser,
        }
    }

    /// Create with custom OutputHandler.
    pub fn with_output_handler(config: AfkConfig, output: OutputHandler) -> Self {
        let stream_parser = if config.ai_cli.uses_stream_json() {
            Some(StreamJsonParser::new(config.ai_cli.detect_cli_format()))
        } else {
            None
        };

        Self {
            config,
            output,
            current_iteration: 0,
            max_iterations: 0,
            current_task_id: None,
            current_task_description: None,
            tui_sender: None,
            stream_parser,
        }
    }

    /// Set a TUI event sender for real-time updates.
    pub fn set_tui_sender(&mut self, sender: Sender<TuiEvent>) {
        self.tui_sender = Some(sender);
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

        // Select model upfront so we can display it
        let selected_model = self.config.ai_cli.select_model().map(|s| s.to_string());

        // Build command with output format args and selected model
        let mut cmd_parts = vec![self.config.ai_cli.command.clone()];
        cmd_parts.extend(
            self.config
                .ai_cli
                .full_args_with_model(selected_model.as_deref()),
        );

        self.output.iteration_header(iteration, self.max_iterations);

        // Display model selection if multiple models configured
        if self.config.ai_cli.models.len() > 1 {
            if let Some(ref model) = selected_model {
                self.output.info(&format!(
                    "Model: {} (1 of {})",
                    model,
                    self.config.ai_cli.models.len()
                ));
            }
        }

        self.output.command_info(&cmd_parts);

        // Send TUI iteration start event
        if let Some(ref sender) = self.tui_sender {
            let _ = sender.send(TuiEvent::IterationStart {
                current: iteration,
                max: self.max_iterations,
            });
            if let (Some(id), Some(title)) = (
                self.current_task_id.as_ref(),
                self.current_task_description.as_ref(),
            ) {
                let _ = sender.send(TuiEvent::TaskInfo {
                    id: id.clone(),
                    title: title.clone(),
                });
            }
        }

        self.execute_command(&cmd_parts, &prompt)
    }

    /// Execute AI CLI command and return result.
    fn execute_command(&mut self, cmd_parts: &[String], prompt: &str) -> IterationResult {
        if cmd_parts.is_empty() {
            return IterationResult::failure("No command specified");
        }

        // Start feedback display (shows live status)
        self.output.set_iteration_context(
            self.current_iteration,
            self.max_iterations,
            self.current_task_id.clone(),
            self.current_task_description.clone(),
        );
        self.output.start_feedback(None);

        // Stream stdout through the unified streaming function
        let mut output = String::with_capacity(64 * 1024);
        let mut sink = ConsoleSink {
            output: &mut self.output,
            tui_sender: self.tui_sender.as_ref(),
        };

        #[cfg(feature = "pty")]
        let (completion_detected, stderr_output, wait_result) = {
            use super::pty_spawn::PtyProcess;

            let mut pty = match PtyProcess::spawn(cmd_parts, prompt) {
                Ok(pty) => pty,
                Err(e) => {
                    self.output.stop_feedback();
                    let err_msg = format!("{e}");
                    if err_msg.contains("No such file")
                        || err_msg.contains("not found")
                        || err_msg.contains("not exist")
                    {
                        return IterationResult::failure(format!(
                            "AI CLI not found: {}. Is it installed and in your PATH?",
                            cmd_parts[0]
                        ));
                    }
                    return IterationResult::failure(format!(
                        "Failed to spawn AI CLI via PTY: {e}"
                    ));
                }
            };

            let reader: Box<dyn std::io::BufRead> = pty.take_reader();
            let mut kill = || pty.kill();
            let stream_result = stream_subprocess_output(
                reader,
                &mut kill,
                &mut self.stream_parser,
                &mut sink,
                &mut output,
            );

            // PTY merges stdout+stderr — no separate stderr capture
            let exit_ok = pty.wait();
            (
                stream_result.completion_detected,
                String::new(),
                Ok::<bool, std::io::Error>(exit_ok),
            )
        };

        #[cfg(not(feature = "pty"))]
        let (completion_detected, stderr_output, wait_result) = {
            let command = &cmd_parts[0];
            let args: Vec<&str> = cmd_parts[1..].iter().map(|s| s.as_str()).collect();

            let mut cmd = Command::new(command);
            cmd.args(&args)
                .arg(prompt)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

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

            let stdout = child.stdout.take().expect("stdout was piped");
            let reader: Box<dyn std::io::BufRead> = Box::new(BufReader::new(stdout));
            let mut kill = || {
                let _ = child.kill();
            };
            let stream_result = stream_subprocess_output(
                reader,
                &mut kill,
                &mut self.stream_parser,
                &mut sink,
                &mut output,
            );

            // Capture stderr before waiting for process
            let stderr_output = if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
                lines.join("\n")
            } else {
                String::new()
            };

            let wait_result = child.wait().map(|status| status.success());
            (
                stream_result.completion_detected,
                stderr_output,
                wait_result,
            )
        };

        // Show iteration summary with stats
        self.output.iteration_summary();

        // Stop feedback display
        self.output.stop_feedback();

        if completion_detected {
            return IterationResult::success(output);
        }

        // Check process exit status
        match wait_result {
            Ok(true) => IterationResult::success(output),
            Ok(false) => {
                let error_msg = if stderr_output.is_empty() {
                    "AI CLI exited with non-zero status".to_string()
                } else {
                    format!(
                        "AI CLI exited with non-zero status\n\x1b[31m{}\x1b[0m",
                        stderr_output.trim()
                    )
                };
                if !stderr_output.is_empty() {
                    self.output.error(&error_msg);
                }
                IterationResult::failure_with_output(error_msg, output)
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
                ..Default::default()
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
