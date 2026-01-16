//! Autonomous loop runner.
//!
//! This module implements the Ralph Wiggum pattern for autonomous AI coding.
//! Each iteration spawns a fresh AI CLI instance with clean context.

use std::sync::OnceLock;

mod controller;
mod iteration;
mod output_handler;
mod quality_gates;

/// Cached current working directory for path relativisation.
static CWD: OnceLock<String> = OnceLock::new();

/// Strip the current working directory from a path to make it relative.
///
/// If the path starts with the cwd, returns the relative portion.
/// Otherwise returns the original path unchanged.
pub fn make_path_relative(path: &str) -> &str {
    let cwd = CWD.get_or_init(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    if cwd.is_empty() {
        return path;
    }

    // Try stripping cwd with trailing slash
    let cwd_with_slash = format!("{}/", cwd);
    if let Some(relative) = path.strip_prefix(&cwd_with_slash) {
        return relative;
    }

    // Also try without trailing slash (for exact matches)
    if let Some(relative) = path.strip_prefix(cwd.as_str()) {
        // If there's a remaining slash, skip it
        return relative.strip_prefix('/').unwrap_or(relative);
    }

    path
}

pub use controller::{run_loop, run_loop_with_options, run_loop_with_tui, LoopController};
pub use iteration::{run_iteration, IterationResult, IterationRunner};
pub use output_handler::{FeedbackMode, OutputHandler, COMPLETION_SIGNALS};

/// Options for running the loop with feedback display.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Maximum iterations (None uses config default).
    pub max_iterations: Option<u32>,
    /// Run until all tasks complete.
    pub until_complete: bool,
    /// Timeout override in minutes.
    pub timeout_minutes: Option<u32>,
    /// Resume from previous session.
    pub resume: bool,
    /// Feedback display mode.
    pub feedback_mode: FeedbackMode,
    /// Show ASCII mascot in feedback.
    pub show_mascot: bool,
}

impl RunOptions {
    /// Create new options with default feedback (minimal).
    pub fn new() -> Self {
        Self {
            feedback_mode: FeedbackMode::Minimal,
            show_mascot: true,
            ..Default::default()
        }
    }

    /// Set max iterations.
    pub fn with_iterations(mut self, n: Option<u32>) -> Self {
        self.max_iterations = n;
        self
    }

    /// Set until_complete flag.
    pub fn with_until_complete(mut self, until_complete: bool) -> Self {
        self.until_complete = until_complete;
        self
    }

    /// Set timeout override.
    pub fn with_timeout(mut self, minutes: Option<u32>) -> Self {
        self.timeout_minutes = minutes;
        self
    }

    /// Set resume flag.
    pub fn with_resume(mut self, resume: bool) -> Self {
        self.resume = resume;
        self
    }

    /// Set feedback mode.
    pub fn with_feedback_mode(mut self, mode: FeedbackMode) -> Self {
        self.feedback_mode = mode;
        self
    }

    /// Set mascot visibility.
    pub fn with_mascot(mut self, show: bool) -> Self {
        self.show_mascot = show;
        self
    }

    /// Parse feedback mode from string.
    pub fn parse_feedback_mode(s: Option<&str>) -> FeedbackMode {
        match s {
            Some("full") => FeedbackMode::Full,
            Some("minimal") => FeedbackMode::Minimal,
            Some("off") | Some("none") => FeedbackMode::None,
            None => FeedbackMode::Minimal, // Default to minimal for visibility
            _ => FeedbackMode::Minimal,
        }
    }

    /// Check if TUI mode is requested.
    pub fn is_tui_mode(s: Option<&str>) -> bool {
        matches!(s, Some("tui"))
    }
}
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
        assert_eq!(StopReason::Timeout.to_string(), "Session timeout reached");
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

    #[test]
    fn test_make_path_relative_strips_cwd() {
        // The function uses OnceLock so we can test the logic with known paths
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if !cwd.is_empty() {
            let abs_path = format!("{}/src/main.rs", cwd);
            let result = make_path_relative(&abs_path);
            assert_eq!(result, "src/main.rs");
        }
    }

    #[test]
    fn test_make_path_relative_preserves_relative_path() {
        // Relative paths should be returned unchanged
        let result = make_path_relative("src/main.rs");
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn test_make_path_relative_preserves_different_absolute_path() {
        // Paths outside cwd should be returned unchanged
        let result = make_path_relative("/some/other/path/file.rs");
        assert_eq!(result, "/some/other/path/file.rs");
    }
}
