//! Output handler for console output and completion signal detection.
//!
//! This module handles streaming output from AI CLI, detecting completion signals,
//! and displaying status messages.

/// Completion signals to detect in AI output (ralf.sh style).
pub const COMPLETION_SIGNALS: &[&str] = &[
    "<promise>COMPLETE</promise>",
    "AFK_COMPLETE",
    "AFK_STOP",
];

/// Handles console output and completion signal detection.
#[derive(Debug, Default)]
pub struct OutputHandler {
    /// Completion signals to look for.
    signals: Vec<String>,
}

impl OutputHandler {
    /// Create a new OutputHandler with default signals.
    pub fn new() -> Self {
        Self {
            signals: COMPLETION_SIGNALS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create with custom completion signals.
    pub fn with_signals(signals: Vec<String>) -> Self {
        Self { signals }
    }

    /// Check if output contains any completion signal.
    pub fn contains_completion_signal(&self, output: &str) -> bool {
        self.signals.iter().any(|signal| output.contains(signal))
    }

    /// Display iteration header.
    pub fn iteration_header(&self, iteration: u32, max_iterations: u32) {
        println!();
        println!(
            "\x1b[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        );
        println!(
            "\x1b[36m│\x1b[0m \x1b[1mIteration {}/{}\x1b[0m",
            iteration, max_iterations
        );
        println!(
            "\x1b[36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        );
    }

    /// Display command info.
    pub fn command_info(&self, cmd: &[String]) {
        let cmd_str = cmd.join(" ");
        println!("\x1b[2m$ {cmd_str}\x1b[0m");
        println!();
    }

    /// Stream a line of output.
    pub fn stream_line(&self, line: &str) {
        print!("{line}");
    }

    /// Display completion detected message.
    pub fn completion_detected(&self) {
        println!();
        println!("\x1b[32m✓ Completion signal detected\x1b[0m");
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
        println!();
        println!("\x1b[36m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m                           \x1b[1mafk - Ralph Wiggum Mode\x1b[0m                          \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m  Max iterations: {:<57}\x1b[36m│\x1b[0m", max_iterations);
        if !branch.is_empty() {
            println!("\x1b[36m│\x1b[0m  Branch: {:<65}\x1b[36m│\x1b[0m", branch);
        }
        println!("\x1b[36m│\x1b[0m                                                                             \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m  \x1b[2mPress Ctrl+C to stop the loop\x1b[0m                                            \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m");
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
        let duration_mins = duration_seconds / 60.0;

        println!();
        println!("\x1b[32m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[32m│\x1b[0m                              \x1b[1mSession Complete\x1b[0m                              \x1b[32m│\x1b[0m");
        println!("\x1b[32m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[32m│\x1b[0m  Iterations: {:<61}\x1b[32m│\x1b[0m", iterations);
        println!("\x1b[32m│\x1b[0m  Tasks completed: {:<56}\x1b[32m│\x1b[0m", tasks_completed);
        println!("\x1b[32m│\x1b[0m  Duration: {:.1} minutes{:<48}\x1b[32m│\x1b[0m", duration_mins, "");
        println!("\x1b[32m│\x1b[0m  Reason: {:<64}\x1b[32m│\x1b[0m", stop_reason);
        println!("\x1b[32m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    }

    #[test]
    fn test_output_handler_with_custom_signals() {
        let handler = OutputHandler::with_signals(vec!["DONE".to_string()]);
        assert_eq!(handler.signals.len(), 1);
        assert!(handler.contains_completion_signal("DONE"));
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
}
