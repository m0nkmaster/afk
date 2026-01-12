//! Autonomous loop runner.
//!
//! This module implements the Ralph Wiggum pattern for autonomous AI coding.
//! Each iteration spawns a fresh AI CLI instance with clean context.

mod controller;
mod iteration;
mod output_handler;
mod quality_gates;

pub use controller::{run_loop, LoopController};
pub use iteration::{run_iteration, IterationResult, IterationRunner};
pub use output_handler::{OutputHandler, COMPLETION_SIGNALS};
pub use quality_gates::{
    get_configured_gate_names, has_configured_gates, run_quality_gates, GateResult,
    QualityGateResult,
};

/// Reasons for stopping the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// All tasks completed successfully.
    Complete,
    /// Maximum iterations reached.
    MaxIterations,
    /// Session timeout reached.
    Timeout,
    /// No tasks available.
    NoTasks,
    /// User interrupted (Ctrl+C).
    UserInterrupt,
    /// AI CLI error.
    AiError,
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::Complete => write!(f, "All tasks completed"),
            StopReason::MaxIterations => write!(f, "Maximum iterations reached"),
            StopReason::Timeout => write!(f, "Session timeout reached"),
            StopReason::NoTasks => write!(f, "No tasks available"),
            StopReason::UserInterrupt => write!(f, "User interrupted"),
            StopReason::AiError => write!(f, "AI CLI error"),
        }
    }
}

/// Result of running the full loop.
#[derive(Debug)]
pub struct RunResult {
    /// Number of iterations completed.
    pub iterations_completed: u32,
    /// Number of tasks completed in this session.
    pub tasks_completed: u32,
    /// Reason for stopping.
    pub stop_reason: StopReason,
    /// Total duration in seconds.
    pub duration_seconds: f64,
    /// Path to archived session, if any.
    pub archived_to: Option<std::path::PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_reason_display() {
        assert_eq!(StopReason::Complete.to_string(), "All tasks completed");
        assert_eq!(
            StopReason::MaxIterations.to_string(),
            "Maximum iterations reached"
        );
        assert_eq!(
            StopReason::Timeout.to_string(),
            "Session timeout reached"
        );
        assert_eq!(StopReason::NoTasks.to_string(), "No tasks available");
        assert_eq!(StopReason::UserInterrupt.to_string(), "User interrupted");
        assert_eq!(StopReason::AiError.to_string(), "AI CLI error");
    }

    #[test]
    fn test_run_result_fields() {
        let result = RunResult {
            iterations_completed: 5,
            tasks_completed: 2,
            stop_reason: StopReason::Complete,
            duration_seconds: 120.5,
            archived_to: None,
        };

        assert_eq!(result.iterations_completed, 5);
        assert_eq!(result.tasks_completed, 2);
        assert_eq!(result.stop_reason, StopReason::Complete);
        assert_eq!(result.duration_seconds, 120.5);
        assert!(result.archived_to.is_none());
    }
}
