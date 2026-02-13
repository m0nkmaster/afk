//! Worker module for team mode.
//!
//! A Worker represents a single agent in team mode. It manages a git worktree,
//! runs iterations on its assigned task, and communicates status back to the
//! orchestrator via channels.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::git;
use crate::persona::Persona;
use crate::prd::{PrdDocument, UserStory};

/// Status of a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus {
    /// Worker is being set up (worktree creation).
    Setting,
    /// Worker is actively running an iteration.
    Working,
    /// Worker is paused (will not start next iteration).
    Paused,
    /// Worker completed its task successfully.
    Done,
    /// Worker's task failed after max retries.
    Failed(String),
    /// Worker was killed by the user.
    Killed,
}

/// Events sent from a worker back to the orchestrator.
#[derive(Debug, Clone)]
pub enum WorkerEvent {
    /// Worker status changed.
    StatusChange {
        /// Worker index.
        worker_id: usize,
        /// New status.
        status: WorkerStatus,
    },
    /// Worker produced output.
    Output {
        /// Worker index.
        worker_id: usize,
        /// Output line.
        line: String,
    },
    /// Worker's task completed (merged to main).
    TaskComplete {
        /// Worker index.
        worker_id: usize,
        /// Task ID that was completed.
        task_id: String,
    },
    /// Worker's task failed.
    TaskFailed {
        /// Worker index.
        worker_id: usize,
        /// Task ID that failed.
        task_id: String,
        /// Failure reason.
        reason: String,
    },
    /// Worker detected a file change.
    FileChange {
        /// Worker index.
        worker_id: usize,
        /// Path to the changed file.
        path: String,
        /// Type of change.
        change_type: String,
    },
    /// Worker detected a tool call.
    ToolCall {
        /// Worker index.
        worker_id: usize,
        /// Tool name/description.
        tool: String,
    },
    /// Iteration started.
    IterationStart {
        /// Worker index.
        worker_id: usize,
        /// Current iteration number.
        current: u32,
        /// Max iterations for this worker.
        max: u32,
    },
    /// Iteration completed.
    IterationComplete {
        /// Worker index.
        worker_id: usize,
        /// Duration in seconds.
        duration_secs: f64,
    },
}

/// A worker manages a single agent in team mode.
#[derive(Debug)]
pub struct Worker {
    /// Worker index (0-based).
    pub id: usize,
    /// Assigned persona (if any).
    pub persona: Option<Persona>,
    /// Assigned task (first/primary task, used for display).
    pub task: Option<UserStory>,
    /// All assigned tasks for this worker.
    pub tasks: Vec<UserStory>,
    /// Current status.
    pub status: WorkerStatus,
    /// Worktree directory path.
    pub worktree_path: PathBuf,
    /// Git branch name for this worker.
    pub branch_name: String,
    /// Max iterations per task.
    pub max_iterations: u32,
    /// Event sender to orchestrator.
    sender: Option<Sender<WorkerEvent>>,
}

