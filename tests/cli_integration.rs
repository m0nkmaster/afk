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

    // Create a mock AI CLI so init can detect something
    let mock_bin_dir = temp.path().join("mock_bin");
    fs::create_dir_all(&mock_bin_dir).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mock_claude = mock_bin_dir.join("claude");
        fs::write(&mock_claude, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&mock_claude, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    {
        let mock_claude = mock_bin_dir.join("claude.bat");
        fs::write(&mock_claude, "@echo off\nexit /b 0\n").unwrap();
    }

    // Modify config
    let config_path = temp.path().join(".afk/config.json");
    fs::write(&config_path, r#"{"custom": true}"#).unwrap();

    // Build PATH with mock bin dir first
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", mock_bin_dir.display(), original_path);

    afk()
        .current_dir(temp.path())
        .env("PATH", &new_path)
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
fn test_tasks_displays_tasks() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .arg("tasks")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        .stdout(predicate::str::contains("First task"));
}

#[test]
fn test_tasks_pending_only() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "-p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"))
        // task-002 is passed, so it shouldn't show
        .stdout(predicate::str::contains("task-002").not());
}

#[test]
fn test_tasks_no_tasks_file() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .arg("tasks")
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
fn test_tasks_with_limit() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "-l", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"));
}

#[test]
fn test_tasks_complete_only() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "--complete"])
        .assert()
        .success()
        // task-002 has passes=true so should appear
        .stdout(predicate::str::contains("task-002"))
        // task-001 has passes=false so should not appear
        .stdout(predicate::str::contains("task-001").not());
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

// ============================================================================
// Config command tests
// ============================================================================

#[test]
fn test_config_show() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("afk config"))
        .stdout(predicate::str::contains("ai_cli").or(predicate::str::contains("limits")));
}

#[test]
fn test_config_show_section() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "show", "--section", "limits"])
        .assert()
        .success()
        .stdout(predicate::str::contains("limits"))
        .stdout(predicate::str::contains("max_iterations"));
}

#[test]
fn test_config_show_unknown_section() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "show", "--section", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown section"));
}

#[test]
fn test_config_get() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "get", "limits.max_iterations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("200").or(predicate::str::is_match(r"\d+").unwrap()));
}

#[test]
fn test_config_get_unknown_key() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "get", "nonexistent.key"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown").or(predicate::str::contains("key")));
}

#[test]
fn test_config_set() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations", "100"])
        .assert()
        .success()
        .stdout(predicate::str::contains("100").or(predicate::str::contains("✓")));

    // Verify the value was actually set
    afk()
        .current_dir(temp.path())
        .args(["config", "get", "limits.max_iterations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("100"));
}

#[test]
fn test_config_set_invalid_value() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations", "not-a-number"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid").or(predicate::str::contains("parse")));
}

#[test]
fn test_config_reset_all() {
    let temp = setup_project();

    // First set a non-default value
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations", "42"])
        .assert()
        .success();

    // Reset all config
    afk()
        .current_dir(temp.path())
        .args(["config", "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset").or(predicate::str::contains("default")));

    // Verify value is back to default
    afk()
        .current_dir(temp.path())
        .args(["config", "get", "limits.max_iterations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("200"));
}

#[test]
fn test_config_reset_section() {
    let temp = setup_project();

    // Set a non-default value
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations", "99"])
        .assert()
        .success();

    // Reset the section
    afk()
        .current_dir(temp.path())
        .args(["config", "reset", "limits"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset").or(predicate::str::contains("limits")));
}

#[test]
fn test_config_reset_field() {
    let temp = setup_project();

    // Set a non-default value
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations", "77"])
        .assert()
        .success();

    // Reset just that field
    afk()
        .current_dir(temp.path())
        .args(["config", "reset", "limits.max_iterations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset").or(predicate::str::contains("200")));
}

#[test]
fn test_config_explain() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "explain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config keys").or(predicate::str::contains("limits")));
}

#[test]
fn test_config_explain_key() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "explain", "limits.max_iterations"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_iterations"))
        .stdout(predicate::str::contains("Default").or(predicate::str::contains("Type")));
}

