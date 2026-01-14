//! Task sync and storage operations.
//!
//! This module implements the Ralph pattern: aggregating tasks from all sources
//! into a unified tasks.json file that the AI reads directly.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::Local;

use crate::config::AfkConfig;
use crate::prd::{PrdDocument, PrdError};
use crate::sources::aggregate_tasks;

/// Sync tasks from all configured sources.
///
/// This aggregates tasks from all sources and writes them to tasks.json.
/// Existing completion status (passes: true) is preserved for matching IDs.
///
/// If no sources are configured but .afk/tasks.json exists with tasks,
/// it's used directly as the source of truth (created by afk import
/// or placed there manually).
///
/// # Arguments
///
/// * `config` - The afk configuration with sources
/// * `branch_name` - Optional branch name for informational purposes (deprecated, prefer None)
///
/// # Returns
///
/// The synced PrdDocument
pub fn sync_prd(config: &AfkConfig) -> Result<PrdDocument, PrdError> {
    sync_prd_with_root(config, None, None)
}

/// Sync tasks from all configured sources with a custom root directory.
///
/// This is the internal implementation that allows specifying a root directory
/// for testing purposes.
pub fn sync_prd_with_root(
    config: &AfkConfig,
    branch_name: Option<&str>,
    root: Option<&Path>,
) -> Result<PrdDocument, PrdError> {
    // Determine tasks path
    let prd_path = root.map(|r| r.join(".afk/tasks.json"));

    // Load existing tasks
    let existing_prd = PrdDocument::load(prd_path.as_deref())?;

    // If no sources configured but tasks.json exists with tasks, use it directly.
    // This handles the case where user created .afk/tasks.json via afk import
    // or placed it there manually — we don't want to overwrite it.
    if config.sources.is_empty() && !existing_prd.user_stories.is_empty() {
        return Ok(existing_prd);
    }

    // Build map of existing stories by ID for merging
    let mut existing_by_id: HashMap<String, crate::prd::UserStory> = existing_prd
        .user_stories
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect();

    // Aggregate from all sources
    let source_stories = aggregate_tasks(&config.sources);

    // Merge: add new tasks from sources, update existing ones (preserving passes status)
    for mut story in source_stories {
        if let Some(existing) = existing_by_id.get(&story.id) {
            // Task exists - preserve completion status
            story.passes = existing.passes;
        }
        // Insert or update (source is authoritative for non-passes fields)
        existing_by_id.insert(story.id.clone(), story);
    }

    // Collect all stories and sort by priority (1 = highest)
    let mut stories: Vec<_> = existing_by_id.into_values().collect();
    stories.sort_by_key(|s| s.priority);

    // Get branch name (informational only, afk does not manage branches)
    let branch = branch_name.map(String::from).unwrap_or_default();

    // Get project name
    let project = get_project_name_from_root(root);

    // Preserve description if already set
    let description = if existing_prd.description.is_empty() {
        "Tasks synced from configured sources".to_string()
    } else {
        existing_prd.description
    };

    // Build PRD document
    let prd = PrdDocument {
        project,
        branch_name: branch,
        description,
        user_stories: stories,
        last_synced: Local::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
    };

    // Save to disk
    prd.save(prd_path.as_deref())?;

    Ok(prd)
}

/// Get the current git branch name.
///
/// Returns "main" if git is not available or not in a git repo.
pub fn get_current_branch() -> String {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch.is_empty() {
                "main".to_string()
            } else {
                branch
            }
        }
        _ => "main".to_string(),
    }
}

/// Get the project name from pyproject.toml, Cargo.toml, or directory name.
pub fn get_project_name() -> String {
    get_project_name_from_root(None)
}

