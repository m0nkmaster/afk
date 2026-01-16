//! Markdown checklist task source adapter.
//!
//! Loads tasks from markdown files with checkbox syntax.

use crate::prd::UserStory;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// Default file paths to check if none specified.
const DEFAULT_PATHS: &[&str] = &["tasks.md", "TODO.md", "prd.md", ".afk/tasks.md"];

/// Regex pattern for markdown checkboxes.
/// Matches: `- [ ]` or `- [x]` or `* [ ]` or `* [x]` with optional leading whitespace.
static CHECKBOX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\s]*[-*]\s*\[([ xX])\]\s*(.+)$").expect("CHECKBOX_PATTERN regex is valid")
});

/// Regex pattern for priority tags like `[HIGH]`, `[LOW]`, `[P0]`, etc.
static PRIORITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[([A-Z0-9]+)\]\s*(.+)$").expect("PRIORITY_PATTERN regex is valid")
});

/// Regex pattern for explicit task IDs like "task-id: description".
static ID_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([a-z0-9_-]+):\s*(.+)$").expect("ID_PATTERN regex is valid")
});

/// Load tasks from a markdown file with checkboxes.
///
/// Supports formats:
/// - `- [ ] Task description` - unchecked task
/// - `- [x] Completed task` - checked (skipped)
/// - `- [ ] [HIGH] Task with priority` - task with priority tag
/// - `- [ ] task-id: Task with explicit ID` - task with explicit ID
///
/// # Arguments
///
/// * `path` - Path to the markdown file. If None, tries default locations.
///
/// # Returns
///
/// A vector of UserStory items (excluding checked items).
pub fn load_markdown_tasks(path: Option<&str>) -> Vec<UserStory> {
    // Determine the file path
    let file_path = match path {
        Some(p) => {
            let path = Path::new(p);
            if path.exists() {
                path.to_path_buf()
            } else {
                return Vec::new();
            }
        }
        None => {
            // Try default locations
            match DEFAULT_PATHS.iter().find(|p| Path::new(p).exists()) {
                Some(p) => Path::new(p).to_path_buf(),
                None => return Vec::new(),
            }
        }
    };

    // Read the file
    let contents = match fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let source_str = format!("markdown:{}", file_path.display());
    let mut tasks = Vec::new();

    // Process each line
    for line in contents.lines() {
        if let Some(caps) = CHECKBOX_PATTERN.captures(line) {
            let checkbox_state = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let text = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();

            // Skip checked items ([x] or [X])
            if checkbox_state.eq_ignore_ascii_case("x") {
                continue;
            }

            // Skip empty descriptions
            if text.is_empty() {
                continue;
            }

            let (task_id, title, priority) = parse_task_line(text);

            tasks.push(UserStory {
                id: task_id,
                title: title.clone(),
                description: title.clone(),
                acceptance_criteria: vec![format!("Complete: {}", title)],
                priority,
                passes: false,
                source: source_str.clone(),
                notes: String::new(),
            });
        }
    }

    tasks
}

/// Parse a task line to extract ID, title, and priority.
///
/// Returns (id, title, priority).
fn parse_task_line(text: &str) -> (String, String, i32) {
    let mut priority = 3;
    let mut title = text.to_string();

    // Check for priority tag: [HIGH], [LOW], [P0], etc.
    if let Some(caps) = PRIORITY_PATTERN.captures(text) {
        let tag = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        title = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();

        priority = match tag.to_uppercase().as_str() {
            "HIGH" | "CRITICAL" | "URGENT" | "P0" | "P1" => 1,
            "MEDIUM" | "NORMAL" | "P2" => 2,
            "LOW" | "MINOR" | "P3" | "P4" => 4,
            _ => 3,
        };
    }

    // Check for explicit ID: "task-id: description"
    let (task_id, final_title) = if let Some(caps) = ID_PATTERN.captures(&title) {
        let id = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_lowercase();
        let new_title = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        (id, new_title)
    } else {
        let id = generate_id(&title);
        (id, title)
    };

    (task_id, final_title, priority)
}

