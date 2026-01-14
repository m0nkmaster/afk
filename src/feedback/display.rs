//! Feedback display for real-time iteration progress.
//!
//! This module provides terminal displays for showing progress,
//! activity metrics, and status information during autonomous loops.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use super::art::{get_mascot, get_spinner_frame};
use super::metrics::{ActivityState, IterationMetrics};

/// Display mode for feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    /// Single-line status bar.
    #[default]
    Minimal,
    /// Multi-panel Rich-like display.
    Full,
}

/// Real-time feedback display for iteration progress.
///
/// Provides either a minimal single-line status bar or a full
/// multi-panel display with ASCII art mascot.
pub struct FeedbackDisplay {
    /// Display mode (minimal or full).
    mode: DisplayMode,
    /// Whether to show the ASCII mascot.
    show_mascot: bool,
    /// Whether the display has been started.
    started: bool,
    /// Start time of the current session.
    start_time: Option<Instant>,
    /// Current spinner frame index.
    spinner_frame: usize,
    /// Current iteration number.
    iteration_current: u32,
    /// Total iterations planned.
    iteration_total: u32,
    /// Current task ID.
    task_id: Option<String>,
    /// Current task description.
    task_description: Option<String>,
    /// Task progress (0.0 to 1.0).
    progress: f32,
    /// Last rendered line count (for clearing).
    last_line_count: usize,
}

impl FeedbackDisplay {
    /// Create a new FeedbackDisplay with default settings.
    pub fn new() -> Self {
        Self {
            mode: DisplayMode::Minimal,
            show_mascot: true,
            started: false,
            start_time: None,
            spinner_frame: 0,
            iteration_current: 0,
            iteration_total: 0,
            task_id: None,
            task_description: None,
            progress: 0.0,
            last_line_count: 0,
        }
    }

    /// Create with specific mode.
    pub fn with_mode(mode: DisplayMode) -> Self {
        Self {
            mode,
            ..Self::new()
        }
    }

    /// Create with mode and mascot settings.
    pub fn with_options(mode: DisplayMode, show_mascot: bool) -> Self {
        Self {
            mode,
            show_mascot,
            ..Self::new()
        }
    }