/// Get the project name from a specific root directory.
pub fn get_project_name_from_root(root: Option<&Path>) -> String {
    let base = root.unwrap_or_else(|| Path::new("."));

    // Try pyproject.toml first
    if let Some(name) = get_name_from_pyproject_at(base) {
        return name;
    }

    // Try Cargo.toml
    if let Some(name) = get_name_from_cargo_toml_at(base) {
        return name;
    }

    // Fall back to directory name
    if let Some(root_path) = root {
        root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Extract project name from pyproject.toml at a given path.
fn get_name_from_pyproject_at(root: &Path) -> Option<String> {
    let path = root.join("pyproject.toml");
    if !path.exists() {
        return None;
    }

    let contents = fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&contents).ok()?;

    // Try project.name first
    if let Some(name) = value
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    // Try tool.poetry.name as fallback
    if let Some(name) = value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    None
}

/// Extract project name from Cargo.toml at a given path.
fn get_name_from_cargo_toml_at(root: &Path) -> Option<String> {
    let path = root.join("Cargo.toml");
    if !path.exists() {
        return None;
    }

    let contents = fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&contents).ok()?;

    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .filter(|n| !n.is_empty())
        .map(String::from)
}

/// Mark a story as in progress and sync to source if needed.
///
/// Returns true if the story was found and synced.
pub fn mark_story_in_progress(story_id: &str) -> Result<bool, PrdError> {
    mark_story_in_progress_with_path(story_id, None)
}

