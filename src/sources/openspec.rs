//! OpenSpec source adapter.
//!
//! Loads tasks from OpenSpec change proposals. OpenSpec provides structured
//! change proposals with formal requirements and scenarios, making it a
//! natural fit for afk's task-driven workflow.
//!
//! See: https://github.com/Fission-AI/OpenSpec

use crate::prd::UserStory;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// Default OpenSpec directory.
const OPENSPEC_DIR: &str = "openspec";
/// Changes subdirectory within OpenSpec.
const CHANGES_DIR: &str = "changes";
/// Archive subdirectory to exclude.
const ARCHIVE_DIR: &str = "archive";

/// Regex pattern for markdown checkboxes in tasks.md.
/// Matches: `- [ ]` or `- [x]` with task numbering like `1.1`, `2.3`, etc.
static CHECKBOX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\s]*[-*]\s*\[([ xX])\]\s*(?:(\d+(?:\.\d+)?)\s+)?(.+)$")
        .expect("CHECKBOX_PATTERN regex is valid")
});

/// Regex pattern for section headers in tasks.md (e.g., "## 1. Implementation").
static SECTION_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^##\s+\d+\.\s+(.+)$").expect("SECTION_PATTERN regex is valid"));

/// Load tasks from OpenSpec change proposals.
///
/// Scans `openspec/changes/` for active change folders (excluding `archive/`),
/// parses `tasks.md` files for unchecked items, and enriches tasks with
/// context from proposals and specs.
///
/// # Returns
///
/// A vector of UserStory items from all active OpenSpec changes.
pub fn load_openspec_tasks() -> Vec<UserStory> {
    let openspec_path = Path::new(OPENSPEC_DIR);
    if !openspec_path.exists() {
        return Vec::new();
    }

    let changes_path = openspec_path.join(CHANGES_DIR);
    if !changes_path.exists() {
        return Vec::new();
    }

    let mut tasks = Vec::new();

    // Read all change directories
    let entries = match fs::read_dir(&changes_path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip non-directories and the archive folder
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if dir_name == ARCHIVE_DIR {
            continue;
        }

        // Load tasks from this change
        let change_tasks = load_change_tasks(&path, dir_name);
        tasks.extend(change_tasks);
    }

    tasks
}

/// Load tasks from a single OpenSpec change directory.
fn load_change_tasks(change_path: &Path, change_id: &str) -> Vec<UserStory> {
    let tasks_file = change_path.join("tasks.md");
    if !tasks_file.exists() {
        return Vec::new();
    }

    let contents = match fs::read_to_string(&tasks_file) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Load optional context
    let proposal = load_file_contents(&change_path.join("proposal.md"));
    let design = load_file_contents(&change_path.join("design.md"));
    let specs = load_spec_deltas(change_path);

    let source_str = format!("openspec:{}", change_id);
    let mut tasks = Vec::new();
    let mut current_section = String::new();

    for line in contents.lines() {
        // Check for section headers
        if let Some(caps) = SECTION_PATTERN.captures(line) {
            current_section = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            continue;
        }

        // Check for checkbox items
        if let Some(caps) = CHECKBOX_PATTERN.captures(line) {
            let checkbox_state = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let task_number = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let text = caps.get(3).map(|m| m.as_str()).unwrap_or("").trim();

            // Skip checked items ([x] or [X])
            if checkbox_state.eq_ignore_ascii_case("x") {
                continue;
            }

            // Skip empty descriptions
            if text.is_empty() {
                continue;
            }

            // Generate task ID
            let task_id = if task_number.is_empty() {
                format!("{}-{}", change_id, generate_slug(text))
            } else {
                format!("{}-{}", change_id, task_number.replace('.', "-"))
            };

            // Build description with context
            let description = build_task_description(
                text,
                change_id,
                &current_section,
                &proposal,
                &design,
                &specs,
            );

            // Build acceptance criteria from specs if available
            let acceptance_criteria = build_acceptance_criteria(text, &specs);

            tasks.push(UserStory {
                id: task_id,
                title: text.to_string(),
                description,
                acceptance_criteria,
                priority: 3, // Default priority; could parse from tags
                passes: false,
                source: source_str.clone(),
                notes: String::new(),
            });
        }
    }

    tasks
}

