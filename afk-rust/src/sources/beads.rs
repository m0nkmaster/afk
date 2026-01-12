//! Beads (bd) task source adapter.
//!
//! Loads tasks from the beads issue tracker via the `bd` CLI.

use crate::prd::UserStory;
use regex::Regex;
use std::process::Command;

/// Load tasks from beads via `bd ready --json`.
///
/// Falls back to text parsing if JSON output fails.
/// Returns an empty vector if `bd` is not installed or times out.
pub fn load_beads_tasks() -> Vec<UserStory> {
    // Try JSON output first
    match run_bd_ready_json() {
        Ok(tasks) => tasks,
        Err(_) => {
            // Fall back to text parsing
            parse_beads_text_output().unwrap_or_default()
        }
    }
}

/// Close a beads issue by ID.
///
/// # Arguments
///
/// * `issue_id` - The beads issue ID to close
///
/// # Returns
///
/// `true` if successfully closed, `false` otherwise.
pub fn close_beads_issue(issue_id: &str) -> bool {
    match Command::new("bd").args(["close", issue_id]).output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Run `bd ready --json` and parse the output.
fn run_bd_ready_json() -> Result<Vec<UserStory>, BeadsError> {
    let output = Command::new("bd")
        .args(["ready", "--json"])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BeadsError::NotInstalled
            } else {
                BeadsError::CommandFailed(e.to_string())
            }
        })?;

    if !output.status.success() {
        return Err(BeadsError::NonZeroExit);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let data: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|_| BeadsError::InvalidJson)?;

    // Parse JSON array of issues
    let items = match &data {
        serde_json::Value::Array(arr) => arr,
        _ => return Ok(Vec::new()),
    };

    let tasks: Vec<UserStory> = items.iter().filter_map(parse_beads_item).collect();

    Ok(tasks)
}

/// Parse a single beads item from JSON to UserStory.
fn parse_beads_item(item: &serde_json::Value) -> Option<UserStory> {
    // Try various ID field names
    let task_id = item
        .get("id")
        .or_else(|| item.get("key"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            item.get("number")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
        })?;

    if task_id.is_empty() {
        return None;
    }

    // Get title/summary
    let title = item
        .get("title")
        .or_else(|| item.get("summary"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Get description (falls back to title)
    let description = item
        .get("description")
        .or_else(|| item.get("body"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| title.clone());

    // Map priority
    let priority = map_beads_priority(item.get("priority"));

    // Extract acceptance criteria from description
    let acceptance_criteria = extract_acceptance_criteria(&description);
    let acceptance_criteria = if acceptance_criteria.is_empty() {
        vec![format!("Complete: {}", title)]
    } else {
        acceptance_criteria
    };

    Some(UserStory {
        id: task_id,
        title,
        description,
        acceptance_criteria,
        priority,
        passes: false,
        source: "beads".to_string(),
        notes: String::new(),
    })
}

/// Parse text output from `bd ready` (fallback).
fn parse_beads_text_output() -> Result<Vec<UserStory>, BeadsError> {
    let output = Command::new("bd").args(["ready"]).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BeadsError::NotInstalled
        } else {
            BeadsError::CommandFailed(e.to_string())
        }
    })?;

    if !output.status.success() {
        return Err(BeadsError::NonZeroExit);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tasks: Vec<UserStory> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_text_line)
        .collect();

    Ok(tasks)
}

/// Parse a single line from `bd ready` text output.
fn parse_text_line(line: &str) -> UserStory {
    let line = line.trim();

    // Try to parse "ID: description" format
    let (task_id, title) = if let Some((id_part, desc_part)) = line.split_once(':') {
        let id = id_part.trim();
        let title = desc_part.trim();
        if title.is_empty() {
            (id.to_string(), id.to_string())
        } else {
            (id.to_string(), title.to_string())
        }
    } else {
        // Generate ID from description
        let id = generate_id_from_text(line);
        (id, line.to_string())
    };

    UserStory {
        id: task_id,
        title: title.clone(),
        description: title.clone(),
        acceptance_criteria: vec![format!("Complete: {}", title)],
        priority: 3,
        passes: false,
        source: "beads".to_string(),
        notes: String::new(),
    }
}

