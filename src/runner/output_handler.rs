//! Output handler for console output and completion signal detection.
//!
//! This module handles streaming output from AI CLI, detecting completion signals,
//! displaying status messages, and integrating feedback display with metrics.

use std::path::Path;

use crate::feedback::{
    ActivityState, DisplayMode, FeedbackDisplay, IterationMetrics, MetricsCollector,
};
use crate::parser::{FileChangeType, OutputParser, ParsedEvent};
use crate::watcher::{ChangeType, FileWatcher};

/// Completion signals to detect in AI output (ralf.sh style).
pub const COMPLETION_SIGNALS: &[&str] =
    &["<promise>COMPLETE</promise>", "AFK_COMPLETE", "AFK_STOP"];

/// Feedback mode for the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedbackMode {
    /// No feedback display, just stream output.
    #[default]
    None,
    /// Single-line status bar.
    Minimal,
    /// Full multi-panel display.
    Full,
}

impl FeedbackMode {
    /// Convert to DisplayMode if feedback is enabled.
    pub fn to_display_mode(&self) -> Option<DisplayMode> {
        match self {
            FeedbackMode::None => None,
            FeedbackMode::Minimal => Some(DisplayMode::Minimal),
            FeedbackMode::Full => Some(DisplayMode::Full),
        }
    }
}

/// Handles console output, completion signal detection, and feedback integration.
pub struct OutputHandler {
    /// Completion signals to look for.
    signals: &'static [&'static str],
    /// Feedback display (optional).
    feedback_display: Option<FeedbackDisplay>,
    /// File watcher (optional).
    file_watcher: Option<FileWatcher>,
    /// Output parser for detecting events.
    output_parser: OutputParser,
    /// Metrics collector.
    metrics_collector: MetricsCollector,
    /// Current feedback mode.
    feedback_mode: FeedbackMode,
    /// Whether to show mascot.
    show_mascot: bool,
    /// Current iteration context.
    iteration_current: u32,
    /// Maximum iterations.
    iteration_max: u32,
    /// Current task ID.
    task_id: Option<String>,
    /// Current task description.
    task_description: Option<String>,
    /// Start time for tracking elapsed time.
    start_time: Option<std::time::Instant>,
}

impl OutputHandler {
    /// Create a new OutputHandler with default signals.
    pub fn new() -> Self {
        Self {
            signals: COMPLETION_SIGNALS,
            feedback_display: None,
            file_watcher: None,
            output_parser: OutputParser::new(),
            metrics_collector: MetricsCollector::new(),
            feedback_mode: FeedbackMode::None,
            show_mascot: true,
            iteration_current: 0,
            iteration_max: 0,
            task_id: None,
            task_description: None,
            start_time: None,
        }
    }

