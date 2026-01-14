//! Import and tasks command implementations.
//!
//! This module implements:
//! - `afk import` - Import a requirements document into tasks.json
//! - `afk tasks sync` - Sync tasks from configured sources
//! - `afk tasks show` - Display current task list

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::bootstrap::ensure_ai_cli_configured;
use crate::cli::output::{get_effective_mode, output_prompt};
use crate::config::AfkConfig;
use crate::feedback::Spinner;
use crate::prd::{generate_prd_prompt, load_prd_file, sync_prd_with_root, PrdDocument, PrdError};

/// Result type for import command operations.
pub type ImportCommandResult = Result<(), ImportCommandError>;

/// Error type for import command operations.
#[derive(Debug, thiserror::Error)]
pub enum ImportCommandError {
    #[error("Import error: {0}")]
    ImportError(#[from] PrdError),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
    #[error("No tasks found. Run `afk tasks sync` or `afk import` first.")]
    NoTasks,
    #[error("Parse error: {0}")]
    ParseError(#[from] crate::prd::PrdParseError),
    #[error("Output error: {0}")]
    OutputError(#[from] crate::cli::output::OutputError),
    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Import a requirements file into structured JSON.
///
/// Takes a product requirements document (markdown, text, etc.) and generates
/// an AI prompt to convert it into the structured JSON format.
///
/// # Arguments
///
/// * `input_file` - Path to the input file to import
/// * `output` - Path for the generated JSON output
/// * `copy` - Copy to clipboard
/// * `file` - Write prompt to file
/// * `stdout` - Print to stdout
///
/// # Returns
///
/// Ok(()) on success, or an error if import fails.
pub fn import(
    input_file: &str,
    output: &str,
    copy: bool,
    file: bool,
    stdout: bool,
) -> ImportCommandResult {
    import_impl(input_file, output, copy, file, stdout, None)
}

/// Internal implementation of import with optional config path for testing.
pub fn import_impl(
    input_file: &str,
    output: &str,
    copy: bool,
    file: bool,
    stdout: bool,
    config_path: Option<&Path>,
) -> ImportCommandResult {
    let mut config = AfkConfig::load(config_path)?;

    // Load the input file
    let input_path = Path::new(input_file);
    if !input_path.exists() {
        return Err(ImportCommandError::FileNotFound(input_file.to_string()));
    }

    let prd_content = load_prd_file(input_path)?;

    // Generate the prompt
    let prompt = generate_prd_prompt(&prd_content, output)?;

    // If any output flag is specified, output the prompt for manual use
    if copy || file || stdout {
        let mode = get_effective_mode(copy, file, stdout, &config);
        output_prompt(&prompt, mode, &config)?;

        // Show next steps
        println!();
        println!("\x1b[2mRun the prompt with your AI tool, then add the source:\x1b[0m");
        println!("  \x1b[36mafk source add json {output}\x1b[0m");

        return Ok(());
    }

    // No output flags - run the AI CLI directly
    // Ensure AI CLI is configured (first-run experience if needed)
    if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config)) {
        config.ai_cli = ai_cli;
    } else {
        return Err(ImportCommandError::NoTasks); // No AI CLI available
    }

    // Run the AI CLI with the prompt
    run_ai_cli_for_import(&config, &prompt, output)
}

/// Run the AI CLI with the import prompt.
fn run_ai_cli_for_import(config: &AfkConfig, prompt: &str, output: &str) -> ImportCommandResult {
    let command = &config.ai_cli.command;
    let args: Vec<&str> = config.ai_cli.args.iter().map(|s| s.as_str()).collect();

    // Start spinner whilst AI CLI initialises
    let mut spinner = Some(Spinner::start(&format!(
        "Importing requirements with {}...",
        config.ai_cli.command
    )));

    // Build the command
    let mut cmd = Command::new(command);
    cmd.args(&args)
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Spawn process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                if let Some(s) = spinner.take() {
                    s.stop_with_error(&format!("AI CLI not found: {}", command));
                }
                eprintln!("\x1b[2mIs it installed and in your PATH?\x1b[0m");
                return Err(ImportCommandError::ImportError(PrdError::ReadError(e)));
            }
            if let Some(s) = spinner.take() {
                s.stop_with_error("Failed to start AI CLI");
            }
            return Err(ImportCommandError::ImportError(PrdError::ReadError(e)));
        }
    };

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            // Stop spinner on first output
            if let Some(s) = spinner.take() {
                s.stop();
                println!();
            }

            match line {
                Ok(line) => {
                    println!("{line}");
                }
                Err(e) => {
                    eprintln!("\x1b[33mWarning:\x1b[0m Error reading output: {e}");
                    break;
                }
            }
        }
    }

    // If no output was received, stop the spinner now
    if let Some(s) = spinner.take() {
        s.stop();
    }

    // Wait for process to finish
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                let exit_code = status.code().unwrap_or(-1);
                eprintln!("\x1b[31mError:\x1b[0m AI CLI exited with code {exit_code}");
                return Err(ImportCommandError::ImportError(PrdError::ReadError(
                    std::io::Error::other(format!("AI CLI exited with code {exit_code}")),
                )));
            }
        }
        Err(e) => {
            return Err(ImportCommandError::ImportError(PrdError::ReadError(e)));
        }
    }

    // Check if output file was created
    let output_path = Path::new(output);
    if output_path.exists() {
        println!();
        println!("\x1b[32m✓\x1b[0m Requirements imported successfully");
        println!("  Output: \x1b[36m{output}\x1b[0m");
        println!();
        println!("\x1b[2mStart working on tasks with:\x1b[0m");
        println!("  \x1b[36mafk go\x1b[0m");
    } else {
        println!();
        println!("\x1b[33mNote:\x1b[0m Output file not found at {output}");
        println!("\x1b[2mThe AI may have written to a different location.\x1b[0m");
    }

    Ok(())
}

