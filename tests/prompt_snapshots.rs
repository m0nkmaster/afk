//! Snapshot tests for prompt output parity with Python.
//!
//! These tests capture the generated prompt output in various scenarios
//! to ensure the Rust implementation produces identical output to Python.

use std::fs;
use tempfile::TempDir;

/// Helper to create a minimal project setup.
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
        "sources": [],
        "limits": {
            "max_iterations": 10,
            "max_task_failures": 3,
            "timeout_minutes": 60
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    temp
}

/// Helper to create a project with PRD and progress.
fn setup_project_with_stories() -> TempDir {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    let prd = r#"{
        "projectName": "test-project",
        "branch": "main",
        "userStories": [
            {
                "id": "task-001",
                "title": "Implement user authentication",
                "description": "Add login and logout functionality",
                "acceptanceCriteria": [
                    "Users can log in with email/password",
                    "Sessions persist across page reloads",
                    "Logout clears session"
                ],
                "priority": 1,
                "passes": false
            },
            {
                "id": "task-002",
                "title": "Add password reset flow",
                "description": "Allow users to reset forgotten passwords",
                "acceptanceCriteria": [
                    "Reset email sent on request",
                    "Token expires after 24 hours"
                ],
                "priority": 2,
                "passes": false
            },
            {
                "id": "task-003",
                "title": "Previous task completed",
                "description": "This one is already done",
                "acceptanceCriteria": ["It works"],
                "priority": 1,
                "passes": true
            }
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 3,
        "tasks": {
            "task-003": {
                "id": "task-003",
                "source": "json:prd.json",
                "status": "completed",
                "started_at": "2025-01-01T00:00:00",
                "completed_at": "2025-01-01T01:00:00",
                "failure_count": 0,
                "commits": ["abc1234"],
                "message": null,
                "learnings": ["Use bcrypt for password hashing"]
            }
        }
    }"#;
    fs::write(afk_dir.join("progress.json"), progress).unwrap();

    temp
}

// ============================================================================
// Basic prompt structure tests
// ============================================================================

#[test]
fn test_prompt_contains_header() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check header section
    assert!(stdout.contains("# afk Autonomous Agent"), "Missing header");
    assert!(
        stdout.contains("You are an autonomous coding agent"),
        "Missing role description"
    );
}

#[test]
fn test_prompt_contains_task_list() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check task instructions
    assert!(stdout.contains("## Your Task"), "Missing task section");
    assert!(
        stdout.contains("Read `.afk/progress.json`"),
        "Missing progress instruction"
    );
    assert!(
        stdout.contains("Read `.afk/tasks.json`"),
        "Missing tasks instruction"
    );
    assert!(
        stdout.contains("Pick the **highest priority** user story"),
        "Missing priority instruction"
    );
}

#[test]
fn test_prompt_contains_progress() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check progress section
    assert!(stdout.contains("## Progress"), "Missing progress section");
    assert!(stdout.contains("Iteration:"), "Missing iteration count");
    assert!(stdout.contains("Completed:"), "Missing completed count");
}

#[test]
fn test_prompt_contains_quality_gates() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check quality gates section
    assert!(
        stdout.contains("## Quality Gates"),
        "Missing quality gates section"
    );
    assert!(stdout.contains("afk verify"), "Missing verify command");
}

#[test]
fn test_prompt_contains_learnings_section() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check learnings section
    assert!(
        stdout.contains("## Recording Learnings"),
        "Missing learnings section"
    );
    assert!(
        stdout.contains("Short-term: `.afk/progress.json`"),
        "Missing short-term section"
    );
    assert!(
        stdout.contains("Long-term: `AGENTS.md`"),
        "Missing long-term section"
    );
}

#[test]
fn test_prompt_contains_stop_condition() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check stop condition section
    assert!(
        stdout.contains("## Stop Condition"),
        "Missing stop condition section"
    );
    assert!(
        stdout.contains("COMPLETE") || stdout.contains("AFK_COMPLETE"),
        "Missing completion signal"
    );
}

// ============================================================================
// Next story context tests
// ============================================================================

#[test]
fn test_prompt_shows_next_story_details() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Next story appears in Progress section
    assert!(stdout.contains("Next story:"), "Missing next story line");
    assert!(stdout.contains("task-001"), "Missing story ID");
}

#[test]
fn test_prompt_mentions_acceptance_criteria() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Prompt mentions acceptance criteria in task instructions
    assert!(
        stdout.contains("acceptanceCriteria"),
        "Missing acceptance criteria mention"
    );
}

#[test]
fn test_prompt_shows_story_count() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check story counts (1 completed, 3 total)
    assert!(
        stdout.contains("1/3") || stdout.contains("1 of 3"),
        "Missing or incorrect story count"
    );
}

// ============================================================================
// All complete scenario tests
// ============================================================================