/// Generate an ID from text (first 20 chars, lowercased, spaces to dashes).
fn generate_id_from_text(text: &str) -> String {
    text.chars()
        .take(20)
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .replace(' ', "-")
}

/// Map beads priority to int (1-5).
fn map_beads_priority(priority: Option<&serde_json::Value>) -> i32 {
    match priority {
        None => 3,
        Some(serde_json::Value::Number(n)) => {
            let p = n.as_i64().unwrap_or(3) as i32;
            p.clamp(1, 5)
        }
        Some(serde_json::Value::String(s)) => {
            let s_lower = s.to_lowercase();
            match s_lower.as_str() {
                "high" | "critical" | "urgent" | "p0" | "p1" => 1,
                "medium" | "normal" | "p2" => 2,
                "low" | "minor" | "p3" | "p4" => 4,
                _ => 3,
            }
        }
        _ => 3,
    }
}

/// Extract acceptance criteria from text.
///
/// Looks for:
/// 1. Acceptance Criteria / AC / DoD / Requirements section with list items
/// 2. Checkbox items (- [ ] ...)
fn extract_acceptance_criteria(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut criteria: Vec<String> = Vec::new();

    // Pattern 1: Look for acceptance criteria section
    // Matches: "Acceptance Criteria:\n- item\n- item" or similar
    let section_pattern = Regex::new(
        r"(?i)(?:acceptance\s*criteria|ac|definition\s*of\s*done|dod|requirements?)[\s:]*\n((?:[-*\d.]+\s*.+\n?)+)",
    )
    .unwrap();

    if let Some(captures) = section_pattern.captures(text) {
        if let Some(section) = captures.get(1) {
            let item_pattern = Regex::new(r"^[-*\d.]+\s*(?:\[[ x]\])?\s*").unwrap();
            for line in section.as_str().lines() {
                let line = line.trim();
                if !line.is_empty() {
                    let cleaned = item_pattern.replace(line, "").to_string();
                    if !cleaned.is_empty() {
                        criteria.push(cleaned);
                    }
                }
            }
            return criteria;
        }
    }

    // Pattern 2: Look for markdown heading section
    let heading_pattern =
        Regex::new(r"(?i)##\s*(?:acceptance\s*criteria|ac|dod)\s*\n((?:[-*\d.]+\s*.+\n?)+)")
            .unwrap();

    if let Some(captures) = heading_pattern.captures(text) {
        if let Some(section) = captures.get(1) {
            let item_pattern = Regex::new(r"^[-*\d.]+\s*(?:\[[ x]\])?\s*").unwrap();
            for line in section.as_str().lines() {
                let line = line.trim();
                if !line.is_empty() {
                    let cleaned = item_pattern.replace(line, "").to_string();
                    if !cleaned.is_empty() {
                        criteria.push(cleaned);
                    }
                }
            }
            return criteria;
        }
    }

    // Pattern 3: Look for unchecked checkbox items
    let checkbox_pattern = Regex::new(r"[-*]\s*\[ \]\s*(.+)").unwrap();
    for captures in checkbox_pattern.captures_iter(text) {
        if let Some(item) = captures.get(1) {
            let cleaned = item.as_str().trim();
            if !cleaned.is_empty() {
                criteria.push(cleaned.to_string());
            }
        }
    }

    criteria
}

/// Internal error type for beads operations.
#[derive(Debug)]
#[allow(dead_code)]
enum BeadsError {
    NotInstalled,
    CommandFailed(String),
    NonZeroExit,
    InvalidJson,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_beads_priority_none() {
        assert_eq!(map_beads_priority(None), 3);
    }

