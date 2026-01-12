//! Progress models for .afk/progress.json.
//!
//! This module tracks session state and task progress,
//! mirroring the Python Pydantic models in src/afk/progress.py.

use crate::config::PROGRESS_FILE;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Task status values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Progress record for a single task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Unique identifier for the task.
    pub id: String,
    /// Source identifier (e.g., "beads", "json:path/to/file.json").
    pub source: String,
    /// Current status of the task.
    #[serde(default)]
    pub status: TaskStatus,
    /// ISO timestamp when the task was started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// ISO timestamp when the task was completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Number of times the task has failed.
    #[serde(default)]
    pub failure_count: u32,
    /// List of commit hashes associated with this task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
    /// Optional message (e.g., failure reason).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Short-term learnings specific to this task, discovered during this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub learnings: Vec<String>,
}

impl TaskProgress {
    /// Create a new TaskProgress with the given ID and source.
    pub fn new(id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            status: TaskStatus::Pending,
            started_at: None,
            completed_at: None,
            failure_count: 0,
            commits: Vec::new(),
            message: None,
            learnings: Vec::new(),
        }
    }
}

/// Progress for the current afk session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProgress {
    /// ISO timestamp when the session was started.
    #[serde(default = "default_started_at")]
    pub started_at: String,
    /// Number of iterations completed.
    #[serde(default)]
    pub iterations: u32,
    /// Map of task ID to task progress.
    #[serde(default)]
    pub tasks: HashMap<String, TaskProgress>,
}

impl Default for SessionProgress {
    fn default() -> Self {
        Self {
            started_at: default_started_at(),
            iterations: 0,
            tasks: HashMap::new(),
        }
    }
}

fn default_started_at() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string()
}

/// Error type for progress operations.
#[derive(Debug, thiserror::Error)]
pub enum ProgressError {
    #[error("Failed to read progress file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse progress JSON: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl SessionProgress {
    /// Create a new session with the current timestamp.
    pub fn new() -> Self {
        Self {
            started_at: default_started_at(),
            iterations: 0,
            tasks: HashMap::new(),
        }
    }

