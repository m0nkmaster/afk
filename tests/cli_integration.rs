//! CLI integration tests.
//!
//! These tests invoke the afk binary and verify command output and behaviour.

#![allow(deprecated)] // cargo_bin is deprecated but still works

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get a Command for the afk binary.
fn afk() -> Command {
    Command::cargo_bin("afk").unwrap()
}

/// Helper to create a temp directory with .afk/config.json.
fn setup_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Minimal config
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": []
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    temp
}

/// Helper to create a project with a PRD file.
fn setup_project_with_prd() -> TempDir {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-001",
                "title": "First task",
                "description": "Do the first thing",
                "acceptanceCriteria": ["It works"],
                "priority": 1,
                "passes": false
            },
            {
                "id": "task-002",
                "title": "Second task",
                "description": "Do the second thing",
                "acceptanceCriteria": ["It also works"],
                "priority": 2,
                "passes": true
            }
        ]
    }"#;
    fs::write(afk_dir.join("tasks.json"), tasks).unwrap();

    temp
}

// ============================================================================
// Basic CLI tests
// ============================================================================

#[test]
fn test_no_args_shows_help_message() {
    afk()
        .assert()
        .success()
        .stdout(predicate::str::contains("afk"))
        .stdout(predicate::str::contains("Quick start"));
}

#[test]
fn test_help_flag() {
    afk()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_version_flag() {
    afk()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("afk"));
}

// ============================================================================
// Init command tests
// ============================================================================

/// Helper to create a mock AI CLI in a temp directory and return the PATH.
#[cfg(unix)]
fn setup_mock_ai_cli(temp: &TempDir) -> String {
    use std::os::unix::fs::PermissionsExt;

    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create a mock 'claude' script that responds to --version
    let mock_cli = bin_dir.join("claude");
    fs::write(&mock_cli, "#!/bin/sh\necho 'claude 1.0.0'\n").unwrap();
    fs::set_permissions(&mock_cli, fs::Permissions::from_mode(0o755)).unwrap();

    // Return modified PATH with our bin dir first
    format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

#[cfg(windows)]
fn setup_mock_ai_cli(temp: &TempDir) -> String {
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Create a mock 'claude.cmd' batch file
    let mock_cli = bin_dir.join("claude.cmd");
    fs::write(&mock_cli, "@echo claude 1.0.0\n").unwrap();

    // Return modified PATH with our bin dir first
    format!(
        "{};{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

#[test]
fn test_init_creates_afk_directory() {
    let temp = TempDir::new().unwrap();
    let path_with_mock = setup_mock_ai_cli(&temp);

    afk()
        .current_dir(temp.path())
        .env("PATH", &path_with_mock)
        .args(["init", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Analysing project"));

    assert!(temp.path().join(".afk").exists());
    assert!(temp.path().join(".afk/config.json").exists());
}

#[test]
fn test_init_dry_run_does_not_create_files() {
    let temp = TempDir::new().unwrap();
    let path_with_mock = setup_mock_ai_cli(&temp);

    afk()
        .current_dir(temp.path())
        .env("PATH", &path_with_mock)
        .args(["init", "-n", "-y"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    assert!(!temp.path().join(".afk").exists());
}

#[test]
fn test_init_force_overwrites_existing() {
    let temp = setup_project();

    // Modify config
    let config_path = temp.path().join(".afk/config.json");
    fs::write(&config_path, r#"{"custom": true}"#).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["init", "-f", "-y"])
        .assert()
        .success();

    // Config should be regenerated (not our custom one)
    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(!contents.contains("\"custom\""));
}

#[test]
fn test_init_already_initialised_warns() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["init"])
        .assert()
        .success()
        // Output goes to stderr for warnings
        .stderr(predicate::str::contains("Already initialised"));
}

// ============================================================================
// Status command tests
// ============================================================================

#[test]
fn test_status_no_config() {
    let temp = TempDir::new().unwrap();

    // Status shows warning but doesn't fail
    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("not initialised"));
}

#[test]
fn test_status_with_config() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("afk status").or(predicate::str::contains("Tasks")));
}

// ============================================================================
// Tasks commands tests
// ============================================================================

#[test]
fn test_tasks_show_displays_tasks() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        .stdout(predicate::str::contains("First task"));
}

#[test]
fn test_tasks_show_pending_only() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "show", "-p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        // task-002 is passed, so it shouldn't show
        .stdout(predicate::str::contains("task-002").not());
}

#[test]
fn test_tasks_show_no_tasks_file() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks").or(predicate::str::contains("empty")));
}

