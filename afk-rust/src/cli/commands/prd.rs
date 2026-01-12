//! PRD command implementations.
//!
//! This module implements the `afk prd sync` and `afk prd show` commands
//! for managing the product requirements document.

use std::path::Path;

use crate::config::AfkConfig;
use crate::prd::{sync_prd_with_root, PrdDocument, PrdError};

/// Result type for PRD command operations.
pub type PrdCommandResult = Result<(), PrdCommandError>;

/// Error type for PRD command operations.
#[derive(Debug, thiserror::Error)]
pub enum PrdCommandError {
    #[error("PRD error: {0}")]
    PrdError(#[from] PrdError),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
    #[error("No PRD found. Run `afk prd sync` or `afk prd parse` first.")]
    NoPrd,
}

/// Sync PRD from all configured sources.
///
/// Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
/// .afk/prd.json file.
///
/// # Arguments
///
/// * `branch` - Optional branch name override.
///
/// # Returns
///
/// Ok(()) on success, or an error if sync fails.
pub fn prd_sync(branch: Option<&str>) -> PrdCommandResult {
    prd_sync_impl(branch, None, None)
}

/// Internal implementation of prd_sync with optional paths for testing.
pub fn prd_sync_impl(
    branch: Option<&str>,
    config_path: Option<&Path>,
    root: Option<&Path>,
) -> PrdCommandResult {
    let config = AfkConfig::load(config_path)?;

    let prd = sync_prd_with_root(&config, branch, root)?;

    // Calculate counts
    let (completed, total) = prd.get_story_counts();
    let pending = total - completed;

    // Display results
    println!("\x1b[32m✓\x1b[0m PRD synced successfully");
    println!();
    println!("  \x1b[36mTotal:\x1b[0m    {total}");
    println!("  \x1b[33mPending:\x1b[0m  {pending}");
    println!("  \x1b[32mComplete:\x1b[0m {completed}");

    if !prd.branch_name.is_empty() {
        println!();
        println!("  \x1b[2mBranch:\x1b[0m   {}", prd.branch_name);
    }

    Ok(())
}

/// Show the current PRD state.
///
/// Displays user stories from .afk/prd.json with their completion status.
///
/// # Arguments
///
/// * `pending_only` - If true, only show stories that haven't passed yet.
///
/// # Returns
///
/// Ok(()) on success, or an error if PRD cannot be loaded.
pub fn prd_show(pending_only: bool) -> PrdCommandResult {
    prd_show_impl(pending_only, None)
}

/// Internal implementation of prd_show with optional path for testing.
pub fn prd_show_impl(pending_only: bool, prd_path: Option<&Path>) -> PrdCommandResult {
    let prd = PrdDocument::load(prd_path)?;

    if prd.user_stories.is_empty() {
        println!("\x1b[2mNo stories in PRD.\x1b[0m");
        println!();
        println!("Run \x1b[36mafk prd sync\x1b[0m to aggregate from sources,");
        println!("or \x1b[36mafk prd parse <file>\x1b[0m to parse a requirements doc.");
        return Ok(());
    }

    // Filter stories if pending_only
    let stories: Vec<_> = if pending_only {
        prd.user_stories.iter().filter(|s| !s.passes).collect()
    } else {
        prd.user_stories.iter().collect()
    };

    if stories.is_empty() && pending_only {
        println!("\x1b[32m✓ All stories complete!\x1b[0m");
        return Ok(());
    }

    // Print header
    println!(
        "\x1b[1m{:<20} {:>3} {:<40} {:>3} {:>8}\x1b[0m",
        "ID", "Pri", "Title", "ACs", "Status"
    );
    println!("{}", "─".repeat(80));

    // Print each story
    for story in &stories {
        let status = if story.passes {
            "\x1b[32m✓ pass\x1b[0m"
        } else {
            "\x1b[33m○ pending\x1b[0m"
        };

        // Truncate title if too long
        let title = if story.title.len() > 38 {
            format!("{}…", &story.title[..37])
        } else {
            story.title.clone()
        };

        // Truncate ID if too long
        let id = if story.id.len() > 18 {
            format!("{}…", &story.id[..17])
        } else {
            story.id.clone()
        };

        let ac_count = story.acceptance_criteria.len();

        println!(
            "{:<20} {:>3} {:<40} {:>3} {}",
            id, story.priority, title, ac_count, status
        );
    }

    // Print footer with summary
    println!("{}", "─".repeat(80));

    let (completed, total) = prd.get_story_counts();
    let pending = total - completed;

    if pending_only {
        println!("\x1b[2mShowing {pending} pending of {total} total stories\x1b[0m");
    } else {
        println!(
            "\x1b[2m{completed}/{total} complete ({pending} pending)\x1b[0m"
        );
    }

    // Show branch and last synced info
    if !prd.branch_name.is_empty() || !prd.last_synced.is_empty() {
        println!();
        if !prd.branch_name.is_empty() {
            println!("\x1b[2mBranch:\x1b[0m     {}", prd.branch_name);
        }
        if !prd.last_synced.is_empty() {
            // Format the timestamp more nicely
            let synced = format_timestamp(&prd.last_synced);
            println!("\x1b[2mLast synced:\x1b[0m {synced}");
        }
    }

    Ok(())
}

/// Format an ISO timestamp for display.
fn format_timestamp(ts: &str) -> String {
    // Try to parse and reformat, or return as-is if it fails
    if ts.contains('T') {
        // Simple formatting: replace T with space and truncate microseconds
        let parts: Vec<&str> = ts.split('.').collect();
        parts[0].replace('T', " ")
    } else {
        ts.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SourceConfig;
    use crate::prd::UserStory;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to set up a temp directory with .afk subdirectory.
    fn setup_temp_dir() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        (temp, afk_dir)
    }

    #[test]
    fn test_prd_sync_no_sources_empty_prd() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Empty config
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = prd_sync_impl(None, Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // Check PRD was created
        let prd_path = afk_dir.join("prd.json");
        assert!(prd_path.exists());
    }

    #[test]
    fn test_prd_sync_preserves_existing_prd() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let prd_path = afk_dir.join("prd.json");

        // Create existing PRD with stories
        let existing_prd = r#"{
            "project": "test-project",
            "branchName": "main",
            "userStories": [
                {"id": "story-1", "title": "Existing Story", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(&prd_path, existing_prd).unwrap();

        // Empty config (no sources)
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = prd_sync_impl(None, Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // PRD should still have the story
        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert_eq!(prd.user_stories.len(), 1);
        assert_eq!(prd.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_prd_sync_with_branch_override() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let prd_path = afk_dir.join("prd.json");

        // Create source file
        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![SourceConfig::json(source_path.to_str().unwrap())],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = prd_sync_impl(Some("feature/custom"), Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert_eq!(prd.branch_name, "feature/custom");
    }

    #[test]
    fn test_prd_show_empty_prd() {
        let (temp, afk_dir) = setup_temp_dir();
        let prd_path = afk_dir.join("prd.json");

        // Empty PRD
        let prd = PrdDocument::default();
        prd.save(Some(&prd_path)).unwrap();

        let result = prd_show_impl(false, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_prd_show_with_stories() {
        let (_temp, afk_dir) = setup_temp_dir();
        let prd_path = afk_dir.join("prd.json");

        let prd = PrdDocument {
            project: "test-project".to_string(),
            branch_name: "main".to_string(),
            user_stories: vec![
                UserStory {
                    id: "story-1".to_string(),
                    title: "First Story".to_string(),
                    priority: 1,
                    passes: true,
                    acceptance_criteria: vec!["AC1".to_string(), "AC2".to_string()],
                    ..Default::default()
                },
                UserStory {
                    id: "story-2".to_string(),
                    title: "Second Story".to_string(),
                    priority: 2,
                    passes: false,
                    acceptance_criteria: vec!["AC3".to_string()],
                    ..Default::default()
                },
            ],
            last_synced: "2024-01-12T10:30:00.000000".to_string(),
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let result = prd_show_impl(false, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_prd_show_pending_only() {
        let (_temp, afk_dir) = setup_temp_dir();
        let prd_path = afk_dir.join("prd.json");

        let prd = PrdDocument {
            project: "test-project".to_string(),
            user_stories: vec![
                UserStory {
                    id: "done-1".to_string(),
                    title: "Done Story".to_string(),
                    priority: 1,
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "pending-1".to_string(),
                    title: "Pending Story".to_string(),
                    priority: 2,
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let result = prd_show_impl(true, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_prd_show_all_complete() {
        let (_temp, afk_dir) = setup_temp_dir();
        let prd_path = afk_dir.join("prd.json");

        let prd = PrdDocument {
            project: "test-project".to_string(),
            user_stories: vec![
                UserStory {
                    id: "done-1".to_string(),
                    title: "Done Story 1".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "done-2".to_string(),
                    title: "Done Story 2".to_string(),
                    passes: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        // With pending_only=true, should show "All complete" message
        let result = prd_show_impl(true, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_prd_show_truncates_long_titles() {
        let (_temp, afk_dir) = setup_temp_dir();
        let prd_path = afk_dir.join("prd.json");

        let prd = PrdDocument {
            project: "test-project".to_string(),
            user_stories: vec![UserStory {
                id: "story-with-very-long-id-that-needs-truncation".to_string(),
                title: "This is a very long title that definitely exceeds the 38 character limit we have set".to_string(),
                priority: 1,
                passes: false,
                ..Default::default()
            }],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let result = prd_show_impl(false, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_prd_show_missing_prd() {
        let (temp, _afk_dir) = setup_temp_dir();
        let prd_path = temp.path().join("nonexistent/.afk/prd.json");

        // Should not error, just show "No stories" message
        let result = prd_show_impl(false, Some(&prd_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(
            format_timestamp("2024-01-12T10:30:00.000000"),
            "2024-01-12 10:30:00"
        );
        assert_eq!(
            format_timestamp("2024-01-12T10:30:00"),
            "2024-01-12 10:30:00"
        );
        assert_eq!(format_timestamp("plain text"), "plain text");
    }

    #[test]
    fn test_prd_command_error_display() {
        let err = PrdCommandError::NoPrd;
        assert_eq!(
            err.to_string(),
            "No PRD found. Run `afk prd sync` or `afk prd parse` first."
        );
    }

    #[test]
    fn test_prd_sync_with_source() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let prd_path = afk_dir.join("prd.json");

        // Create source file with multiple tasks
        let source_json = r#"[
            {"id": "high", "title": "High Priority", "priority": 1},
            {"id": "low", "title": "Low Priority", "priority": 3},
            {"id": "medium", "title": "Medium Priority", "priority": 2}
        ]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![SourceConfig::json(source_path.to_str().unwrap())],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = prd_sync_impl(None, Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // Verify PRD was created with sorted stories
        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert_eq!(prd.user_stories.len(), 3);
        // Should be sorted by priority
        assert_eq!(prd.user_stories[0].id, "high");
        assert_eq!(prd.user_stories[1].id, "medium");
        assert_eq!(prd.user_stories[2].id, "low");
    }

    #[test]
    fn test_prd_sync_preserves_passes_status() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let prd_path = afk_dir.join("prd.json");

        // Create existing PRD with completed story
        let existing_prd = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "priority": 1, "passes": true}
            ]
        }"#;
        fs::write(&prd_path, existing_prd).unwrap();

        // Create source with same story (without passes)
        let source_json = r#"[{"id": "story-1", "title": "Story 1 Updated", "priority": 1}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![SourceConfig::json(source_path.to_str().unwrap())],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = prd_sync_impl(None, Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // story-1 should still have passes: true
        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert!(prd.user_stories[0].passes);
    }
}