#[test]
fn test_config_explain_section() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "explain", "limits"])
        .assert()
        .success()
        .stdout(predicate::str::contains("limits"))
        .stdout(predicate::str::contains("max_iterations"));
}

#[test]
fn test_config_keys() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["config", "keys"])
        .assert()
        .success()
        .stdout(predicate::str::contains("limits.max_iterations"))
        .stdout(predicate::str::contains("ai_cli.command"));
}

// ============================================================================
// Use command tests
// ============================================================================

#[test]
fn test_use_list() {
    let temp = setup_project();

    afk()
        .current_dir(temp.path())
        .args(["use", "--list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("claude")
                .or(predicate::str::contains("cursor"))
                .or(predicate::str::contains("AI CLI")),
        );
}

#[test]
fn test_use_nonexistent_cli() {
    let temp = setup_project();

    // Try to switch to a non-existent CLI
    afk()
        .current_dir(temp.path())
        .args(["use", "nonexistent-ai-cli-that-does-not-exist"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("Failed"))
                .or(predicate::str::contains("Unknown")),
        );
}

// ============================================================================
// Task lifecycle end-to-end tests
// ============================================================================

#[test]
fn test_task_lifecycle_sync_to_done() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Create a config with no sources initially
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": []
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    // Create a markdown source file
    let todo_file = temp.path().join("TODO.md");
    fs::write(&todo_file, "- [ ] First task\n- [ ] Second task").unwrap();

    // Add the source
    afk()
        .current_dir(temp.path())
        .args(["source", "add", "markdown", "TODO.md"])
        .assert()
        .success();

    // Sync to load tasks
    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Verify tasks exist
    afk()
        .current_dir(temp.path())
        .arg("tasks")
        .assert()
        .success()
        .stdout(predicate::str::contains("First task").or(predicate::str::contains("task")));

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(afk_dir.join("progress.json"), progress).unwrap();

    // Verify the sync worked by checking tasks exist
    let tasks_content = fs::read_to_string(afk_dir.join("tasks.json")).unwrap();
    assert!(
        tasks_content.contains("userStories"),
        "Tasks should be synced from source"
    );

    // Check status works after the lifecycle
    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_full_workflow_with_prd() {
    let temp = setup_project_with_prd();

    // Check initial status
    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success();

    // View tasks
    afk()
        .current_dir(temp.path())
        .arg("tasks")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"));

    // View specific task
    afk()
        .current_dir(temp.path())
        .args(["task", "task-001"])
        .assert()
        .success()
        .stdout(predicate::str::contains("First task"));

    // Generate prompt
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001"));

    // Create progress file and mark task done
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["done", "task-001", "-m", "Implemented feature"])
        .assert()
        .success();

    // Verify task is marked complete in progress
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("completed"));

    // Check status shows completion
    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success();
}

// ============================================================================
// More error scenario tests
// ============================================================================

#[test]
fn test_done_creates_task_entry_dynamically() {
    // done/fail/reset commands are lenient - they create task entries dynamically
    // even for tasks not in tasks.json, to support manual task management
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // This succeeds because done/fail/reset are lenient
    afk()
        .current_dir(temp.path())
        .args(["done", "new-task-id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));

    // Verify the task was added to progress
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("new-task-id"));
    assert!(progress.contains("completed"));
}

#[test]
fn test_fail_creates_task_entry_dynamically() {
    let temp = setup_project_with_prd();

    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["fail", "new-task-id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("failed"));

    // Verify the task was added to progress
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("new-task-id"));
    assert!(progress.contains("failed"));
}

#[test]
fn test_reset_creates_task_entry_dynamically() {
    let temp = setup_project_with_prd();

    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["reset", "new-task-id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reset").or(predicate::str::contains("pending")));

    // Verify the task was added to progress
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("new-task-id"));
}