#[test]
fn test_tasks_sync_creates_tasks_file() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Should create/update tasks.json even with no sources
    assert!(temp.path().join(".afk/tasks.json").exists());
}

#[test]
fn test_import_file_not_found() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["import", "nonexistent.md"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("No such file")));
}

#[test]
fn test_import_generates_prompt() {
    let temp = setup_project();
    let requirements = temp.path().join("requirements.md");
    fs::write(&requirements, "# My App\n\n- Feature 1\n- Feature 2").unwrap();

    afk()
        .current_dir(temp.path())
        .args(["import", "requirements.md", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("My App").or(predicate::str::contains("Feature")));
}

// ============================================================================
// Source commands tests
// ============================================================================

#[test]
fn test_source_list_empty() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["source", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No sources"));
}

#[test]
fn test_source_add_json() {
    let temp = setup_project();
    let tasks_file = temp.path().join("tasks.json");
    fs::write(&tasks_file, r#"{"tasks": []}"#).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["source", "add", "json", "tasks.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"));

    // Verify it's in the config
    let config = fs::read_to_string(temp.path().join(".afk/config.json")).unwrap();
    assert!(config.contains("json") || config.contains("tasks.json"));
}

#[test]
fn test_source_add_markdown() {
    let temp = setup_project();
    let tasks_file = temp.path().join("TODO.md");
    fs::write(&tasks_file, "- [ ] Task 1\n- [ ] Task 2").unwrap();

    afk()
        .current_dir(temp.path())
        .args(["source", "add", "markdown", "TODO.md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"));
}

#[test]
fn test_source_add_file_not_found() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["source", "add", "json", "nonexistent.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_source_add_beads() {
    let temp = setup_project();

    // This will work (beads doesn't require a file)
    afk()
        .current_dir(temp.path())
        .args(["source", "add", "beads"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"));
}

#[test]
fn test_source_remove_no_sources() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["source", "remove", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No sources"));
}

// ============================================================================
// Done/Fail/Reset commands tests
// ============================================================================

#[test]
fn test_done_marks_task_complete() {
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["done", "task-001", "-m", "All tests pass"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));

    // Verify progress file updated
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("task-001"));
    assert!(progress.contains("completed"));
}

#[test]
fn test_fail_marks_task_failed() {
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["fail", "task-001", "-m", "Tests failing"])
        .assert()
        .success()
        .stdout(predicate::str::contains("failed"));

    // Verify progress file updated
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("task-001"));
    assert!(progress.contains("failed"));
}

#[test]
fn test_reset_resets_task() {
    let temp = setup_project_with_prd();

    // Create progress file with failed task
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "manual",
                "status": "failed",
                "started_at": "2025-01-01T00:00:00",
                "completed_at": null,
                "failure_count": 3,
                "commits": [],
                "message": "Tests failing",
                "learnings": []
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["reset", "task-001"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reset"));

    // Verify progress file updated
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("pending"));
}

// ============================================================================
// Prompt command tests (renamed from 'next')
// ============================================================================

#[test]
fn test_prompt_generates_prompt() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001").or(predicate::str::contains("First task")));
}

#[test]
fn test_prompt_no_tasks() {
    let temp = setup_project();

    // Empty tasks
    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": []
    }"#;
    fs::write(temp.path().join(".afk/tasks.json"), tasks).unwrap();

    // When all stories are complete, the prompt includes AFK_COMPLETE
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPLETE").or(predicate::str::contains("AFK_COMPLETE")));
}