/// Error type for worker operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// Failed to create git worktree.
    #[error("Failed to create worktree at {path}: {reason}")]
    WorktreeCreateError {
        /// Path where worktree creation was attempted.
        path: String,
        /// Reason for failure.
        reason: String,
    },
    /// Failed to write scoped task file.
    #[error("Failed to write tasks to worktree: {0}")]
    TaskWriteError(#[from] std::io::Error),
    /// No task assigned to this worker.
    #[error("No task assigned to worker {0}")]
    NoTask(usize),
    /// Worker PRD save error.
    #[error("Failed to save PRD: {0}")]
    PrdError(#[from] crate::prd::PrdError),
}

/// Default worktree base directory.
pub const TEAM_DIR: &str = ".afk/team";

impl Worker {
    /// Create a new worker.
    pub fn new(
        id: usize,
        persona: Option<Persona>,
        max_iterations: u32,
        sender: Option<Sender<WorkerEvent>>,
    ) -> Self {
        let worktree_path = PathBuf::from(format!("{}/agent-{}", TEAM_DIR, id));
        Self {
            id,
            persona,
            task: None,
            tasks: Vec::new(),
            status: WorkerStatus::Setting,
            worktree_path,
            branch_name: String::new(),
            max_iterations,
            sender,
        }
    }

    /// Get the display name for this worker.
    pub fn display_name(&self) -> String {
        match &self.persona {
            Some(p) => format!("{} {}", p.emoji, p.name),
            None => format!("Agent {}", self.id + 1),
        }
    }

    /// Assign a task to this worker.
    pub fn assign_task(&mut self, task: UserStory) {
        let sanitised_id = task.id.replace(['/', ' '], "-");
        self.branch_name = format!("afk/agent-{}/{}", self.id, sanitised_id);
        self.task = Some(task.clone());
        self.tasks = vec![task];
    }

    /// Assign multiple tasks to this worker.
    ///
    /// All tasks are written to the worker's scoped tasks.json so the AI
    /// agent can work through them across iterations.
    pub fn assign_tasks(&mut self, tasks: Vec<UserStory>) {
        if tasks.is_empty() {
            return;
        }
        self.task = Some(tasks[0].clone());
        self.branch_name = format!("afk/agent-{}", self.id);
        self.tasks = tasks;
    }

    /// Set up the worker's git worktree and scoped task file.
    ///
    /// Creates a worktree branching from HEAD with a scoped `.afk/tasks.json`
    /// containing only this worker's assigned task.
    pub fn setup(&mut self) -> Result<(), WorkerError> {
        let task = self.task.clone().ok_or(WorkerError::NoTask(self.id))?;

        self.send_status(WorkerStatus::Setting);

        let wt_path = self.worktree_path.to_string_lossy().to_string();

        // Clean up any existing worktree at this path
        if self.worktree_path.exists() {
            git::remove_worktree(&wt_path);
            // Also try to remove the directory if worktree remove didn't clean it
            let _ = fs::remove_dir_all(&self.worktree_path);
        }

        // Delete the branch if it exists from a previous run
        git::delete_branch(&self.branch_name);

        // Create worktree
        if let Err(git_err) = git::create_worktree(&wt_path, &self.branch_name, "HEAD") {
            return Err(WorkerError::WorktreeCreateError {
                path: wt_path,
                reason: git_err,
            });
        }

        // Write scoped tasks.json with this worker's assigned tasks
        let afk_dir = self.worktree_path.join(".afk");
        fs::create_dir_all(&afk_dir)?;

        let stories = if self.tasks.is_empty() {
            vec![task.clone()]
        } else {
            self.tasks.clone()
        };

        let prd = PrdDocument {
            project: String::new(),
            branch_name: self.branch_name.clone(),
            description: String::new(),
            user_stories: stories,
            last_synced: String::new(),
        };
        prd.save(Some(&afk_dir.join("tasks.json")))?;

        // Write persona instruction to AGENTS.md in worktree if persona exists
        if let Some(persona) = &self.persona {
            let agents_path = self.worktree_path.join("AGENTS.md");
            let existing = fs::read_to_string(&agents_path).unwrap_or_default();

            let persona_section = format!(
                "\n\n## Agent Persona: {}\n\n{}\n",
                persona.name, persona.instruction
            );

            // Append persona section if not already present
            if !existing.contains(&format!("## Agent Persona: {}", persona.name)) {
                fs::write(&agents_path, format!("{}{}", existing, persona_section))?;
            }
        }

        self.send_output(format!(
            "Worktree ready at {} on branch {}",
            wt_path, self.branch_name
        ));

        Ok(())
    }

    /// Clean up the worker's git worktree.
    pub fn teardown(&mut self) {
        let wt_path = self.worktree_path.to_string_lossy().to_string();
        git::remove_worktree(&wt_path);
        // Clean up directory if worktree remove didn't
        let _ = fs::remove_dir_all(&self.worktree_path);
        // Delete the branch
        git::delete_branch(&self.branch_name);
    }

    /// Get the working directory for this worker.
    pub fn working_dir(&self) -> &Path {
        &self.worktree_path
    }

    /// Send a status change event.
    fn send_status(&mut self, status: WorkerStatus) {
        self.status = status.clone();
        if let Some(tx) = &self.sender {
            let _ = tx.send(WorkerEvent::StatusChange {
                worker_id: self.id,
                status,
            });
        }
    }

    /// Send an output event.
    fn send_output(&self, line: String) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(WorkerEvent::Output {
                worker_id: self.id,
                line,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_new() {
        let worker = Worker::new(0, None, 5, None);
        assert_eq!(worker.id, 0);
        assert!(worker.persona.is_none());
        assert!(worker.task.is_none());
        assert_eq!(worker.status, WorkerStatus::Setting);
        assert_eq!(worker.max_iterations, 5);
    }

    #[test]
    fn test_worker_display_name_no_persona() {
        let worker = Worker::new(0, None, 5, None);
        assert_eq!(worker.display_name(), "Agent 1");

        let worker = Worker::new(2, None, 5, None);
        assert_eq!(worker.display_name(), "Agent 3");
    }

    #[test]
    fn test_worker_display_name_with_persona() {
        let persona = Persona {
            name: "Builder".to_string(),
            emoji: "🔧".to_string(),
            instruction: "Build things.".to_string(),
            path: PathBuf::from("builder.md"),
        };
        let worker = Worker::new(0, Some(persona), 5, None);
        assert_eq!(worker.display_name(), "🔧 Builder");
    }

    #[test]
    fn test_worker_assign_task() {
        let mut worker = Worker::new(1, None, 5, None);
        let task = UserStory {
            id: "auth-login".to_string(),
            title: "Add login".to_string(),
            ..Default::default()
        };
        worker.assign_task(task.clone());

        assert_eq!(worker.task.as_ref().unwrap().id, "auth-login");
        assert_eq!(worker.branch_name, "afk/agent-1/auth-login");
    }

    #[test]
    fn test_worker_assign_tasks_multiple() {
        let mut worker = Worker::new(0, None, 5, None);
        let tasks = vec![
            UserStory {
                id: "html-structure".to_string(),
                title: "Create HTML".to_string(),
                ..Default::default()
            },
            UserStory {
                id: "css-styling".to_string(),
                title: "Style CSS".to_string(),
                ..Default::default()
            },
        ];
        worker.assign_tasks(tasks);

        assert_eq!(worker.tasks.len(), 2);
        assert_eq!(worker.task.as_ref().unwrap().id, "html-structure");
        assert_eq!(worker.branch_name, "afk/agent-0");
    }

    #[test]
    fn test_worker_assign_tasks_empty() {
        let mut worker = Worker::new(0, None, 5, None);
        worker.assign_tasks(vec![]);
        assert!(worker.task.is_none());
        assert!(worker.tasks.is_empty());
    }

    #[test]
    fn test_worker_assign_task_sanitises_id() {
        let mut worker = Worker::new(0, None, 5, None);
        let task = UserStory {
            id: "feat/some task".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };
        worker.assign_task(task);
        assert_eq!(worker.branch_name, "afk/agent-0/feat-some-task");
    }

    #[test]
    fn test_worker_no_task_error() {
        let mut worker = Worker::new(0, None, 5, None);
        let result = worker.setup();
        assert!(result.is_err());
        match result.unwrap_err() {
            WorkerError::NoTask(id) => assert_eq!(id, 0),
            e => panic!("Expected NoTask, got: {e}"),
        }
    }

    #[test]
    fn test_worker_status_variants() {
        assert_eq!(WorkerStatus::Setting, WorkerStatus::Setting);
        assert_eq!(WorkerStatus::Working, WorkerStatus::Working);
        assert_eq!(WorkerStatus::Paused, WorkerStatus::Paused);
        assert_eq!(WorkerStatus::Done, WorkerStatus::Done);
        assert_eq!(WorkerStatus::Killed, WorkerStatus::Killed);
        assert_ne!(WorkerStatus::Working, WorkerStatus::Paused);
    }

    #[test]
    fn test_worker_event_variants() {
        // Verify events can be constructed
        let _evt = WorkerEvent::StatusChange {
            worker_id: 0,
            status: WorkerStatus::Working,
        };
        let _evt = WorkerEvent::Output {
            worker_id: 0,
            line: "test".to_string(),
        };
        let _evt = WorkerEvent::TaskComplete {
            worker_id: 0,
            task_id: "test".to_string(),
        };
        let _evt = WorkerEvent::TaskFailed {
            worker_id: 0,
            task_id: "test".to_string(),
            reason: "oops".to_string(),
        };
    }
}