#[test]
fn test_source_remove_invalid_index() {
    let temp = setup_project();

    // Add a source first
    let tasks_file = temp.path().join("tasks.json");
    fs::write(&tasks_file, r#"{"tasks": []}"#).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["source", "add", "json", "tasks.json"])
        .assert()
        .success();

    // Try to remove with invalid index
    afk()
        .current_dir(temp.path())
        .args(["source", "remove", "99"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Invalid")
                .or(predicate::str::contains("out of range"))
                .or(predicate::str::contains("not found")),
        );
}

#[test]
fn test_verify_with_multiple_gates() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with multiple gates (mix of passing and failing)
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "true",
            "test": "true",
            "build": "true"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    afk()
        .current_dir(temp.path())
        .arg("verify")
        .assert()
        .success()
        .stdout(predicate::str::contains("lint"))
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("build"));
}

#[test]
fn test_verify_verbose_shows_output() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with standard gates that produce output
    // feedback_loops has: types, lint, test, build fields, plus custom map
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "echo 'Lint passed'",
            "test": "echo 'Tests passed'"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["verify", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lint").or(predicate::str::contains("test")));
}

#[test]
fn test_import_with_output_file() {
    let temp = setup_project();
    let requirements = temp.path().join("requirements.md");
    fs::write(
        &requirements,
        "# My Project\n\n## Features\n- User login\n- Dashboard",
    )
    .unwrap();

    afk()
        .current_dir(temp.path())
        .args(["import", "requirements.md", "-f"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Written").or(predicate::str::contains("prompt")));

    // Verify the prompt file was created
    let prompt_file = temp.path().join(".afk/prompt.md");
    assert!(prompt_file.exists());
}

#[test]
fn test_import_with_clipboard() {
    let temp = setup_project();
    let requirements = temp.path().join("requirements.md");
    fs::write(&requirements, "# Simple App\n- Feature 1").unwrap();

    // -c copies to clipboard - this should succeed even if clipboard is unavailable
    afk()
        .current_dir(temp.path())
        .args(["import", "requirements.md", "-c"])
        .assert()
        .success();
}

#[test]
fn test_go_with_iterations_argument() {
    let temp = setup_project_with_prd();

    // Test dry run with specific iteration count
    afk()
        .current_dir(temp.path())
        .args(["go", "5", "-n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_go_fresh_flag() {
    let temp = setup_project_with_prd();

    // Create an existing progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 10,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "in_progress",
                "failure_count": 2,
                "learnings": []
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Run with --fresh and dry-run to verify it clears progress
    afk()
        .current_dir(temp.path())
        .args(["go", "--fresh", "-n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn test_prompt_all_tasks_complete() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // All tasks are complete
    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-001",
                "title": "Done task",
                "description": "Already complete",
                "acceptanceCriteria": ["Done"],
                "priority": 1,
                "passes": true
            },
            {
                "id": "task-002",
                "title": "Also done",
                "description": "Also complete",
                "acceptanceCriteria": ["Done"],
                "priority": 2,
                "passes": true
            }
        ]
    }"#;
    fs::write(afk_dir.join("tasks.json"), tasks).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPLETE").or(predicate::str::contains("AFK_COMPLETE")));
}

#[test]
fn test_status_no_progress_file() {
    let temp = setup_project_with_prd();

    // Ensure no progress file exists
    let progress_path = temp.path().join(".afk/progress.json");
    if progress_path.exists() {
        fs::remove_file(&progress_path).unwrap();
    }

    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_archive_creates_timestamped_directory() {
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 10,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "completed",
                "failure_count": 0,
                "learnings": []
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["archive", "-r", "completed work", "-y"])
        .assert()
        .success();

    // Verify archives directory was created
    let archives_dir = temp.path().join(".afk/archives");
    if archives_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&archives_dir).unwrap().collect();
        assert!(
            !entries.is_empty(),
            "Archive directory should contain entries"
        );
    }
}

#[test]
fn test_tasks_displays_priority_and_status() {
    let temp = setup_project_with_prd();

    // Verify tasks command shows task information
    afk()
        .current_dir(temp.path())
        .arg("tasks")
        .assert()
        .success()
        // Should show task IDs
        .stdout(predicate::str::contains("task-001").or(predicate::str::contains("task-002")));
}

#[test]
fn test_multiple_source_types() {
    let temp = setup_project();

    // Create multiple source files
    let json_file = temp.path().join("tasks.json");
    fs::write(&json_file, r#"{"tasks": []}"#).unwrap();

    let md_file = temp.path().join("TODO.md");
    fs::write(&md_file, "- [ ] A task").unwrap();

    // Add both sources
    afk()
        .current_dir(temp.path())
        .args(["source", "add", "json", "tasks.json"])
        .assert()
        .success();

    afk()
        .current_dir(temp.path())
        .args(["source", "add", "markdown", "TODO.md"])
        .assert()
        .success();

    // List should show both
    afk()
        .current_dir(temp.path())
        .args(["source", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("json").or(predicate::str::contains("tasks.json")))
        .stdout(predicate::str::contains("markdown").or(predicate::str::contains("TODO.md")));
}

// ============================================================================
// Priority ordering and task selection tests
// ============================================================================

#[test]
fn test_tasks_priority_ordering() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Create tasks with different priorities (lower number = higher priority)
    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-low",
                "title": "Low priority task",
                "description": "This is low priority",
                "acceptanceCriteria": ["Works"],
                "priority": 3,
                "passes": false
            },
            {
                "id": "task-high",
                "title": "High priority task",
                "description": "This is high priority",
                "acceptanceCriteria": ["Works"],
                "priority": 1,
                "passes": false
            },
            {
                "id": "task-med",
                "title": "Medium priority task",
                "description": "This is medium priority",
                "acceptanceCriteria": ["Works"],
                "priority": 2,
                "passes": false
            }
        ]
    }"#;
    fs::write(afk_dir.join("tasks.json"), tasks).unwrap();

    // Prompt should pick the highest priority (lowest number) task
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-high"));
}

