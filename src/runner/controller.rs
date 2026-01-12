//! Loop controller for managing the autonomous loop.
//!
//! This module implements the main loop lifecycle, including limits,
//! stop conditions, and session management.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::config::AfkConfig;
use crate::prd::{PrdDocument, sync_prd_with_root};

use super::iteration::IterationRunner;
use super::output_handler::{FeedbackMode, OutputHandler};
use super::{RunOptions, RunResult, StopReason};

/// Controls the main loop lifecycle.
pub struct LoopController {
    config: AfkConfig,
    output: OutputHandler,
    iteration_runner: IterationRunner,
    interrupted: Arc<AtomicBool>,
    #[allow(dead_code)]
    feedback_mode: FeedbackMode,
    #[allow(dead_code)]
    show_mascot: bool,
}

impl LoopController {
    /// Create a new LoopController with default feedback (minimal).
    pub fn new(config: AfkConfig) -> Self {
        Self::with_feedback(config, FeedbackMode::Minimal, true)
    }

    /// Create with specific feedback settings.
    pub fn with_feedback(config: AfkConfig, feedback_mode: FeedbackMode, show_mascot: bool) -> Self {
        let mut output = OutputHandler::with_feedback(feedback_mode, show_mascot);
        output.set_feedback_mode(feedback_mode);
        output.set_show_mascot(show_mascot);

        let mut iter_output = OutputHandler::with_feedback(feedback_mode, show_mascot);
        iter_output.set_feedback_mode(feedback_mode);
        iter_output.set_show_mascot(show_mascot);

        let iteration_runner = IterationRunner::with_output_handler(config.clone(), iter_output);

        Self {
            config,
            output,
            iteration_runner,
            interrupted: Arc::new(AtomicBool::new(false)),
            feedback_mode,
            show_mascot,
        }
    }

    /// Create with custom output handler (legacy).
    pub fn with_output(config: AfkConfig, output: OutputHandler) -> Self {
        let iteration_runner =
            IterationRunner::with_output_handler(config.clone(), OutputHandler::new());

        Self {
            config,
            output,
            iteration_runner,
            interrupted: Arc::new(AtomicBool::new(false)),
            feedback_mode: FeedbackMode::None,
            show_mascot: true,
        }
    }