/// Generate an ID from text.
fn generate_id(text: &str) -> String {
    let clean: String = text
        .chars()
        .take(30)
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();

    let result = clean.replace(' ', "-");
    let trimmed = result.trim_matches('-');

    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_markdown_file(dir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(filename);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_markdown_tasks_basic() {
        let temp = TempDir::new().unwrap();
        let content = r#"
# Tasks

- [ ] First task
- [ ] Second task
- [x] Completed task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "First task");
        assert_eq!(tasks[1].title, "Second task");
    }

    #[test]
    fn test_load_markdown_tasks_skips_checked() {
        let temp = TempDir::new().unwrap();
        let content = r#"
- [x] Done task 1
- [X] Done task 2
- [ ] Pending task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Pending task");
    }

    #[test]
    fn test_load_markdown_tasks_with_priority() {
        let temp = TempDir::new().unwrap();
        let content = r#"
- [ ] [HIGH] High priority task
- [ ] [LOW] Low priority task
- [ ] [P0] Critical task
- [ ] [P2] Medium task
- [ ] Normal task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].title, "High priority task");
        assert_eq!(tasks[0].priority, 1);
        assert_eq!(tasks[1].title, "Low priority task");
        assert_eq!(tasks[1].priority, 4);
        assert_eq!(tasks[2].title, "Critical task");
        assert_eq!(tasks[2].priority, 1);
        assert_eq!(tasks[3].title, "Medium task");
        assert_eq!(tasks[3].priority, 2);
        assert_eq!(tasks[4].title, "Normal task");
        assert_eq!(tasks[4].priority, 3);
    }

    #[test]
    fn test_load_markdown_tasks_with_explicit_id() {
        let temp = TempDir::new().unwrap();
        let content = r#"
- [ ] feature-123: Implement new feature
- [ ] bug-fix: Fix the bug
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "feature-123");
        assert_eq!(tasks[0].title, "Implement new feature");
        assert_eq!(tasks[1].id, "bug-fix");
        assert_eq!(tasks[1].title, "Fix the bug");
    }

    #[test]
    fn test_load_markdown_tasks_priority_with_explicit_id() {
        let temp = TempDir::new().unwrap();
        let content = r#"
- [ ] [HIGH] task-001: Important task with ID
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-001");
        assert_eq!(tasks[0].title, "Important task with ID");
        assert_eq!(tasks[0].priority, 1);
    }

    #[test]
    fn test_load_markdown_tasks_asterisk_bullets() {
        let temp = TempDir::new().unwrap();
        let content = r#"
* [ ] Task with asterisk
* [x] Completed asterisk task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task with asterisk");
    }

    #[test]
    fn test_load_markdown_tasks_indented() {
        let temp = TempDir::new().unwrap();
        let content = r#"
## Subtasks
  - [ ] Indented task
    - [ ] More indented task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Indented task");
        assert_eq!(tasks[1].title, "More indented task");
    }

    #[test]
    fn test_load_markdown_tasks_missing_file() {
        let tasks = load_markdown_tasks(Some("/nonexistent/file.md"));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_markdown_tasks_none_path_no_defaults() {
        // When no path given and no default files exist
        let tasks = load_markdown_tasks(None);
        // Should return empty (assuming no default files in current dir)
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_markdown_tasks_source_field() {
        let temp = TempDir::new().unwrap();
        let content = "- [ ] Test task\n";
        let path = write_markdown_file(&temp, "custom.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].source.starts_with("markdown:"));
        assert!(tasks[0].source.contains("custom.md"));
    }

    #[test]
    fn test_load_markdown_tasks_acceptance_criteria() {
        let temp = TempDir::new().unwrap();
        let content = "- [ ] My task\n";
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].acceptance_criteria, vec!["Complete: My task"]);
    }

    #[test]
    fn test_load_markdown_tasks_empty_file() {
        let temp = TempDir::new().unwrap();
        let path = write_markdown_file(&temp, "empty.md", "");

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_markdown_tasks_no_checkboxes() {
        let temp = TempDir::new().unwrap();
        let content = r#"
# README

This is just a normal markdown file.

- Item 1
- Item 2
"#;
        let path = write_markdown_file(&temp, "readme.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_generate_id_basic() {
        assert_eq!(generate_id("Hello World"), "hello-world");
        assert_eq!(generate_id("Test Task!"), "test-task");
        assert_eq!(generate_id("  Leading Spaces  "), "leading-spaces");
        assert_eq!(generate_id("UPPERCASE"), "uppercase");
        assert_eq!(generate_id(""), "task");
    }

    #[test]
    fn test_generate_id_truncation() {
        assert_eq!(
            generate_id("A very long title that exceeds thirty characters limit"),
            "a-very-long-title-that-exceeds"
        );
    }

    #[test]
    fn test_generate_id_special_chars() {
        assert_eq!(generate_id("test@#$%task"), "testtask");
        assert_eq!(generate_id("feature: login"), "feature-login");
    }

    #[test]
    fn test_parse_task_line_basic() {
        let (id, title, priority) = parse_task_line("Simple task");
        assert_eq!(id, "simple-task");
        assert_eq!(title, "Simple task");
        assert_eq!(priority, 3);
    }

    #[test]
    fn test_parse_task_line_with_priority() {
        let (id, title, priority) = parse_task_line("[HIGH] Important task");
        assert_eq!(title, "Important task");
        assert_eq!(priority, 1);
        assert_eq!(id, "important-task");
    }

    #[test]
    fn test_parse_task_line_with_id() {
        let (id, title, priority) = parse_task_line("my-task-id: Task description");
        assert_eq!(id, "my-task-id");
        assert_eq!(title, "Task description");
        assert_eq!(priority, 3);
    }

    #[test]
    fn test_parse_task_line_priority_and_id() {
        let (id, title, priority) = parse_task_line("[CRITICAL] task-123: Critical fix");
        assert_eq!(id, "task-123");
        assert_eq!(title, "Critical fix");
        assert_eq!(priority, 1);
    }

    #[test]
    fn test_parse_task_line_all_priority_tags() {
        // High priority tags
        for tag in &["HIGH", "CRITICAL", "URGENT", "P0", "P1"] {
            let (_, _, priority) = parse_task_line(&format!("[{}] Task", tag));
            assert_eq!(priority, 1, "Failed for tag: {}", tag);
        }

        // Medium priority tags
        for tag in &["MEDIUM", "NORMAL", "P2"] {
            let (_, _, priority) = parse_task_line(&format!("[{}] Task", tag));
            assert_eq!(priority, 2, "Failed for tag: {}", tag);
        }

        // Low priority tags
        for tag in &["LOW", "MINOR", "P3", "P4"] {
            let (_, _, priority) = parse_task_line(&format!("[{}] Task", tag));
            assert_eq!(priority, 4, "Failed for tag: {}", tag);
        }

        // Unknown tag defaults to 3
        let (_, _, priority) = parse_task_line("[UNKNOWN] Task");
        assert_eq!(priority, 3);
    }

    #[test]
    fn test_parse_task_line_case_insensitive_id() {
        let (id, title, _) = parse_task_line("MY-TASK-ID: My task");
        assert_eq!(id, "my-task-id"); // ID should be lowercased
        assert_eq!(title, "My task");
    }

    #[test]
    fn test_load_markdown_tasks_mixed_content() {
        let temp = TempDir::new().unwrap();
        let content = r#"
# Project Tasks

Some introductory text.

## High Priority
- [ ] [P0] fix-login: Fix login bug
- [x] Already done task

## Normal Tasks
- [ ] Implement feature A
- [ ] feature-b: Implement feature B

## Low Priority
- [ ] [LOW] Nice to have
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 4);

        assert_eq!(tasks[0].id, "fix-login");
        assert_eq!(tasks[0].title, "Fix login bug");
        assert_eq!(tasks[0].priority, 1);

        assert_eq!(tasks[1].id, "implement-feature-a");
        assert_eq!(tasks[1].title, "Implement feature A");
        assert_eq!(tasks[1].priority, 3);

        assert_eq!(tasks[2].id, "feature-b");
        assert_eq!(tasks[2].title, "Implement feature B");
        assert_eq!(tasks[2].priority, 3);

        assert_eq!(tasks[3].id, "nice-to-have");
        assert_eq!(tasks[3].title, "Nice to have");
        assert_eq!(tasks[3].priority, 4);
    }

    #[test]
    fn test_load_markdown_tasks_skips_empty_descriptions() {
        let temp = TempDir::new().unwrap();
        let content = r#"
- [ ] 
- [ ]   
- [ ] Valid task
"#;
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Valid task");
    }

    #[test]
    fn test_id_with_underscores() {
        let (id, title, _) = parse_task_line("my_task_id: Task with underscores");
        assert_eq!(id, "my_task_id");
        assert_eq!(title, "Task with underscores");
    }

    #[test]
    fn test_id_with_numbers() {
        let (id, title, _) = parse_task_line("task123: Numeric ID");
        assert_eq!(id, "task123");
        assert_eq!(title, "Numeric ID");
    }

    #[test]
    fn test_passes_always_false() {
        let temp = TempDir::new().unwrap();
        let content = "- [ ] Task\n";
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert!(!tasks[0].passes);
    }

    #[test]
    fn test_description_equals_title() {
        let temp = TempDir::new().unwrap();
        let content = "- [ ] My Task Title\n";
        let path = write_markdown_file(&temp, "tasks.md", content);

        let tasks = load_markdown_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].title, tasks[0].description);
    }
}
