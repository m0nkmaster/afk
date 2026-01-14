//! Task models for .afk/tasks.json.
//!
//! This module contains Serde models for user stories and task documents.
//! The "PRD" terminology is retained internally for backwards compatibility.

pub mod parse;
pub mod store;

pub use parse::{generate_prd_prompt, load_prd_file, PrdParseError, PRD_PARSE_TEMPLATE};
pub use store::{
    get_current_branch, get_project_name, get_project_name_from_root, mark_story_complete,
    mark_story_complete_with_path, mark_story_in_progress, mark_story_in_progress_with_path,
    sync_prd, sync_prd_with_root,
};

use crate::config::TASKS_FILE;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A user story in Ralph format with acceptance criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStory {
    /// Unique identifier for the story.
    pub id: String,
    /// Short title describing the story.
    pub title: String,
    /// Longer description of the story.
    pub description: String,
    /// List of acceptance criteria.
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// Priority level (1-5, 1 = highest).
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Whether the story has been completed and verified.
    #[serde(default)]
    pub passes: bool,
    /// Source identifier (e.g., "beads", "json:path/to/file.json").
    #[serde(default = "default_source")]
    pub source: String,
    /// Additional notes.
    #[serde(default)]
    pub notes: String,
}

fn default_priority() -> i32 {
    3
}

fn default_source() -> String {
    "unknown".to_string()
}

impl Default for UserStory {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            description: String::new(),
            acceptance_criteria: Vec::new(),
            priority: default_priority(),
            passes: false,
            source: default_source(),
            notes: String::new(),
        }
    }
}