    /// Create with custom completion signals.
    pub fn with_signals(signals: &'static [&'static str]) -> Self {
        Self {
            signals,
            ..Self::new()
        }
    }

    /// Create with feedback mode and mascot settings.
    pub fn with_feedback(mode: FeedbackMode, show_mascot: bool) -> Self {
        Self {
            feedback_mode: mode,
            show_mascot,
            ..Self::new()
        }
    }

    /// Set feedback mode.
    pub fn set_feedback_mode(&mut self, mode: FeedbackMode) {
        self.feedback_mode = mode;
    }

    /// Set whether to show mascot.
    pub fn set_show_mascot(&mut self, show: bool) {
        self.show_mascot = show;
    }

    /// Set activity thresholds for the metrics collector.
    ///
    /// # Arguments
    ///
    /// * `active_threshold_secs` - Seconds before transitioning to "thinking" state.
    /// * `thinking_threshold_secs` - Seconds before transitioning to "stalled" state.
    pub fn set_activity_thresholds(
        &mut self,
        active_threshold_secs: u64,
        thinking_threshold_secs: u64,
    ) {
        self.metrics_collector =
            MetricsCollector::with_thresholds(active_threshold_secs, thinking_threshold_secs);
    }

    /// Set iteration context for display updates.
    pub fn set_iteration_context(
        &mut self,
        current: u32,
        max: u32,
        task_id: Option<String>,
        task_description: Option<String>,
    ) {
        self.iteration_current = current;
        self.iteration_max = max;
        self.task_id = task_id;
        self.task_description = task_description;
    }

    /// Start feedback display and file watcher.
    pub fn start_feedback(&mut self, project_root: Option<&Path>) {
        // Reset metrics and start timer
        self.metrics_collector.reset();
        self.start_time = Some(std::time::Instant::now());

        // Create and start feedback display if mode is not None
        if let Some(display_mode) = self.feedback_mode.to_display_mode() {
            let mut display = FeedbackDisplay::with_options(display_mode, self.show_mascot);
            display.start();
            self.feedback_display = Some(display);
        }

        // Create and start file watcher
        if let Some(root) = project_root {
            let mut watcher = FileWatcher::new(root);
            if watcher.start().is_ok() {
                self.file_watcher = Some(watcher);
            }
        }
    }

    /// Stop feedback display and file watcher.
    pub fn stop_feedback(&mut self) {
        // Poll final changes from watcher (with deduplication)
        if let Some(ref watcher) = self.file_watcher {
            let changes = watcher.get_changes();
            for change in changes {
                let change_type = match change.change_type {
                    ChangeType::Created => "created",
                    ChangeType::Modified => "modified",
                    ChangeType::Deleted => "deleted",
                };
                let path_str = change.path.to_string_lossy().to_string();
                let metrics = self.metrics_collector.get_metrics();

                if !self.path_already_recorded(&path_str, metrics) {
                    self.metrics_collector
                        .record_file_change(&path_str, change_type);
                }
            }
        }

        // Stop display
        if let Some(ref mut display) = self.feedback_display {
            display.stop();
        }
        self.feedback_display = None;

        // Stop watcher
        if let Some(ref mut watcher) = self.file_watcher {
            watcher.stop();
        }
        self.file_watcher = None;
    }

    /// Check if output contains any completion signal.
    pub fn contains_completion_signal(&self, output: &str) -> bool {
        self.signals.iter().any(|signal| output.contains(signal))
    }

    /// Display iteration header.
    pub fn iteration_header(&self, iteration: u32, max_iterations: u32) {
        use crate::feedback::get_spinner_frame;

        println!();
        println!(
            "\x1b[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        );

        // Build header line with iteration and optional task info
        let spinner = get_spinner_frame("dots", iteration as usize);
        let mut header = format!(
            "\x1b[36m│\x1b[0m \x1b[36m{}\x1b[0m \x1b[1mIteration {}/{}\x1b[0m",
            spinner, iteration, max_iterations
        );

        // Add task info if available
        if let Some(ref task_id) = self.task_id {
            header.push_str(&format!("  \x1b[2m│\x1b[0m  \x1b[33m{}\x1b[0m", task_id));
        }

        println!("{}", header);

        // Show task description if available
        if let Some(ref desc) = self.task_description {
            let truncated = if desc.len() > 70 {
                format!("{}...", &desc[..67])
            } else {
                desc.clone()
            };
            println!("\x1b[36m│\x1b[0m \x1b[2;3m{}\x1b[0m", truncated);
        }

        println!(
            "\x1b[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        );
    }

    /// Display command info.
    pub fn command_info(&self, cmd: &[String]) {
        let cmd_str = cmd.join(" ");
        println!("\x1b[2m$ {cmd_str}\x1b[0m");
        println!();
        // Show working indicator
        self.show_working_indicator();
    }

    /// Show a working indicator to signal activity is starting.
    fn show_working_indicator(&self) {
        use crate::feedback::get_spinner_frame;

        let spinner = get_spinner_frame("dots", 0);
        println!("\x1b[36m{}\x1b[0m \x1b[2mAI is working...\x1b[0m", spinner);
        println!();
    }

    /// Stream a line of output with parsing and metrics collection.
    ///
    /// This method:
    /// 1. Prints the line to stdout
    /// 2. Parses the line for events (tool calls, file changes, errors)
    /// 3. Records metrics from events
    /// 4. Polls file watcher for changes
    /// 5. Updates feedback display
    pub fn stream_line(&mut self, line: &str) {
        // Print the line
        print!("{line}");

        // Parse line for events
        let events = self.output_parser.parse(line);
        for event in events {
            self.record_event(&event);
        }

        // Poll file watcher
        self.poll_file_watcher();

        // Update feedback display
        self.update_feedback_display();
    }

    /// Stream a line without parsing (simple mode).
    pub fn stream_line_simple(&self, line: &str) {
        print!("{line}");
    }

    /// Record an event in metrics.
    fn record_event(&mut self, event: &ParsedEvent) {
        match event {
            ParsedEvent::ToolCall(e) => {
                self.metrics_collector.record_tool_call(&e.tool_name);
            }
            ParsedEvent::FileChange(e) => {
                let change_type = match e.change_type {
                    FileChangeType::Created => "created",
                    FileChangeType::Modified => "modified",
                    FileChangeType::Deleted => "deleted",
                    FileChangeType::Read => "read",
                };
                self.metrics_collector
                    .record_file_change(&e.file_path, change_type);
            }
            ParsedEvent::Error(_) => {
                self.metrics_collector.record_error();
            }
            ParsedEvent::Warning(_) => {
                self.metrics_collector.record_warning();
            }
        }
    }

    /// Poll the file watcher for changes.
    fn poll_file_watcher(&mut self) {
        if let Some(ref watcher) = self.file_watcher {
            let changes = watcher.get_changes();
            for change in changes {
                let change_type = match change.change_type {
                    ChangeType::Created => "created",
                    ChangeType::Modified => "modified",
                    ChangeType::Deleted => "deleted",
                };
                let path_str = change.path.to_string_lossy().to_string();

                // Only record if not already recorded from parser.
                // The parser records relative paths (e.g., "src/main.rs"), but the
                // watcher records absolute paths (e.g., "/full/path/src/main.rs").
                // We check if the absolute path ends with any already-recorded relative path.
                let metrics = self.metrics_collector.get_metrics();
                let already_recorded = self.path_already_recorded(&path_str, metrics);

                if !already_recorded {
                    self.metrics_collector
                        .record_file_change(&path_str, change_type);
                }
            }
        }
    }

    /// Check if a path has already been recorded (handles absolute vs relative paths).
    fn path_already_recorded(&self, abs_path: &str, metrics: &IterationMetrics) -> bool {
        // Check all categories - a file created then modified should only count once
        let all_paths = metrics
            .files_created
            .iter()
            .chain(metrics.files_modified.iter())
            .chain(metrics.files_deleted.iter());

        for recorded_path in all_paths {
            // Direct match
            if abs_path == recorded_path {
                return true;
            }
            // Absolute path ends with relative path (e.g., "/full/path/src/main.rs" ends with "src/main.rs")
            if abs_path.ends_with(&format!("/{}", recorded_path)) {
                return true;
            }
            // Or the recorded path was already absolute and matches
            if recorded_path.ends_with(&format!("/{}", abs_path.rsplit('/').next().unwrap_or(""))) {
                // This catches cases where paths are stored differently
                if abs_path.ends_with(recorded_path.as_str()) || recorded_path.ends_with(abs_path) {
                    return true;
                }
            }
        }
        false
    }

    /// Update the feedback display.
    fn update_feedback_display(&mut self) {
        if let Some(ref mut display) = self.feedback_display {
            let metrics = self.metrics_collector.get_metrics();
            let activity_state = self.metrics_collector.get_activity_state();

            display.update(
                metrics,
                self.iteration_current,
                self.iteration_max,
                self.task_id.as_deref(),
                self.task_description.as_deref(),
                0.0, // Progress not tracked here
                activity_state,
            );
        }
    }

    /// Display completion detected message.
    pub fn completion_detected(&self) {
        println!();
        println!("\x1b[32m✓ Completion signal detected\x1b[0m");
    }

    /// Display iteration summary with metrics.
    pub fn iteration_summary(&self) {
        let metrics = self.get_metrics();
        let activity = self.get_activity_state();
        let elapsed = self.get_elapsed_time();

        println!();

        // Build summary line with colour-coded stats
        let mut summary = String::new();

        // Activity indicator
        let indicator = match activity {
            ActivityState::Active => "\x1b[32m●\x1b[0m",
            ActivityState::Thinking => "\x1b[33m●\x1b[0m",
            ActivityState::Stalled => "\x1b[31m●\x1b[0m",
        };
        summary.push_str(indicator);
        summary.push(' ');

        // Elapsed time
        if let Some(elapsed) = elapsed {
            let secs = elapsed.as_secs();
            let mins = secs / 60;
            let secs = secs % 60;
            summary.push_str(&format!("\x1b[2m{:02}:{:02}\x1b[0m ", mins, secs));
        }

        // Tool calls
        if metrics.tool_calls > 0 {
            summary.push_str(&format!(
                "\x1b[33m{}\x1b[0m \x1b[2mcalls\x1b[0m  ",
                metrics.tool_calls
            ));
        }

        // Files changed (only created/modified/deleted, not reads)
        let total_files = metrics.files_changed();
        if total_files > 0 {
            summary.push_str(&format!(
                "\x1b[34m{}\x1b[0m \x1b[2mfiles\x1b[0m  ",
                total_files
            ));
        }

        // Lines added/removed
        if metrics.lines_added > 0 || metrics.lines_removed > 0 {
            summary.push_str(&format!(
                "\x1b[32m+{}\x1b[0m\x1b[2m/\x1b[0m\x1b[31m-{}\x1b[0m  ",
                metrics.lines_added, metrics.lines_removed
            ));
        }

        // Errors/warnings
        if metrics.errors > 0 {
            summary.push_str(&format!("\x1b[31m{} errors\x1b[0m  ", metrics.errors));
        }
        if metrics.warnings > 0 {
            summary.push_str(&format!("\x1b[33m{} warnings\x1b[0m  ", metrics.warnings));
        }

        // Print if we have anything to show
        if !summary.trim().is_empty() {
            println!("{}", summary);
        }

        println!(
            "\x1b[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        );
    }

    /// Get elapsed time since feedback started.
    fn get_elapsed_time(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    /// Display error message.
    pub fn error(&self, msg: &str) {
        eprintln!("\x1b[31mError:\x1b[0m {msg}");
    }

    /// Display warning message.
    pub fn warning(&self, msg: &str) {
        eprintln!("\x1b[33mWarning:\x1b[0m {msg}");
    }

    /// Display success message.
    pub fn success(&self, msg: &str) {
        println!("\x1b[32m✓\x1b[0m {msg}");
    }

    /// Display info message.
    pub fn info(&self, msg: &str) {
        println!("\x1b[36mℹ\x1b[0m {msg}");
    }

    /// Display dim message.
    pub fn dim(&self, msg: &str) {
        println!("\x1b[2m{msg}\x1b[0m");
    }

    /// Display loop start panel.
    pub fn loop_start_panel(&self, max_iterations: u32, branch: &str) {
        use crate::feedback::get_mascot;

        println!();
        println!(
            "\x1b[36m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m"
        );
        println!(
            "\x1b[36m│\x1b[0m  \x1b[32;1m◉\x1b[0m \x1b[1;36mafk\x1b[0m \x1b[2m─\x1b[0m \x1b[1mRalph Wiggum Mode\x1b[0m                                               \x1b[36m│\x1b[0m"
        );
        println!(
            "\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
        );

        // Show mascot if enabled
        if self.show_mascot {
            let mascot = get_mascot("working");
            for line in mascot.lines() {
                let padding = 77_usize.saturating_sub(line.chars().count() + 4);
                println!(
                    "\x1b[36m│\x1b[0m  \x1b[33m{}\x1b[0m{}\x1b[36m│\x1b[0m",
                    line,
                    " ".repeat(padding)
                );
            }
            println!(
                "\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
            );
        }

        // Session info
        let iter_display = if max_iterations == u32::MAX {
            "∞ (until complete)".to_string()
        } else {
            format!("{}", max_iterations)
        };
        println!(
            "\x1b[36m│\x1b[0m  \x1b[2mIterations:\x1b[0m  \x1b[1m{:<60}\x1b[36m│\x1b[0m",
            iter_display
        );

        if !branch.is_empty() {
            println!(
                "\x1b[36m│\x1b[0m  \x1b[2mBranch:\x1b[0m      \x1b[34m{:<60}\x1b[36m│\x1b[0m",
                branch
            );
        }

        // Mode indicator
        let mode_str = match self.feedback_mode {
            FeedbackMode::None => "quiet",
            FeedbackMode::Minimal => "minimal",
            FeedbackMode::Full => "full",
        };
        println!(
            "\x1b[36m│\x1b[0m  \x1b[2mFeedback:\x1b[0m    \x1b[35m{:<60}\x1b[36m│\x1b[0m",
            mode_str
        );

        println!(
            "\x1b[36m│\x1b[0m                                                                             \x1b[36m│\x1b[0m"
        );
        println!(
            "\x1b[36m│\x1b[0m  \x1b[2;3mPress Ctrl+C to stop\x1b[0m                                                      \x1b[36m│\x1b[0m"
        );
        println!(
            "\x1b[36m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m"
        );
        println!();
    }

    /// Display session complete panel.
    pub fn session_complete_panel(
        &self,
        iterations: u32,
        tasks_completed: u32,
        duration_seconds: f64,
        stop_reason: &str,
    ) {
        use crate::feedback::get_mascot;

        let duration_mins = duration_seconds / 60.0;

        // Determine colour based on outcome
        // Check for failure conditions first to avoid showing success on partial completion
        let is_error = stop_reason.contains("error")
            || stop_reason.contains("Error")
            || stop_reason.contains("Failed");
        let is_interrupt = stop_reason.contains("interrupt");
        let is_timeout = stop_reason.contains("timeout") || stop_reason.contains("Timeout");

        let (border_colour, icon, title) = if is_error {
            ("\x1b[31m", "✗", "Session Failed") // Red for errors
        } else if is_interrupt {
            ("\x1b[33m", "⚠", "Session Interrupted") // Yellow for interrupt
        } else if is_timeout {
            ("\x1b[33m", "⏱", "Session Timeout") // Yellow for timeout
        } else if tasks_completed > 0 || stop_reason.contains("complete") {
            ("\x1b[32m", "✓", "Session Complete") // Green for success
        } else {
            ("\x1b[36m", "●", "Session Ended") // Cyan for neutral
        };

        // Only show celebration on actual success (not error/interrupt/timeout)
        let show_celebration = !is_error && !is_interrupt && !is_timeout;

        println!();
        println!(
            "{}┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m",
            border_colour
        );
        // Calculate padding for title line
        let title_content = format!("{} {}", icon, title);
        let title_padding = 75_usize.saturating_sub(title_content.chars().count());
        println!(
            "{}│\x1b[0m  \x1b[1m{}\x1b[0m{}{}│\x1b[0m",
            border_colour,
            title_content,
            " ".repeat(title_padding),
            border_colour
        );
        println!(
            "{}├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m",
            border_colour
        );

        // Show celebration mascot only for successful completions (not on error/interrupt/timeout)
        if self.show_mascot && tasks_completed > 0 && show_celebration {
            let mascot = get_mascot("celebration");
            for line in mascot.lines() {
                let padding = 77_usize.saturating_sub(line.chars().count() + 4);
                println!(
                    "{}│\x1b[0m  \x1b[32m{}\x1b[0m{}{}│\x1b[0m",
                    border_colour,
                    line,
                    " ".repeat(padding),
                    border_colour
                );
            }
            println!(
                "{}├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m",
                border_colour
            );
        }

        // Stats
        println!(
            "{}│\x1b[0m  \x1b[2mIterations:\x1b[0m      \x1b[1m{:<55}{}│\x1b[0m",
            border_colour, iterations, border_colour
        );
        println!(
            "{}│\x1b[0m  \x1b[2mTasks completed:\x1b[0m \x1b[32;1m{:<55}{}│\x1b[0m",
            border_colour, tasks_completed, border_colour
        );

        let duration_str = if duration_mins >= 1.0 {
            format!("{:.1} minutes", duration_mins)
        } else {
            format!("{:.0} seconds", duration_seconds)
        };
        println!(
            "{}│\x1b[0m  \x1b[2mDuration:\x1b[0m        {:<55}{}│\x1b[0m",
            border_colour, duration_str, border_colour
        );
        println!(
            "{}│\x1b[0m  \x1b[2mReason:\x1b[0m          {:<55}{}│\x1b[0m",
            border_colour, stop_reason, border_colour
        );
        println!(
            "{}└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m",
            border_colour
        );
        println!();
    }

    /// Show celebration for task completion.
    pub fn show_celebration(&self, task_id: &str) {
        if let Some(ref display) = self.feedback_display {
            display.show_celebration(task_id);
        } else {
            // Fallback to simple message
            println!();
            println!("\x1b[32m✓ Task complete: {}\x1b[0m", task_id);
            println!();
        }
    }

    /// Show quality gates passed.
    pub fn show_gates_passed(&self, gates: &[String]) {
        if let Some(ref display) = self.feedback_display {
            display.show_gates_passed(gates);
        } else {
            for gate in gates {
                println!("  \x1b[32m✓\x1b[0m {} passed", gate);
            }
        }
    }

    /// Show quality gates failed.
    pub fn show_gates_failed(&self, failed_gates: &[String], continuing: bool) {
        if let Some(ref display) = self.feedback_display {
            display.show_gates_failed(failed_gates, continuing);
        } else {
            print!(
                "\x1b[33m⚠\x1b[0m Quality gates failed: \x1b[31m{}\x1b[0m",
                failed_gates.join(", ")
            );
            if continuing {
                print!(" - continuing...");
            }
            println!();
        }
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> &IterationMetrics {
        self.metrics_collector.get_metrics()
    }

    /// Get current activity state.
    pub fn get_activity_state(&self) -> ActivityState {
        self.metrics_collector.get_activity_state()
    }

    /// Reset metrics for new iteration.
    pub fn reset_metrics(&mut self) {
        self.metrics_collector.reset();
    }
}

impl Default for OutputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_completion_signals_not_empty() {
        assert!(!COMPLETION_SIGNALS.is_empty());
    }

    #[test]
    fn test_completion_signals_contains_expected() {
        assert!(COMPLETION_SIGNALS.contains(&"<promise>COMPLETE</promise>"));
        assert!(COMPLETION_SIGNALS.contains(&"AFK_COMPLETE"));
        assert!(COMPLETION_SIGNALS.contains(&"AFK_STOP"));
    }

    #[test]
    fn test_output_handler_new() {
        let handler = OutputHandler::new();
        assert_eq!(handler.signals.len(), COMPLETION_SIGNALS.len());
        assert_eq!(handler.feedback_mode, FeedbackMode::None);
    }

    #[test]
    fn test_output_handler_with_custom_signals() {
        static CUSTOM_SIGNALS: &[&str] = &["DONE"];
        let handler = OutputHandler::with_signals(CUSTOM_SIGNALS);
        assert_eq!(handler.signals.len(), 1);
        assert!(handler.contains_completion_signal("DONE"));
    }

    #[test]
    fn test_output_handler_with_feedback() {
        let handler = OutputHandler::with_feedback(FeedbackMode::Minimal, false);
        assert_eq!(handler.feedback_mode, FeedbackMode::Minimal);
        assert!(!handler.show_mascot);
    }

    #[test]
    fn test_contains_completion_signal_true() {
        let handler = OutputHandler::new();
        assert!(handler.contains_completion_signal("<promise>COMPLETE</promise>"));
        assert!(handler.contains_completion_signal("Some text AFK_COMPLETE more text"));
        assert!(handler.contains_completion_signal("AFK_STOP"));
    }

    #[test]
    fn test_contains_completion_signal_false() {
        let handler = OutputHandler::new();
        assert!(!handler.contains_completion_signal("Hello, world!"));
        assert!(!handler.contains_completion_signal("Doing some work..."));
        assert!(!handler.contains_completion_signal("AFK"));
        assert!(!handler.contains_completion_signal("COMPLETE"));
    }

    #[test]
    fn test_contains_completion_signal_case_sensitive() {
        let handler = OutputHandler::new();
        // Signals are case-sensitive
        assert!(!handler.contains_completion_signal("afk_complete"));
        assert!(!handler.contains_completion_signal("Afk_Complete"));
    }

    #[test]
    fn test_set_feedback_mode() {
        let mut handler = OutputHandler::new();
        assert_eq!(handler.feedback_mode, FeedbackMode::None);

        handler.set_feedback_mode(FeedbackMode::Full);
        assert_eq!(handler.feedback_mode, FeedbackMode::Full);
    }

    #[test]
    fn test_set_iteration_context() {
        let mut handler = OutputHandler::new();
        handler.set_iteration_context(
            5,
            10,
            Some("task-1".to_string()),
            Some("Description".to_string()),
        );

        assert_eq!(handler.iteration_current, 5);
        assert_eq!(handler.iteration_max, 10);
        assert_eq!(handler.task_id, Some("task-1".to_string()));
    }

    #[test]
    fn test_feedback_mode_to_display_mode() {
        assert!(FeedbackMode::None.to_display_mode().is_none());
        assert_eq!(
            FeedbackMode::Minimal.to_display_mode(),
            Some(DisplayMode::Minimal)
        );
        assert_eq!(
            FeedbackMode::Full.to_display_mode(),
            Some(DisplayMode::Full)
        );
    }

    #[test]
    fn test_get_metrics() {
        let handler = OutputHandler::new();
        let metrics = handler.get_metrics();
        assert_eq!(metrics.tool_calls, 0);
    }

    #[test]
    fn test_reset_metrics() {
        let mut handler = OutputHandler::new();
        handler.metrics_collector.record_tool_call("test");
        assert_eq!(handler.get_metrics().tool_calls, 1);

        handler.reset_metrics();
        assert_eq!(handler.get_metrics().tool_calls, 0);
    }

    #[test]
    fn test_feedback_mode_default() {
        let mode = FeedbackMode::default();
        assert_eq!(mode, FeedbackMode::None);
    }

    #[test]
    fn test_path_already_recorded_exact_match() {
        let mut handler = OutputHandler::new();
        handler
            .metrics_collector
            .record_file_change("src/main.rs", "created");

        let metrics = handler.get_metrics();
        assert!(handler.path_already_recorded("src/main.rs", metrics));
    }

    #[test]
    fn test_path_already_recorded_absolute_vs_relative() {
        let mut handler = OutputHandler::new();
        // Parser records a relative path
        handler
            .metrics_collector
            .record_file_change("src/main.rs", "created");

        let metrics = handler.get_metrics();
        // Watcher would see an absolute path - should still match
        assert!(handler.path_already_recorded("/Users/test/project/src/main.rs", metrics));
    }

    #[test]
    fn test_path_already_recorded_different_files() {
        let mut handler = OutputHandler::new();
        handler
            .metrics_collector
            .record_file_change("src/main.rs", "created");

        let metrics = handler.get_metrics();
        // Different file should not match
        assert!(!handler.path_already_recorded("/Users/test/project/src/lib.rs", metrics));
    }

    #[test]
    fn test_path_already_recorded_same_filename_different_dir() {
        let mut handler = OutputHandler::new();
        handler
            .metrics_collector
            .record_file_change("src/main.rs", "created");

        let metrics = handler.get_metrics();
        // Same filename but different directory should not match
        assert!(!handler.path_already_recorded("/Users/test/project/tests/main.rs", metrics));
    }

    #[test]
    fn test_path_already_recorded_across_categories() {
        let mut handler = OutputHandler::new();
        // File was created by parser
        handler
            .metrics_collector
            .record_file_change("script.js", "created");

        let metrics = handler.get_metrics();
        // Watcher sees it as modified - should still be detected as already recorded
        assert!(handler.path_already_recorded("/full/path/to/script.js", metrics));
    }
}