    /// Get the interrupt flag for external signaling.
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        self.interrupted.clone()
    }

    /// Run the autonomous loop.
    ///
    /// # Arguments
    ///
    /// * `max_iterations` - Override for max iterations (uses config default if None)
    /// * `branch` - Branch name to create/checkout
    /// * `until_complete` - If true, run until all tasks done
    /// * `timeout_override` - Override timeout in minutes
    /// * `resume` - If true, continue from last session
    ///
    /// # Returns
    ///
    /// RunResult with session statistics.
    pub fn run(
        &mut self,
        max_iterations: Option<u32>,
        branch: Option<&str>,
        until_complete: bool,
        timeout_override: Option<u32>,
        _resume: bool,
    ) -> RunResult {
        let start_time = Instant::now();

        // Determine effective max iterations
        let max_iter = if until_complete {
            u32::MAX
        } else {
            max_iterations.unwrap_or(self.config.limits.max_iterations)
        };

        // Sync PRD before loop
        let prd = match sync_prd_with_root(&self.config, branch, None) {
            Ok(prd) => prd,
            Err(e) => {
                self.output.error(&format!("Failed to sync PRD: {e}"));
                return RunResult {
                    iterations_completed: 0,
                    tasks_completed: 0,
                    stop_reason: StopReason::AiError,
                    duration_seconds: start_time.elapsed().as_secs_f64(),
                    archived_to: None,
                };
            }
        };

        // Check if there are any tasks
        let pending_stories = prd.get_pending_stories();
        if pending_stories.is_empty() {
            if prd.user_stories.is_empty() {
                self.output.info("No tasks found. Add tasks to continue.");
                return RunResult {
                    iterations_completed: 0,
                    tasks_completed: 0,
                    stop_reason: StopReason::NoTasks,
                    duration_seconds: start_time.elapsed().as_secs_f64(),
                    archived_to: None,
                };
            } else {
                self.output.success("All tasks complete!");
                return RunResult {
                    iterations_completed: 0,
                    tasks_completed: 0,
                    stop_reason: StopReason::Complete,
                    duration_seconds: start_time.elapsed().as_secs_f64(),
                    archived_to: None,
                };
            }
        }

        // Display loop start panel
        self.output.loop_start_panel(max_iter, branch.unwrap_or(""));

        // Get first task info
        let first_task = pending_stories.first();
        let task_id = first_task.map(|t| t.id.clone());
        let task_description = first_task.map(|t| t.title.clone());

        // Set iteration context
        self.iteration_runner
            .set_iteration_context(1, max_iter, task_id, task_description);

        // Main loop
        let result =
            self.run_main_loop(max_iter, until_complete, timeout_override, start_time, &prd);

        // Display session complete panel
        self.output.session_complete_panel(
            result.iterations_completed,
            result.tasks_completed,
            result.duration_seconds,
            &result.stop_reason.to_string(),
        );

        result
    }

    /// Execute the main loop.
    fn run_main_loop(
        &mut self,
        max_iter: u32,
        until_complete: bool,
        timeout_override: Option<u32>,
        start_time: Instant,
        prd: &PrdDocument,
    ) -> RunResult {
        let mut iterations_completed: u32 = 0;
        let mut tasks_completed: u32 = 0;
        #[allow(unused_assignments)]
        let mut stop_reason = StopReason::Complete;

        let timeout_minutes = timeout_override.unwrap_or(self.config.limits.timeout_minutes);
        let timeout_duration = std::time::Duration::from_secs(timeout_minutes as u64 * 60);

        loop {
            // Check for user interrupt
            if self.interrupted.load(Ordering::SeqCst) {
                stop_reason = StopReason::UserInterrupt;
                self.output.info("User interrupted");
                break;
            }

            // Check timeout
            if start_time.elapsed() >= timeout_duration {
                stop_reason = StopReason::Timeout;
                self.output.warning("Session timeout reached");
                break;
            }

            // Check iteration limit
            if iterations_completed >= max_iter {
                stop_reason = StopReason::MaxIterations;
                self.output.info("Maximum iterations reached");
                break;
            }

            // Reload PRD to check completion
            let current_prd = match PrdDocument::load(None) {
                Ok(p) => p,
                Err(_) => prd.clone(),
            };

            // Check if all tasks complete
            if current_prd.all_stories_complete() {
                stop_reason = StopReason::Complete;
                self.output.success("All tasks completed!");
                break;
            }

            // Get next task
            let pending = current_prd.get_pending_stories();
            if pending.is_empty() && !until_complete {
                stop_reason = StopReason::NoTasks;
                self.output.info("No more pending tasks");
                break;
            }

            // Run iteration
            let iteration = iterations_completed + 1;
            let result = self.iteration_runner.run(iteration, None);

            iterations_completed += 1;

            // Handle result
            if !result.success {
                if let Some(ref error) = result.error {
                    if error == "AFK_COMPLETE" {
                        stop_reason = StopReason::Complete;
                        break;
                    } else if error == "AFK_LIMIT_REACHED" {
                        stop_reason = StopReason::MaxIterations;
                        break;
                    } else {
                        self.output.error(error);
                        stop_reason = StopReason::AiError;
                        break;
                    }
                }
            }

            // Check if task was completed (PRD updated)
            let updated_prd = PrdDocument::load(None).unwrap_or(current_prd.clone());
            let old_completed = current_prd.user_stories.iter().filter(|s| s.passes).count();
            let new_completed = updated_prd.user_stories.iter().filter(|s| s.passes).count();
            if new_completed > old_completed {
                tasks_completed += (new_completed - old_completed) as u32;
            }
        }

        // Archive session if interrupted or complete
        let archived_to = if stop_reason == StopReason::UserInterrupt {
            if self.config.archive.enabled {
                match crate::progress::archive_session("interrupted") {
                    Ok(Some(path)) => {
                        self.output
                            .info(&format!("Session archived to: {}", path.display()));
                        Some(path)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        self.output
                            .warning(&format!("Failed to archive session: {e}"));
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        RunResult {
            iterations_completed,
            tasks_completed,
            stop_reason,
            duration_seconds: start_time.elapsed().as_secs_f64(),
            archived_to,
        }
    }

    /// Get reference to output handler.
    pub fn output_handler(&self) -> &OutputHandler {
        &self.output
    }
}

/// Run the autonomous afk loop with options.
///
/// Convenience function that creates a LoopController and runs it with full options.
pub fn run_loop_with_options(config: &AfkConfig, options: RunOptions) -> RunResult {
    let mut controller =
        LoopController::with_feedback(config.clone(), options.feedback_mode, options.show_mascot);

    // Set up Ctrl+C handler
    let interrupt_flag = controller.interrupt_flag();
    let handler_result = ctrlc::set_handler(move || {
        // Set the interrupt flag
        interrupt_flag.store(true, Ordering::SeqCst);
        eprintln!("\n\x1b[33mInterrupting... press Ctrl+C again to force quit\x1b[0m");
    });

    if let Err(e) = handler_result {
        // Non-fatal: just log and continue without handler
        eprintln!("\x1b[2mWarning: Could not set up Ctrl+C handler: {e}\x1b[0m");
    }

    controller.run(
        options.max_iterations,
        options.branch.as_deref(),
        options.until_complete,
        options.timeout_minutes,
        options.resume,
    )
}

/// Run the autonomous afk loop.
///
/// Convenience function that creates a LoopController and runs it.
/// Uses minimal feedback display by default.
pub fn run_loop(
    config: &AfkConfig,
    max_iterations: Option<u32>,
    branch: Option<&str>,
    until_complete: bool,
    timeout_override: Option<u32>,
    resume: bool,
) -> RunResult {
    let options = RunOptions {
        max_iterations,
        branch: branch.map(|s| s.to_string()),
        until_complete,
        timeout_minutes: timeout_override,
        resume,
        feedback_mode: FeedbackMode::Minimal,
        show_mascot: true,
    };
    run_loop_with_options(config, options)
}

/// Run the autonomous afk loop with TUI (Terminal User Interface).
///
/// Provides a rich, animated dashboard showing live AI output,
/// statistics, and progress.
pub fn run_loop_with_tui(config: &AfkConfig, options: RunOptions) -> RunResult {
    use crate::tui::{TuiApp, TuiEvent};
    use crate::watcher::{ChangeType, FileWatcher};
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::thread;

    // Try to create TUI app
    let mut tui_app = match TuiApp::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("\x1b[33mWarning:\x1b[0m Failed to start TUI: {e}");
            eprintln!("Falling back to standard output...");
            return run_loop_with_options(config, options);
        }
    };

    let tx = tui_app.sender();
    let tx_watcher = tx.clone();

    // Clone config and options for the runner thread
    let config_clone = config.clone();
    let options_clone = options.clone();

    // Start file watcher in a separate thread
    let watcher_running = Arc::new(AtomicBool::new(true));
    let watcher_running_clone = watcher_running.clone();
    let watcher_handle = thread::spawn(move || {
        let mut watcher = FileWatcher::new(".");
        if watcher.start().is_ok() {
            while watcher_running_clone.load(AtomicOrdering::SeqCst) {
                // Poll for changes every 200ms
                thread::sleep(std::time::Duration::from_millis(200));
                let changes = watcher.get_changes();
                for change in changes {
                    let change_type = match change.change_type {
                        ChangeType::Created => "created",
                        ChangeType::Modified => "modified",
                        ChangeType::Deleted => "deleted",
                    };
                    let _ = tx_watcher.send(TuiEvent::FileChange {
                        path: change.path.to_string_lossy().to_string(),
                        change_type: change_type.to_string(),
                    });
                }
            }
            watcher.stop();
        }
    });

    // Spawn the runner in a background thread
    let runner_handle = thread::spawn(move || {
        run_loop_with_tui_sender(&config_clone, options_clone, tx)
    });

    // Run TUI in main thread (handles input and rendering)
    if let Err(e) = tui_app.run() {
        eprintln!("TUI error: {e}");
    }

    // Stop file watcher
    watcher_running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = watcher_handle.join();

    // Clean up TUI
    let _ = tui_app.cleanup();

    // Wait for runner to finish
    match runner_handle.join() {
        Ok(result) => result,
        Err(_) => RunResult {
            iterations_completed: 0,
            tasks_completed: 0,
            stop_reason: super::StopReason::AiError,
            duration_seconds: 0.0,
            archived_to: None,
        },
    }
}

/// Run loop with TUI event sender (internal).
fn run_loop_with_tui_sender(
    config: &AfkConfig,
    options: RunOptions,
    tx: std::sync::mpsc::Sender<crate::tui::TuiEvent>,
) -> RunResult {
    use crate::tui::TuiEvent;

    let start_time = Instant::now();

    // Determine effective max iterations
    let max_iter = if options.until_complete {
        u32::MAX
    } else {
        options.max_iterations.unwrap_or(config.limits.max_iterations)
    };

    // Sync PRD before loop
    let prd = match sync_prd_with_root(config, options.branch.as_deref(), None) {
        Ok(prd) => prd,
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Failed to sync PRD: {e}")));
            let _ = tx.send(TuiEvent::SessionComplete {
                iterations: 0,
                tasks: 0,
                duration: start_time.elapsed().as_secs_f64(),
                reason: "PRD sync failed".to_string(),
            });
            return RunResult {
                iterations_completed: 0,
                tasks_completed: 0,
                stop_reason: super::StopReason::AiError,
                duration_seconds: start_time.elapsed().as_secs_f64(),
                archived_to: None,
            };
        }
    };

    // Check if there are any tasks
    let pending_stories = prd.get_pending_stories();
    if pending_stories.is_empty() {
        let reason = if prd.user_stories.is_empty() {
            "No tasks found"
        } else {
            "All tasks complete"
        };
        let _ = tx.send(TuiEvent::SessionComplete {
            iterations: 0,
            tasks: 0,
            duration: start_time.elapsed().as_secs_f64(),
            reason: reason.to_string(),
        });
        return RunResult {
            iterations_completed: 0,
            tasks_completed: 0,
            stop_reason: if prd.user_stories.is_empty() {
                super::StopReason::NoTasks
            } else {
                super::StopReason::Complete
            },
            duration_seconds: start_time.elapsed().as_secs_f64(),
            archived_to: None,
        };
    }

    // Send initial task info
    if let Some(task) = pending_stories.first() {
        let _ = tx.send(TuiEvent::TaskInfo {
            id: task.id.clone(),
            title: task.title.clone(),
        });
    }

    // Main loop
    let mut iterations_completed: u32 = 0;
    let mut tasks_completed: u32 = 0;
    #[allow(unused_assignments)]
    let mut stop_reason = super::StopReason::Complete;

    let timeout_minutes = options.timeout_minutes.unwrap_or(config.limits.timeout_minutes);
    let timeout_duration = std::time::Duration::from_secs(timeout_minutes as u64 * 60);

    loop {
        // Check timeout
        if start_time.elapsed() >= timeout_duration {
            stop_reason = super::StopReason::Timeout;
            break;
        }

        // Check iteration limit
        if iterations_completed >= max_iter {
            stop_reason = super::StopReason::MaxIterations;
            break;
        }

        // Reload PRD to check completion
        let current_prd = match PrdDocument::load(None) {
            Ok(p) => p,
            Err(_) => prd.clone(),
        };

        // Check if all tasks complete
        if current_prd.all_stories_complete() {
            stop_reason = super::StopReason::Complete;
            break;
        }

        // Get next task
        let pending = current_prd.get_pending_stories();
        if pending.is_empty() && !options.until_complete {
            stop_reason = super::StopReason::NoTasks;
            break;
        }

        // Send iteration start event
        let iteration = iterations_completed + 1;
        let _ = tx.send(TuiEvent::IterationStart {
            current: iteration,
            max: max_iter,
        });

        // Update task info
        if let Some(task) = pending.first() {
            let _ = tx.send(TuiEvent::TaskInfo {
                id: task.id.clone(),
                title: task.title.clone(),
            });
        }

        // Run iteration with TUI output
        let iter_start = Instant::now();
        let result = run_iteration_with_tui(config, iteration, tx.clone());

        iterations_completed += 1;

        let _ = tx.send(TuiEvent::IterationComplete {
            duration_secs: iter_start.elapsed().as_secs_f64(),
        });

        // Handle result
        if !result.success {
            if let Some(ref error) = result.error {
                if error == "AFK_COMPLETE" {
                    stop_reason = super::StopReason::Complete;
                    break;
                } else if error == "AFK_LIMIT_REACHED" {
                    stop_reason = super::StopReason::MaxIterations;
                    break;
                } else {
                    let _ = tx.send(TuiEvent::Error(error.clone()));
                    stop_reason = super::StopReason::AiError;
                    break;
                }
            }
        }

        // Check if task was completed
        let updated_prd = PrdDocument::load(None).unwrap_or(current_prd.clone());
        let old_completed = current_prd.user_stories.iter().filter(|s| s.passes).count();
        let new_completed = updated_prd.user_stories.iter().filter(|s| s.passes).count();
        if new_completed > old_completed {
            tasks_completed += (new_completed - old_completed) as u32;
        }
    }

    // Send session complete
    let _ = tx.send(TuiEvent::SessionComplete {
        iterations: iterations_completed,
        tasks: tasks_completed,
        duration: start_time.elapsed().as_secs_f64(),
        reason: stop_reason.to_string(),
    });

    // Give TUI time to receive and display
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = tx.send(TuiEvent::Quit);

    RunResult {
        iterations_completed,
        tasks_completed,
        stop_reason,
        duration_seconds: start_time.elapsed().as_secs_f64(),
        archived_to: None,
    }
}