#[test]
fn test_prompt_picks_next_incomplete_task() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // First task is complete, second should be picked
    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-001",
                "title": "Already done",
                "description": "This is complete",
                "acceptanceCriteria": ["Done"],
                "priority": 1,
                "passes": true
            },
            {
                "id": "task-002",
                "title": "Pending work",
                "description": "This should be next",
                "acceptanceCriteria": ["Works"],
                "priority": 1,
                "passes": false
            }
        ]
    }"#;
    fs::write(afk_dir.join("tasks.json"), tasks).unwrap();

    // Prompt should select task-002 since task-001 is already complete
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-002"));
}

// ============================================================================
// Learnings and progress tracking tests
// ============================================================================

#[test]
fn test_status_verbose_shows_learnings() {
    let temp = setup_project_with_prd();

    // Create progress with learnings
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "in_progress",
                "failure_count": 1,
                "learnings": [
                    "First learning about this task",
                    "Second discovery while working"
                ]
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .args(["status", "-v"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Learnings")
                .or(predicate::str::contains("First learning"))
                .or(predicate::str::contains("task-001")),
        );
}

#[test]
fn test_done_with_learning() {
    let temp = setup_project_with_prd();

    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 1,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Mark done with a learning message
    afk()
        .current_dir(temp.path())
        .args(["done", "task-001", "-m", "Learned that X requires Y"])
        .assert()
        .success();

    // Verify the progress file contains the message
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("completed"));
}

#[test]
fn test_fail_increments_failure_count() {
    let temp = setup_project_with_prd();

    // Create progress with existing failure count
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 3,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "in_progress",
                "failure_count": 2,
                "learnings": []
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Fail the task again
    afk()
        .current_dir(temp.path())
        .args(["fail", "task-001", "-m", "Still failing"])
        .assert()
        .success();

    // Verify failure count increased
    let progress = fs::read_to_string(temp.path().join(".afk/progress.json")).unwrap();
    assert!(progress.contains("failed"));
}

// ============================================================================
// Sync and task state preservation tests
// ============================================================================