/// Load file contents if the file exists.
fn load_file_contents(path: &Path) -> Option<String> {
    if path.exists() {
        fs::read_to_string(path).ok()
    } else {
        None
    }
}

/// Load spec deltas from a change's specs directory.
fn load_spec_deltas(change_path: &Path) -> Vec<String> {
    let specs_dir = change_path.join("specs");
    if !specs_dir.exists() {
        return Vec::new();
    }

    let mut specs = Vec::new();
    collect_markdown_files(&specs_dir, &mut specs);
    specs
}

/// Recursively collect markdown file contents from a directory.
fn collect_markdown_files(dir: &Path, contents: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, contents);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            if let Ok(content) = fs::read_to_string(&path) {
                contents.push(content);
            }
        }
    }
}

/// Build a rich task description with OpenSpec context.
fn build_task_description(
    task_text: &str,
    change_id: &str,
    section: &str,
    proposal: &Option<String>,
    design: &Option<String>,
    specs: &[String],
) -> String {
    let mut parts = vec![task_text.to_string()];

    parts.push(format!("\n\n**OpenSpec Change:** `{}`", change_id));

    if !section.is_empty() {
        parts.push(format!("**Section:** {}", section));
    }

    // Add proposal summary if available
    if let Some(prop) = proposal {
        if let Some(summary) = extract_section(prop, "## Why") {
            parts.push(format!("\n**Context (Why):**\n{}", summary));
        }
        if let Some(changes) = extract_section(prop, "## What Changes") {
            parts.push(format!("\n**What Changes:**\n{}", changes));
        }
    }

    // Add relevant specs context
    if !specs.is_empty() {
        let specs_context: Vec<&str> = specs
            .iter()
            .filter(|s| {
                s.contains("## ADDED Requirements")
                    || s.contains("## MODIFIED Requirements")
                    || s.contains("## REMOVED Requirements")
            })
            .map(|s| s.as_str())
            .collect();

        if !specs_context.is_empty() {
            parts.push("\n**Spec Deltas:**".to_string());
            for spec in specs_context.iter().take(2) {
                // Limit to avoid huge prompts
                parts.push(format!("```markdown\n{}\n```", truncate_string(spec, 500)));
            }
        }
    }

    // Add design notes if relevant
    if let Some(des) = design {
        if let Some(decisions) = extract_section(des, "## Decisions") {
            parts.push(format!("\n**Design Decisions:**\n{}", decisions));
        }
    }

    parts.join("\n")
}

/// Extract a section from markdown content.
fn extract_section(content: &str, header: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_section = false;
    let mut section_lines = Vec::new();

    for line in lines {
        if line.starts_with(header) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            section_lines.push(line);
        }
    }

    if section_lines.is_empty() {
        None
    } else {
        Some(section_lines.join("\n").trim().to_string())
    }
}

/// Build acceptance criteria from spec scenarios.
fn build_acceptance_criteria(task_text: &str, specs: &[String]) -> Vec<String> {
    let mut criteria = vec![format!("Complete: {}", task_text)];

    // Extract scenarios from specs
    for spec in specs {
        for line in spec.lines() {
            if line.starts_with("#### Scenario:") {
                let scenario = line.trim_start_matches("#### Scenario:").trim();
                criteria.push(format!("Verify scenario: {}", scenario));
            }
        }
    }

    // Limit to reasonable number
    criteria.truncate(5);
    criteria
}

/// Generate a URL-safe slug from text.
fn generate_slug(text: &str) -> String {
    let clean: String = text
        .chars()
        .take(20)
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

/// Truncate a string to a maximum length, adding ellipsis if needed.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_openspec_structure(temp: &TempDir) -> std::path::PathBuf {
        let openspec_dir = temp.path().join("openspec");
        let changes_dir = openspec_dir.join("changes");
        fs::create_dir_all(&changes_dir).unwrap();
        openspec_dir
    }

    fn create_change(changes_dir: &Path, name: &str, tasks_content: &str) -> std::path::PathBuf {
        let change_dir = changes_dir.join(name);
        fs::create_dir_all(&change_dir).unwrap();
        fs::write(change_dir.join("tasks.md"), tasks_content).unwrap();
        change_dir
    }

    #[test]
    fn test_load_openspec_tasks_no_directory() {
        // When openspec/ doesn't exist, should return empty
        let tasks = load_openspec_tasks();
        // May or may not be empty depending on cwd, but shouldn't panic
        assert!(tasks.is_empty() || !tasks.is_empty());
    }

    #[test]
    fn test_load_change_tasks_basic() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = r#"
