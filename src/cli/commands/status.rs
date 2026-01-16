//! Status command implementation.
//!
//! This module implements the `afk status` command for showing current status.

use std::path::Path;

use crate::config::AfkConfig;
use crate::prd::PrdDocument;
use crate::progress::{SessionProgress, TaskStatus};

/// Result type for status command operations.
pub type StatusCommandResult = Result<(), StatusCommandError>;

/// Error type for status command operations.
#[derive(Debug, thiserror::Error)]
pub enum StatusCommandError {
    /// The afk project is not initialised.
    #[error("afk not initialised")]
    NotInitialised,
}

/// Execute the status command.
pub fn status(verbose: bool) -> StatusCommandResult {
    // Check if initialised
    if !Path::new(".afk").exists() {
        println!("\x1b[33mafk not initialised.\x1b[0m");
        println!("Run \x1b[1mafk init\x1b[0m or \x1b[1mafk go\x1b[0m to get started.");
        return Ok(());
    }

    let config = AfkConfig::load(None).unwrap_or_default();
    let prd = PrdDocument::load(None).unwrap_or_default();
    let progress = SessionProgress::load(None).unwrap_or_default();

    println!("\x1b[1m=== afk status ===\x1b[0m");
    println!();

    // Task summary
    let (completed, total) = prd.get_story_counts();
    let pending = total - completed;

    println!("\x1b[1mTasks\x1b[0m");
    if total == 0 {
        println!("  No tasks configured.");
    } else {
        println!("  Total: {total} ({completed} complete, {pending} pending)");

        // Show current in-progress task(s)
        let in_progress_tasks = progress.get_in_progress_tasks();
        for task in &in_progress_tasks {
            // Look up story details from PRD for title
            let title = prd
                .user_stories
                .iter()
                .find(|s| s.id == task.id)
                .map(|s| {
                    if s.title.len() > 50 {
                        format!("{}...", &s.title[..47])
                    } else {
                        s.title.clone()
                    }
                })
                .unwrap_or_else(|| "(unknown)".to_string());
            println!("  Current: \x1b[33m{}\x1b[0m - {}", task.id, title);
        }

        // Show next pending task
        if let Some(next) = prd.get_next_story() {
            let title = if next.title.len() > 50 {
                format!("{}...", &next.title[..47])
            } else {
                next.title.clone()
            };
            println!("  Next: \x1b[36m{}\x1b[0m - {}", next.id, title);
        }
    }
    println!();

    // Session progress
    println!("\x1b[1mSession\x1b[0m");
    println!(
        "  Started: {}",
        &progress.started_at[..19].replace('T', " ")
    );
    println!("  Iterations: {}", progress.iterations);

    // Calculate task counts from PRD with session status overlays
    // This ensures counts are consistent with the Tasks section
    let (pend, in_prog, comp, fail, skip) = calculate_merged_task_counts(&prd, &progress);
    if pend + in_prog + comp + fail + skip > 0 {
        println!(
            "  Tasks: {} pending, {} in-progress, {} complete, {} failed, {} skipped",
            pend, in_prog, comp, fail, skip
        );
    }
    println!();

    // Sources
    println!("\x1b[1mSources\x1b[0m");
    if config.sources.is_empty() {
        println!("  (none configured)");
    } else {
        for (i, source) in config.sources.iter().enumerate() {
            let desc = match &source.source_type {
                crate::config::SourceType::Beads => "beads".to_string(),
                crate::config::SourceType::Json => {
                    format!("json: {}", source.path.as_deref().unwrap_or("?"))
                }
                crate::config::SourceType::Markdown => {
                    format!("markdown: {}", source.path.as_deref().unwrap_or("?"))
                }
                crate::config::SourceType::Github => {
                    format!(
                        "github: {}",
                        source.repo.as_deref().unwrap_or("current repo")
                    )
                }
            };
            println!("  {}. {}", i + 1, desc);
        }
    }
    println!();

    // AI CLI
    println!("\x1b[1mAI CLI\x1b[0m");
    println!(
        "  Command: {} {}",
        config.ai_cli.command,
        config.ai_cli.args.join(" ")
    );

    // Verbose mode: show additional details
    if verbose {
        print_verbose_details(&config, &prd, &progress);
    }

    Ok(())
}

/// Calculate task counts by merging PRD data with session progress.
///
/// This ensures the Session section's counts are consistent with the Tasks section.
/// For each story in the PRD:
/// - If it exists in progress.json, use the session status
/// - If it doesn't exist in progress.json:
///   - If `passes` is true, count as "completed"
///   - If `passes` is false, count as "pending"
fn calculate_merged_task_counts(
    prd: &PrdDocument,
    progress: &SessionProgress,
) -> (usize, usize, usize, usize, usize) {
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for story in &prd.user_stories {
        if let Some(task_progress) = progress.tasks.get(&story.id) {
            // Use session status if task is tracked
            match task_progress.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Failed => failed += 1,
                TaskStatus::Skipped => skipped += 1,
            }
        } else {
            // Fall back to PRD passes status
            if story.passes {
                completed += 1;
            } else {
                pending += 1;
            }
        }
    }

    (pending, in_progress, completed, failed, skipped)
}