/// Run a single iteration with TUI output.
fn run_iteration_with_tui(
    config: &AfkConfig,
    _iteration: u32,
    tx: std::sync::mpsc::Sender<crate::tui::TuiEvent>,
) -> super::iteration::IterationResult {
    use crate::prompt::generate_prompt_with_root;
    use crate::tui::TuiEvent;
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Generate prompt
    let prompt = match generate_prompt_with_root(config, true, None, None) {
        Ok(result) => result.prompt,
        Err(e) => {
            return super::iteration::IterationResult::failure(format!(
                "Failed to generate prompt: {e}"
            ));
        }
    };

    // Check for stop signals in prompt
    if prompt.contains("AFK_COMPLETE") {
        return super::iteration::IterationResult {
            success: true,
            task_id: None,
            error: Some("AFK_COMPLETE".to_string()),
            output: String::new(),
        };
    }
    if prompt.contains("AFK_LIMIT_REACHED") {
        return super::iteration::IterationResult {
            success: false,
            task_id: None,
            error: Some("AFK_LIMIT_REACHED".to_string()),
            output: String::new(),
        };
    }

    // Build command
    let mut cmd_parts = vec![config.ai_cli.command.clone()];
    cmd_parts.extend(config.ai_cli.args.clone());

    if cmd_parts.is_empty() {
        return super::iteration::IterationResult::failure("No command specified");
    }

    let command = &cmd_parts[0];
    let args: Vec<&str> = cmd_parts[1..].iter().map(|s| s.as_str()).collect();

    let _ = tx.send(TuiEvent::OutputLine(format!("$ {} {}", command, args.join(" "))));

    // Build and spawn command
    let mut cmd = Command::new(command);
    cmd.args(&args)
        .arg(&prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return super::iteration::IterationResult::failure(format!(
                    "AI CLI not found: {}",
                    command
                ));
            }
            return super::iteration::IterationResult::failure(format!(
                "Failed to spawn AI CLI: {e}"
            ));
        }
    };

    // Stream stdout to TUI
    let mut output_buffer = Vec::new();
    let mut completion_detected = false;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Send to TUI
                    let _ = tx.send(TuiEvent::OutputLine(line.clone()));

                    // Parse for tool calls and file changes
                    // Look for file paths in the output
                    if let Some(path) = extract_file_path(&line) {
                        let change_type = if line.contains("Created") || line.contains("create") || line.contains("write_file") {
                            "created"
                        } else if line.contains("Modified") || line.contains("edit") || line.contains("edit_file") {
                            "modified"
                        } else if line.contains("Deleted") || line.contains("delete") {
                            "deleted"
                        } else {
                            "modified"
                        };
                        let _ = tx.send(TuiEvent::FileChange {
                            path: path.to_string(),
                            change_type: change_type.to_string(),
                        });
                    }

                    // Track tool calls - detect common AI CLI tool invocation patterns
                    let line_lower = line.to_lowercase();
                    if line.contains("antml:invoke") || line.contains("<tool_call>") {
                        // XML-style tool invocations
                        let _ = tx.send(TuiEvent::ToolCall("tool".to_string()));
                    } else if line_lower.contains("edit_file") || line_lower.contains("write_file") 
                        || line_lower.contains("write(") || line_lower.contains("search_replace")
                        || line.contains("Created ") || line.contains("Modified ")
                    {
                        let _ = tx.send(TuiEvent::ToolCall("write".to_string()));
                    } else if line_lower.contains("read_file") || line_lower.contains("read(") 
                        || line.contains("Reading ")
                    {
                        let _ = tx.send(TuiEvent::ToolCall("read".to_string()));
                    } else if line_lower.contains("run_terminal") || line_lower.contains("run_terminal_cmd")
                        || line.contains("$ ") || line.contains("Executing ")
                    {
                        let _ = tx.send(TuiEvent::ToolCall("terminal".to_string()));
                    } else if line_lower.contains("grep") || line_lower.contains("codebase_search")
                        || line_lower.contains("file_search") || line_lower.contains("glob_file_search")
                    {
                        let _ = tx.send(TuiEvent::ToolCall("search".to_string()));
                    } else if line_lower.contains("list_dir") || line_lower.contains("listing ") {
                        let _ = tx.send(TuiEvent::ToolCall("list".to_string()));
                    }

                    output_buffer.push(format!("{line}\n"));

                    // Check for completion signal
                    if line.contains("<promise>COMPLETE</promise>")
                        || line.contains("AFK_COMPLETE")
                        || line.contains("AFK_STOP")
                    {
                        completion_detected = true;
                        let _ = tx.send(TuiEvent::OutputLine("✓ Completion signal detected".to_string()));
                        let _ = child.kill();
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(TuiEvent::Warning(format!("Error reading output: {e}")));
                    break;
                }
            }
        }
    }

    let output = output_buffer.concat();

    if completion_detected {
        return super::iteration::IterationResult::success(output);
    }

    // Wait for process
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                let exit_code = status.code().unwrap_or(-1);
                return super::iteration::IterationResult::failure_with_output(
                    format!("AI CLI exited with code {exit_code}"),
                    output,
                );
            }
            super::iteration::IterationResult::success(output)
        }
        Err(e) => super::iteration::IterationResult::failure_with_output(
            format!("Failed to wait for AI CLI: {e}"),
            output,
        ),
    }
}