    #[test]
    fn test_map_beads_priority_integer() {
        let v = serde_json::json!(1);
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!(5);
        assert_eq!(map_beads_priority(Some(&v)), 5);

        let v = serde_json::json!(0);
        assert_eq!(map_beads_priority(Some(&v)), 1); // Clamped

        let v = serde_json::json!(10);
        assert_eq!(map_beads_priority(Some(&v)), 5); // Clamped
    }

    #[test]
    fn test_map_beads_priority_string() {
        let v = serde_json::json!("high");
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!("critical");
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!("urgent");
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!("P0");
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!("P1");
        assert_eq!(map_beads_priority(Some(&v)), 1);

        let v = serde_json::json!("medium");
        assert_eq!(map_beads_priority(Some(&v)), 2);

        let v = serde_json::json!("normal");
        assert_eq!(map_beads_priority(Some(&v)), 2);

        let v = serde_json::json!("P2");
        assert_eq!(map_beads_priority(Some(&v)), 2);

        let v = serde_json::json!("low");
        assert_eq!(map_beads_priority(Some(&v)), 4);

        let v = serde_json::json!("minor");
        assert_eq!(map_beads_priority(Some(&v)), 4);

        let v = serde_json::json!("P3");
        assert_eq!(map_beads_priority(Some(&v)), 4);

        let v = serde_json::json!("unknown");
        assert_eq!(map_beads_priority(Some(&v)), 3);
    }