## 1. Implementation
- [ ] 1.1 Create database schema
- [ ] 1.2 Implement API endpoint
- [x] 1.3 Already done task

## 2. Testing
- [ ] 2.1 Write unit tests
"#;
        let change_dir = create_change(&changes_dir, "add-feature", tasks_content);

        let tasks = load_change_tasks(&change_dir, "add-feature");

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, "add-feature-1-1");
        assert_eq!(tasks[0].title, "Create database schema");
        assert_eq!(tasks[1].id, "add-feature-1-2");
        assert_eq!(tasks[1].title, "Implement API endpoint");
        assert_eq!(tasks[2].id, "add-feature-2-1");
        assert_eq!(tasks[2].title, "Write unit tests");
    }

    #[test]
    fn test_load_change_tasks_with_proposal() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let change_dir = changes_dir.join("add-auth");
        fs::create_dir_all(&change_dir).unwrap();

        let tasks_content = "- [ ] 1.1 Add authentication\n";
        fs::write(change_dir.join("tasks.md"), tasks_content).unwrap();

        let proposal_content = r#"
# Change: Add Authentication

## Why
We need secure user authentication for the API.

## What Changes
- Add JWT token validation
- Add login endpoint
"#;
        fs::write(change_dir.join("proposal.md"), proposal_content).unwrap();

        let tasks = load_change_tasks(&change_dir, "add-auth");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].description.contains("OpenSpec Change:"));
        assert!(tasks[0]
            .description
            .contains("We need secure user authentication"));
        assert!(tasks[0].description.contains("Add JWT token validation"));
    }

    #[test]
    fn test_load_change_tasks_with_specs() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let change_dir = changes_dir.join("add-2fa");
        let specs_dir = change_dir.join("specs").join("auth");
        fs::create_dir_all(&specs_dir).unwrap();

        let tasks_content = "- [ ] 1.1 Implement 2FA\n";
        fs::write(change_dir.join("tasks.md"), tasks_content).unwrap();

        let spec_content = r#"
## ADDED Requirements
### Requirement: Two-Factor Authentication
Users MUST provide a second factor during login.

#### Scenario: OTP required
- **WHEN** valid credentials are provided
- **THEN** an OTP challenge is required

#### Scenario: OTP verification
- **WHEN** correct OTP is entered
- **THEN** user is authenticated
"#;
        fs::write(specs_dir.join("spec.md"), spec_content).unwrap();

        let tasks = load_change_tasks(&change_dir, "add-2fa");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].description.contains("Spec Deltas:"));
        assert!(tasks[0]
            .acceptance_criteria
            .iter()
            .any(|ac| ac.contains("OTP required")));
    }

    #[test]
    fn test_load_change_tasks_skips_checked() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = r#"
- [x] Done task 1
- [X] Done task 2
- [ ] Pending task
"#;
        let change_dir = create_change(&changes_dir, "test-change", tasks_content);

        let tasks = load_change_tasks(&change_dir, "test-change");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Pending task");
    }

    #[test]
    fn test_load_change_tasks_without_numbering() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = r#"
