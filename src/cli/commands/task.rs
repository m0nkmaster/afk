//! Task command implementation.
//!
//! This module implements the `afk task` command for showing task details.

use crate::prd::PrdDocument;
use crate::progress::SessionProgress;

/// Result type for task command operations.
pub type TaskCommandResult = Result<(), TaskCommandError>;

/// Error type for task command operations.
#[derive(Debug, thiserror::Error)]
pub enum TaskCommandError {
    /// The specified task was not found.
    #[error("Task not found: {0}")]
    TaskNotFound(String),
}

/// Execute the task command.
pub fn task(task_id: &str) -> TaskCommandResult {
    let prd = PrdDocument::load(None).unwrap_or_default();
    let progress = SessionProgress::load(None).unwrap_or_default();

    // Find the story
    let story = prd
        .user_stories
        .iter()
        .find(|s| s.id == task_id)
        .ok_or_else(|| TaskCommandError::TaskNotFound(task_id.to_string()))?;

    // Get task progress if available
    let task_progress = progress.get_task(task_id);

    println!("\x1b[1m=== {} ===\x1b[0m", story.id);
    println!();

    println!("\x1b[1mTitle:\x1b[0m {}", story.title);
    println!(
        "\x1b[1mStatus:\x1b[0m {}",
        if story.passes { "complete" } else { "pending" }
    );
    println!("\x1b[1mPriority:\x1b[0m {}", story.priority);
    println!();

    if !story.description.is_empty() {
        println!("\x1b[1mDescription:\x1b[0m");
        for line in story.description.lines() {
            println!("  {line}");
        }
        println!();
    }

    if !story.acceptance_criteria.is_empty() {
        println!("\x1b[1mAcceptance Criteria:\x1b[0m");
        for criterion in &story.acceptance_criteria {
            let check = if story.passes { "✓" } else { "○" };
            println!("  {check} {criterion}");
        }
        println!();
    }

    // Show learnings from progress
    if let Some(task) = task_progress {
        if !task.learnings.is_empty() {
            println!("\x1b[1mLearnings:\x1b[0m");
            for learning in &task.learnings {
                println!("  - {learning}");
            }
            println!();
        }

        println!("\x1b[1mAttempts:\x1b[0m {}", task.failure_count + 1);
        if let Some(ref started) = task.started_at {
            println!(
                "\x1b[1mStarted:\x1b[0m {}",
                &started[..19].replace('T', " ")
            );
        }
        if let Some(ref completed) = task.completed_at {
            println!(
                "\x1b[1mCompleted:\x1b[0m {}",
                &completed[..19].replace('T', " ")
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_command_error_display() {
        let err = TaskCommandError::TaskNotFound("task-123".to_string());
        assert_eq!(err.to_string(), "Task not found: task-123");
    }
}
