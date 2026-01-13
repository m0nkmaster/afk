//! PRD sync and storage operations.
//!
//! This module implements the Ralph pattern: aggregating tasks from all sources
//! into a unified prd.json file that the AI reads directly.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::Local;

use crate::config::AfkConfig;
use crate::prd::{PrdDocument, PrdError};
use crate::sources::aggregate_tasks;

/// Sync PRD from all configured sources.
///
/// This aggregates tasks from all sources and writes them to prd.json.
/// Existing completion status (passes: true) is preserved for matching IDs.
///
/// If no sources are configured but .afk/prd.json exists with stories,
/// it's used directly as the source of truth (created by afk prd parse
/// or placed it there manually).
///
/// # Arguments
///
/// * `config` - The afk configuration with sources
/// * `branch_name` - Optional branch name override; if None, gets from git
///
/// # Returns
///
/// The synced PrdDocument
pub fn sync_prd(config: &AfkConfig, branch_name: Option<&str>) -> Result<PrdDocument, PrdError> {
    sync_prd_with_root(config, branch_name, None)
}

/// Sync PRD from all configured sources with a custom root directory.
///
/// This is the internal implementation that allows specifying a root directory
/// for testing purposes.
pub fn sync_prd_with_root(
    config: &AfkConfig,
    branch_name: Option<&str>,
    root: Option<&Path>,
) -> Result<PrdDocument, PrdError> {
    // Determine PRD path
    let prd_path = root.map(|r| r.join(".afk/prd.json"));

    // Load existing PRD
    let existing_prd = PrdDocument::load(prd_path.as_deref())?;

    // If no sources configured but PRD exists with stories, use it directly.
    // This handles the case where user created .afk/prd.json via afk prd parse
    // or placed it there manually — we don't want to overwrite it.
    if config.sources.is_empty() && !existing_prd.user_stories.is_empty() {
        return Ok(existing_prd);
    }

    // Build map of existing completion status
    let existing_status: HashMap<String, bool> = existing_prd
        .user_stories
        .iter()
        .map(|s| (s.id.clone(), s.passes))
        .collect();

    // Aggregate from all sources
    let mut stories = aggregate_tasks(&config.sources);

    // Safety check: don't wipe a populated PRD with an empty sync.
    // This protects against sources returning nothing (e.g., empty beads).
    if stories.is_empty() && !existing_prd.user_stories.is_empty() {
        return Ok(existing_prd);
    }

    // Preserve completion status from previous sync
    for story in &mut stories {
        if let Some(&passes) = existing_status.get(&story.id) {
            story.passes = passes;
        }
    }

    // Sort by priority (1 = highest)
    stories.sort_by_key(|s| s.priority);

    // Get branch name
    let branch = branch_name
        .map(String::from)
        .unwrap_or_else(get_current_branch);

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
    fn test_sync_prd_no_sources_with_existing_prd() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing PRD with stories
        let existing_prd = r#"{
            "project": "test-project",
            "branchName": "main",
            "userStories": [
                {"id": "story-1", "title": "Existing Story", "description": "Test", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("prd.json"), existing_prd).unwrap();

        // Config with no sources
        let config = AfkConfig::default();

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should return existing PRD as-is
        assert_eq!(result.project, "test-project");
        assert_eq!(result.user_stories.len(), 1);
        assert_eq!(result.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_sync_prd_empty_sync_preserves_existing() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing PRD with stories
        let existing_prd = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "story-1", "title": "Existing Story", "description": "Test", "priority": 1}
            ]
        }"#;
        fs::write(afk_dir.join("prd.json"), existing_prd).unwrap();

        // Config with a source that returns empty (non-existent file)
        let config = AfkConfig {
            sources: vec![crate::config::SourceConfig::json("/nonexistent/path.json")],
            ..Default::default()
        };

        let result = sync_prd_with_root(&config, None, Some(temp.path())).unwrap();

        // Should preserve existing PRD
        assert_eq!(result.user_stories.len(), 1);
        assert_eq!(result.user_stories[0].id, "story-1");
    }

    #[test]
    fn test_sync_prd_preserves_passes_status() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing PRD with a completed story
        let existing_prd = r#"{
            "project": "test-project",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "priority": 1, "passes": true},
                {"id": "story-2", "title": "Story 2", "description": "Test", "priority": 2, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("prd.json"), existing_prd).unwrap();

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
    fn test_sync_prd_sorts_by_priority() {
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
    fn test_sync_prd_sets_last_synced() {
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
    fn test_sync_prd_with_branch_name_override() {
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

        let result =
            sync_prd_with_root(&config, Some("feature/custom-branch"), Some(temp.path())).unwrap();

        assert_eq!(result.branch_name, "feature/custom-branch");
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

        let prd = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false},
                {"id": "story-2", "title": "Story 2", "description": "Test", "passes": false}
            ]
        }"#;
        let prd_path = afk_dir.join("prd.json");
        fs::write(&prd_path, prd).unwrap();

        let result = mark_story_complete_with_path("story-1", Some(&prd_path)).unwrap();
        assert!(result);

        // Reload and verify
        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert!(prd.user_stories[0].passes);
        assert!(!prd.user_stories[1].passes);
    }

    #[test]
    fn test_mark_story_complete_not_found() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let prd = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false}
            ]
        }"#;
        let prd_path = afk_dir.join("prd.json");
        fs::write(&prd_path, prd).unwrap();

        let result = mark_story_complete_with_path("nonexistent", Some(&prd_path)).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_mark_story_in_progress() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let prd = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false, "source": "json"}
            ]
        }"#;
        let prd_path = afk_dir.join("prd.json");
        fs::write(&prd_path, prd).unwrap();

        // Should return true when story is found (even if source doesn't support in_progress)
        let result = mark_story_in_progress_with_path("story-1", Some(&prd_path)).unwrap();
        assert!(result);
    }

    #[test]
    fn test_mark_story_in_progress_not_found() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let prd = r#"{
            "project": "test",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "description": "Test", "passes": false}
            ]
        }"#;
        let prd_path = afk_dir.join("prd.json");
        fs::write(&prd_path, prd).unwrap();

        let result = mark_story_in_progress_with_path("nonexistent", Some(&prd_path)).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sync_prd_preserves_existing_description() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        // Create existing PRD with a custom description
        let existing_prd = r#"{
            "project": "test-project",
            "description": "My custom description",
            "userStories": []
        }"#;
        fs::write(afk_dir.join("prd.json"), existing_prd).unwrap();

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
    fn test_sync_prd_sets_default_description() {
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
    fn test_sync_prd_writes_to_disk() {
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

        let _ = sync_prd_with_root(&config, Some("test-branch"), Some(temp.path())).unwrap();

        // Verify file was written
        let prd_path = afk_dir.join("prd.json");
        assert!(prd_path.exists());

        // Verify contents
        let contents = fs::read_to_string(&prd_path).unwrap();
        assert!(contents.contains("task-1"));
        assert!(contents.contains("test-branch"));
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