    /// Load progress from a file, or return a new session if file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to progress file. Defaults to `.afk/progress.json` if None.
    pub fn load(path: Option<&Path>) -> Result<Self, ProgressError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(PROGRESS_FILE));

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&path)?;
        let progress: SessionProgress = serde_json::from_str(&contents)?;
        Ok(progress)
    }

    /// Save progress to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save to. Defaults to `.afk/progress.json` if None.
    ///
    /// Creates parent directories if they don't exist.
    pub fn save(&self, path: Option<&Path>) -> Result<(), ProgressError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(PROGRESS_FILE));

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Increment and return the iteration count.
    ///
    /// Note: This saves the progress to file.
    pub fn increment_iteration(&mut self) -> u32 {
        self.iterations += 1;
        self.iterations
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&TaskProgress> {
        self.tasks.get(task_id)
    }

    /// Get a mutable reference to a task by ID.
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut TaskProgress> {
        self.tasks.get_mut(task_id)
    }

    /// Set or update task status.
    ///
    /// Creates the task if it doesn't exist. Handles timestamp updates
    /// and failure count increments automatically.
    ///
    /// Returns a reference to the updated task.
    pub fn set_task_status(
        &mut self,
        task_id: &str,
        status: TaskStatus,
        source: &str,
        message: Option<String>,
    ) -> &TaskProgress {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string();

        let task = self
            .tasks
            .entry(task_id.to_string())
            .or_insert_with(|| TaskProgress::new(task_id, source));

        task.status = status.clone();
        task.message = message;

        match status {
            TaskStatus::InProgress => {
                if task.started_at.is_none() {
                    task.started_at = Some(now);
                }
            }
            TaskStatus::Completed => {
                task.completed_at = Some(now);
            }
            TaskStatus::Failed => {
                task.failure_count += 1;
            }
            _ => {}
        }

        self.tasks.get(task_id).unwrap()
    }

    /// Get all pending tasks.
    pub fn get_pending_tasks(&self) -> Vec<&TaskProgress> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .collect()
    }

    /// Get all completed tasks.
    pub fn get_completed_tasks(&self) -> Vec<&TaskProgress> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Completed)
            .collect()
    }

    /// Get all in-progress tasks.
    pub fn get_in_progress_tasks(&self) -> Vec<&TaskProgress> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::InProgress)
            .collect()
    }

    /// Get all failed tasks.
    pub fn get_failed_tasks(&self) -> Vec<&TaskProgress> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Failed)
            .collect()
    }

    /// Get all skipped tasks.
    pub fn get_skipped_tasks(&self) -> Vec<&TaskProgress> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Skipped)
            .collect()
    }

    /// Check if all tasks are complete.
    ///
    /// Returns true if there are tasks and all are either completed or skipped.
    /// Returns false if there are no tasks.
    pub fn is_complete(&self) -> bool {
        if self.tasks.is_empty() {
            return false;
        }
        self.tasks
            .values()
            .all(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Skipped))
    }

    /// Add a learning to a specific task.
    ///
    /// Creates the task if it doesn't exist.
    pub fn add_learning(&mut self, task_id: &str, learning: impl Into<String>, source: &str) {
        let task = self
            .tasks
            .entry(task_id.to_string())
            .or_insert_with(|| TaskProgress::new(task_id, source));

        task.learnings.push(learning.into());
    }

    /// Get all learnings grouped by task ID.
    ///
    /// Only returns tasks that have at least one learning.
    pub fn get_all_learnings(&self) -> HashMap<&str, &Vec<String>> {
        self.tasks
            .iter()
            .filter(|(_, task)| !task.learnings.is_empty())
            .map(|(id, task)| (id.as_str(), &task.learnings))
            .collect()
    }

    /// Add a commit hash to a task.
    pub fn add_commit(&mut self, task_id: &str, commit_hash: impl Into<String>, source: &str) {
        let task = self
            .tasks
            .entry(task_id.to_string())
            .or_insert_with(|| TaskProgress::new(task_id, source));

        task.commits.push(commit_hash.into());
    }

    /// Get task counts by status.
    ///
    /// Returns (pending, in_progress, completed, failed, skipped).
    pub fn get_task_counts(&self) -> (usize, usize, usize, usize, usize) {
        let pending = self.get_pending_tasks().len();
        let in_progress = self.get_in_progress_tasks().len();
        let completed = self.get_completed_tasks().len();
        let failed = self.get_failed_tasks().len();
        let skipped = self.get_skipped_tasks().len();
        (pending, in_progress, completed, failed, skipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_task_status_default() {
        let status = TaskStatus::default();
        assert_eq!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_status_serialisation() {
        // Test that status serialises with snake_case
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            r#""in_progress""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Failed).unwrap(),
            r#""failed""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Skipped).unwrap(),
            r#""skipped""#
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Pending).unwrap(),
            r#""pending""#
        );
    }

    #[test]
    fn test_task_status_deserialisation() {
        let status: TaskStatus = serde_json::from_str(r#""in_progress""#).unwrap();
        assert_eq!(status, TaskStatus::InProgress);

        let status: TaskStatus = serde_json::from_str(r#""completed""#).unwrap();
        assert_eq!(status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_progress_new() {
        let task = TaskProgress::new("task-001", "beads");
        assert_eq!(task.id, "task-001");
        assert_eq!(task.source, "beads");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
        assert_eq!(task.failure_count, 0);
        assert!(task.commits.is_empty());
        assert!(task.message.is_none());
        assert!(task.learnings.is_empty());
    }

    #[test]
    fn test_task_progress_serialisation() {
        let task = TaskProgress {
            id: "task-001".to_string(),
            source: "json:tasks.json".to_string(),
            status: TaskStatus::InProgress,
            started_at: Some("2024-01-01T12:00:00".to_string()),
            completed_at: None,
            failure_count: 2,
            commits: vec!["abc123".to_string()],
            message: Some("Working on it".to_string()),
            learnings: vec!["Learned something".to_string()],
        };

        let json = serde_json::to_string_pretty(&task).unwrap();
        assert!(json.contains(r#""id": "task-001""#));
        assert!(json.contains(r#""status": "in_progress""#));
        assert!(json.contains(r#""failure_count": 2"#));
        assert!(json.contains(r#""started_at": "2024-01-01T12:00:00""#));
        // completed_at should be skipped when None
        assert!(!json.contains("completed_at"));
    }

    #[test]
    fn test_task_progress_deserialisation() {
        let json = r#"{
            "id": "task-002",
            "source": "beads",
            "status": "completed",
            "started_at": "2024-01-01T10:00:00",
            "completed_at": "2024-01-01T12:00:00",
            "failure_count": 1,
            "commits": ["def456", "ghi789"],
            "message": "Done!",
            "learnings": ["First", "Second"]
        }"#;

        let task: TaskProgress = serde_json::from_str(json).unwrap();
        assert_eq!(task.id, "task-002");
        assert_eq!(task.source, "beads");
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.started_at, Some("2024-01-01T10:00:00".to_string()));
        assert_eq!(task.completed_at, Some("2024-01-01T12:00:00".to_string()));
        assert_eq!(task.failure_count, 1);
        assert_eq!(task.commits, vec!["def456", "ghi789"]);
        assert_eq!(task.message, Some("Done!".to_string()));
        assert_eq!(task.learnings, vec!["First", "Second"]);
    }

    #[test]
    fn test_task_progress_deserialisation_minimal() {
        let json = r#"{
            "id": "task-003",
            "source": "markdown"
        }"#;

        let task: TaskProgress = serde_json::from_str(json).unwrap();
        assert_eq!(task.id, "task-003");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.failure_count, 0);
        assert!(task.commits.is_empty());
    }

    #[test]
    fn test_session_progress_new() {
        let session = SessionProgress::new();
        assert!(!session.started_at.is_empty());
        assert_eq!(session.iterations, 0);
        assert!(session.tasks.is_empty());
    }

    #[test]
    fn test_session_progress_default() {
        let session = SessionProgress::default();
        assert!(!session.started_at.is_empty());
        assert_eq!(session.iterations, 0);
        assert!(session.tasks.is_empty());
    }

    #[test]
    fn test_session_progress_load_missing_file() {
        let temp = TempDir::new().unwrap();
        let progress_path = temp.path().join(".afk/progress.json");
        let session = SessionProgress::load(Some(&progress_path)).unwrap();
        assert_eq!(session.iterations, 0);
        assert!(session.tasks.is_empty());
    }

    #[test]
    fn test_session_progress_load_existing_file() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let progress_path = afk_dir.join("progress.json");

        let sample = r#"{
            "started_at": "2024-01-01T10:00:00",
            "iterations": 5,
            "tasks": {
                "task-001": {
                    "id": "task-001",
                    "source": "beads",
                    "status": "completed"
                }
            }
        }"#;
        fs::write(&progress_path, sample).unwrap();

        let session = SessionProgress::load(Some(&progress_path)).unwrap();
        assert_eq!(session.started_at, "2024-01-01T10:00:00");
        assert_eq!(session.iterations, 5);
        assert_eq!(session.tasks.len(), 1);
        assert!(session.tasks.contains_key("task-001"));
    }

    #[test]
    fn test_session_progress_save_creates_directory() {
        let temp = TempDir::new().unwrap();
        let progress_path = temp.path().join(".afk/progress.json");

        let mut session = SessionProgress::new();
        session.iterations = 10;
        session.save(Some(&progress_path)).unwrap();

        assert!(progress_path.exists());
        let contents = fs::read_to_string(&progress_path).unwrap();
        assert!(contents.contains(r#""iterations": 10"#));
    }

    #[test]
    fn test_session_progress_round_trip() {
        let temp = TempDir::new().unwrap();
        let progress_path = temp.path().join(".afk/progress.json");

        let mut original = SessionProgress {
            started_at: "2024-01-01T09:00:00".to_string(),
            iterations: 15,
            tasks: HashMap::new(),
        };
        original.tasks.insert(
            "task-001".to_string(),
            TaskProgress {
                id: "task-001".to_string(),
                source: "json:tasks.json".to_string(),
                status: TaskStatus::Completed,
                started_at: Some("2024-01-01T10:00:00".to_string()),
                completed_at: Some("2024-01-01T11:00:00".to_string()),
                failure_count: 1,
                commits: vec!["abc123".to_string()],
                message: Some("Done".to_string()),
                learnings: vec!["Learning 1".to_string()],
            },
        );

        original.save(Some(&progress_path)).unwrap();
        let loaded = SessionProgress::load(Some(&progress_path)).unwrap();

        assert_eq!(loaded.started_at, "2024-01-01T09:00:00");
        assert_eq!(loaded.iterations, 15);
        assert_eq!(loaded.tasks.len(), 1);
        let task = loaded.tasks.get("task-001").unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.commits, vec!["abc123"]);
        assert_eq!(task.learnings, vec!["Learning 1"]);
    }

    #[test]
    fn test_increment_iteration() {
        let mut session = SessionProgress::new();
        assert_eq!(session.iterations, 0);

        let count = session.increment_iteration();
        assert_eq!(count, 1);
        assert_eq!(session.iterations, 1);

        let count = session.increment_iteration();
        assert_eq!(count, 2);
        assert_eq!(session.iterations, 2);
    }

    #[test]
    fn test_get_task() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress::new("task-001", "beads"),
        );

        let task = session.get_task("task-001");
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, "task-001");

        assert!(session.get_task("nonexistent").is_none());
    }

    #[test]
    fn test_get_task_mut() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress::new("task-001", "beads"),
        );

        if let Some(task) = session.get_task_mut("task-001") {
            task.failure_count = 5;
        }

        assert_eq!(session.tasks.get("task-001").unwrap().failure_count, 5);
    }

    #[test]
    fn test_set_task_status_creates_task() {
        let mut session = SessionProgress::new();

        session.set_task_status("new-task", TaskStatus::Pending, "beads", None);

        assert!(session.tasks.contains_key("new-task"));
        let task = session.tasks.get("new-task").unwrap();
        assert_eq!(task.id, "new-task");
        assert_eq!(task.source, "beads");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_set_task_status_in_progress_sets_started_at() {
        let mut session = SessionProgress::new();

        session.set_task_status("task-001", TaskStatus::InProgress, "beads", None);

        let task = session.tasks.get("task-001").unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.started_at.is_some());
    }

    #[test]
    fn test_set_task_status_in_progress_doesnt_overwrite_started_at() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress {
                id: "task-001".to_string(),
                source: "beads".to_string(),
                status: TaskStatus::Pending,
                started_at: Some("original-time".to_string()),
                ..TaskProgress::new("", "")
            },
        );

        session.set_task_status("task-001", TaskStatus::InProgress, "beads", None);

        let task = session.tasks.get("task-001").unwrap();
        assert_eq!(task.started_at, Some("original-time".to_string()));
    }

    #[test]
    fn test_set_task_status_completed_sets_completed_at() {
        let mut session = SessionProgress::new();

        session.set_task_status("task-001", TaskStatus::Completed, "beads", Some("Done!".to_string()));

        let task = session.tasks.get("task-001").unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
        assert_eq!(task.message, Some("Done!".to_string()));
    }

    #[test]
    fn test_set_task_status_failed_increments_failure_count() {
        let mut session = SessionProgress::new();

        session.set_task_status("task-001", TaskStatus::Failed, "beads", Some("Error".to_string()));
        assert_eq!(session.tasks.get("task-001").unwrap().failure_count, 1);

        session.set_task_status("task-001", TaskStatus::Failed, "beads", Some("Error again".to_string()));
        assert_eq!(session.tasks.get("task-001").unwrap().failure_count, 2);

        session.set_task_status("task-001", TaskStatus::Failed, "beads", None);
        assert_eq!(session.tasks.get("task-001").unwrap().failure_count, 3);
    }

    #[test]
    fn test_get_pending_tasks() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "pending-1".to_string(),
            TaskProgress::new("pending-1", "beads"),
        );
        session.tasks.insert(
            "pending-2".to_string(),
            TaskProgress::new("pending-2", "json"),
        );
        session.tasks.insert(
            "completed".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed", "beads")
            },
        );

        let pending = session.get_pending_tasks();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_get_completed_tasks() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "pending".to_string(),
            TaskProgress::new("pending", "beads"),
        );
        session.tasks.insert(
            "completed-1".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-1", "beads")
            },
        );
        session.tasks.insert(
            "completed-2".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-2", "beads")
            },
        );

        let completed = session.get_completed_tasks();
        assert_eq!(completed.len(), 2);
    }

    #[test]
    fn test_get_in_progress_tasks() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "in-progress".to_string(),
            TaskProgress {
                status: TaskStatus::InProgress,
                ..TaskProgress::new("in-progress", "beads")
            },
        );

        let in_progress = session.get_in_progress_tasks();
        assert_eq!(in_progress.len(), 1);
    }

    #[test]
    fn test_get_failed_tasks() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "failed".to_string(),
            TaskProgress {
                status: TaskStatus::Failed,
                failure_count: 3,
                ..TaskProgress::new("failed", "beads")
            },
        );

        let failed = session.get_failed_tasks();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].failure_count, 3);
    }

    #[test]
    fn test_get_skipped_tasks() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "skipped".to_string(),
            TaskProgress {
                status: TaskStatus::Skipped,
                ..TaskProgress::new("skipped", "beads")
            },
        );

        let skipped = session.get_skipped_tasks();
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn test_is_complete_empty_returns_false() {
        let session = SessionProgress::new();
        assert!(!session.is_complete());
    }

    #[test]
    fn test_is_complete_all_completed() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "completed-1".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-1", "beads")
            },
        );
        session.tasks.insert(
            "completed-2".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-2", "beads")
            },
        );

        assert!(session.is_complete());
    }

    #[test]
    fn test_is_complete_with_skipped() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "completed".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed", "beads")
            },
        );
        session.tasks.insert(
            "skipped".to_string(),
            TaskProgress {
                status: TaskStatus::Skipped,
                ..TaskProgress::new("skipped", "beads")
            },
        );

        assert!(session.is_complete());
    }

    #[test]
    fn test_is_complete_with_pending_returns_false() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "completed".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed", "beads")
            },
        );
        session.tasks.insert(
            "pending".to_string(),
            TaskProgress::new("pending", "beads"),
        );

        assert!(!session.is_complete());
    }

    #[test]
    fn test_is_complete_with_in_progress_returns_false() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "in-progress".to_string(),
            TaskProgress {
                status: TaskStatus::InProgress,
                ..TaskProgress::new("in-progress", "beads")
            },
        );

        assert!(!session.is_complete());
    }

    #[test]
    fn test_add_learning_creates_task() {
        let mut session = SessionProgress::new();

        session.add_learning("new-task", "Learned something", "beads");

        assert!(session.tasks.contains_key("new-task"));
        let task = session.tasks.get("new-task").unwrap();
        assert_eq!(task.learnings, vec!["Learned something"]);
    }

    #[test]
    fn test_add_learning_to_existing_task() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress::new("task-001", "beads"),
        );

        session.add_learning("task-001", "First learning", "beads");
        session.add_learning("task-001", "Second learning", "beads");

        let task = session.tasks.get("task-001").unwrap();
        assert_eq!(task.learnings, vec!["First learning", "Second learning"]);
    }

    #[test]
    fn test_get_all_learnings() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-with-learnings".to_string(),
            TaskProgress {
                learnings: vec!["Learning 1".to_string(), "Learning 2".to_string()],
                ..TaskProgress::new("task-with-learnings", "beads")
            },
        );
        session.tasks.insert(
            "task-without-learnings".to_string(),
            TaskProgress::new("task-without-learnings", "beads"),
        );
        session.tasks.insert(
            "another-with-learnings".to_string(),
            TaskProgress {
                learnings: vec!["Learning 3".to_string()],
                ..TaskProgress::new("another-with-learnings", "beads")
            },
        );

        let learnings = session.get_all_learnings();
        assert_eq!(learnings.len(), 2);
        assert!(learnings.contains_key("task-with-learnings"));
        assert!(learnings.contains_key("another-with-learnings"));
        assert!(!learnings.contains_key("task-without-learnings"));
    }

    #[test]
    fn test_get_all_learnings_empty() {
        let session = SessionProgress::new();
        let learnings = session.get_all_learnings();
        assert!(learnings.is_empty());
    }

    #[test]
    fn test_add_commit() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress::new("task-001", "beads"),
        );

        session.add_commit("task-001", "abc123", "beads");
        session.add_commit("task-001", "def456", "beads");

        let task = session.tasks.get("task-001").unwrap();
        assert_eq!(task.commits, vec!["abc123", "def456"]);
    }

    #[test]
    fn test_add_commit_creates_task() {
        let mut session = SessionProgress::new();

        session.add_commit("new-task", "commit-hash", "beads");

        assert!(session.tasks.contains_key("new-task"));
        assert_eq!(
            session.tasks.get("new-task").unwrap().commits,
            vec!["commit-hash"]
        );
    }

    #[test]
    fn test_get_task_counts() {
        let mut session = SessionProgress::new();
        session.tasks.insert(
            "pending".to_string(),
            TaskProgress::new("pending", "beads"),
        );
        session.tasks.insert(
            "in-progress".to_string(),
            TaskProgress {
                status: TaskStatus::InProgress,
                ..TaskProgress::new("in-progress", "beads")
            },
        );
        session.tasks.insert(
            "completed-1".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-1", "beads")
            },
        );
        session.tasks.insert(
            "completed-2".to_string(),
            TaskProgress {
                status: TaskStatus::Completed,
                ..TaskProgress::new("completed-2", "beads")
            },
        );
        session.tasks.insert(
            "failed".to_string(),
            TaskProgress {
                status: TaskStatus::Failed,
                ..TaskProgress::new("failed", "beads")
            },
        );
        session.tasks.insert(
            "skipped".to_string(),
            TaskProgress {
                status: TaskStatus::Skipped,
                ..TaskProgress::new("skipped", "beads")
            },
        );

        let (pending, in_progress, completed, failed, skipped) = session.get_task_counts();
        assert_eq!(pending, 1);
        assert_eq!(in_progress, 1);
        assert_eq!(completed, 2);
        assert_eq!(failed, 1);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn test_get_task_counts_empty() {
        let session = SessionProgress::new();
        let (pending, in_progress, completed, failed, skipped) = session.get_task_counts();
        assert_eq!(pending, 0);
        assert_eq!(in_progress, 0);
        assert_eq!(completed, 0);
        assert_eq!(failed, 0);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn test_with_real_progress_json_format() {
        // Test with the actual progress.json format from the Python version
        let json = r#"{
            "started_at": "2026-01-12T09:48:44.675953",
            "iterations": 3,
            "tasks": {
                "rust-002-config-models": {
                    "id": "rust-002-config-models",
                    "source": "json:docs/prds/rust-rewrite-tasks.json",
                    "status": "completed",
                    "started_at": "2026-01-12T09:48:44.675959",
                    "completed_at": "2026-01-12T09:51:49.146517",
                    "failure_count": 0,
                    "commits": ["c7bb92d"],
                    "message": null,
                    "learnings": [
                        "Python version outputs snake_case JSON",
                        "Removed rename_all = camelCase from Rust structs"
                    ]
                },
                "rust-003-prd-models": {
                    "id": "rust-003-prd-models",
                    "source": "json:docs/prds/rust-rewrite-tasks.json",
                    "status": "completed",
                    "started_at": "2026-01-12T09:51:50.668288",
                    "completed_at": "2026-01-12T09:56:38.814480",
                    "failure_count": 0,
                    "commits": ["200672e"],
                    "learnings": [
                        "PRD JSON uses camelCase"
                    ]
                },
                "rust-004-progress-models": {
                    "id": "rust-004-progress-models",
                    "source": "json:docs/prds/rust-rewrite-tasks.json",
                    "status": "in_progress",
                    "started_at": "2026-01-12T09:56:40.332299",
                    "failure_count": 0
                }
            }
        }"#;

        let session: SessionProgress = serde_json::from_str(json).unwrap();

        assert_eq!(session.started_at, "2026-01-12T09:48:44.675953");
        assert_eq!(session.iterations, 3);
        assert_eq!(session.tasks.len(), 3);

        // Check completed task
        let task = session.tasks.get("rust-002-config-models").unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.commits, vec!["c7bb92d"]);
        assert_eq!(task.learnings.len(), 2);

        // Check in-progress task
        let task = session.tasks.get("rust-004-progress-models").unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.completed_at.is_none());

        // Test helper methods
        let completed = session.get_completed_tasks();
        assert_eq!(completed.len(), 2);

        let in_progress = session.get_in_progress_tasks();
        assert_eq!(in_progress.len(), 1);

        let learnings = session.get_all_learnings();
        assert_eq!(learnings.len(), 2);

        assert!(!session.is_complete());
    }

    #[test]
    fn test_serialisation_round_trip_matches_python() {
        // Create a session that matches the Python format exactly
        let mut session = SessionProgress {
            started_at: "2024-01-01T10:00:00.000000".to_string(),
            iterations: 5,
            tasks: HashMap::new(),
        };
        session.tasks.insert(
            "task-001".to_string(),
            TaskProgress {
                id: "task-001".to_string(),
                source: "beads".to_string(),
                status: TaskStatus::Completed,
                started_at: Some("2024-01-01T10:00:00.000000".to_string()),
                completed_at: Some("2024-01-01T11:00:00.000000".to_string()),
                failure_count: 0,
                commits: vec!["abc123".to_string()],
                message: None,
                learnings: vec!["A learning".to_string()],
            },
        );

        let json = serde_json::to_string_pretty(&session).unwrap();

        // Verify key formatting matches Python's output
        assert!(json.contains("\"started_at\":"));
        assert!(json.contains("\"completed_at\":"));
        assert!(json.contains("\"failure_count\":"));
        assert!(json.contains("\"status\": \"completed\""));

        // Parse it back
        let parsed: SessionProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.iterations, 5);
        assert_eq!(parsed.tasks.get("task-001").unwrap().status, TaskStatus::Completed);
    }
}
