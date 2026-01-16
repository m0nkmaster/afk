//! Task source adapters.
//!
//! This module aggregates tasks from various sources (beads, json, markdown, github).

pub mod beads;
pub mod github;
pub mod json;
pub mod markdown;

pub use beads::{close_beads_issue, load_beads_tasks, start_beads_issue};
pub use github::{close_github_issue, load_github_tasks, parse_github_issue_number};
pub use json::load_json_tasks;
pub use markdown::load_markdown_tasks;

use crate::config::{SourceConfig, SourceType};
use crate::prd::UserStory;

/// Aggregate tasks from all configured sources.
///
/// Dispatches to the appropriate loader based on source type, concatenates
/// all results, and handles errors from individual sources gracefully (by
/// logging and continuing with other sources).
///
/// # Arguments
///
/// * `sources` - Slice of source configurations to load tasks from.
///
/// # Returns
///
/// A vector of UserStory items from all sources combined.
///
/// # Example
///
/// ```ignore
/// use afk::config::{SourceConfig, SourceType};
/// use afk::sources::aggregate_tasks;
///
/// let sources = vec![
///     SourceConfig::beads(),
///     SourceConfig::json("tasks.json"),
/// ];
/// let tasks = aggregate_tasks(&sources);
/// ```
pub fn aggregate_tasks(sources: &[SourceConfig]) -> Vec<UserStory> {
    let mut all_tasks: Vec<UserStory> = Vec::new();

    for source in sources {
        let tasks = load_from_source(source);
        all_tasks.extend(tasks);
    }

    all_tasks
}

