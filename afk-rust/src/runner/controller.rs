//! Loop controller for managing the autonomous loop.
//!
//! This module implements the main loop lifecycle, including limits,
//! stop conditions, and session management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::config::AfkConfig;
use crate::prd::{sync_prd_with_root, PrdDocument};

use super::iteration::IterationRunner;
use super::output_handler::OutputHandler;
use super::{RunResult, StopReason};

/// Controls the main loop lifecycle.
pub struct LoopController {
    config: AfkConfig,
    output: OutputHandler,
    iteration_runner: IterationRunner,
    interrupted: Arc<AtomicBool>,
}

impl LoopController {
    /// Create a new LoopController.
    pub fn new(config: AfkConfig) -> Self {
        let output = OutputHandler::new();
        let iteration_runner = IterationRunner::new(config.clone());

        Self {
            config,
            output,
            iteration_runner,
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with custom output handler.
    pub fn with_output(config: AfkConfig, output: OutputHandler) -> Self {
        let iteration_runner = IterationRunner::with_output_handler(config.clone(), OutputHandler::new());

        Self {
            config,
            output,
            iteration_runner,
            interrupted: Arc::new(AtomicBool::new(false)),
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
        resume: bool,
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
        self.iteration_runner.set_iteration_context(1, max_iter, task_id, task_description);

        // Main loop
        let result = self.run_main_loop(max_iter, until_complete, timeout_override, start_time, &prd);

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

        RunResult {
            iterations_completed,
            tasks_completed,
            stop_reason,
            duration_seconds: start_time.elapsed().as_secs_f64(),
            archived_to: None,
        }
    }

    /// Get reference to output handler.
    pub fn output_handler(&self) -> &OutputHandler {
        &self.output
    }
}

/// Run the autonomous afk loop.
///
/// Convenience function that creates a LoopController and runs it.
pub fn run_loop(
    config: &AfkConfig,
    max_iterations: Option<u32>,
    branch: Option<&str>,
    until_complete: bool,
    timeout_override: Option<u32>,
    resume: bool,
) -> RunResult {
    let mut controller = LoopController::new(config.clone());
    controller.run(max_iterations, branch, until_complete, timeout_override, resume)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::UserStory;
    use std::fs;
    use tempfile::TempDir;

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

    // Note: Full integration tests would require mocking the AI CLI
    // or running with a test fixture.
}