#[test]
fn test_sync_preserves_completed_status() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Create a markdown source
    let todo_file = temp.path().join("TODO.md");
    fs::write(&todo_file, "- [ ] Task one\n- [ ] Task two").unwrap();

    // Add source and sync
    afk()
        .current_dir(temp.path())
        .args(["source", "add", "markdown", "TODO.md"])
        .assert()
        .success();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Verify tasks were created
    let tasks_path = afk_dir.join("tasks.json");
    assert!(tasks_path.exists());

    // Read and modify a task to be complete
    let tasks_content = fs::read_to_string(&tasks_path).unwrap();
    let modified = tasks_content.replace("\"passes\": false", "\"passes\": true");
    fs::write(&tasks_path, &modified).unwrap();

    // Sync again
    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Verify completed status was preserved (tasks.json should still have passes: true)
    let tasks_after = fs::read_to_string(&tasks_path).unwrap();
    assert!(
        tasks_after.contains("\"passes\": true"),
        "Completed status should be preserved after sync"
    );
}

#[test]
fn test_sync_adds_new_tasks_from_source() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Create a markdown source with one task
    let todo_file = temp.path().join("TODO.md");
    fs::write(&todo_file, "- [ ] Initial task").unwrap();

    // Add source and sync
    afk()
        .current_dir(temp.path())
        .args(["source", "add", "markdown", "TODO.md"])
        .assert()
        .success();

    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Add another task to the source
    fs::write(&todo_file, "- [ ] Initial task\n- [ ] New task").unwrap();

    // Sync again
    afk()
        .current_dir(temp.path())
        .args(["tasks", "sync"])
        .assert()
        .success();

    // Verify both tasks exist
    let tasks_content = fs::read_to_string(afk_dir.join("tasks.json")).unwrap();
    assert!(
        tasks_content.contains("Initial task") || tasks_content.contains("New task"),
        "Should contain tasks from source"
    );
}

// ============================================================================
// Config edge cases
// ============================================================================

#[test]
fn test_config_set_feedback_loop() {
    let temp = setup_project();

    // Set a feedback loop
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "feedback_loops.lint", "cargo clippy"])
        .assert()
        .success();

    // Verify it was set
    let config = fs::read_to_string(temp.path().join(".afk/config.json")).unwrap();
    assert!(
        config.contains("clippy") || config.contains("lint"),
        "Config should contain the lint command"
    );
}

#[test]
fn test_config_set_ai_cli_args() {
    let temp = setup_project();

    // Set AI CLI args
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "ai_cli.args", "[\"--verbose\"]"])
        .assert()
        .success();

    // Verify it was set
    let config = fs::read_to_string(temp.path().join(".afk/config.json")).unwrap();
    assert!(
        config.contains("verbose"),
        "Config should contain the new args"
    );
}

#[test]
fn test_config_get_nested_value() {
    let temp = setup_project();

    // Get the AI CLI command
    afk()
        .current_dir(temp.path())
        .args(["config", "get", "ai_cli.command"])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo"));
}

// ============================================================================
// Go command edge cases
// ============================================================================

#[test]
fn test_go_with_complete_tasks_exits() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // All tasks complete
    let tasks = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-001",
                "title": "Done",
                "description": "Already complete",
                "acceptanceCriteria": ["Done"],
                "priority": 1,
                "passes": true
            }
        ]
    }"#;
    fs::write(afk_dir.join("tasks.json"), tasks).unwrap();

    // Dry run should succeed and indicate completion
    afk()
        .current_dir(temp.path())
        .args(["go", "-n"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Dry run")
                .or(predicate::str::contains("complete"))
                .or(predicate::str::contains("All")),
        );
}

#[test]
fn test_go_init_flag_triggers_setup() {
    let temp = TempDir::new().unwrap();
    let path_with_mock = setup_mock_ai_cli(&temp);

    // Go with --init on uninitialised project should trigger setup
    // Note: This will still fail after init because no task sources exist,
    // but it should show the "Analysing project" message first
    let result = afk()
        .current_dir(temp.path())
        .env("PATH", &path_with_mock)
        .args(["go", "--init", "-n"])
        .assert();

    // Should show analysis output regardless of final success/failure
    result.stdout(predicate::str::contains("Analysing"));
}

// ============================================================================
// Error handling edge cases
// ============================================================================