impl UserStory {
    /// Create a new UserStory with the given ID and title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            id: id.into(),
            title: title.clone(),
            description: title,
            ..Default::default()
        }
    }

    /// Create from a JSON dict that may use various key names.
    ///
    /// Supports:
    /// - `acceptanceCriteria` (camelCase)
    /// - `acceptance_criteria` (snake_case)
    /// - `steps` (alternative name)
    pub fn from_json_value(data: &serde_json::Value) -> Self {
        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = data
            .get("title")
            .or_else(|| data.get("description"))
            .or_else(|| data.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = data
            .get("description")
            .or_else(|| data.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let acceptance_criteria = data
            .get("acceptanceCriteria")
            .or_else(|| data.get("acceptance_criteria"))
            .or_else(|| data.get("steps"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let priority = data
            .get("priority")
            .and_then(|v| v.as_i64())
            .map(|p| p as i32)
            .unwrap_or(default_priority());

        let passes = data
            .get("passes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let source = data
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("json:.afk/tasks.json")
            .to_string();

        let notes = data
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Self {
            id,
            title,
            description,
            acceptance_criteria,
            priority,
            passes,
            source,
            notes,
        }
    }
}

/// The unified PRD document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrdDocument {
    /// Project name.
    #[serde(default)]
    pub project: String,
    /// Branch name for this PRD.
    #[serde(default)]
    pub branch_name: String,
    /// Description of the PRD.
    #[serde(default)]
    pub description: String,
    /// List of user stories.
    #[serde(default)]
    pub user_stories: Vec<UserStory>,
    /// ISO timestamp of last sync.
    #[serde(default)]
    pub last_synced: String,
}

/// Error type for PRD operations.
#[derive(Debug, thiserror::Error)]
pub enum PrdError {
    #[error("Failed to read PRD file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse PRD JSON: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl PrdDocument {
    /// Create from a JSON value that may use various key names.
    ///
    /// Supports multiple key names for stories:
    /// - `userStories` (canonical)
    /// - `tasks` (afk import output)
    /// - `items` (legacy)
    pub fn from_json_value(data: &serde_json::Value) -> Self {
        let project = data
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let branch_name = data
            .get("branchName")
            .or_else(|| data.get("branch_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let last_synced = data
            .get("lastSynced")
            .or_else(|| data.get("last_synced"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Support multiple key names for stories
        let story_data = data
            .get("userStories")
            .or_else(|| data.get("tasks"))
            .or_else(|| data.get("items"))
            .and_then(|v| v.as_array());

        let user_stories = story_data
            .map(|arr| arr.iter().map(UserStory::from_json_value).collect())
            .unwrap_or_default();

        Self {
            project,
            branch_name,
            description,
            user_stories,
            last_synced,
        }
    }

    /// Load task document from a file, or return defaults if file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to tasks file. Defaults to `.afk/tasks.json` if None.
    pub fn load(path: Option<&Path>) -> Result<Self, PrdError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(TASKS_FILE));

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let data: serde_json::Value = serde_json::from_str(&contents)?;

        Ok(Self::from_json_value(&data))
    }

    /// Save task document to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save to. Defaults to `.afk/tasks.json` if None.
    ///
    /// Creates parent directories if they don't exist.
    pub fn save(&self, path: Option<&Path>) -> Result<(), PrdError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(TASKS_FILE));

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Get stories that haven't passed yet, sorted by priority.
    pub fn get_pending_stories(&self) -> Vec<&UserStory> {
        let mut pending: Vec<&UserStory> = self.user_stories.iter().filter(|s| !s.passes).collect();
        pending.sort_by_key(|s| s.priority);
        pending
    }

    /// Get the next story to work on (highest priority, not passed).
    pub fn get_next_story(&self) -> Option<&UserStory> {
        self.get_pending_stories().into_iter().next()
    }

    /// Check if all stories have passed.
    pub fn all_stories_complete(&self) -> bool {
        if self.user_stories.is_empty() {
            return true;
        }
        self.user_stories.iter().all(|s| s.passes)
    }

    /// Mark a story as complete (passes: true).
    ///
    /// Returns true if the story was found and updated.
    pub fn mark_story_complete(&mut self, story_id: &str) -> bool {
        for story in &mut self.user_stories {
            if story.id == story_id {
                story.passes = true;
                return true;
            }
        }
        false
    }

    /// Get a story by ID.
    pub fn get_story(&self, story_id: &str) -> Option<&UserStory> {
        self.user_stories.iter().find(|s| s.id == story_id)
    }

    /// Get a mutable reference to a story by ID.
    pub fn get_story_mut(&mut self, story_id: &str) -> Option<&mut UserStory> {
        self.user_stories.iter_mut().find(|s| s.id == story_id)
    }

    /// Get counts of completed and total stories.
    pub fn get_story_counts(&self) -> (usize, usize) {
        let completed = self.user_stories.iter().filter(|s| s.passes).count();
        let total = self.user_stories.len();
        (completed, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_user_story_defaults() {
        let story = UserStory::default();
        assert!(story.id.is_empty());
        assert!(story.title.is_empty());
        assert!(story.description.is_empty());
        assert!(story.acceptance_criteria.is_empty());
        assert_eq!(story.priority, 3);
        assert!(!story.passes);
        assert_eq!(story.source, "unknown");
        assert!(story.notes.is_empty());
    }

    #[test]
    fn test_user_story_new() {
        let story = UserStory::new("test-001", "Test Story");
        assert_eq!(story.id, "test-001");
        assert_eq!(story.title, "Test Story");
        assert_eq!(story.description, "Test Story");
        assert_eq!(story.priority, 3);
        assert!(!story.passes);
    }

    #[test]
    fn test_user_story_serialisation_camel_case() {
        let story = UserStory {
            id: "test-001".to_string(),
            title: "Test Story".to_string(),
            description: "A test story".to_string(),
            acceptance_criteria: vec!["AC1".to_string(), "AC2".to_string()],
            priority: 1,
            passes: true,
            source: "json:test.json".to_string(),
            notes: "Some notes".to_string(),
        };

        let json = serde_json::to_string(&story).unwrap();
        assert!(json.contains(r#""acceptanceCriteria""#));
        assert!(!json.contains(r#""acceptance_criteria""#));
    }

    #[test]
    fn test_user_story_from_json_camel_case() {
        let json = r#"{
            "id": "test-001",
            "title": "Test Story",
            "description": "A test story",
            "acceptanceCriteria": ["AC1", "AC2"],
            "priority": 1,
            "passes": true,
            "source": "beads",
            "notes": "Test notes"
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        assert_eq!(story.id, "test-001");
        assert_eq!(story.title, "Test Story");
        assert_eq!(story.description, "A test story");
        assert_eq!(story.acceptance_criteria, vec!["AC1", "AC2"]);
        assert_eq!(story.priority, 1);
        assert!(story.passes);
        assert_eq!(story.source, "beads");
        assert_eq!(story.notes, "Test notes");
    }

    #[test]
    fn test_user_story_from_json_snake_case() {
        let json = r#"{
            "id": "test-002",
            "title": "Snake Case Story",
            "description": "Testing snake_case",
            "acceptance_criteria": ["Step 1", "Step 2"]
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        assert_eq!(story.id, "test-002");
        assert_eq!(story.acceptance_criteria, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_user_story_from_json_steps_key() {
        let json = r#"{
            "id": "test-003",
            "title": "Steps Key Story",
            "description": "Testing steps key",
            "steps": ["Do this", "Then that"]
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        assert_eq!(story.acceptance_criteria, vec!["Do this", "Then that"]);
    }

    #[test]
    fn test_user_story_from_json_title_fallback_to_description() {
        let json = r#"{
            "id": "test-004",
            "description": "Only description provided"
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        assert_eq!(story.title, "Only description provided");
        assert_eq!(story.description, "Only description provided");
    }

    #[test]
    fn test_user_story_from_json_title_fallback_to_summary() {
        let json = r#"{
            "id": "test-005",
            "summary": "Summary as title"
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        assert_eq!(story.title, "Summary as title");
    }

    #[test]
    fn test_prd_document_defaults() {
        let prd = PrdDocument::default();
        assert!(prd.project.is_empty());
        assert!(prd.branch_name.is_empty());
        assert!(prd.description.is_empty());
        assert!(prd.user_stories.is_empty());
        assert!(prd.last_synced.is_empty());
    }

    #[test]
    fn test_prd_document_serialisation_camel_case() {
        let prd = PrdDocument {
            project: "test-project".to_string(),
            branch_name: "main".to_string(),
            description: "Test PRD".to_string(),
            user_stories: vec![UserStory::new("test-001", "Test Story")],
            last_synced: "2024-01-01T00:00:00".to_string(),
        };

        let json = serde_json::to_string(&prd).unwrap();
        assert!(json.contains(r#""branchName""#));
        assert!(json.contains(r#""userStories""#));
        assert!(json.contains(r#""lastSynced""#));
        assert!(!json.contains(r#""branch_name""#));
        assert!(!json.contains(r#""user_stories""#));
    }

    #[test]
    fn test_prd_document_from_json_user_stories() {
        let json = r#"{
            "project": "test-project",
            "branchName": "feature/test",
            "description": "Test PRD",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "priority": 1},
                {"id": "story-2", "title": "Story 2", "priority": 2}
            ],
            "lastSynced": "2024-01-01T00:00:00"
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let prd = PrdDocument::from_json_value(&data);

        assert_eq!(prd.project, "test-project");
        assert_eq!(prd.branch_name, "feature/test");
        assert_eq!(prd.description, "Test PRD");
        assert_eq!(prd.user_stories.len(), 2);
        assert_eq!(prd.user_stories[0].id, "story-1");
        assert_eq!(prd.user_stories[1].id, "story-2");
        assert_eq!(prd.last_synced, "2024-01-01T00:00:00");
    }

    #[test]
    fn test_prd_document_from_json_tasks_key() {
        let json = r#"{
            "project": "test-project",
            "tasks": [
                {"id": "task-1", "title": "Task 1"},
                {"id": "task-2", "title": "Task 2"}
            ]
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let prd = PrdDocument::from_json_value(&data);

        assert_eq!(prd.user_stories.len(), 2);
        assert_eq!(prd.user_stories[0].id, "task-1");
    }

    #[test]
    fn test_prd_document_from_json_items_key() {
        let json = r#"{
            "items": [
                {"id": "item-1", "title": "Item 1"}
            ]
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let prd = PrdDocument::from_json_value(&data);

        assert_eq!(prd.user_stories.len(), 1);
        assert_eq!(prd.user_stories[0].id, "item-1");
    }

    #[test]
    fn test_prd_document_from_json_snake_case_keys() {
        let json = r#"{
            "project": "test",
            "branch_name": "main",
            "last_synced": "2024-01-01"
        }"#;

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let prd = PrdDocument::from_json_value(&data);

        assert_eq!(prd.branch_name, "main");
        assert_eq!(prd.last_synced, "2024-01-01");
    }

    #[test]
    fn test_prd_document_load_missing_file() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join(".afk/tasks.json");
        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert!(prd.user_stories.is_empty());
    }

    #[test]
    fn test_prd_document_load_existing_file() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let prd_path = afk_dir.join("tasks.json");

        let sample_prd = r#"{
            "project": "loaded-project",
            "branchName": "main",
            "userStories": [
                {"id": "story-1", "title": "Story 1", "passes": true}
            ]
        }"#;
        fs::write(&prd_path, sample_prd).unwrap();

        let prd = PrdDocument::load(Some(&prd_path)).unwrap();
        assert_eq!(prd.project, "loaded-project");
        assert_eq!(prd.user_stories.len(), 1);
        assert!(prd.user_stories[0].passes);
    }

    #[test]
    fn test_prd_document_save_creates_directory() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join(".afk/tasks.json");

        let prd = PrdDocument {
            project: "saved-project".to_string(),
            user_stories: vec![UserStory::new("story-1", "Story 1")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        assert!(prd_path.exists());
        let contents = fs::read_to_string(&prd_path).unwrap();
        assert!(contents.contains(r#""project": "saved-project""#));
    }

    #[test]
    fn test_prd_document_round_trip() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join(".afk/tasks.json");

        let original = PrdDocument {
            project: "roundtrip-project".to_string(),
            branch_name: "feature/test".to_string(),
            description: "Test description".to_string(),
            user_stories: vec![
                UserStory {
                    id: "story-1".to_string(),
                    title: "First Story".to_string(),
                    description: "Description 1".to_string(),
                    acceptance_criteria: vec!["AC1".to_string(), "AC2".to_string()],
                    priority: 1,
                    passes: false,
                    source: "beads".to_string(),
                    notes: "Notes 1".to_string(),
                },
                UserStory {
                    id: "story-2".to_string(),
                    title: "Second Story".to_string(),
                    description: "Description 2".to_string(),
                    acceptance_criteria: vec!["AC3".to_string()],
                    priority: 2,
                    passes: true,
                    source: "json:test.json".to_string(),
                    notes: String::new(),
                },
            ],
            last_synced: "2024-01-01T12:00:00".to_string(),
        };

        original.save(Some(&prd_path)).unwrap();
        let loaded = PrdDocument::load(Some(&prd_path)).unwrap();

        assert_eq!(loaded.project, "roundtrip-project");
        assert_eq!(loaded.branch_name, "feature/test");
        assert_eq!(loaded.user_stories.len(), 2);
        assert_eq!(loaded.user_stories[0].id, "story-1");
        assert_eq!(
            loaded.user_stories[0].acceptance_criteria,
            vec!["AC1", "AC2"]
        );
        assert!(!loaded.user_stories[0].passes);
        assert!(loaded.user_stories[1].passes);
    }

    #[test]
    fn test_get_pending_stories() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "high-priority".to_string(),
                    priority: 1,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "completed".to_string(),
                    priority: 1,
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "low-priority".to_string(),
                    priority: 3,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "medium-priority".to_string(),
                    priority: 2,
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let pending = prd.get_pending_stories();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].id, "high-priority");
        assert_eq!(pending[1].id, "medium-priority");
        assert_eq!(pending[2].id, "low-priority");
    }

    #[test]
    fn test_get_next_story() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "completed".to_string(),
                    priority: 1,
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "pending-low".to_string(),
                    priority: 3,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "pending-high".to_string(),
                    priority: 1,
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let next = prd.get_next_story();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "pending-high");
    }

    #[test]
    fn test_get_next_story_empty() {
        let prd = PrdDocument::default();
        assert!(prd.get_next_story().is_none());
    }

    #[test]
    fn test_get_next_story_all_complete() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "done-1".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "done-2".to_string(),
                    passes: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(prd.get_next_story().is_none());
    }

    #[test]
    fn test_all_stories_complete_empty() {
        let prd = PrdDocument::default();
        assert!(prd.all_stories_complete());
    }

    #[test]
    fn test_all_stories_complete_true() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "done-1".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "done-2".to_string(),
                    passes: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(prd.all_stories_complete());
    }

    #[test]
    fn test_all_stories_complete_false() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "done".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "pending".to_string(),
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(!prd.all_stories_complete());
    }

    #[test]
    fn test_mark_story_complete() {
        let mut prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "story-1".to_string(),
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "story-2".to_string(),
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(prd.mark_story_complete("story-1"));
        assert!(prd.user_stories[0].passes);
        assert!(!prd.user_stories[1].passes);
    }

    #[test]
    fn test_mark_story_complete_not_found() {
        let mut prd = PrdDocument {
            user_stories: vec![UserStory {
                id: "story-1".to_string(),
                passes: false,
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(!prd.mark_story_complete("nonexistent"));
        assert!(!prd.user_stories[0].passes);
    }

    #[test]
    fn test_get_story() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory::new("story-1", "First"),
                UserStory::new("story-2", "Second"),
            ],
            ..Default::default()
        };

        let story = prd.get_story("story-2");
        assert!(story.is_some());
        assert_eq!(story.unwrap().title, "Second");

        assert!(prd.get_story("nonexistent").is_none());
    }

    #[test]
    fn test_get_story_mut() {
        let mut prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Original Title")],
            ..Default::default()
        };

        if let Some(story) = prd.get_story_mut("story-1") {
            story.title = "Updated Title".to_string();
        }

        assert_eq!(prd.user_stories[0].title, "Updated Title");
    }

    #[test]
    fn test_get_story_counts() {
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "done-1".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "done-2".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "pending".to_string(),
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let (completed, total) = prd.get_story_counts();
        assert_eq!(completed, 2);
        assert_eq!(total, 3);
    }

    #[test]
    fn test_get_story_counts_empty() {
        let prd = PrdDocument::default();
        let (completed, total) = prd.get_story_counts();
        assert_eq!(completed, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_with_real_prd_json_format() {
        // Test with the actual tasks.json format from the Python version
        let json = r#"{
            "project": "afk",
            "branchName": "rust-conversion",
            "description": "Tasks synced from configured sources",
            "userStories": [
                {
                    "id": "rust-001",
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
                    "id": "rust-002",
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

        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let prd = PrdDocument::from_json_value(&data);

        assert_eq!(prd.project, "afk");
        assert_eq!(prd.branch_name, "rust-conversion");
        assert_eq!(prd.user_stories.len(), 2);
        assert!(prd.user_stories[0].passes);
        assert!(!prd.user_stories[1].passes);
        assert_eq!(
            prd.user_stories[0].source,
            "json:docs/prds/rust-rewrite-tasks.json"
        );

        let pending = prd.get_pending_stories();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "rust-002");

        let (completed, total) = prd.get_story_counts();
        assert_eq!(completed, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_default_source_value() {
        // When loading from JSON without a source field
        let json = r#"{"id": "test", "title": "Test"}"#;
        let data: serde_json::Value = serde_json::from_str(json).unwrap();
        let story = UserStory::from_json_value(&data);

        // Default source should be json:.afk/tasks.json for from_json_value
        assert_eq!(story.source, "json:.afk/tasks.json");
    }
}
