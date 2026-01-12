//! Metrics collection for iteration statistics.
//!
//! This module provides the MetricsCollector for tracking tool calls,
//! file changes, errors, and warnings during AI CLI execution.

use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Activity state based on time since last activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityState {
    /// Recent activity (within 2 seconds).
    Active,
    /// No activity for 2-10 seconds.
    Thinking,
    /// No activity for more than 10 seconds.
    Stalled,
}

/// Metrics for a single iteration.
#[derive(Debug, Clone, Default)]
pub struct IterationMetrics {
    /// Number of tool calls made.
    pub tool_calls: u32,
    /// Set of files that were modified.
    pub files_modified: HashSet<String>,
    /// Set of files that were created.
    pub files_created: HashSet<String>,
    /// Set of files that were deleted.
    pub files_deleted: HashSet<String>,
    /// Set of files that were read.
    pub files_read: HashSet<String>,
    /// Number of lines added (estimated).
    pub lines_added: u32,
    /// Number of lines removed (estimated).
    pub lines_removed: u32,
    /// Number of errors detected.
    pub errors: u32,
    /// Number of warnings detected.
    pub warnings: u32,
}

impl IterationMetrics {
    /// Create new empty metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of file operations.
    pub fn total_file_ops(&self) -> usize {
        self.files_modified.len()
            + self.files_created.len()
            + self.files_deleted.len()
            + self.files_read.len()
    }
}

/// Activity thresholds in seconds.
const ACTIVE_THRESHOLD_SECS: u64 = 2;
const THINKING_THRESHOLD_SECS: u64 = 10;

/// Collector for iteration metrics.
#[derive(Debug)]
pub struct MetricsCollector {
    /// Current iteration metrics.
    metrics: IterationMetrics,
    /// Time of last recorded activity.
    last_activity: Option<Instant>,
}

impl MetricsCollector {
    /// Create a new MetricsCollector.
    pub fn new() -> Self {
        Self {
            metrics: IterationMetrics::new(),
            last_activity: None,
        }
    }

    /// Record a tool call.
    pub fn record_tool_call(&mut self, _tool_name: &str) {
        self.metrics.tool_calls += 1;
        self.last_activity = Some(Instant::now());
    }

    /// Record a file change.
    pub fn record_file_change(&mut self, path: &str, change_type: &str) {
        match change_type {
            "created" => {
                self.metrics.files_created.insert(path.to_string());
            }
            "modified" => {
                self.metrics.files_modified.insert(path.to_string());
            }
            "deleted" => {
                self.metrics.files_deleted.insert(path.to_string());
            }
            "read" => {
                self.metrics.files_read.insert(path.to_string());
            }
            _ => {}
        }
        self.last_activity = Some(Instant::now());
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.metrics.errors += 1;
        self.last_activity = Some(Instant::now());
    }

    /// Record a warning.
    pub fn record_warning(&mut self) {
        self.metrics.warnings += 1;
        self.last_activity = Some(Instant::now());
    }

    /// Record line changes (estimated from diffs).
    pub fn record_line_changes(&mut self, added: u32, removed: u32) {
        self.metrics.lines_added += added;
        self.metrics.lines_removed += removed;
        self.last_activity = Some(Instant::now());
    }

    /// Reset metrics for a new iteration.
    pub fn reset(&mut self) {
        self.metrics = IterationMetrics::new();
        self.last_activity = None;
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> &IterationMetrics {
        &self.metrics
    }

    /// Get activity state based on time since last activity.
    pub fn get_activity_state(&self) -> ActivityState {
        match self.last_activity {
            None => ActivityState::Thinking,
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed < Duration::from_secs(ACTIVE_THRESHOLD_SECS) {
                    ActivityState::Active
                } else if elapsed < Duration::from_secs(THINKING_THRESHOLD_SECS) {
                    ActivityState::Thinking
                } else {
                    ActivityState::Stalled
                }
            }
        }
    }