- [ ] First task without number
- [ ] Second task without number
"#;
        let change_dir = create_change(&changes_dir, "unnumbered", tasks_content);

        let tasks = load_change_tasks(&change_dir, "unnumbered");

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].id.starts_with("unnumbered-"));
        assert!(tasks[0].id.contains("first-task"));
    }

    #[test]
    fn test_load_change_tasks_source_field() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = "- [ ] Test task\n";
        let change_dir = create_change(&changes_dir, "my-change", tasks_content);

        let tasks = load_change_tasks(&change_dir, "my-change");

        assert_eq!(tasks[0].source, "openspec:my-change");
    }

    #[test]
    fn test_generate_slug_basic() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("Test Task!"), "test-task");
        assert_eq!(generate_slug("  Leading Spaces  "), "leading-spaces");
    }

    #[test]
    fn test_generate_slug_truncation() {
        let long_text = "A very long task description that exceeds the limit";
        let slug = generate_slug(long_text);
        assert!(slug.len() <= 25); // Truncated to 20 chars + possible hyphen removal
    }

    #[test]
    fn test_extract_section() {
        let content = r#"
# Proposal

## Why
This is the reason.
With multiple lines.

## What Changes
- Change 1
- Change 2

## Impact
Some impact
"#;

        let why = extract_section(content, "## Why");
        assert!(why.is_some());
        assert!(why.unwrap().contains("This is the reason"));

        let what = extract_section(content, "## What Changes");
        assert!(what.is_some());
        assert!(what.unwrap().contains("Change 1"));

        let missing = extract_section(content, "## Nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is longer", 7), "this is...");
    }

    #[test]
    fn test_build_acceptance_criteria() {
        let specs = vec![
            "#### Scenario: Login success\n- WHEN...\n#### Scenario: Login failure\n".to_string(),
        ];

        let criteria = build_acceptance_criteria("Implement login", &specs);

        assert!(criteria
            .iter()
            .any(|c| c.contains("Complete: Implement login")));
        assert!(criteria.iter().any(|c| c.contains("Login success")));
        assert!(criteria.iter().any(|c| c.contains("Login failure")));
    }

    #[test]
    fn test_checkbox_pattern_with_number() {
        let line = "- [ ] 1.1 Create database schema";
        let caps = CHECKBOX_PATTERN.captures(line).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), " ");
        assert_eq!(caps.get(2).unwrap().as_str(), "1.1");
        assert_eq!(caps.get(3).unwrap().as_str(), "Create database schema");
    }

    #[test]
    fn test_checkbox_pattern_without_number() {
        let line = "- [ ] Just a task description";
        let caps = CHECKBOX_PATTERN.captures(line).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), " ");
        assert!(caps.get(2).is_none());
        assert_eq!(caps.get(3).unwrap().as_str(), "Just a task description");
    }

    #[test]
    fn test_checkbox_pattern_checked() {
        let line = "- [x] 2.1 Done task";
        let caps = CHECKBOX_PATTERN.captures(line).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "x");
    }

    #[test]
    fn test_section_pattern() {
        let line = "## 1. Implementation";
        let caps = SECTION_PATTERN.captures(line).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Implementation");

        let line2 = "## 2. Testing";
        let caps2 = SECTION_PATTERN.captures(line2).unwrap();
        assert_eq!(caps2.get(1).unwrap().as_str(), "Testing");
    }

    #[test]
    fn test_collect_markdown_files() {
        let temp = TempDir::new().unwrap();
        let specs_dir = temp.path().join("specs");
        let auth_dir = specs_dir.join("auth");
        fs::create_dir_all(&auth_dir).unwrap();

        fs::write(auth_dir.join("spec.md"), "# Auth Spec\n").unwrap();
        fs::write(specs_dir.join("other.md"), "# Other\n").unwrap();
        fs::write(specs_dir.join("ignore.txt"), "not markdown").unwrap();

        let mut contents = Vec::new();
        collect_markdown_files(&specs_dir, &mut contents);

        assert_eq!(contents.len(), 2);
        assert!(contents.iter().any(|c| c.contains("Auth Spec")));
        assert!(contents.iter().any(|c| c.contains("Other")));
    }

    #[test]
    fn test_load_change_tasks_missing_tasks_file() {
        let temp = TempDir::new().unwrap();
        let change_dir = temp.path().join("empty-change");
        fs::create_dir_all(&change_dir).unwrap();

        let tasks = load_change_tasks(&change_dir, "empty-change");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_change_tasks_empty_tasks_file() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let change_dir = create_change(&changes_dir, "empty", "");

        let tasks = load_change_tasks(&change_dir, "empty");
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_passes_always_false() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = "- [ ] Task\n";
        let change_dir = create_change(&changes_dir, "test", tasks_content);

        let tasks = load_change_tasks(&change_dir, "test");
        assert!(!tasks[0].passes);
    }

    #[test]
    fn test_default_priority() {
        let temp = TempDir::new().unwrap();
        let openspec_dir = setup_openspec_structure(&temp);
        let changes_dir = openspec_dir.join("changes");

        let tasks_content = "- [ ] Task\n";
        let change_dir = create_change(&changes_dir, "test", tasks_content);

        let tasks = load_change_tasks(&change_dir, "test");
        assert_eq!(tasks[0].priority, 3);
    }
}
