//! Limit checking for iterations and failures.
//!
//! This module provides functions to check various limits during the loop.

use super::{SessionProgress, TaskStatus};

/// Signals for limit checking results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitSignal {
    /// No limit reached, can continue.
    Continue,
    /// All tasks are complete (or skipped).
    Complete,
    /// Maximum iterations reached.
    MaxIterations,
    /// Session timeout reached.
    Timeout,
    /// No tasks available.
    NoTasks,
}

impl std::fmt::Display for LimitSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitSignal::Continue => write!(f, "Continue"),
            LimitSignal::Complete => write!(f, "AFK_COMPLETE"),
            LimitSignal::MaxIterations => write!(f, "AFK_LIMIT_REACHED"),
            LimitSignal::Timeout => write!(f, "AFK_TIMEOUT"),
            LimitSignal::NoTasks => write!(f, "AFK_NO_TASKS"),
        }
    }
}

/// Result of limit checking.
#[derive(Debug, Clone)]
pub struct LimitCheckResult {
    /// Whether the loop can continue.
    pub can_continue: bool,
    /// The signal to return if can_continue is false.
    pub signal: LimitSignal,
    /// Tasks that were auto-skipped due to too many failures.
    pub auto_skipped: Vec<String>,
}

/// Check all limits and return whether to continue.
///
/// # Arguments
///
/// * `progress` - Current session progress
/// * `current_iteration` - The current iteration number (1-based)
/// * `max_iterations` - Maximum allowed iterations
/// * `max_task_failures` - Maximum failures before a task is auto-skipped
///
/// # Returns
///
/// LimitCheckResult indicating whether to continue and any signals.
pub fn check_limits(
    progress: &mut SessionProgress,
    current_iteration: u32,
    max_iterations: u32,
    max_task_failures: u32,
) -> LimitCheckResult {
    let mut auto_skipped = Vec::new();

    // Check iteration limit
    if current_iteration > max_iterations {
        return LimitCheckResult {
            can_continue: false,
            signal: LimitSignal::MaxIterations,
            auto_skipped,
        };
    }

    // Check for tasks with too many failures and auto-skip them
    let tasks_to_skip: Vec<String> = progress
        .tasks
        .iter()
        .filter(|(_, task)| {
            task.status != TaskStatus::Completed
                && task.status != TaskStatus::Skipped
                && task.failure_count >= max_task_failures
        })
        .map(|(id, _)| id.clone())
        .collect();

    for task_id in tasks_to_skip {
        progress.set_task_status(
            &task_id,
            TaskStatus::Skipped,
            "auto",
            Some(format!("Auto-skipped after {} failures", max_task_failures)),
        );
        auto_skipped.push(task_id);
    }

    // Check completion (all tracked tasks are completed or skipped)
    if progress.is_complete() && !progress.tasks.is_empty() {
        return LimitCheckResult {
            can_continue: false,
            signal: LimitSignal::Complete,
            auto_skipped,
        };
    }

    LimitCheckResult {
        can_continue: true,
        signal: LimitSignal::Continue,
        auto_skipped,
    }
}

/// Check if a specific task should be skipped.
///
/// Returns true if the task has reached max failures.
pub fn should_skip_task(progress: &SessionProgress, task_id: &str, max_failures: u32) -> bool {
    if let Some(task) = progress.get_task(task_id) {
        task.failure_count >= max_failures
    } else {
        false
    }
}

/// Get the failure count for a task.
pub fn get_failure_count(progress: &SessionProgress, task_id: &str) -> u32 {
    progress
        .get_task(task_id)
        .map(|t| t.failure_count)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::TaskProgress;

    fn create_test_progress() -> SessionProgress {
        let mut progress = SessionProgress::new();
        progress.iterations = 5;
        progress.tasks.insert(
            "task-001".to_string(),
            TaskProgress {
                id: "task-001".to_string(),
                source: "test".to_string(),
                status: TaskStatus::Pending,
                failure_count: 0,
                ..TaskProgress::new("task-001", "test")
            },
        );
        progress.tasks.insert(
            "task-002".to_string(),
            TaskProgress {
                id: "task-002".to_string(),
                source: "test".to_string(),
                status: TaskStatus::Pending,
                failure_count: 2,
                ..TaskProgress::new("task-002", "test")
            },
        );
        progress
    }

    #[test]
    fn test_check_limits_can_continue() {
        let mut progress = create_test_progress();
        let result = check_limits(&mut progress, 1, 10, 3);

        assert!(result.can_continue);
        assert_eq!(result.signal, LimitSignal::Continue);
        assert!(result.auto_skipped.is_empty());
    }

    #[test]
    fn test_check_limits_max_iterations() {
        let mut progress = create_test_progress();
        let result = check_limits(&mut progress, 11, 10, 3);

        assert!(!result.can_continue);
        assert_eq!(result.signal, LimitSignal::MaxIterations);
    }

    #[test]
    fn test_check_limits_auto_skip_failed_task() {
        let mut progress = create_test_progress();

        // Set task-002 to have 3 failures (equals max)
        progress.tasks.get_mut("task-002").unwrap().failure_count = 3;

        let result = check_limits(&mut progress, 1, 10, 3);

        assert!(result.can_continue);
        assert_eq!(result.auto_skipped, vec!["task-002".to_string()]);

        // Verify task was marked as skipped
        let task = progress.get_task("task-002").unwrap();
        assert_eq!(task.status, TaskStatus::Skipped);
    }

    #[test]
    fn test_check_limits_all_complete() {
        let mut progress = SessionProgress::new();
        progress.tasks.insert(
            "task-001".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("task-001", "test")
            },
        );
        progress.tasks.insert(
            "task-002".to_string(),
            TaskProgress {
                status: TaskStatus::Skipped,
                ..TaskProgress::new("task-002", "test")
            },
        );

        let result = check_limits(&mut progress, 1, 10, 3);

        assert!(!result.can_continue);
        assert_eq!(result.signal, LimitSignal::Complete);
    }

    #[test]
    fn test_should_skip_task() {
        let mut progress = create_test_progress();
        progress.tasks.get_mut("task-002").unwrap().failure_count = 3;

        assert!(!should_skip_task(&progress, "task-001", 3));
        assert!(should_skip_task(&progress, "task-002", 3));
        assert!(!should_skip_task(&progress, "nonexistent", 3));
    }

    #[test]
    fn test_get_failure_count() {
        let progress = create_test_progress();

        assert_eq!(get_failure_count(&progress, "task-001"), 0);
        assert_eq!(get_failure_count(&progress, "task-002"), 2);
        assert_eq!(get_failure_count(&progress, "nonexistent"), 0);
    }

    #[test]
    fn test_limit_signal_display() {
        assert_eq!(LimitSignal::Complete.to_string(), "AFK_COMPLETE");
        assert_eq!(LimitSignal::MaxIterations.to_string(), "AFK_LIMIT_REACHED");
        assert_eq!(LimitSignal::Timeout.to_string(), "AFK_TIMEOUT");
        assert_eq!(LimitSignal::NoTasks.to_string(), "AFK_NO_TASKS");
        assert_eq!(LimitSignal::Continue.to_string(), "Continue");
    }
}
