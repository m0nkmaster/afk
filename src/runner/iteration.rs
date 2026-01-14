//! Single iteration execution.
//!
//! This module handles spawning AI CLI, streaming output, and detecting completion signals.
//! Supports both plain text and NDJSON stream-json output formats.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::config::AfkConfig;
use crate::parser::{StreamEvent, StreamJsonParser};
use crate::prompt::{generate_prompt_with_root, PromptError};
use crate::tui::TuiEvent;

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

        Self {
            config,
            output: OutputHandler::new(),
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

        // Build command with output format args
        let mut cmd_parts = vec![self.config.ai_cli.command.clone()];
        cmd_parts.extend(self.config.ai_cli.full_args());

        self.output.iteration_header(iteration, self.max_iterations);
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
                        // Parse and display based on output format
                        if self.stream_parser.is_some() {
                            // NDJSON mode: parse and convert to display text
                            // Falls back to raw line if parsing fails (CLI doesn't support stream-json)
                            if let Some(ref mut parser) = self.stream_parser {
                                if let Some(event) = parser.parse_line(&line) {
                                    // Check for completion signal only in assistant messages
                                    // (not in user messages which may contain the prompt with examples)
                                    if let crate::parser::StreamEvent::AssistantMessage {
                                        ref text,
                                    } = event
                                    {
                                        if self.output.contains_completion_signal(text) {
                                            completion_detected = true;
                                            self.output.completion_detected();
                                            // Terminate the process
                                            let _ = child.kill();
                                            break;
                                        }
                                    }

                                    // Convert event to display text and emit TUI event
                                    let (display, tui_event) = self.stream_event_to_display(&event);

                                    // Send TUI event if we have a sender
                                    if let (Some(ref sender), Some(tui_event)) =
                                        (&self.tui_sender, tui_event)
                                    {
                                        let _ = sender.send(tui_event);
                                    }

                                    if let Some(display) = display {
                                        self.output.stream_line(&format!("{display}\n"));
                                    }
                                } else {
                                    // Parsing failed - display raw line as fallback
                                    // In this case, check raw line for completion signals (plain text mode fallback)
                                    self.output.stream_line(&format!("{line}\n"));
                                    if self.output.contains_completion_signal(&line) {
                                        completion_detected = true;
                                        self.output.completion_detected();
                                        let _ = child.kill();
                                        break;
                                    }
                                }
                            }
                        } else {
                            // Plain text mode: display as-is and check raw line
                            self.output.stream_line(&format!("{line}\n"));
                            if self.output.contains_completion_signal(&line) {
                                completion_detected = true;
                                self.output.completion_detected();
                                // Terminate the process
                                let _ = child.kill();
                                break;
                            }
                        }
                        output_buffer.push(format!("{line}\n"));
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

    /// Convert a StreamEvent to display text and optional TuiEvent.
    fn stream_event_to_display(&self, event: &StreamEvent) -> (Option<String>, Option<TuiEvent>) {
        match event {
            StreamEvent::SystemInit { model, .. } => {
                let display = model
                    .as_ref()
                    .map(|m| format!("\x1b[2m◉ Model: {}\x1b[0m", m));
                (display, None)
            }
            StreamEvent::UserMessage { .. } => {
                // Don't display user message (it's the prompt we sent)
                (None, None)
            }
            StreamEvent::AssistantMessage { text } => {
                // Truncate very long messages for display
                let display_text = if text.len() > 200 {
                    format!("{}...", &text[..197])
                } else {
                    text.clone()
                };
                let display = format!("\x1b[37m{}\x1b[0m", display_text);
                let tui_event = TuiEvent::OutputLine(text.clone());
                (Some(display), Some(tui_event))
            }
            StreamEvent::ToolStarted {
                tool_name,
                tool_type,
                path,
            } => {
                let path_str = path.as_ref().map(|p| format!(" {}", p)).unwrap_or_default();
                let display = format!("\x1b[33m→ {}{}\x1b[0m", tool_type, path_str);
                let tui_event = TuiEvent::ToolCall(tool_name.clone());
                (Some(display), Some(tui_event))
            }
            StreamEvent::ToolCompleted {
                tool_type,
                path,
                success,
                lines,
                ..
            } => {
                let status = if *success { "✓" } else { "✗" };
                let lines_str = lines.map(|l| format!(" ({} lines)", l)).unwrap_or_default();
                let path_str = path.as_ref().map(|p| format!(" {}", p)).unwrap_or_default();
                let colour = if *success { "\x1b[32m" } else { "\x1b[31m" };
                let display = format!(
                    "{}{} {}{}{}\x1b[0m",
                    colour, status, tool_type, path_str, lines_str
                );

                // Emit file change event for file operations
                let tui_event = path.as_ref().map(|p| {
                    let change_type = match tool_type {
                        crate::parser::ToolType::Read => "read",
                        crate::parser::ToolType::Write => "created",
                        crate::parser::ToolType::Edit => "modified",
                        crate::parser::ToolType::Delete => "deleted",
                        _ => "modified",
                    };
                    TuiEvent::FileChange {
                        path: p.clone(),
                        change_type: change_type.to_string(),
                    }
                });

                (Some(display), tui_event)
            }
            StreamEvent::Result {
                success,
                duration_ms,
                ..
            } => {
                let status = if *success {
                    "✓ Complete"
                } else {
                    "✗ Failed"
                };
                let duration_str = duration_ms
                    .map(|ms| format!(" ({:.1}s)", ms as f64 / 1000.0))
                    .unwrap_or_default();
                let colour = if *success { "\x1b[32m" } else { "\x1b[31m" };
                let display = format!("{}{}{}\x1b[0m", colour, status, duration_str);

                let tui_event = duration_ms.map(|ms| TuiEvent::IterationComplete {
                    duration_secs: ms as f64 / 1000.0,
                });

                (Some(display), tui_event)
            }
            StreamEvent::Error { message } => {
                let display = format!("\x1b[31m✗ Error: {}\x1b[0m", message);
                let tui_event = TuiEvent::Error(message.clone());
                (Some(display), Some(tui_event))
            }
            StreamEvent::Unknown { .. } => {
                // Don't display unknown events
                (None, None)
            }
        }
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