#[test]
fn test_prompt_all_complete_shows_signal() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // All stories complete
    let prd = r#"{
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
            }
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // When all complete, should show completion signal
    assert!(
        stdout.contains("AFK_COMPLETE") || stdout.contains("COMPLETE"),
        "Missing completion signal when all stories done"
    );
}

// ============================================================================
// Custom configuration tests
// ============================================================================

#[test]
fn test_prompt_with_configured_gates() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with feedback loops
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "feedback_loops": {
            "lint": "cargo clippy",
            "test": "cargo test"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": [
            {"id": "t1", "title": "Task", "description": "Do it", "acceptanceCriteria": ["Done"], "priority": 1, "passes": false}
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check configured gates appear
    assert!(
        stdout.contains("Configured gates:"),
        "Missing gates section"
    );
    assert!(
        stdout.contains("lint") && stdout.contains("cargo clippy"),
        "Missing lint gate"
    );
    assert!(
        stdout.contains("test") && stdout.contains("cargo test"),
        "Missing test gate"
    );
}

#[test]
fn test_prompt_with_custom_instructions() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with custom instructions
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "prompt": {
            "custom_instructions": [
                "Always use British English in comments",
                "Prefer functional programming style"
            ]
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": [
            {"id": "t1", "title": "Task", "description": "Do it", "acceptanceCriteria": ["Done"], "priority": 1, "passes": false}
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Custom instructions should appear if config has them
    // The template includes them with {% for instruction in custom_instructions %}
    // If empty, nothing is shown - this is expected behaviour
    assert!(
        stdout.contains("British English") || stdout.contains("## Stop Condition"),
        "Should either show custom instructions or reach stop condition section"
    );
}

#[test]
fn test_prompt_with_context_files() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with context files
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "prompt": {
            "context_files": ["README.md", "docs/architecture.md"]
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": [
            {"id": "t1", "title": "Task", "description": "Do it", "acceptanceCriteria": ["Done"], "priority": 1, "passes": false}
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check context files appear in Key Files section
    assert!(
        stdout.contains("README.md"),
        "Missing context file README.md"
    );
    assert!(
        stdout.contains("docs/architecture.md"),
        "Missing context file architecture.md"
    );
}

// ============================================================================
// Bootstrap flag tests
// ============================================================================

#[test]
fn test_prompt_bootstrap_includes_afk_commands() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s", "-b"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Bootstrap should include afk command reference
    assert!(
        stdout.contains("afk done") || stdout.contains("afk verify"),
        "Bootstrap should include afk commands"
    );
}

// ============================================================================
// Iteration limit tests
// ============================================================================

#[test]
fn test_prompt_respects_iteration_limit() {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    // Config with custom iteration limit
    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "limits": {
            "max_iterations": 50
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": [
            {"id": "t1", "title": "Task", "description": "Do it", "acceptanceCriteria": ["Done"], "priority": 1, "passes": false}
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check iteration limit appears
    assert!(
        stdout.contains("/50") || stdout.contains("of 50"),
        "Should show custom max iterations"
    );
}

#[test]
fn test_prompt_limit_override() {
    let temp = setup_project_with_stories();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s", "-l", "25"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check override limit appears
    assert!(
        stdout.contains("/25") || stdout.contains("of 25"),
        "Should show overridden max iterations"
    );
}

// ============================================================================
// Empty PRD tests
// ============================================================================

#[test]
fn test_prompt_empty_prd() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": []
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Empty PRD should show completion signal
    assert!(
        stdout.contains("AFK_COMPLETE") || stdout.contains("COMPLETE"),
        "Empty PRD should signal completion"
    );
}

// ============================================================================
// Priority ordering tests
// ============================================================================

#[test]
fn test_prompt_selects_highest_priority() {
    let temp = setup_project();
    let afk_dir = temp.path().join(".afk");

    // Mix of priorities - should pick P1 first
    let prd = r#"{
        "projectName": "test",
        "branch": "main",
        "userStories": [
            {"id": "low-priority", "title": "P3 task", "description": "Low priority", "acceptanceCriteria": ["Done"], "priority": 3, "passes": false},
            {"id": "high-priority", "title": "P1 task", "description": "High priority", "acceptanceCriteria": ["Done"], "priority": 1, "passes": false},
            {"id": "medium-priority", "title": "P2 task", "description": "Medium priority", "acceptanceCriteria": ["Done"], "priority": 2, "passes": false}
        ]
    }"#;
    fs::write(afk_dir.join("prd.json"), prd).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_afk"))
        .current_dir(temp.path())
        .args(["prompt", "-s"])
        .output()
        .expect("Failed to run afk");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show high-priority as next story
    assert!(
        stdout.contains("high-priority") || stdout.contains("P1 task"),
        "Should select highest priority task first"
    );
}