/// Print verbose status details.
fn print_verbose_details(config: &AfkConfig, prd: &PrdDocument, progress: &SessionProgress) {
    println!();

    // Feedback Loops
    println!("\x1b[1mFeedback Loops\x1b[0m");
    let fb = &config.feedback_loops;
    let has_any = fb.types.is_some()
        || fb.lint.is_some()
        || fb.test.is_some()
        || fb.build.is_some()
        || !fb.custom.is_empty();
    if has_any {
        if let Some(ref cmd) = fb.types {
            println!("  types: {cmd}");
        }
        if let Some(ref cmd) = fb.lint {
            println!("  lint: {cmd}");
        }
        if let Some(ref cmd) = fb.test {
            println!("  test: {cmd}");
        }
        if let Some(ref cmd) = fb.build {
            println!("  build: {cmd}");
        }
        for (name, cmd) in &fb.custom {
            println!("  {name}: {cmd}");
        }
    } else {
        println!("  (none configured)");
    }
    println!();

    // Pending Stories
    println!("\x1b[1mPending Stories\x1b[0m");
    let pending_stories = prd.get_pending_stories();
    if pending_stories.is_empty() {
        println!("  (none)");
    } else {
        for story in pending_stories.iter().take(5) {
            println!("  - {} (P{}) {}", story.id, story.priority, story.title);
        }
        if pending_stories.len() > 5 {
            println!("  ... and {} more", pending_stories.len() - 5);
        }
    }
    println!();

    // Recent Learnings
    println!("\x1b[1mRecent Learnings\x1b[0m");
    let learnings = progress.get_recent_learnings(5);
    if learnings.is_empty() {
        println!("  (none recorded)");
    } else {
        for (i, (task_id, learning)) in learnings.iter().enumerate() {
            let truncated = if learning.len() > 60 {
                format!("{}...", &learning[..57])
            } else {
                learning.clone()
            };
            println!("  {}. [{}] {}", i + 1, task_id, truncated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::UserStory;
    use crate::progress::TaskProgress;

    #[test]
    fn test_status_command_error_display() {
        let err = StatusCommandError::NotInitialised;
        assert_eq!(err.to_string(), "afk not initialised");
    }

    #[test]
    fn test_calculate_merged_task_counts_empty() {
        let prd = PrdDocument::default();
        let progress = SessionProgress::default();

        let (pend, in_prog, comp, fail, skip) = calculate_merged_task_counts(&prd, &progress);
        assert_eq!((pend, in_prog, comp, fail, skip), (0, 0, 0, 0, 0));
    }

    #[test]
    fn test_calculate_merged_task_counts_prd_only() {
        // Tasks in PRD but not tracked in progress
        let mut prd = PrdDocument::default();
        prd.user_stories = vec![
            UserStory {
                id: "task-1".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "task-2".to_string(),
                passes: true,
                ..Default::default()
            },
            UserStory {
                id: "task-3".to_string(),
                passes: false,
                ..Default::default()
            },
        ];
        let progress = SessionProgress::default();

        let (pend, in_prog, comp, fail, skip) = calculate_merged_task_counts(&prd, &progress);
        // 2 pending (passes=false), 1 completed (passes=true)
        assert_eq!((pend, in_prog, comp, fail, skip), (2, 0, 1, 0, 0));
    }

    #[test]
    fn test_calculate_merged_task_counts_with_progress_overlay() {
        // Tasks in PRD with session progress overlay
        let mut prd = PrdDocument::default();
        prd.user_stories = vec![
            UserStory {
                id: "task-1".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "task-2".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "task-3".to_string(),
                passes: false,
                ..Default::default()
            },
        ];

        let mut progress = SessionProgress::default();
        // task-1 is in_progress in session
        let mut tp1 = TaskProgress::new("task-1", "prd");
        tp1.status = TaskStatus::InProgress;
        progress.tasks.insert("task-1".to_string(), tp1);

        let (pend, in_prog, comp, fail, skip) = calculate_merged_task_counts(&prd, &progress);
        // task-1 is in_progress (from session), task-2 and task-3 are pending (from PRD)
        assert_eq!((pend, in_prog, comp, fail, skip), (2, 1, 0, 0, 0));
    }

    #[test]
    fn test_calculate_merged_task_counts_all_statuses() {
        let mut prd = PrdDocument::default();
        prd.user_stories = vec![
            UserStory {
                id: "pending-prd".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "completed-prd".to_string(),
                passes: true,
                ..Default::default()
            },
            UserStory {
                id: "in-progress".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "failed".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "skipped".to_string(),
                passes: false,
                ..Default::default()
            },
            UserStory {
                id: "completed-session".to_string(),
                passes: false, // Not yet marked in PRD
                ..Default::default()
            },
        ];

        let mut progress = SessionProgress::default();

        let mut tp_in_prog = TaskProgress::new("in-progress", "prd");
        tp_in_prog.status = TaskStatus::InProgress;
        progress.tasks.insert("in-progress".to_string(), tp_in_prog);

        let mut tp_failed = TaskProgress::new("failed", "prd");
        tp_failed.status = TaskStatus::Failed;
        progress.tasks.insert("failed".to_string(), tp_failed);

        let mut tp_skipped = TaskProgress::new("skipped", "prd");
        tp_skipped.status = TaskStatus::Skipped;
        progress.tasks.insert("skipped".to_string(), tp_skipped);

        let mut tp_comp = TaskProgress::new("completed-session", "prd");
        tp_comp.status = TaskStatus::Completed;
        progress
            .tasks
            .insert("completed-session".to_string(), tp_comp);

        let (pend, in_prog, comp, fail, skip) = calculate_merged_task_counts(&prd, &progress);
        // pending-prd: pending (from PRD)
        // completed-prd: completed (from PRD)
        // in-progress: in_progress (from session)
        // failed: failed (from session)
        // skipped: skipped (from session)
        // completed-session: completed (from session)
        assert_eq!((pend, in_prog, comp, fail, skip), (1, 1, 2, 1, 1));
    }
}