/// Load tasks from a single source.
///
/// Dispatches to the appropriate loader based on source type. Each loader
/// handles its own errors gracefully and returns an empty vector on failure.
fn load_from_source(source: &SourceConfig) -> Vec<UserStory> {
    match source.source_type {
        SourceType::Beads => load_beads_tasks(),
        SourceType::Json => {
            let path = source.path.as_deref();
            load_json_tasks(path)
        }
        SourceType::Markdown => {
            let path = source.path.as_deref();
            load_markdown_tasks(path)
        }
        SourceType::Github => {
            let repo = source.repo.as_deref();
            load_github_tasks(repo, &source.labels)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_aggregate_tasks_empty_sources() {
        let sources: Vec<SourceConfig> = vec![];
        let tasks = aggregate_tasks(&sources);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_aggregate_tasks_single_json_source() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"{
            "tasks": [
                {"id": "task-1", "title": "First task"},
                {"id": "task-2", "title": "Second task"}
            ]
        }"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![SourceConfig::json(json_path.to_str().unwrap())];
        let tasks = aggregate_tasks(&sources);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "task-1");
        assert_eq!(tasks[1].id, "task-2");
    }

    #[test]
    fn test_aggregate_tasks_single_markdown_source() {
        let temp = TempDir::new().unwrap();
        let md_path = temp.path().join("tasks.md");
        let md_content = r#"
# Tasks

- [ ] First markdown task
- [ ] Second markdown task
- [x] Completed task
"#;
        fs::write(&md_path, md_content).unwrap();

        let sources = vec![SourceConfig::markdown(md_path.to_str().unwrap())];
        let tasks = aggregate_tasks(&sources);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "First markdown task");
        assert_eq!(tasks[1].title, "Second markdown task");
    }

    #[test]
    fn test_aggregate_tasks_multiple_sources() {
        let temp = TempDir::new().unwrap();

        // Create JSON file
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"[{"id": "json-task", "title": "JSON Task"}]"#;
        fs::write(&json_path, json_content).unwrap();

        // Create Markdown file
        let md_path = temp.path().join("tasks.md");
        let md_content = "- [ ] Markdown Task\n";
        fs::write(&md_path, md_content).unwrap();

        let sources = vec![
            SourceConfig::json(json_path.to_str().unwrap()),
            SourceConfig::markdown(md_path.to_str().unwrap()),
        ];
        let tasks = aggregate_tasks(&sources);

        assert_eq!(tasks.len(), 2);
        // JSON task first (from first source)
        assert_eq!(tasks[0].id, "json-task");
        // Markdown task second (from second source)
        assert_eq!(tasks[1].title, "Markdown Task");
    }

    #[test]
    fn test_aggregate_tasks_handles_missing_file_gracefully() {
        let sources = vec![SourceConfig::json("/nonexistent/path/tasks.json")];
        let tasks = aggregate_tasks(&sources);
        // Should return empty, not panic
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_aggregate_tasks_continues_on_source_error() {
        let temp = TempDir::new().unwrap();

        // Create one valid JSON file
        let json_path = temp.path().join("valid.json");
        let json_content = r#"[{"id": "valid-task", "title": "Valid Task"}]"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![
            SourceConfig::json("/nonexistent/path/tasks.json"), // Will fail
            SourceConfig::json(json_path.to_str().unwrap()),    // Will succeed
        ];
        let tasks = aggregate_tasks(&sources);

        // Should still get the valid task from the second source
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "valid-task");
    }

    #[test]
    fn test_aggregate_tasks_with_github_source_not_implemented() {
        let sources = vec![SourceConfig::github("owner/repo", vec!["bug".to_string()])];
        let tasks = aggregate_tasks(&sources);
        // GitHub source returns error, which is handled gracefully
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_aggregate_tasks_mixed_sources_with_github() {
        let temp = TempDir::new().unwrap();

        // Create valid JSON file
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"[{"id": "json-task", "title": "JSON Task"}]"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![
            SourceConfig::json(json_path.to_str().unwrap()),
            SourceConfig::github("owner/repo", vec![]), // Not implemented yet
        ];
        let tasks = aggregate_tasks(&sources);

        // Should still get JSON task even though GitHub source fails
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "json-task");
    }

    #[test]
    fn test_aggregate_tasks_preserves_source_info() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("test.json");
        let json_content = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![SourceConfig::json(json_path.to_str().unwrap())];
        let tasks = aggregate_tasks(&sources);

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].source.starts_with("json:"));
        assert!(tasks[0].source.contains("test.json"));
    }

    #[test]
    fn test_aggregate_tasks_skips_completed_tasks() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"{
            "tasks": [
                {"id": "done", "title": "Done Task", "passes": true},
                {"id": "pending", "title": "Pending Task", "passes": false}
            ]
        }"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![SourceConfig::json(json_path.to_str().unwrap())];
        let tasks = aggregate_tasks(&sources);

        // Only pending task should be returned
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "pending");
    }

    #[test]
    fn test_aggregate_tasks_real_prd_format() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"{
            "project": "test-project",
            "branchName": "main",
            "userStories": [
                {
                    "id": "story-1",
                    "title": "First Story",
                    "description": "Description",
                    "acceptanceCriteria": ["AC1", "AC2"],
                    "priority": 1,
                    "passes": false,
                    "source": "json:tasks.json",
                    "notes": ""
                },
                {
                    "id": "story-2",
                    "title": "Second Story",
                    "priority": 2,
                    "passes": true
                }
            ]
        }"#;
        fs::write(&json_path, json_content).unwrap();

        let sources = vec![SourceConfig::json(json_path.to_str().unwrap())];
        let tasks = aggregate_tasks(&sources);

        // Only story-1 should be returned (story-2 has passes: true)
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "story-1");
        assert_eq!(tasks[0].title, "First Story");
        assert_eq!(tasks[0].priority, 1);
        assert_eq!(tasks[0].acceptance_criteria, vec!["AC1", "AC2"]);
    }

    #[test]
    fn test_load_from_source_beads() {
        let source = SourceConfig::beads();
        // Beads loader may return empty if `bd` is not installed
        let _tasks = load_from_source(&source);
    }

    #[test]
    fn test_load_from_source_json() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("tasks.json");
        let json_content = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        fs::write(&json_path, json_content).unwrap();

        let source = SourceConfig::json(json_path.to_str().unwrap());
        let tasks = load_from_source(&source);

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-1");
    }

    #[test]
    fn test_load_from_source_json_no_path() {
        // When no path is specified, tries default locations
        let source = SourceConfig {
            source_type: SourceType::Json,
            path: None,
            repo: None,
            labels: Vec::new(),
        };
        // Should return empty if no default files exist
        let _tasks = load_from_source(&source);
    }

    #[test]
    fn test_load_from_source_markdown() {
        let temp = TempDir::new().unwrap();
        let md_path = temp.path().join("tasks.md");
        let md_content = "- [ ] Task 1\n- [ ] Task 2\n";
        fs::write(&md_path, md_content).unwrap();

        let source = SourceConfig::markdown(md_path.to_str().unwrap());
        let tasks = load_from_source(&source);

        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_load_from_source_github() {
        let source = SourceConfig::github("owner/repo", vec![]);
        // GitHub loader returns empty if gh not available
        let _tasks = load_from_source(&source);
    }

    #[test]
    fn test_aggregate_tasks_order_preserved() {
        let temp = TempDir::new().unwrap();

        // Create first JSON file
        let json1_path = temp.path().join("first.json");
        let json1_content = r#"[
            {"id": "first-1", "title": "First 1"},
            {"id": "first-2", "title": "First 2"}
        ]"#;
        fs::write(&json1_path, json1_content).unwrap();

        // Create second JSON file
        let json2_path = temp.path().join("second.json");
        let json2_content = r#"[
            {"id": "second-1", "title": "Second 1"},
            {"id": "second-2", "title": "Second 2"}
        ]"#;
        fs::write(&json2_path, json2_content).unwrap();

        let sources = vec![
            SourceConfig::json(json1_path.to_str().unwrap()),
            SourceConfig::json(json2_path.to_str().unwrap()),
        ];
        let tasks = aggregate_tasks(&sources);

        // Order should be preserved: first source tasks, then second source tasks
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].id, "first-1");
        assert_eq!(tasks[1].id, "first-2");
        assert_eq!(tasks[2].id, "second-1");
        assert_eq!(tasks[3].id, "second-2");
    }
}
