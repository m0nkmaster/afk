//! JSON PRD task source adapter.
//!
//! Loads tasks from JSON PRD files in various formats.

use crate::prd::UserStory;
use std::fs;
use std::path::Path;

/// Load tasks from a JSON PRD file.
///
/// Supports formats:
///
/// 1. Full afk style:
/// ```json
/// {
///     "tasks": [
///         {
///             "id": "feature-id",
///             "title": "Feature title",
///             "description": "Feature description",
///             "priority": 1,
///             "acceptanceCriteria": ["Step 1", "Step 2"],
///             "passes": false
///         }
///     ]
/// }
/// ```
///
/// 2. Simple array:
/// ```json
/// [
///     {"id": "...", "title": "..."}
/// ]
/// ```
///
/// # Arguments
///
/// * `path` - Path to the JSON file. If None, tries default locations.
///
/// # Returns
///
/// A vector of UserStory items (excluding those with `passes: true`).
pub fn load_json_tasks(path: Option<&str>) -> Vec<UserStory> {
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
            let defaults = ["prd.json", "tasks.json", ".afk/prd.json"];
            match defaults.iter().find(|p| Path::new(p).exists()) {
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

    // Parse JSON
    let data: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    // Extract items array
    let items = extract_items(&data);

    // Convert to UserStory, filtering out completed tasks
    let source_str = format!("json:{}", file_path.display());
    items
        .into_iter()
        .filter_map(|item| parse_task_item(item, &source_str))
        .collect()
}

/// Extract items array from various JSON formats.
fn extract_items(data: &serde_json::Value) -> Vec<&serde_json::Value> {
    match data {
        // Simple array format
        serde_json::Value::Array(arr) => arr.iter().collect(),
        // Object with tasks/userStories/items key
        serde_json::Value::Object(obj) => {
            let key_options = ["tasks", "userStories", "items"];
            for key in key_options {
                if let Some(serde_json::Value::Array(arr)) = obj.get(key) {
                    return arr.iter().collect();
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Parse a single task item from JSON to UserStory.
///
/// Returns None if the task should be skipped (passes: true or no valid ID).
fn parse_task_item(item: &serde_json::Value, source: &str) -> Option<UserStory> {
    // Skip completed tasks
    if item.get("passes").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }

    // Get title first (needed for ID generation)
    let title = item
        .get("title")
        .or_else(|| item.get("summary"))
        .or_else(|| item.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Get or generate ID
    let id = item
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| generate_id(&title));

    // Skip if no valid ID
    if id.is_empty() {
        return None;
    }

    // Get description (falls back to title)
    let description = item
        .get("description")
        .or_else(|| item.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Get priority
    let priority = map_priority(item.get("priority"));

    // Get acceptance criteria (support multiple key names)
    let acceptance_criteria = extract_acceptance_criteria(item, &title);

    // Get notes
    let notes = item
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(UserStory {
        id,
        title,
        description,
        acceptance_criteria,
        priority,
        passes: false,
        source: source.to_string(),
        notes,
    })
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

/// Map various priority formats to int (1-5).
fn map_priority(priority: Option<&serde_json::Value>) -> i32 {
    match priority {
        None => 3,
        Some(serde_json::Value::Number(n)) => {
            let p = n.as_i64().unwrap_or(3) as i32;
            p.clamp(1, 5)
        }
        Some(serde_json::Value::String(s)) => {
            let s_lower = s.to_lowercase();
            match s_lower.as_str() {
                "high" | "critical" | "urgent" | "1" | "p0" | "p1" => 1,
                "medium" | "normal" | "2" | "p2" => 2,
                "low" | "minor" | "3" | "4" | "5" | "p3" | "p4" => 4,
                _ => 3,
            }
        }
        _ => 3,
    }
}

/// Extract acceptance criteria from various key names.
fn extract_acceptance_criteria(item: &serde_json::Value, title: &str) -> Vec<String> {
    let criteria = item
        .get("acceptanceCriteria")
        .or_else(|| item.get("acceptance_criteria"))
        .or_else(|| item.get("steps"));

    match criteria {
        Some(serde_json::Value::Array(arr)) => {
            let result: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            if result.is_empty() {
                vec![format!("Complete: {}", title)]
            } else {
                result
            }
        }
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        _ => vec![format!("Complete: {}", title)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_json_file(dir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(filename);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_json_tasks_full_format() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "feature-1",
                    "title": "Implement feature",
                    "description": "Detailed description",
                    "priority": 1,
                    "acceptanceCriteria": ["Step 1", "Step 2"],
                    "passes": false
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "feature-1");
        assert_eq!(tasks[0].title, "Implement feature");
        assert_eq!(tasks[0].description, "Detailed description");
        assert_eq!(tasks[0].priority, 1);
        assert_eq!(tasks[0].acceptance_criteria, vec!["Step 1", "Step 2"]);
        assert!(!tasks[0].passes);
    }

    #[test]
    fn test_load_json_tasks_simple_array() {
        let temp = TempDir::new().unwrap();
        let json = r#"[
            {"id": "task-1", "title": "First task"},
            {"id": "task-2", "title": "Second task"}
        ]"#;
        let path = write_json_file(&temp, "tasks.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[1].id, "task-2");
    }

    #[test]
    fn test_load_json_tasks_user_stories_key() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "userStories": [
                {"id": "story-1", "title": "User story 1"}
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "story-1");
    }

    #[test]
    fn test_load_json_tasks_items_key() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "items": [
                {"id": "item-1", "title": "Item 1"}
            ]
        }"#;
        let path = write_json_file(&temp, "items.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "item-1");
    }

    #[test]
    fn test_load_json_tasks_skips_passes_true() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {"id": "done", "title": "Done task", "passes": true},
                {"id": "pending", "title": "Pending task", "passes": false}
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "pending");
    }

    #[test]
    fn test_load_json_tasks_missing_file() {
        let tasks = load_json_tasks(Some("/nonexistent/file.json"));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_json_tasks_invalid_json() {
        let temp = TempDir::new().unwrap();
        let path = write_json_file(&temp, "invalid.json", "not valid json {{{");

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_load_json_tasks_none_path_no_defaults() {
        // When no path given and no default files exist
        let tasks = load_json_tasks(None);
        // Should return empty (assuming no default files in current dir)
        // This test relies on test environment not having prd.json etc.
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_generate_id_from_title() {
        assert_eq!(generate_id("Hello World"), "hello-world");
        assert_eq!(generate_id("Test Task!"), "test-task");
        assert_eq!(generate_id("  Leading Spaces  "), "leading-spaces");
        assert_eq!(generate_id("UPPERCASE"), "uppercase");
        assert_eq!(generate_id(""), "task");
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
    fn test_map_priority_integer() {
        let v = serde_json::json!(1);
        assert_eq!(map_priority(Some(&v)), 1);

        let v = serde_json::json!(5);
        assert_eq!(map_priority(Some(&v)), 5);

        let v = serde_json::json!(0);
        assert_eq!(map_priority(Some(&v)), 1);

        let v = serde_json::json!(10);
        assert_eq!(map_priority(Some(&v)), 5);
    }

    #[test]
    fn test_map_priority_string() {
        let v = serde_json::json!("high");
        assert_eq!(map_priority(Some(&v)), 1);

        let v = serde_json::json!("critical");
        assert_eq!(map_priority(Some(&v)), 1);

        let v = serde_json::json!("P0");
        assert_eq!(map_priority(Some(&v)), 1);

        let v = serde_json::json!("medium");
        assert_eq!(map_priority(Some(&v)), 2);

        let v = serde_json::json!("P2");
        assert_eq!(map_priority(Some(&v)), 2);

        let v = serde_json::json!("low");
        assert_eq!(map_priority(Some(&v)), 4);

        let v = serde_json::json!("unknown");
        assert_eq!(map_priority(Some(&v)), 3);
    }

    #[test]
    fn test_map_priority_none() {
        assert_eq!(map_priority(None), 3);
    }

    #[test]
    fn test_acceptance_criteria_camel_case() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "Test",
                    "acceptanceCriteria": ["AC1", "AC2"]
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].acceptance_criteria, vec!["AC1", "AC2"]);
    }

    #[test]
    fn test_acceptance_criteria_snake_case() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "Test",
                    "acceptance_criteria": ["Step 1", "Step 2"]
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].acceptance_criteria, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_acceptance_criteria_steps_key() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "Test",
                    "steps": ["Do this", "Then that"]
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].acceptance_criteria, vec!["Do this", "Then that"]);
    }

    #[test]
    fn test_acceptance_criteria_default() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "My Task"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].acceptance_criteria, vec!["Complete: My Task"]);
    }

    #[test]
    fn test_acceptance_criteria_string() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "Test",
                    "acceptanceCriteria": "Single criterion"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].acceptance_criteria, vec!["Single criterion"]);
    }

    #[test]
    fn test_title_fallback_to_summary() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "summary": "Summary as title"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].title, "Summary as title");
    }

    #[test]
    fn test_title_fallback_to_description() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "description": "Description as title"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].title, "Description as title");
    }

    #[test]
    fn test_id_generation_when_missing() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "title": "My New Feature"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].id, "my-new-feature");
    }

    #[test]
    fn test_source_field_set_correctly() {
        let temp = TempDir::new().unwrap();
        let json = r#"[{"id": "test-1", "title": "Test"}]"#;
        let path = write_json_file(&temp, "custom.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert!(tasks[0].source.starts_with("json:"));
        assert!(tasks[0].source.contains("custom.json"));
    }

    #[test]
    fn test_notes_field() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {
                    "id": "test-1",
                    "title": "Test",
                    "notes": "Some notes here"
                }
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].notes, "Some notes here");
    }

    #[test]
    fn test_real_prd_json_format() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "project": "afk",
            "branchName": "rust-conversion",
            "description": "Tasks synced from configured sources",
            "userStories": [
                {
                    "id": "rust-001-project-scaffold",
                    "title": "Create Rust project scaffold",
                    "description": "Create Rust project scaffold with Cargo workspace",
                    "acceptanceCriteria": [
                        "Create Cargo.toml",
                        "Create src/main.rs",
                        "Verify cargo build"
                    ],
                    "priority": 1,
                    "passes": true,
                    "source": "json:docs/prds/rust-rewrite-tasks.json",
                    "notes": ""
                },
                {
                    "id": "rust-002-config-models",
                    "title": "Implement config models",
                    "description": "Implement Serde models for config",
                    "acceptanceCriteria": [
                        "Create config structs",
                        "Write unit tests"
                    ],
                    "priority": 1,
                    "passes": false,
                    "source": "json:docs/prds/rust-rewrite-tasks.json",
                    "notes": ""
                }
            ],
            "lastSynced": "2024-01-12T09:48:44.670500"
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        // Should only return the one with passes: false
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "rust-002-config-models");
        assert_eq!(tasks[0].priority, 1);
    }

    #[test]
    fn test_empty_array() {
        let temp = TempDir::new().unwrap();
        let json = r#"[]"#;
        let path = write_json_file(&temp, "empty.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_empty_tasks_object() {
        let temp = TempDir::new().unwrap();
        let json = r#"{"tasks": []}"#;
        let path = write_json_file(&temp, "empty.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_skip_tasks_without_title_or_id() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {},
                {"id": "valid", "title": "Valid task"}
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        // First task has empty title -> generates "task" ID, which is valid
        // So we get 2 tasks (the empty one gets default values)
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "task");
        assert_eq!(tasks[1].id, "valid");
    }

    #[test]
    fn test_all_tasks_passed() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {"id": "done-1", "title": "Done 1", "passes": true},
                {"id": "done-2", "title": "Done 2", "passes": true}
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_priority_clamping() {
        let temp = TempDir::new().unwrap();
        let json = r#"{
            "tasks": [
                {"id": "low-prio", "title": "Low", "priority": -5},
                {"id": "high-prio", "title": "High", "priority": 100}
            ]
        }"#;
        let path = write_json_file(&temp, "prd.json", json);

        let tasks = load_json_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks[0].priority, 1); // Clamped from -5 to 1
        assert_eq!(tasks[1].priority, 5); // Clamped from 100 to 5
    }
}