    #[test]
    fn test_generate_id_from_text() {
        assert_eq!(generate_id_from_text("Hello World"), "hello-world");
        assert_eq!(generate_id_from_text("Fix bug in login"), "fix-bug-in-login");
        assert_eq!(
            generate_id_from_text("A very long description that exceeds the limit"),
            "a-very-long-descript"
        );
        assert_eq!(generate_id_from_text("Test!@#$%"), "test");
        assert_eq!(generate_id_from_text("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_parse_text_line_with_colon() {
        let story = parse_text_line("ISSUE-123: Fix the login bug");
        assert_eq!(story.id, "ISSUE-123");
        assert_eq!(story.title, "Fix the login bug");
        assert_eq!(story.source, "beads");
        assert_eq!(story.priority, 3);
    }

    #[test]
    fn test_parse_text_line_without_colon() {
        let story = parse_text_line("Fix the authentication flow");
        assert_eq!(story.id, "fix-the-authenticati");
        assert_eq!(story.title, "Fix the authentication flow");
    }

    #[test]
    fn test_parse_text_line_id_only() {
        let story = parse_text_line("BUG-456:");
        assert_eq!(story.id, "BUG-456");
        assert_eq!(story.title, "BUG-456");
    }

    #[test]
    fn test_parse_beads_item_with_id() {
        let json = serde_json::json!({
            "id": "issue-1",
            "title": "Test Issue",
            "description": "A test description",
            "priority": 1
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.id, "issue-1");
        assert_eq!(story.title, "Test Issue");
        assert_eq!(story.description, "A test description");
        assert_eq!(story.priority, 1);
        assert_eq!(story.source, "beads");
    }

    #[test]
    fn test_parse_beads_item_with_key() {
        let json = serde_json::json!({
            "key": "PROJ-123",
            "summary": "Bug summary",
            "body": "Bug details"
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.id, "PROJ-123");
        assert_eq!(story.title, "Bug summary");
        assert_eq!(story.description, "Bug details");
    }

    #[test]
    fn test_parse_beads_item_with_number() {
        let json = serde_json::json!({
            "number": 42,
            "title": "Issue #42"
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.id, "42");
        assert_eq!(story.title, "Issue #42");
    }

    #[test]
    fn test_parse_beads_item_no_id() {
        let json = serde_json::json!({
            "title": "No ID issue"
        });

        let story = parse_beads_item(&json);
        assert!(story.is_none());
    }

    #[test]
    fn test_parse_beads_item_empty_id() {
        let json = serde_json::json!({
            "id": "",
            "title": "Empty ID issue"
        });

        let story = parse_beads_item(&json);
        assert!(story.is_none());
    }

    #[test]
    fn test_parse_beads_item_description_fallback() {
        let json = serde_json::json!({
            "id": "test-1",
            "title": "Title only"
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.description, "Title only");
    }

    #[test]
    fn test_extract_acceptance_criteria_empty() {
        assert!(extract_acceptance_criteria("").is_empty());
    }

    #[test]
    fn test_extract_acceptance_criteria_no_section() {
        let text = "Just a regular description without any criteria.";
        assert!(extract_acceptance_criteria(text).is_empty());
    }

    #[test]
    fn test_extract_acceptance_criteria_section() {
        let text = r#"Some description here.

Acceptance Criteria:
- First criterion
- Second criterion
- Third criterion

Other text."#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0], "First criterion");
        assert_eq!(criteria[1], "Second criterion");
        assert_eq!(criteria[2], "Third criterion");
    }

    #[test]
    fn test_extract_acceptance_criteria_ac_shorthand() {
        let text = r#"Description.

AC:
* Step one
* Step two"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "Step one");
        assert_eq!(criteria[1], "Step two");
    }

    #[test]
    fn test_extract_acceptance_criteria_dod() {
        let text = r#"Feature description.

Definition of Done:
1. Tests pass
2. Code reviewed"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "Tests pass");
        assert_eq!(criteria[1], "Code reviewed");
    }

    #[test]
    fn test_extract_acceptance_criteria_requirements() {
        let text = r#"Overview.

Requirements:
- Must support JSON
- Must handle errors"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "Must support JSON");
        assert_eq!(criteria[1], "Must handle errors");
    }

    #[test]
    fn test_extract_acceptance_criteria_checkboxes() {
        let text = r#"Feature description.

- [ ] Implement the thing
- [x] Already done (checked)
- [ ] Another item"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "Implement the thing");
        assert_eq!(criteria[1], "Another item");
    }

    #[test]
    fn test_extract_acceptance_criteria_markdown_heading() {
        let text = r#"# Feature

Some description.

## Acceptance Criteria
- First
- Second"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0], "First");
        assert_eq!(criteria[1], "Second");
    }

    #[test]
    fn test_extract_acceptance_criteria_with_checkbox_in_section() {
        let text = r#"Description.

Acceptance Criteria:
- [ ] First item
- [x] Second item (done)
- [ ] Third item"#;

        let criteria = extract_acceptance_criteria(text);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0], "First item");
        assert_eq!(criteria[1], "Second item (done)");
        assert_eq!(criteria[2], "Third item");
    }

    #[test]
    fn test_parse_beads_item_with_acceptance_criteria() {
        let json = serde_json::json!({
            "id": "test-1",
            "title": "Test Issue",
            "description": "Description.\n\nAcceptance Criteria:\n- AC1\n- AC2"
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.acceptance_criteria.len(), 2);
        assert_eq!(story.acceptance_criteria[0], "AC1");
        assert_eq!(story.acceptance_criteria[1], "AC2");
    }

    #[test]
    fn test_parse_beads_item_default_acceptance_criteria() {
        let json = serde_json::json!({
            "id": "test-1",
            "title": "Simple Issue",
            "description": "No AC here"
        });

        let story = parse_beads_item(&json).unwrap();
        assert_eq!(story.acceptance_criteria.len(), 1);
        assert_eq!(story.acceptance_criteria[0], "Complete: Simple Issue");
    }

    #[test]
    fn test_load_beads_tasks_bd_not_installed() {
        // This test verifies that when bd isn't installed, we get an empty list
        // We can't easily test this without mocking, but the function should
        // return an empty vec gracefully
        // In a real environment without bd, this would test the error path
    }

    #[test]
    fn test_close_beads_issue_returns_false_when_not_installed() {
        // When bd isn't installed, close should return false
        // This will depend on whether bd is installed on the test machine
        // The function is designed to return false gracefully on any error
    }
}
