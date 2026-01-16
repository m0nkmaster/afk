//! Progress/task status command implementations.
//!
//! This module implements the `afk done`, `afk fail`, and `afk reset` commands
//! for managing task status.

use crate::prd::PrdDocument;
use crate::progress::{SessionProgress, TaskStatus};

/// Result type for progress command operations.
pub type ProgressCommandResult = Result<(), ProgressCommandError>;

/// Error type for progress command operations.
#[derive(Debug, thiserror::Error)]
pub enum ProgressCommandError {
    /// Error loading the progress file.
    #[error("Failed to load progress: {0}")]
    LoadError(#[from] crate::progress::ProgressError),
    /// Error saving the progress file.
    #[error("Failed to save progress: {0}")]
    SaveError(std::io::Error),
}

/// Mark a task as complete.
pub fn done(task_id: &str, message: Option<&str>) -> ProgressCommandResult {
    // Load progress
    let mut progress = SessionProgress::load(None)?;

    // Mark task as completed in progress
    progress.set_task_status(
        task_id,
        TaskStatus::Completed,
        "manual",
        message.map(ToOwned::to_owned),
    );

    progress
        .save(None)
        .map_err(|e| ProgressCommandError::SaveError(std::io::Error::other(e.to_string())))?;

    // Also mark as passed in PRD
    if let Ok(mut prd) = PrdDocument::load(None) {
        prd.mark_story_complete(task_id);
        let _ = prd.save(None);
    }

    println!(
        "\x1b[32m✓\x1b[0m Task \x1b[1m{}\x1b[0m marked complete",
        task_id
    );
    if let Some(msg) = message {
        println!("  \x1b[2m{msg}\x1b[0m");
    }

    Ok(())
}

/// Mark a task as failed.
pub fn fail(task_id: &str, message: Option<&str>) -> ProgressCommandResult {
    // Load progress
    let mut progress = SessionProgress::load(None)?;

    // Mark task as failed in progress
    progress.set_task_status(
        task_id,
        TaskStatus::Failed,
        "manual",
        message.map(ToOwned::to_owned),
    );

    progress
        .save(None)
        .map_err(|e| ProgressCommandError::SaveError(std::io::Error::other(e.to_string())))?;

    let task = progress.get_task(task_id);
    let count = task.map(|t| t.failure_count).unwrap_or(1);

    println!(
        "\x1b[31m✗\x1b[0m Task \x1b[1m{}\x1b[0m marked failed (attempt {count})",
        task_id
    );
    if let Some(msg) = message {
        println!("  \x1b[2m{msg}\x1b[0m");
    }

    Ok(())
}

/// Reset a stuck task to pending state.
pub fn reset(task_id: &str) -> ProgressCommandResult {
    // Load progress
    let mut progress = SessionProgress::load(None)?;

    // Reset task to pending
    progress.set_task_status(task_id, TaskStatus::Pending, "manual", None);

    // Clear failure count if the task exists
    if let Some(task) = progress.get_task_mut(task_id) {
        task.failure_count = 0;
        task.started_at = None;
        task.completed_at = None;
    }

    progress
        .save(None)
        .map_err(|e| ProgressCommandError::SaveError(std::io::Error::other(e.to_string())))?;

    // Also reset passes in PRD
    if let Ok(mut prd) = PrdDocument::load(None) {
        if let Some(story) = prd.user_stories.iter_mut().find(|s| s.id == task_id) {
            story.passes = false;
        }
        let _ = prd.save(None);
    }

    println!(
        "\x1b[33m↺\x1b[0m Task \x1b[1m{}\x1b[0m reset to pending",
        task_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_command_error_display() {
        let err = ProgressCommandError::SaveError(std::io::Error::other("test error"));
        assert!(err.to_string().contains("Failed to save progress"));
    }
}