#[test]
fn test_prompt_with_file_output() {
    let temp = setup_project_with_prd();

    // -f flag writes to default file (prompt.md in .afk/)
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-f"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Written to").or(predicate::str::contains("prompt")));

    // Check prompt file was created
    let prompt_file = temp.path().join(".afk/prompt.md");
    assert!(
        prompt_file.exists(),
        "Expected prompt file at {:?}",
        prompt_file
    );
    let contents = fs::read_to_string(&prompt_file).unwrap();
    assert!(!contents.is_empty());
}

// ============================================================================
// Status command tests (verbose now absorbs explain behaviour)
// ============================================================================

#[test]
fn test_status_verbose_shows_details() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["status", "-v"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Feedback Loops")
                .or(predicate::str::contains("Pending Stories"))
                .or(predicate::str::contains("Learnings")),
        );
}

// ============================================================================
// Verify command tests
// ============================================================================

#[test]
fn test_verify_no_gates_configured() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .arg("verify")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No quality gates").or(predicate::str::contains("No gates")),
        );
}

#[test]
fn test_verify_with_passing_gate() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with a gate that passes (true command)
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "true"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    afk()
        .current_dir(temp.path())
        .arg("verify")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓").or(predicate::str::contains("pass")));
}

#[test]
fn test_verify_with_failing_gate() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with a gate that fails (false command)
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "false"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    afk()
        .current_dir(temp.path())
        .arg("verify")
        .assert()
        .failure()
        .stdout(predicate::str::contains("✗").or(predicate::str::contains("fail")));
}

// ============================================================================
// Archive commands tests
// ============================================================================

#[test]
fn test_archive_list_empty() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["archive", "list"])
        .assert()
        .success()
        // Empty list still succeeds, may show "No archives" or just be empty
        .stdout(predicate::str::is_empty().or(predicate::str::contains("archive")));
}

#[test]
fn test_archive() {
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["archive", "-r", "test archive", "-y"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Archived").or(predicate::str::contains("Session archived")),
        );

    // Check archive was created (could be 'archives' or 'archive' dir)
    let archives_dir = temp.path().join(".afk/archives");
    let archive_dir = temp.path().join(".afk/archive");
    assert!(archives_dir.exists() || archive_dir.exists());
}

#[test]
fn test_archive_no_session() {
    let temp = setup_project();

    // Without a progress file, archiving may succeed silently or warn
    afk()
        .current_dir(temp.path())
        .args(["archive", "-r", "test", "-y"])
        .assert()
        .success();
}

// ============================================================================
// Completions command tests
// ============================================================================

#[test]
fn test_completions_bash() {
    afk()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_afk"));
}

#[test]
fn test_completions_zsh() {
    afk()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn test_completions_fish() {
    afk()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

// ============================================================================
// Go command tests
// ============================================================================

#[test]
fn test_go_dry_run() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["go", "-n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_go_no_sources_no_prd() {
    let temp = TempDir::new().unwrap();

    afk()
        .current_dir(temp.path())
        .arg("go")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No task sources").or(predicate::str::contains("No AI CLI")),
        );
}

// ============================================================================
// List and Task command tests
// ============================================================================

#[test]
fn test_list_shows_tasks() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        .stdout(predicate::str::contains("task-002"));
}

#[test]
fn test_list_pending_only() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["list", "-p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        // task-002 has passes=true so should not appear
        .stdout(predicate::str::contains("task-002").not());
}

#[test]
fn test_list_with_limit() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["list", "-l", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"));
}

#[test]
fn test_task_shows_details() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["task", "task-001"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        .stdout(predicate::str::contains("First task"))
        .stdout(predicate::str::contains("Priority"));
}

#[test]
fn test_task_not_found() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["task", "nonexistent-task"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ============================================================================
// Update command tests
// ============================================================================

#[test]
#[ignore = "requires network access"]
fn test_update_check_only() {
    // This test requires network access to check for updates
    // Run with: cargo test -- --ignored
    afk()
        .args(["update", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("version").or(predicate::str::contains("update")));
}