    /// Get time since last activity.
    pub fn time_since_activity(&self) -> Option<Duration> {
        self.last_activity.map(|t| t.elapsed())
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_new_metrics_collector() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.metrics.tool_calls, 0);
        assert_eq!(collector.metrics.errors, 0);
        assert!(collector.last_activity.is_none());
    }

    #[test]
    fn test_record_tool_call() {
        let mut collector = MetricsCollector::new();
        collector.record_tool_call("write_file");
        assert_eq!(collector.metrics.tool_calls, 1);
        assert!(collector.last_activity.is_some());

        collector.record_tool_call("read_file");
        assert_eq!(collector.metrics.tool_calls, 2);
    }

    #[test]
    fn test_record_file_change_created() {
        let mut collector = MetricsCollector::new();
        collector.record_file_change("src/new.rs", "created");
        assert!(collector.metrics.files_created.contains("src/new.rs"));
        assert_eq!(collector.metrics.files_created.len(), 1);
    }

    #[test]
    fn test_record_file_change_modified() {
        let mut collector = MetricsCollector::new();
        collector.record_file_change("src/main.rs", "modified");
        assert!(collector.metrics.files_modified.contains("src/main.rs"));
    }

    #[test]
    fn test_record_file_change_deleted() {
        let mut collector = MetricsCollector::new();
        collector.record_file_change("src/old.rs", "deleted");
        assert!(collector.metrics.files_deleted.contains("src/old.rs"));
    }

    #[test]
    fn test_record_file_change_read() {
        let mut collector = MetricsCollector::new();
        collector.record_file_change("README.md", "read");
        assert!(collector.metrics.files_read.contains("README.md"));
    }

    #[test]
    fn test_record_error() {
        let mut collector = MetricsCollector::new();
        collector.record_error();
        assert_eq!(collector.metrics.errors, 1);
        collector.record_error();
        assert_eq!(collector.metrics.errors, 2);
    }

    #[test]
    fn test_record_warning() {
        let mut collector = MetricsCollector::new();
        collector.record_warning();
        assert_eq!(collector.metrics.warnings, 1);
    }

    #[test]
    fn test_record_line_changes() {
        let mut collector = MetricsCollector::new();
        collector.record_line_changes(50, 10);
        assert_eq!(collector.metrics.lines_added, 50);
        assert_eq!(collector.metrics.lines_removed, 10);

        collector.record_line_changes(20, 5);
        assert_eq!(collector.metrics.lines_added, 70);
        assert_eq!(collector.metrics.lines_removed, 15);
    }

    #[test]
    fn test_reset() {
        let mut collector = MetricsCollector::new();
        collector.record_tool_call("test");
        collector.record_error();
        collector.record_file_change("test.rs", "created");

        collector.reset();

        assert_eq!(collector.metrics.tool_calls, 0);
        assert_eq!(collector.metrics.errors, 0);
        assert!(collector.metrics.files_created.is_empty());
        assert!(collector.last_activity.is_none());
    }

    #[test]
    fn test_total_file_ops() {
        let mut collector = MetricsCollector::new();
        collector.record_file_change("a.rs", "created");
        collector.record_file_change("b.rs", "modified");
        collector.record_file_change("c.rs", "deleted");
        collector.record_file_change("d.rs", "read");

        assert_eq!(collector.metrics.total_file_ops(), 4);
    }

    #[test]
    fn test_activity_state_no_activity() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_activity_state(), ActivityState::Thinking);
    }

    #[test]
    fn test_activity_state_active() {
        let mut collector = MetricsCollector::new();
        collector.record_tool_call("test");
        assert_eq!(collector.get_activity_state(), ActivityState::Active);
    }

    #[test]
    fn test_activity_state_thinking() {
        let mut collector = MetricsCollector::new();
        collector.last_activity = Some(Instant::now() - Duration::from_secs(5));
        assert_eq!(collector.get_activity_state(), ActivityState::Thinking);
    }

    #[test]
    fn test_activity_state_stalled() {
        let mut collector = MetricsCollector::new();
        collector.last_activity = Some(Instant::now() - Duration::from_secs(15));
        assert_eq!(collector.get_activity_state(), ActivityState::Stalled);
    }

    #[test]
    fn test_time_since_activity() {
        let collector = MetricsCollector::new();
        assert!(collector.time_since_activity().is_none());

        let mut collector = MetricsCollector::new();
        collector.record_tool_call("test");
        sleep(Duration::from_millis(10));
        let elapsed = collector.time_since_activity();
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= Duration::from_millis(10));
    }

    #[test]
    fn test_file_deduplication() {
        let mut collector = MetricsCollector::new();
        // Record same file multiple times
        collector.record_file_change("src/main.rs", "modified");
        collector.record_file_change("src/main.rs", "modified");
        collector.record_file_change("src/main.rs", "modified");

        // Should only count once (HashSet)
        assert_eq!(collector.metrics.files_modified.len(), 1);
    }
}