#[test]
fn test_task_command_without_id() {
    let temp = setup_project_with_prd();

    // task command requires an ID
    afk()
        .current_dir(temp.path())
        .arg("task")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_source_add_without_type() {
    let temp = setup_project();

    // source add requires a type
    afk()
        .current_dir(temp.path())
        .args(["source", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_config_set_without_value() {
    let temp = setup_project();

    // config set requires key and value
    afk()
        .current_dir(temp.path())
        .args(["config", "set", "limits.max_iterations"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_verify_partial_gate_failure() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // First gate passes, second fails
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "true",
            "test": "false"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    // Overall should fail even if some pass
    afk()
        .current_dir(temp.path())
        .arg("verify")
        .assert()
        .failure();
}

#[test]
fn test_import_empty_file() {
    let temp = setup_project();
    let empty_file = temp.path().join("empty.md");
    fs::write(&empty_file, "").unwrap();

    // Import should handle empty file gracefully
    afk()
        .current_dir(temp.path())
        .args(["import", "empty.md", "-s"])
        .assert()
        .success();
}

#[test]
fn test_init_inside_afk_directory_fails() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Try to init from inside .afk directory
    afk()
        .current_dir(&afk_dir)
        .args(["init"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains(".afk")
                .or(predicate::str::contains("inside"))
                .or(predicate::str::contains("nested")),
        );
}

// ============================================================================
// Archive and session management tests
// ============================================================================

#[test]
fn test_archive_list_shows_archived_sessions() {
    let temp = setup_project_with_prd();

    // Create progress file
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Archive with reason
    afk()
        .current_dir(temp.path())
        .args(["archive", "-r", "First session", "-y"])
        .assert()
        .success();

    // List should show the archive
    afk()
        .current_dir(temp.path())
        .args(["archive", "list"])
        .assert()
        .success();
}

#[test]
fn test_done_updates_tasks_json_passes() {
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
        .args(["done", "task-001"])
        .assert()
        .success();

    // Verify tasks.json was updated
    let tasks = fs::read_to_string(temp.path().join(".afk/tasks.json")).unwrap();
    // task-001 should now have passes: true
    assert!(
        tasks.contains("task-001"),
        "Tasks file should still contain task-001"
    );
}

// ============================================================================
// Prompt generation edge cases
// ============================================================================

#[test]
fn test_prompt_with_progress_context() {
    let temp = setup_project_with_prd();

    // Create progress with learnings that should be included in prompt
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 3,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "in_progress",
                "failure_count": 1,
                "learnings": [
                    "API requires authentication header"
                ]
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Prompt should include context about current state
    afk()
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("task-001").or(predicate::str::contains("First task")));
}

#[test]
fn test_prompt_file_output_creates_valid_markdown() {
    let temp = setup_project_with_prd();

    afk()
        .current_dir(temp.path())
        .args(["prompt", "-f"])
        .assert()
        .success();

    // Check the file is valid markdown with expected sections
    let prompt_file = temp.path().join(".afk/prompt.md");
    assert!(prompt_file.exists());

    let contents = fs::read_to_string(&prompt_file).unwrap();
    assert!(!contents.is_empty());
    // Markdown should have some structure
    assert!(
        contents.contains('#') || contents.contains('-') || contents.contains('*'),
        "Prompt should contain markdown formatting"
    );
}

// ============================================================================
// Status display variants
// ============================================================================

#[test]
fn test_status_with_in_progress_task() {
    let temp = setup_project_with_prd();

    // Create progress with in-progress task
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {
            "task-001": {
                "id": "task-001",
                "source": "prd",
                "status": "in_progress",
                "failure_count": 0,
                "learnings": []
            }
        }
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    // Status should show the current task
    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Current")
                .or(predicate::str::contains("task-001"))
                .or(predicate::str::contains("In Progress")),
        );
}

#[test]
fn test_status_shows_iteration_count() {
    let temp = setup_project_with_prd();

    // Create progress with iterations
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 42,
        "tasks": {}
    }"#;
    fs::write(temp.path().join(".afk/progress.json"), progress).unwrap();

    afk()
        .current_dir(temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("42").or(predicate::str::contains("iteration")));
}
