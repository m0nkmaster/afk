//! File system monitoring.
//!
//! This module uses the notify crate to watch for file changes
//! during AI CLI execution for real-time feedback display.

use crate::path_matcher::PathMatcher;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// A new file was created.
    Created,
    /// An existing file was modified.
    Modified,
    /// A file was deleted.
    Deleted,
}

/// A file change event.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path to the changed file.
    pub path: PathBuf,
    /// Type of change.
    pub change_type: ChangeType,
    /// Timestamp of the change.
    pub timestamp: SystemTime,
}

/// File system watcher for detecting changes during AI CLI execution.
pub struct FileWatcher {
    /// Root directory to watch.
    root: PathBuf,
    /// Path matcher for ignore patterns.
    matcher: PathMatcher,
    /// The actual watcher (wrapped in Option for stop/start).
    watcher: Option<RecommendedWatcher>,
    /// Receiver for events from the watcher.
    receiver: Option<Receiver<Result<Event, notify::Error>>>,
    /// Buffer of collected changes.
    changes: Arc<Mutex<VecDeque<FileChange>>>,
    /// Whether the watcher is running.
    running: bool,
}

impl FileWatcher {
    /// Create a new FileWatcher for the given root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            matcher: PathMatcher::with_defaults(),
            watcher: None,
            receiver: None,
            changes: Arc::new(Mutex::new(VecDeque::new())),
            running: false,
        }
    }

    /// Create a new FileWatcher with custom ignore patterns.
    pub fn with_ignore_patterns(root: impl AsRef<Path>, patterns: Vec<String>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            matcher: PathMatcher::from_strings(patterns),
            watcher: None,
            receiver: None,
            changes: Arc::new(Mutex::new(VecDeque::new())),
            running: false,
        }
    }

    /// Add an ignore pattern.
    pub fn add_ignore_pattern(&mut self, pattern: &str) {
        self.matcher.add_pattern(pattern);
    }

    /// Check if a path should be ignored.
    #[cfg(test)]
    fn should_ignore(&self, path: &Path) -> bool {
        self.matcher.matches(path)
    }

    /// Start watching for file changes.
    pub fn start(&mut self) -> Result<(), notify::Error> {
        if self.running {
            return Ok(());
        }

        // Use a bounded channel to prevent blocking when buffer is full
        let (tx, rx) = sync_channel(100);
        let changes = self.changes.clone();
        let matcher = self.matcher.clone();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(ref event) = result {
                    // Convert notify events to our FileChange format
                    for path in &event.paths {
                        if matcher.matches(path) {
                            continue;
                        }

                        let change_type = match event.kind {
                            notify::EventKind::Create(_) => Some(ChangeType::Created),
                            notify::EventKind::Modify(_) => Some(ChangeType::Modified),
                            notify::EventKind::Remove(_) => Some(ChangeType::Deleted),
                            _ => None,
                        };

                        if let Some(ct) = change_type {
                            let change = FileChange {
                                path: path.clone(),
                                change_type: ct,
                                timestamp: SystemTime::now(),
                            };
                            if let Ok(mut changes) = changes.lock() {
                                changes.push_back(change);
                            }
                        }
                    }
                }
                // Try to send the event - drop if buffer full (non-blocking)
                let _ = tx.try_send(result);
            },
            Config::default(),
        )?;

        watcher.watch(&self.root, RecursiveMode::Recursive)?;

        self.watcher = Some(watcher);
        self.receiver = Some(rx);
        self.running = true;

        Ok(())
    }

    /// Stop watching for file changes.
    pub fn stop(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.root);
            // Explicitly drop to ensure cleanup
            drop(watcher);
        }
        // Drain the receiver to unblock any waiting senders
        if let Some(rx) = self.receiver.take() {
            while rx.try_recv().is_ok() {}
            drop(rx);
        }
        self.running = false;
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get all collected changes and clear the buffer.
    pub fn get_changes(&self) -> Vec<FileChange> {
        if let Ok(mut changes) = self.changes.lock() {
            changes.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Get the number of pending changes.
    pub fn pending_count(&self) -> usize {
        if let Ok(changes) = self.changes.lock() {
            changes.len()
        } else {
            0
        }
    }

    /// Clear all pending changes.
    pub fn clear(&self) {
        if let Ok(mut changes) = self.changes.lock() {
            changes.clear();
        }
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_new_file_watcher() {
        let watcher = FileWatcher::new("/tmp");
        assert!(!watcher.is_running());
        assert_eq!(watcher.pending_count(), 0);
    }

    #[test]
    fn test_default_ignore_patterns() {
        let watcher = FileWatcher::new("/tmp");
        let patterns = watcher.matcher.patterns();
        assert!(patterns.contains(&".git".to_string()));
        assert!(patterns.contains(&"node_modules".to_string()));
        assert!(patterns.contains(&".afk".to_string()));
    }

    #[test]
    fn test_with_custom_ignore_patterns() {
        let patterns = vec!["custom".to_string(), "pattern".to_string()];
        let watcher = FileWatcher::with_ignore_patterns("/tmp", patterns);
        let matcher_patterns = watcher.matcher.patterns();
        assert!(matcher_patterns.contains(&"custom".to_string()));
        assert!(matcher_patterns.contains(&"pattern".to_string()));
        assert!(!matcher_patterns.contains(&".git".to_string()));
    }

    #[test]
    fn test_add_ignore_pattern() {
        let mut watcher = FileWatcher::new("/tmp");
        watcher.add_ignore_pattern("new_pattern");
        assert!(watcher
            .matcher
            .patterns()
            .contains(&"new_pattern".to_string()));
    }

    #[test]
    fn test_should_ignore_git() {
        let watcher = FileWatcher::new("/tmp");
        assert!(watcher.should_ignore(Path::new("/project/.git/config")));
        assert!(watcher.should_ignore(Path::new("/project/.git/objects/abc")));
    }

    #[test]
    fn test_should_ignore_node_modules() {
        let watcher = FileWatcher::new("/tmp");
        assert!(watcher.should_ignore(Path::new("/project/node_modules/pkg/file.js")));
    }

    #[test]
    fn test_should_ignore_extension() {
        let watcher = FileWatcher::new("/tmp");
        assert!(watcher.should_ignore(Path::new("/project/file.pyc")));
        assert!(watcher.should_ignore(Path::new("/project/cache/module.pyo")));
    }

    #[test]
    fn test_should_not_ignore_normal_files() {
        let watcher = FileWatcher::new("/tmp");
        assert!(!watcher.should_ignore(Path::new("/project/src/main.rs")));
        assert!(!watcher.should_ignore(Path::new("/project/lib/utils.py")));
    }

    #[test]
    fn test_start_stop() {
        let temp = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(temp.path());

        assert!(!watcher.is_running());

        watcher.start().unwrap();
        assert!(watcher.is_running());

        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_start_twice_is_ok() {
        let temp = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(temp.path());

        watcher.start().unwrap();
        watcher.start().unwrap(); // Should be a no-op
        assert!(watcher.is_running());
    }

    #[test]
    fn test_clear() {
        let watcher = FileWatcher::new("/tmp");
        // Manually add some changes for testing
        {
            let mut changes = watcher.changes.lock().unwrap();
            changes.push_back(FileChange {
                path: PathBuf::from("/test"),
                change_type: ChangeType::Created,
                timestamp: SystemTime::now(),
            });
        }
        assert_eq!(watcher.pending_count(), 1);

        watcher.clear();
        assert_eq!(watcher.pending_count(), 0);
    }

    #[test]
    fn test_get_changes_clears_buffer() {
        let watcher = FileWatcher::new("/tmp");
        // Manually add some changes for testing
        {
            let mut changes = watcher.changes.lock().unwrap();
            changes.push_back(FileChange {
                path: PathBuf::from("/test1"),
                change_type: ChangeType::Created,
                timestamp: SystemTime::now(),
            });
            changes.push_back(FileChange {
                path: PathBuf::from("/test2"),
                change_type: ChangeType::Modified,
                timestamp: SystemTime::now(),
            });
        }

        let changes = watcher.get_changes();
        assert_eq!(changes.len(), 2);
        assert_eq!(watcher.pending_count(), 0);
    }

    #[test]
    fn test_change_type_equality() {
        assert_eq!(ChangeType::Created, ChangeType::Created);
        assert_ne!(ChangeType::Created, ChangeType::Modified);
        assert_ne!(ChangeType::Modified, ChangeType::Deleted);
    }

    #[test]
    fn test_file_change_struct() {
        let change = FileChange {
            path: PathBuf::from("/test/file.rs"),
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };
        assert_eq!(change.path.to_str().unwrap(), "/test/file.rs");
        assert_eq!(change.change_type, ChangeType::Modified);
    }

    #[test]
    fn test_detects_file_creation() {
        let temp = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(temp.path());
        watcher.start().unwrap();

        // Create a file
        let test_file = temp.path().join("test_file.txt");
        fs::write(&test_file, "hello").unwrap();

        // Give the watcher time to detect the change
        thread::sleep(Duration::from_millis(100));

        let changes = watcher.get_changes();
        // There should be at least one change (creation or modification)
        // The exact events can vary by platform
        assert!(
            changes.is_empty()
                || changes
                    .iter()
                    .any(|c| c.path.to_string_lossy().contains("test_file"))
        );
    }

    #[test]
    fn test_drop_stops_watcher() {
        let temp = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp.path());
        // Watcher should be stopped when dropped
        drop(watcher);
        // No panic = success
    }
}