    /// Start the feedback display.
    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.start_time = Some(Instant::now());
        self.started = true;
        self.last_line_count = 0;
    }

    /// Stop the feedback display.
    pub fn stop(&mut self) {
        if !self.started {
            return;
        }

        // Clear the last rendered content
        self.clear_last_render();
        self.started = false;
    }

    /// Clear the last rendered content.
    fn clear_last_render(&self) {
        if self.last_line_count > 0 {
            // Move cursor up and clear lines
            for _ in 0..self.last_line_count {
                print!("\x1b[1A\x1b[2K"); // Move up, clear line
            }
            let _ = io::stdout().flush();
        }
    }

    /// Format elapsed time as mm:ss.
    fn format_elapsed_time(&self) -> String {
        match self.start_time {
            Some(start) => {
                let elapsed = start.elapsed();
                let total_seconds = elapsed.as_secs();
                let minutes = total_seconds / 60;
                let seconds = total_seconds % 60;
                format!("{:02}:{:02}", minutes, seconds)
            }
            None => String::new(),
        }
    }

    /// Update the display with new metrics.
    ///
    /// # Arguments
    ///
    /// * `metrics` - The current iteration metrics.
    /// * `iteration_current` - Current iteration number (1-indexed).
    /// * `iteration_total` - Total number of iterations planned.
    /// * `task_id` - ID of the current task being worked on.
    /// * `task_description` - Description of the current task.
    /// * `progress` - Task completion percentage (0.0 to 1.0).
    /// * `activity_state` - Current activity state.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        metrics: &IterationMetrics,
        iteration_current: u32,
        iteration_total: u32,
        task_id: Option<&str>,
        task_description: Option<&str>,
        progress: f32,
        activity_state: ActivityState,
    ) {
        if !self.started {
            return;
        }

        // Update state
        self.iteration_current = iteration_current;
        self.iteration_total = iteration_total;
        self.task_id = task_id.map(|s| s.to_string());
        self.task_description = task_description.map(|s| s.to_string());
        self.progress = progress.clamp(0.0, 1.0);

        // Increment spinner frame
        self.spinner_frame = self.spinner_frame.wrapping_add(1);

        // Clear previous content
        self.clear_last_render();

        // Render appropriate mode
        let lines = match self.mode {
            DisplayMode::Minimal => self.render_minimal(metrics, activity_state),
            DisplayMode::Full => self.render_full(metrics, activity_state),
        };

        // Print lines
        for line in &lines {
            println!("{}", line);
        }
        let _ = io::stdout().flush();

        // Track line count for next clear
        self.last_line_count = lines.len();
    }

    /// Render minimal mode single-line status bar.
    ///
    /// Format: ◉ afk [x/y] mm:ss │ ⣾ N calls │ N files │ +N/-N
    fn render_minimal(
        &self,
        metrics: &IterationMetrics,
        activity_state: ActivityState,
    ) -> Vec<String> {
        let mut bar = String::new();

        // Prefix: ◉ afk
        bar.push_str("\x1b[32;1m◉\x1b[0m ");
        bar.push_str("\x1b[1;36mafk\x1b[0m");

        // Iteration count: [x/y]
        if self.iteration_total > 0 {
            bar.push_str(&format!(
                " \x1b[36m[{}/{}]\x1b[0m",
                self.iteration_current, self.iteration_total
            ));
        }

        // Elapsed time: mm:ss
        let elapsed = self.format_elapsed_time();
        if !elapsed.is_empty() {
            bar.push_str(&format!(" \x1b[2m{}\x1b[0m", elapsed));
        }

        // Separator
        bar.push_str(" \x1b[2m│\x1b[0m ");

        // Spinner with colour based on activity state
        let spinner = get_spinner_frame("dots", self.spinner_frame);
        match activity_state {
            ActivityState::Stalled => {
                bar.push_str(&format!("\x1b[31;1m{}\x1b[0m ", spinner));
                bar.push_str("\x1b[31mstalled?\x1b[0m ");
            }
            ActivityState::Thinking => {
                bar.push_str(&format!("\x1b[33;1m{}\x1b[0m ", spinner));
            }
            ActivityState::Active => {
                bar.push_str(&format!("\x1b[36;1m{}\x1b[0m ", spinner));
            }
        }
        bar.push_str(&format!("\x1b[33m{} calls\x1b[0m", metrics.tool_calls));

        // Separator
        bar.push_str(" \x1b[2m│\x1b[0m ");

        // Files count (only changed files, not reads)
        let files_count = metrics.files_changed();
        bar.push_str(&format!("\x1b[34m{} files\x1b[0m", files_count));

        // Separator
        bar.push_str(" \x1b[2m│\x1b[0m ");

        // Line changes
        bar.push_str(&format!(
            "\x1b[32;1m+{}\x1b[0m\x1b[2m/\x1b[0m\x1b[31;1m-{}\x1b[0m",
            metrics.lines_added, metrics.lines_removed
        ));

        vec![bar]
    }

    /// Render full mode multi-panel display.
    fn render_full(
        &self,
        metrics: &IterationMetrics,
        activity_state: ActivityState,
    ) -> Vec<String> {
        let mut lines = Vec::new();

        // Top border
        lines.push("\x1b[36m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m".to_string());

        // Header line
        let mut header = String::from("\x1b[36m│\x1b[0m ");
        header.push_str("\x1b[32;1m◉\x1b[0m ");
        header.push_str("\x1b[1;36mafk\x1b[0m");
        header.push_str(" \x1b[2mrunning...\x1b[0m");

        if self.iteration_total > 0 {
            header.push_str(&format!(
                "  \x1b[36mIteration {}/{}\x1b[0m",
                self.iteration_current, self.iteration_total
            ));
        }

        let elapsed = self.format_elapsed_time();
        if !elapsed.is_empty() {
            header.push_str(&format!("  \x1b[2m{}\x1b[0m", elapsed));
        }

        // Pad to panel width
        let header_visible_len = self.visible_len(&header);
        let padding = 77_usize.saturating_sub(header_visible_len);
        header.push_str(&" ".repeat(padding));
        header.push_str("\x1b[36m│\x1b[0m");
        lines.push(header);

        // Activity section
        lines.push("\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m".to_string());
        lines.extend(self.render_activity_section(metrics, activity_state));

        // Files section
        lines.push("\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m".to_string());
        lines.extend(self.render_files_section(metrics));

        // Mascot section (if enabled)
        if self.show_mascot {
            lines.push("\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m".to_string());
            lines.extend(self.render_mascot_section(activity_state));
        }

        // Task section (if task info available)
        if self.task_id.is_some() {
            lines.push("\x1b[36m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m".to_string());
            lines.extend(self.render_task_section());
        }

        // Bottom border
        lines.push("\x1b[36m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m".to_string());

        lines
    }

    /// Render the activity section.
    fn render_activity_section(
        &self,
        metrics: &IterationMetrics,
        activity_state: ActivityState,
    ) -> Vec<String> {
        let mut lines = Vec::new();

        // Spinner and status line
        let spinner = get_spinner_frame("dots", self.spinner_frame);
        let (spinner_style, state_text, state_style) = match activity_state {
            ActivityState::Stalled => ("\x1b[31;1m", "Connection may be stalled...", "\x1b[31;1m"),
            ActivityState::Thinking => ("\x1b[33;1m", "Thinking", "\x1b[33;1m"),
            ActivityState::Active => ("\x1b[36;1m", "Working", "\x1b[1m"),
        };

        let activity_line = format!(
            "\x1b[36m│\x1b[0m  {}{}\x1b[0m {}{}{}",
            spinner_style, spinner, state_style, state_text, "\x1b[0m"
        );
        lines.push(self.pad_line(&activity_line));

        // Tool calls
        let tools_line = format!(
            "\x1b[36m│\x1b[0m    \x1b[2mTools:\x1b[0m \x1b[33;1m{}\x1b[0m",
            metrics.tool_calls
        );
        lines.push(self.pad_line(&tools_line));

        // Files changed (only created/modified/deleted, not reads)
        let files_count = metrics.files_changed();
        let files_line = format!(
            "\x1b[36m│\x1b[0m    \x1b[2mFiles:\x1b[0m \x1b[34;1m{}\x1b[0m",
            files_count
        );
        lines.push(self.pad_line(&files_line));

        // Lines added/removed
        let lines_line = format!(
            "\x1b[36m│\x1b[0m    \x1b[2mLines:\x1b[0m \x1b[32;1m+{}\x1b[0m \x1b[2m/\x1b[0m \x1b[31;1m-{}\x1b[0m",
            metrics.lines_added, metrics.lines_removed
        );
        lines.push(self.pad_line(&lines_line));

        lines
    }

    /// Render the files section.
    fn render_files_section(&self, metrics: &IterationMetrics) -> Vec<String> {
        let mut lines = Vec::new();

        // Collect recent files (created then modified)
        let mut file_entries: Vec<(&str, &str)> = Vec::new();
        for path in &metrics.files_created {
            file_entries.push(("+", path));
        }
        for path in &metrics.files_modified {
            file_entries.push(("✎", path));
        }

        // Take last 5 files
        let recent_files: Vec<_> = file_entries.iter().rev().take(5).collect();

        if recent_files.is_empty() {
            let empty_line = "\x1b[36m│\x1b[0m    \x1b[2;3mNo files changed yet\x1b[0m".to_string();
            lines.push(self.pad_line(&empty_line));
        } else {
            for (prefix, path) in recent_files.iter().rev() {
                let style = if *prefix == "+" {
                    "\x1b[32;1m"
                } else {
                    "\x1b[33m"
                };
                let truncated_path = self.truncate_path(path, 60);
                let file_line = format!(
                    "\x1b[36m│\x1b[0m    {}{}\x1b[0m \x1b[2m{}\x1b[0m",
                    style, prefix, truncated_path
                );
                lines.push(self.pad_line(&file_line));
            }
        }

        lines
    }

    /// Render the mascot section.
    fn render_mascot_section(&self, activity_state: ActivityState) -> Vec<String> {
        let mut lines = Vec::new();

        // Map activity state to mascot state
        let mascot_state = match activity_state {
            ActivityState::Stalled => "error",
            ActivityState::Thinking => "waiting",
            ActivityState::Active => "working",
        };

        let mascot_art = get_mascot(mascot_state);
        for mascot_line in mascot_art.lines() {
            let line = format!("\x1b[36m│\x1b[0m  \x1b[36m{}\x1b[0m", mascot_line);
            lines.push(self.pad_line(&line));
        }

        lines
    }

    /// Render the task section.
    fn render_task_section(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Task ID
        if let Some(ref task_id) = self.task_id {
            let task_line = format!(
                "\x1b[36m│\x1b[0m    \x1b[2mTask:\x1b[0m \x1b[36;1m{}\x1b[0m",
                task_id
            );
            lines.push(self.pad_line(&task_line));
        }

        // Task description
        if let Some(ref desc) = self.task_description {
            let truncated = if desc.len() > 50 {
                format!("{}...", &desc[..47])
            } else {
                desc.clone()
            };
            let desc_line = format!("\x1b[36m│\x1b[0m    \x1b[2;3m{}\x1b[0m", truncated);
            lines.push(self.pad_line(&desc_line));
        }

        // Progress bar
        let progress_pct = (self.progress * 100.0) as u32;
        let bar_width = 40;
        let filled = ((self.progress * bar_width as f32) as usize).min(bar_width);
        let empty = bar_width - filled;
        let progress_bar = format!(
            "\x1b[36m│\x1b[0m    [\x1b[32m{}\x1b[0m{}] {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            progress_pct
        );
        lines.push(self.pad_line(&progress_bar));

        lines
    }

    /// Pad a line to the panel width.
    fn pad_line(&self, line: &str) -> String {
        let visible_len = self.visible_len(line);
        let padding = 77_usize.saturating_sub(visible_len);
        format!("{}{}\x1b[36m│\x1b[0m", line, " ".repeat(padding))
    }

    /// Calculate visible length (excluding ANSI escape codes).
    fn visible_len(&self, s: &str) -> usize {
        // Strip ANSI escape codes
        let mut len = 0;
        let mut in_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                len += 1;
            }
        }
        len
    }

    /// Truncate a file path to fit within max_length.
    fn truncate_path(&self, path: &str, max_length: usize) -> String {
        if path.len() <= max_length {
            return path.to_string();
        }

        // Split into directory and filename
        if let Some(pos) = path.rfind('/') {
            let (directory, filename) = path.split_at(pos + 1);
            if filename.len() >= max_length - 4 {
                // Filename alone is too long
                return format!(
                    "...{}",
                    &filename[filename.len().saturating_sub(max_length - 3)..]
                );
            }

            // Truncate directory
            let remaining = max_length.saturating_sub(filename.len()).saturating_sub(4);
            if remaining > 0 {
                let dir_truncated = &directory[directory.len().saturating_sub(remaining)..];
                return format!("...{}{}", dir_truncated, filename);
            }
            return format!(".../{}", filename);
        }

        // No directory, just truncate
        format!("{}...", &path[..max_length.saturating_sub(3)])
    }

    // =========================================================================
    // Celebration and feedback methods
    // =========================================================================

    /// Display visual feedback when quality gates fail.
    pub fn show_gates_failed(&self, failed_gates: &[String], continuing: bool) {
        let mut msg = String::new();
        msg.push_str("\x1b[33;1m⚠\x1b[0m ");
        msg.push_str("\x1b[31;1mQuality gates failed:\x1b[0m ");
        msg.push_str(&format!("\x1b[31m{}\x1b[0m", failed_gates.join(", ")));

        if continuing {
            msg.push_str(" \x1b[2m│\x1b[0m ");
            msg.push_str("\x1b[33mContinuing...\x1b[0m");
        }

        println!("{}", msg);
    }

    /// Display visual feedback when quality gates pass.
    pub fn show_gates_passed(&self, gates: &[String]) {
        for gate in gates {
            println!(
                "  \x1b[32;1m✓\x1b[0m \x1b[32m{}\x1b[0m \x1b[2mpassed\x1b[0m",
                gate
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Display a celebration when a task is completed.
    pub fn show_celebration(&self, task_id: &str) {
        let celebration_art = get_mascot("celebration");

        println!();
        println!(
            "\x1b[32m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m"
        );
        println!(
            "\x1b[32m│\x1b[0m                          \x1b[32;1m🎉 Celebration 🎉\x1b[0m                               \x1b[32m│\x1b[0m"
        );
        println!(
            "\x1b[32m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
        );
        println!(
            "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
            "★ ".repeat(16),
            " ".repeat(45 - 32)
        );
        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

        for line in celebration_art.lines() {
            let padded = format!("\x1b[32m│\x1b[0m  \x1b[32;1m{}\x1b[0m", line);
            let visible_len = self.visible_len(&padded);
            let padding = 77_usize.saturating_sub(visible_len);
            println!("{}{}\x1b[32m│\x1b[0m", padded, " ".repeat(padding));
        }

        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
        let msg = format!(
            "  \x1b[32;1m✓ Task Complete!\x1b[0m \x1b[36;1m{}\x1b[0m",
            task_id
        );
        let msg_len = self.visible_len(&msg);
        let msg_padding = 77_usize.saturating_sub(msg_len);
        println!(
            "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
            msg,
            " ".repeat(msg_padding)
        );
        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
        println!(
            "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
            "★ ".repeat(16),
            " ".repeat(45 - 32)
        );
        println!(
            "\x1b[32m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m"
        );
        println!();

        std::thread::sleep(Duration::from_millis(500));
    }

    /// Display a full celebration when the session is complete.
    pub fn show_session_complete(
        &self,
        tasks_completed: u32,
        iterations: u32,
        duration_seconds: f64,
    ) {
        let celebration_art = get_mascot("celebration");
        let total_seconds = duration_seconds as u64;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let duration_str = format!("{}m {}s", minutes, seconds);

        println!();
        println!(
            "\x1b[32m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m"
        );
        println!(
            "\x1b[32m│\x1b[0m                        \x1b[32;1m🎉 Session Complete 🎉\x1b[0m                             \x1b[32m│\x1b[0m"
        );
        println!(
            "\x1b[32m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
        );
        println!(
            "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
            "★ ".repeat(20),
            " ".repeat(77 - 42)
        );
        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

        for line in celebration_art.lines() {
            let padded = format!("\x1b[32m│\x1b[0m  \x1b[32;1m{}\x1b[0m", line);
            let visible_len = self.visible_len(&padded);
            let padding = 77_usize.saturating_sub(visible_len);
            println!("{}{}\x1b[32m│\x1b[0m", padded, " ".repeat(padding));
        }

        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
        println!(
            "\x1b[32m│\x1b[0m  \x1b[32;1m✓ All Tasks Complete!\x1b[0m{}\x1b[32m│\x1b[0m",
            " ".repeat(77 - 24)
        );
        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

        let stats1 = format!(
            "  \x1b[2mTasks completed:\x1b[0m \x1b[36;1m{}\x1b[0m",
            tasks_completed
        );
        let stats1_len = self.visible_len(&stats1);
        println!(
            "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
            stats1,
            " ".repeat(77 - stats1_len)
        );

        let stats2 = format!(
            "  \x1b[2mIterations:\x1b[0m \x1b[36;1m{}\x1b[0m",
            iterations
        );
        let stats2_len = self.visible_len(&stats2);
        println!(
            "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
            stats2,
            " ".repeat(77 - stats2_len)
        );

        let stats3 = format!(
            "  \x1b[2mTotal time:\x1b[0m \x1b[36;1m{}\x1b[0m",
            duration_str
        );
        let stats3_len = self.visible_len(&stats3);
        println!(
            "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
            stats3,
            " ".repeat(77 - stats3_len)
        );

        println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
        println!(
            "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
            "★ ".repeat(20),
            " ".repeat(77 - 42)
        );
        println!(
            "\x1b[32m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m"
        );
        println!();

        std::thread::sleep(Duration::from_millis(500));
    }
}

impl Default for FeedbackDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_display_new() {
        let display = FeedbackDisplay::new();
        assert_eq!(display.mode, DisplayMode::Minimal);
        assert!(display.show_mascot);
        assert!(!display.started);
    }

    #[test]
    fn test_feedback_display_with_mode() {
        let display = FeedbackDisplay::with_mode(DisplayMode::Full);
        assert_eq!(display.mode, DisplayMode::Full);
    }

    #[test]
    fn test_feedback_display_with_options() {
        let display = FeedbackDisplay::with_options(DisplayMode::Full, false);
        assert_eq!(display.mode, DisplayMode::Full);
        assert!(!display.show_mascot);
    }

    #[test]
    fn test_start_sets_started() {
        let mut display = FeedbackDisplay::new();
        assert!(!display.started);
        display.start();
        assert!(display.started);
        assert!(display.start_time.is_some());
    }

    #[test]
    fn test_start_idempotent() {
        let mut display = FeedbackDisplay::new();
        display.start();
        let start_time = display.start_time;
        display.start(); // Should be no-op
        assert_eq!(display.start_time, start_time);
    }

    #[test]
    fn test_stop_clears_started() {
        let mut display = FeedbackDisplay::new();
        display.start();
        assert!(display.started);
        display.stop();
        assert!(!display.started);
    }

    #[test]
    fn test_format_elapsed_time() {
        let display = FeedbackDisplay::new();
        // No start time
        assert_eq!(display.format_elapsed_time(), "");
    }

    #[test]
    fn test_visible_len() {
        let display = FeedbackDisplay::new();

        // Plain text
        assert_eq!(display.visible_len("hello"), 5);

        // With ANSI codes
        assert_eq!(display.visible_len("\x1b[31mred\x1b[0m"), 3);
        assert_eq!(display.visible_len("\x1b[1;32mbold green\x1b[0m"), 10);
    }

    #[test]
    fn test_truncate_path_short() {
        let display = FeedbackDisplay::new();
        assert_eq!(display.truncate_path("src/main.rs", 40), "src/main.rs");
    }

    #[test]
    fn test_truncate_path_long() {
        let display = FeedbackDisplay::new();
        let result = display.truncate_path("very/long/path/to/some/deeply/nested/file.rs", 30);
        assert!(result.len() <= 30 || result.contains("..."));
        assert!(result.contains("file.rs"));
    }

    #[test]
    fn test_render_minimal() {
        let display = FeedbackDisplay::new();
        let metrics = IterationMetrics::default();
        let lines = display.render_minimal(&metrics, ActivityState::Active);

        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("afk"));
    }

    #[test]
    fn test_render_full() {
        let display = FeedbackDisplay::with_options(DisplayMode::Full, true);
        let metrics = IterationMetrics::default();
        let lines = display.render_full(&metrics, ActivityState::Active);

        // Should have multiple lines (borders, sections)
        assert!(lines.len() > 5);
        // Should have top border
        assert!(lines[0].contains("┌"));
        // Should have bottom border
        assert!(lines.last().unwrap().contains("└"));
    }

    #[test]
    fn test_render_with_files() {
        let display = FeedbackDisplay::new();
        let mut metrics = IterationMetrics::default();
        metrics.files_created.insert("src/new.rs".to_string());
        metrics.files_modified.insert("src/main.rs".to_string());

        let lines = display.render_full(&metrics, ActivityState::Active);
        // The files section should be rendered
        let has_files = lines
            .iter()
            .any(|l| l.contains("new.rs") || l.contains("main.rs"));
        assert!(has_files);
    }

    #[test]
    fn test_display_mode_default() {
        let mode = DisplayMode::default();
        assert_eq!(mode, DisplayMode::Minimal);
    }

    // Celebration functions tests
    // Note: These primarily verify the functions don't panic.
    // Visual output is tested manually.

    #[test]
    fn test_show_gates_passed_empty() {
        let display = FeedbackDisplay::new();
        display.show_gates_passed(&[]);
        // Should not panic
    }

    #[test]
    fn test_show_gates_passed_with_gates() {
        let display = FeedbackDisplay::new();
        display.show_gates_passed(&["lint".to_string(), "test".to_string()]);
        // Should not panic
    }

    #[test]
    fn test_show_gates_failed_continuing() {
        let display = FeedbackDisplay::new();
        display.show_gates_failed(&["test".to_string()], true);
        // Should not panic
    }

    #[test]
    fn test_show_gates_failed_not_continuing() {
        let display = FeedbackDisplay::new();
        display.show_gates_failed(&["lint".to_string(), "test".to_string()], false);
        // Should not panic
    }

    #[test]
    fn test_show_celebration() {
        let display = FeedbackDisplay::new();
        // This will print to stdout and sleep briefly
        // We just verify it doesn't panic
        display.show_celebration("test-task-001");
    }

    #[test]
    fn test_show_session_complete() {
        let display = FeedbackDisplay::new();
        // This will print to stdout and sleep briefly
        // We just verify it doesn't panic
        display.show_session_complete(5, 10, 120.5);
    }

    #[test]
    fn test_activity_state_spinner_colors() {
        let display = FeedbackDisplay::new();
        let metrics = IterationMetrics::default();

        // Each activity state should render without panic
        let _ = display.render_minimal(&metrics, ActivityState::Active);
        let _ = display.render_minimal(&metrics, ActivityState::Thinking);
        let _ = display.render_minimal(&metrics, ActivityState::Stalled);
    }

    #[test]
    fn test_full_mode_with_task_info() {
        let mut display = FeedbackDisplay::with_options(DisplayMode::Full, true);
        display.task_id = Some("rust-001".to_string());
        display.task_description = Some("Implement feature X".to_string());
        display.progress = 0.5;

        let metrics = IterationMetrics::default();
        let lines = display.render_full(&metrics, ActivityState::Active);

        // Should include task section
        let has_task = lines.iter().any(|l| l.contains("rust-001"));
        assert!(has_task);
    }
}
