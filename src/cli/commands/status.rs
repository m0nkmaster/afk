//! Status command implementation.
//!
//! This module implements the `afk status` command for showing current status.

use std::path::Path;

use crate::config::AfkConfig;
use crate::prd::PrdDocument;
use crate::progress::SessionProgress;

/// Result type for status command operations.
pub type StatusCommandResult = Result<(), StatusCommandError>;

/// Error type for status command operations.
#[derive(Debug, thiserror::Error)]
pub enum StatusCommandError {
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
    let (pend, in_prog, comp, fail, skip) = progress.get_task_counts();
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

/// Print verbose status details.
fn print_verbose_details(
    config: &AfkConfig,
    prd: &PrdDocument,
    progress: &SessionProgress,
) {
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

    #[test]
    fn test_status_command_error_display() {
        let err = StatusCommandError::NotInitialised;
        assert_eq!(err.to_string(), "afk not initialised");
    }
}