/// Mark a story as in progress with a custom PRD path.
pub fn mark_story_in_progress_with_path(
    story_id: &str,
    prd_path: Option<&Path>,
) -> Result<bool, PrdError> {
    let prd = PrdDocument::load(prd_path)?;

    // Find the story
    let story = prd.user_stories.iter().find(|s| s.id == story_id);

    if let Some(story) = story {
        let source = story.source.clone();

        // Sync in_progress status to source
        if source == "beads" {
            use crate::sources::start_beads_issue;
            start_beads_issue(story_id);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Mark a story as complete and sync back to source if needed.
///
/// Returns true if the story was found and marked complete.
pub fn mark_story_complete(story_id: &str) -> Result<bool, PrdError> {
    mark_story_complete_with_path(story_id, None)
}

/// Mark a story as complete with a custom PRD path.
pub fn mark_story_complete_with_path(
    story_id: &str,
    prd_path: Option<&Path>,
) -> Result<bool, PrdError> {
    let mut prd = PrdDocument::load(prd_path)?;

    // Find the story
    let story = prd.user_stories.iter_mut().find(|s| s.id == story_id);

    if let Some(story) = story {
        story.passes = true;
        let source = story.source.clone();
        prd.save(prd_path)?;

        // Sync completion back to source
        if source == "beads" {
            use crate::sources::close_beads_issue;
            close_beads_issue(story_id);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_tasks_no_sources_with_existing_tasks() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with stories
        let existing_tasks = r#"{
            "project": "test-project",
            "branchName": "main",
            "userStories": [
                {"id": "story-1", "title": "Existing Story", "description": "Test", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Config with no sources
        let config = AfkConfig::default();

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should return existing tasks as-is
        assert_eq!(result.project, "test-project");
        assert_eq!(result.user_stories.len(), 1);
        assert_eq!(result.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_sync_tasks_empty_sync_preserves_pending_tasks() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with PENDING stories (passes: false)
        let existing_tasks = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "story-1", "title": "Pending Story", "description": "Test", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Config with a source that returns empty (non-existent file)
        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json("/nonexistent/path.json")],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should preserve existing pending tasks to protect work in progress
        assert_eq!(result.user_stories.len(), 1);
        assert_eq!(result.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_sync_tasks_merges_new_tasks_with_existing_completed() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with completed story
        let existing_tasks = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "old-story", "title": "Completed Story", "description": "Test", "priority": 1, "passes": true}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Config with a source that returns a NEW task
        let source_json = r#"[{"id": "new-story", "title": "New Task", "priority": 2}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should have BOTH the old completed task AND the new task
        assert_eq!(result.user_stories.len(), 2);

        // Old task should still be marked complete
        let old_story = result.user_stories.iter().find(|s| s.id == "old-story");
        assert!(old_story.is_some());
        assert!(old_story.unwrap().passes);

        // New task should be pending
        let new_story = result.user_stories.iter().find(|s| s.id == "new-story");
        assert!(new_story.is_some());
        assert!(!new_story.unwrap().passes);
    }

    #[test]
    fn test_sync_tasks_preserves_existing_when_sources_empty() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with completed story
        let existing_tasks = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "old-story", "title": "Completed Story", "description": "Test", "priority": 1, "passes": true}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Config with a source that returns empty (non-existent file)
        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json("/nonexistent/path.json")],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should preserve existing tasks even when sources return empty
        assert_eq!(result.user_stories.len(), 1);
        assert_eq!(result.user_stories[0].id, "old-story");
        assert!(result.user_stories[0].passes);
    }

    #[test]
    fn test_sync_tasks_preserves_passes_status() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with a completed story
        let existing_tasks = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "priority": 1, "passes": true},
                {"id": "story-2", "title": "Story 2", "description": "Test", "priority": 2, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Create source file with the same stories
        let source_json = r#"[
            {"id": "story-1", "title": "Story 1 Updated", "description": "Updated", "priority": 1},
            {"id": "story-2", "title": "Story 2 Updated", "description": "Updated", "priority": 2}
        ]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // story-1 should still have passes: true
        let story1 = result.user_stories.iter().find(|s| s.id == "story-1");
        assert!(story1.is_some());
        assert!(story1.unwrap().passes);

        // story-2 should still have passes: false
        let story2 = result.user_stories.iter().find(|s| s.id == "story-2");
        assert!(story2.is_some());
        assert!(!story2.unwrap().passes);
    }

    #[test]
    fn test_sync_tasks_sorts_by_priority() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create source with stories in wrong order
        let source_json = r#"[
            {"id": "low", "title": "Low Priority", "priority": 3},
            {"id": "high", "title": "High Priority", "priority": 1},
            {"id": "medium", "title": "Medium Priority", "priority": 2}
        ]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should be sorted by priority
        assert_eq!(result.user_stories[0].id, "high");
        assert_eq!(result.user_stories[1].id, "medium");
        assert_eq!(result.user_stories[2].id, "low");
    }

    #[test]
    fn test_sync_tasks_sets_last_synced() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should have a last_synced timestamp
        assert!(!result.last_synced.is_empty());
        // Timestamp should be in ISO format
        assert!(result.last_synced.contains('T'));
    }

    #[test]
    fn test_sync_tasks_branch_name_defaults_to_empty() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        // afk no longer manages branches - branch_name should be empty by default
        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        assert!(result.branch_name.is_empty());
    }

    #[test]
    fn test_get_name_from_pyproject() {
        let temp = TempDir::new().unwrap();

        let pyproject = r#"
[project]
name = "test-project"
version = "1.0.0"
"#;
        fs::write(temp.path().join("pyproject.toml"), pyproject).unwrap();

        let name = get_name_from_pyproject_at(temp.path());
        assert_eq!(name, Some("test-project".to_string()));
    }

    #[test]
    fn test_get_name_from_pyproject_poetry() {
        let temp = TempDir::new().unwrap();

        let pyproject = r#"
[tool.poetry]
name = "poetry-project"
version = "1.0.0"
"#;
        fs::write(temp.path().join("pyproject.toml"), pyproject).unwrap();

        let name = get_name_from_pyproject_at(temp.path());
        assert_eq!(name, Some("poetry-project".to_string()));
    }

    #[test]
    fn test_get_name_from_cargo_toml() {
        let temp = TempDir::new().unwrap();

        let cargo = r#"
[package]
name = "rust-project"
version = "1.0.0"
"#;
        fs::write(temp.path().join("Cargo.toml"), cargo).unwrap();

        let name = get_name_from_cargo_toml_at(temp.path());
        assert_eq!(name, Some("rust-project".to_string()));
    }

    #[test]
    fn test_get_project_name_fallback_to_directory() {
        let temp = TempDir::new().unwrap();

        // No project files present
        let name = get_project_name_from_root(Some(temp.path()));

        // Should fall back to directory name
        let expected = temp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(name, expected);
    }

    #[test]
    fn test_get_current_branch_fallback() {
        // The function should always return a non-empty string
        let branch = get_current_branch();
        // May return "main" or actual branch depending on test environment
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_mark_story_complete() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let tasks = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false},
                {"id": "story-2", "title": "Story 2", "description": "Test", "passes": false}
            ]
        }"#;
        let tasks_path = afk_dir.join("tasks.json");
        fs::write(&tasks_path, tasks).unwrap();

        let result = mark_story_complete_with_path("story-1", Some(&tasks_path)).unwrap();
        assert!(result);

        // Reload and verify
        let prd = PrdDocument::load(Some(&tasks_path)).unwrap();
        assert!(prd.user_stories[0].passes);
        assert!(!prd.user_stories[1].passes);
    }

    #[test]
    fn test_mark_story_complete_not_found() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let tasks = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false}
            ]
        }"#;
        let tasks_path = afk_dir.join("tasks.json");
        fs::write(&tasks_path, tasks).unwrap();

        let result = mark_story_complete_with_path("nonexistent", Some(&tasks_path)).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_mark_story_in_progress() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let tasks = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false, "source": "json"}
            ]
        }"#;
        let tasks_path = afk_dir.join("tasks.json");
        fs::write(&tasks_path, tasks).unwrap();

        // Should return true when story is found (even if source doesn't support in_progress)
        let result = mark_story_in_progress_with_path("story-1", Some(&tasks_path)).unwrap();
        assert!(result);
    }

    #[test]
    fn test_mark_story_in_progress_not_found() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let tasks = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false}
            ]
        }"#;
        let tasks_path = afk_dir.join("tasks.json");
        fs::write(&tasks_path, tasks).unwrap();

        let result = mark_story_in_progress_with_path("nonexistent", Some(&tasks_path)).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sync_tasks_preserves_existing_description() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing tasks with a custom description
        let existing_tasks = r#"{
            "project": "test-project",
            "description": "My custom description",
            "userStories": []
        }"#;
        fs::write(afk_dir.join("tasks.json"), existing_tasks).unwrap();

        // Create source file
        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should preserve the custom description
        assert_eq!(result.description, "My custom description");
    }

    #[test]
    fn test_sync_tasks_sets_default_description() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create source file
        let source_json = r#"[{"id": "task-1", "title": "Task 1"}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should set default description
        assert_eq!(result.description, "Tasks synced from configured sources");
    }

    #[test]
    fn test_sync_tasks_writes_to_disk() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let source_json = r#"[{"id": "task-1", "title": "Task 1", "priority": 1}]"#;
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, source_json).unwrap();

        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json(
                source_path.to_str().unwrap(),
            )],
            ..Default::default()
        };

        let _ = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Verify file was written
        let tasks_path = afk_dir.join("tasks.json");
        assert!(tasks_path.exists());

        // Verify contents
        let contents = fs::read_to_string(&tasks_path).unwrap();
        assert!(contents.contains("task-1"));
    }

    #[test]
    fn test_get_project_name_from_pyproject() {
        let temp = TempDir::new().unwrap();

        let pyproject = r#"
[project]
name = "pyproject-name"
version = "1.0.0"
"#;
        fs::write(temp.path().join("pyproject.toml"), pyproject).unwrap();

        let name = get_project_name_from_root(Some(temp.path()));
        assert_eq!(name, "pyproject-name");
    }

    #[test]
    fn test_get_project_name_from_cargo() {
        let temp = TempDir::new().unwrap();

        let cargo = r#"
[package]
name = "cargo-name"
version = "1.0.0"
"#;
        fs::write(temp.path().join("Cargo.toml"), cargo).unwrap();

        let name = get_project_name_from_root(Some(temp.path()));
        assert_eq!(name, "cargo-name");
    }

    #[test]
    fn test_get_project_name_pyproject_preferred() {
        let temp = TempDir::new().unwrap();

        // Both files present - pyproject.toml should be preferred
        let pyproject = r#"
[project]
name = "python-project"
"#;
        fs::write(temp.path().join("pyproject.toml"), pyproject).unwrap();

        let cargo = r#"
[package]
name = "rust-project"
"#;
        fs::write(temp.path().join("Cargo.toml"), cargo).unwrap();

        let name = get_project_name_from_root(Some(temp.path()));
        assert_eq!(name, "python-project");
    }
}