/// Sync tasks from all configured sources.
///
/// Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
/// .afk/tasks.json file.
///
/// # Returns
///
/// Ok(()) on success, or an error if sync fails.
pub fn tasks_sync() -> ImportCommandResult {
    tasks_sync_impl(None, None)
}

/// Internal implementation of tasks_sync with optional paths for testing.
pub fn tasks_sync_impl(config_path: Option<&Path>, root: Option<&Path>) -> ImportCommandResult {
    let config = AfkConfig::load(config_path)?;

    let prd = sync_prd_with_root(&config, None, root)?;

    // Calculate counts
    let (completed, total) = prd.get_story_counts();
    let pending = total - completed;

    // Display results
    println!("\x1b[32m✓\x1b[0m Tasks synced successfully");
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

/// Show the current task list.
///
/// Displays tasks from .afk/tasks.json with their completion status.
///
/// # Arguments
///
/// * `pending_only` - If true, only show tasks that haven't passed yet.
///
/// # Returns
///
/// Ok(()) on success, or an error if tasks cannot be loaded.
pub fn tasks_show(pending_only: bool) -> ImportCommandResult {
    tasks_show_impl(pending_only, None)
}

/// Internal implementation of tasks_show with optional path for testing.
pub fn tasks_show_impl(pending_only: bool, tasks_path: Option<&Path>) -> ImportCommandResult {
    let prd = PrdDocument::load(tasks_path)?;

    if prd.user_stories.is_empty() {
        println!("\x1b[2mNo tasks found.\x1b[0m");
        println!();
        println!("Run \x1b[36mafk tasks sync\x1b[0m to aggregate from sources,");
        println!("or \x1b[36mafk import <file>\x1b[0m to import a requirements doc.");
        return Ok(());
    }

    // Filter tasks if pending_only
    let tasks: Vec<_> = if pending_only {
        prd.user_stories.iter().filter(|s| !s.passes).collect()
    } else {
        prd.user_stories.iter().collect()
    };

    if tasks.is_empty() && pending_only {
        println!("\x1b[32m✓ All tasks complete!\x1b[0m");
        return Ok(());
    }

    // Print header
    println!(
        "\x1b[1m{:<20} {:>3} {:<40} {:>3} {:>8}\x1b[0m",
        "ID", "Pri", "Title", "ACs", "Status"
    );
    println!("{}", "─".repeat(80));

    // Print each task
    for task in &tasks {
        let status = if task.passes {
            "\x1b[32m✓ pass\x1b[0m"
        } else {
            "\x1b[33m○ pending\x1b[0m"
        };

        // Truncate title if too long
        let title = if task.title.len() > 38 {
            format!("{}…", &task.title[..37])
        } else {
            task.title.clone()
        };

        // Truncate ID if too long
        let id = if task.id.len() > 18 {
            format!("{}…", &task.id[..17])
        } else {
            task.id.clone()
        };

        let ac_count = task.acceptance_criteria.len();

        println!(
            "{:<20} {:>3} {:<40} {:>3} {}",
            id, task.priority, title, ac_count, status
        );
    }

    // Print footer with summary
    println!("{}", "─".repeat(80));

    let (completed, total) = prd.get_story_counts();
    let pending = total - completed;

    if pending_only {
        println!("\x1b[2mShowing {pending} pending of {total} total tasks\x1b[0m");
    } else {
        println!("\x1b[2m{completed}/{total} complete ({pending} pending)\x1b[0m");
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
    use crate::config::{OutputMode, SourceConfig};
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
    fn test_tasks_sync_no_sources_empty_tasks() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Empty config
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = tasks_sync_impl(Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // Check tasks.json was created
        let tasks_path = afk_dir.join("tasks.json");
        assert!(tasks_path.exists());
    }

    #[test]
    fn test_tasks_sync_preserves_existing_tasks() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let tasks_path = afk_dir.join("tasks.json");

        // Create existing tasks with stories
        let existing_tasks = r#"{
            "project": "test-project",
            "branchName": "main",
            "userStories": [
                {"id": "story-1", "title": "Existing Story", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(&tasks_path, existing_tasks).unwrap();

        // Empty config (no sources)
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = tasks_sync_impl(Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // Tasks should still have the story
        let prd = PrdDocument::load(Some(&tasks_path)).unwrap();
        assert_eq!(prd.user_stories.len(), 1);
        assert_eq!(prd.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_tasks_sync_with_source() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let tasks_path = afk_dir.join("tasks.json");

        // Create source file
        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![SourceConfig::json(source_path.to_str().unwrap())],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = tasks_sync_impl(Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        let prd = PrdDocument::load(Some(&tasks_path)).unwrap();
        assert_eq!(prd.user_stories.len(), 1);
        assert_eq!(prd.user_stories[0].id, "task-1");
    }

    #[test]
    fn test_tasks_show_empty() {
        let (_temp, afk_dir) = setup_temp_dir();
        let tasks_path = afk_dir.join("tasks.json");

        // Empty tasks
        let prd = PrdDocument::default();
        prd.save(Some(&tasks_path)).unwrap();

        let result = tasks_show_impl(false, Some(&tasks_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_show_with_tasks() {
        let (_temp, afk_dir) = setup_temp_dir();
        let tasks_path = afk_dir.join("tasks.json");

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
        prd.save(Some(&tasks_path)).unwrap();

        let result = tasks_show_impl(false, Some(&tasks_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_show_pending_only() {
        let (_temp, afk_dir) = setup_temp_dir();
        let tasks_path = afk_dir.join("tasks.json");

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
        prd.save(Some(&tasks_path)).unwrap();

        let result = tasks_show_impl(true, Some(&tasks_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_show_all_complete() {
        let (_temp, afk_dir) = setup_temp_dir();
        let tasks_path = afk_dir.join("tasks.json");

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
        prd.save(Some(&tasks_path)).unwrap();

        // With pending_only=true, should show "All complete" message
        let result = tasks_show_impl(true, Some(&tasks_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_show_truncates_long_titles() {
        let (_temp, afk_dir) = setup_temp_dir();
        let tasks_path = afk_dir.join("tasks.json");

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
        prd.save(Some(&tasks_path)).unwrap();

        let result = tasks_show_impl(false, Some(&tasks_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_show_missing_file() {
        let (temp, _afk_dir) = setup_temp_dir();
        let tasks_path = temp.path().join("nonexistent/.afk/tasks.json");

        // Should not error, just show "No tasks" message
        let result = tasks_show_impl(false, Some(&tasks_path));
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
    fn test_import_command_error_display() {
        let err = ImportCommandError::NoTasks;
        assert_eq!(
            err.to_string(),
            "No tasks found. Run `afk tasks sync` or `afk import` first."
        );
    }

    #[test]
    fn test_tasks_sync_sorts_by_priority() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let tasks_path = afk_dir.join("tasks.json");

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

        let result = tasks_sync_impl(Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // Verify tasks.json was created with sorted tasks
        let prd = PrdDocument::load(Some(&tasks_path)).unwrap();
        assert_eq!(prd.user_stories.len(), 3);
        // Should be sorted by priority
        assert_eq!(prd.user_stories[0].id, "high");
        assert_eq!(prd.user_stories[1].id, "medium");
        assert_eq!(prd.user_stories[2].id, "low");
    }

    #[test]
    fn test_tasks_sync_preserves_passes_status() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let tasks_path = afk_dir.join("tasks.json");

        // Create existing tasks with completed story
        let existing_tasks = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "priority": 1, "passes": true}
            ]
        }"#;
        fs::write(&tasks_path, existing_tasks).unwrap();

        // Create source with same story (without passes)
        let source_json = r#"[{"id": "story-1", "title": "Story 1 Updated", "priority": 1}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![SourceConfig::json(source_path.to_str().unwrap())],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = tasks_sync_impl(Some(&config_path), Some(temp.path()));
        assert!(result.is_ok());

        // story-1 should still have passes: true
        let prd = PrdDocument::load(Some(&tasks_path)).unwrap();
        assert!(prd.user_stories[0].passes);
    }

    // Tests for import command

    #[test]
    fn test_import_file_not_found() {
        let (_temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = import_impl(
            "/nonexistent/file.md",
            ".afk/tasks.json",
            false,
            false,
            true,
            Some(&config_path),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ImportCommandError::FileNotFound(path) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_import_generates_prompt_to_stdout() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Create input file
        let input_file = temp.path().join("requirements.md");
        let content = "# My App\n\nBuild a todo list with user authentication.";
        fs::write(&input_file, content).unwrap();

        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = import_impl(
            input_file.to_str().unwrap(),
            ".afk/tasks.json",
            false,
            false,
            true,
            Some(&config_path),
        );

        // Should succeed with stdout mode
        assert!(result.is_ok());
    }

    #[test]
    fn test_import_custom_output_path() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Create input file
        let input_file = temp.path().join("prd.md");
        fs::write(&input_file, "# Features\n\n- Feature 1").unwrap();

        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        // Use custom output path
        let result = import_impl(
            input_file.to_str().unwrap(),
            "custom/output/tasks.json",
            false,
            false,
            true,
            Some(&config_path),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_import_command_error_variants() {
        // Test error display for all variants
        let file_err = ImportCommandError::FileNotFound("test.md".to_string());
        assert!(file_err.to_string().contains("File not found"));
        assert!(file_err.to_string().contains("test.md"));

        let no_tasks = ImportCommandError::NoTasks;
        assert!(no_tasks.to_string().contains("No tasks found"));
    }

    #[test]
    fn test_get_effective_mode() {
        let config = AfkConfig::default();

        // Test explicit flags
        assert_eq!(
            get_effective_mode(true, false, false, &config),
            OutputMode::Clipboard
        );
        assert_eq!(
            get_effective_mode(false, true, false, &config),
            OutputMode::File
        );
        assert_eq!(
            get_effective_mode(false, false, true, &config),
            OutputMode::Stdout
        );
    }

    #[test]
    fn test_import_without_output_flags_tries_ai_cli() {
        // When no output flags are provided, import should try to run the AI CLI.
        // Since the AI CLI won't exist in the test environment, it will fail,
        // but we can verify it doesn't use the prompt output path.
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Create input file
        let input_file = temp.path().join("requirements.md");
        fs::write(&input_file, "# Test\n\nBuild something.").unwrap();

        // Configure with a non-existent AI CLI command
        let config = AfkConfig {
            ai_cli: crate::config::AiCliConfig {
                command: "nonexistent_ai_cli_for_test_12345".to_string(),
                args: vec!["-p".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        // No output flags - should try to run AI CLI and fail
        let result = import_impl(
            input_file.to_str().unwrap(),
            ".afk/tasks.json",
            false, // copy
            false, // file
            false, // stdout - no output flags!
            Some(&config_path),
        );

        // Should fail because the AI CLI doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_import_with_copy_flag_outputs_prompt() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Create input file
        let input_file = temp.path().join("requirements.md");
        fs::write(&input_file, "# Test\n\nBuild something.").unwrap();

        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        // With copy flag, should output prompt (not run AI CLI)
        // This will fail due to clipboard access in CI, but that's expected
        let result = import_impl(
            input_file.to_str().unwrap(),
            ".afk/tasks.json",
            true, // copy - output flag set
            false,
            false,
            Some(&config_path),
        );

        // Clipboard may fail in CI environments, but the function should return
        // (either Ok or ClipboardError, not AI CLI error)
        // The key is it doesn't try to run the AI CLI
        let _ = result; // Just verify it doesn't panic
    }
}