/// Extract a file path from an output line.
///
/// Looks for common patterns like:
/// - "file: path/to/file.rs"
/// - "path/to/file.rs"
/// - Quoted paths: "path/to/file.rs" or 'path/to/file.rs'
fn extract_file_path(line: &str) -> Option<&str> {
    // Common file extensions to look for (longer ones first to avoid partial matches)
    let extensions = [
        ".json", ".yaml", ".yml", ".toml", ".scss", ".bash",
        ".tsx", ".jsx", ".html",
        ".rs", ".js", ".ts", ".py", ".go", ".java", ".css",
        ".md", ".txt", ".sh", ".zsh",
    ];

    // Try to find a path with a known extension
    for ext in extensions {
        if let Some(pos) = line.find(ext) {
            // Find the start of the path (work backwards)
            let end = pos + ext.len();
            let before = &line[..pos];

            // Find where the path starts
            let start = before
                .rfind(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '`' || c == ':')
                .map(|p| p + 1)
                .unwrap_or(0);

            let path = line[start..end].trim();

            // Validate it looks like a path
            if !path.is_empty() && (path.contains('/') || path.contains('.')) {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[allow(dead_code)]
    fn setup_test_env(temp: &TempDir) -> std::path::PathBuf {
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        afk_dir
    }

    #[test]
    fn test_loop_controller_new() {
        let config = AfkConfig::default();
        let controller = LoopController::new(config);
        assert!(!controller.interrupted.load(Ordering::SeqCst));
    }

    #[test]
    fn test_loop_controller_interrupt_flag() {
        let config = AfkConfig::default();
        let controller = LoopController::new(config);
        let flag = controller.interrupt_flag();

        assert!(!flag.load(Ordering::SeqCst));
        flag.store(true, Ordering::SeqCst);
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_stop_reason_in_result() {
        let result = RunResult {
            iterations_completed: 0,
            tasks_completed: 0,
            stop_reason: StopReason::NoTasks,
            duration_seconds: 0.0,
            archived_to: None,
        };

        assert_eq!(result.stop_reason, StopReason::NoTasks);
    }

    #[test]
    fn test_extract_file_path_basic() {
        assert_eq!(extract_file_path("Created src/main.rs"), Some("src/main.rs"));
        assert_eq!(extract_file_path("Modified styles.css"), Some("styles.css"));
        assert_eq!(extract_file_path("Reading app.js"), Some("app.js"));
    }

    #[test]
    fn test_extract_file_path_with_quotes() {
        assert_eq!(extract_file_path("file: \"src/lib.rs\""), Some("src/lib.rs"));
        assert_eq!(extract_file_path("editing 'index.html'"), Some("index.html"));
    }

    #[test]
    fn test_extract_file_path_no_match() {
        assert_eq!(extract_file_path("No file path here"), None);
        assert_eq!(extract_file_path("Just some text"), None);
    }

    #[test]
    fn test_extract_file_path_various_extensions() {
        assert_eq!(extract_file_path("file config.json"), Some("config.json"));
        assert_eq!(extract_file_path("file Cargo.toml"), Some("Cargo.toml"));
        assert_eq!(extract_file_path("file script.py"), Some("script.py"));
    }

    // Note: Full integration tests would require mocking the AI CLI
    // or running with a test fixture.
}
