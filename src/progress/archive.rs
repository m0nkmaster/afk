//! Session archiving functionality.
//!
//! This module handles archiving and clearing afk sessions,
//! including moving session files to timestamped archive directories.

use crate::config::{ARCHIVE_DIR, PROGRESS_FILE, TASKS_FILE};
use crate::git::get_current_branch;
use crate::progress::{ProgressError, SessionProgress};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for an archived session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// ISO timestamp when the session was archived.
    pub archived_at: String,
    /// Branch name at time of archiving.
    pub branch: Option<String>,
    /// Reason for archiving (e.g., "manual", "branch_change", "session_complete").
    pub reason: String,
    /// Number of iterations completed in the session.
    pub iterations: u32,
    /// Number of tasks completed.
    pub tasks_completed: usize,
    /// Number of tasks pending.
    pub tasks_pending: usize,
}

/// Archive and clear the current session.
///
/// Moves the current session files (progress.json and tasks.json) to an archive
/// directory with a timestamp. The session is cleared, ready for fresh work.
///
/// # Arguments
///
/// * `reason` - Reason for archiving (e.g., "manual", "completed")
///
/// # Returns
///
/// The path to the archive directory, or None if there's nothing to archive.
pub fn archive_session(reason: &str) -> Result<Option<PathBuf>, ProgressError> {
    let progress_path = Path::new(PROGRESS_FILE);
    let tasks_path = Path::new(TASKS_FILE);

    // Need at least one file to archive
    if !progress_path.exists() && !tasks_path.exists() {
        return Ok(None);
    }

    // Load progress to get stats (if it exists)
    let progress = if progress_path.exists() {
        Some(SessionProgress::load(None)?)
    } else {
        None
    };

    // Create archive directory name with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let archive_name = timestamp;
    let archive_dir = Path::new(ARCHIVE_DIR).join(&archive_name);

    // Create archive directory
    fs::create_dir_all(&archive_dir)?;

    // Move progress.json to archive (if it exists)
    if progress_path.exists() {
        let archive_progress = archive_dir.join("progress.json");
        fs::rename(progress_path, &archive_progress)?;
    }

    // Move tasks.json to archive (if it exists)
    if tasks_path.exists() {
        let archive_tasks = archive_dir.join("tasks.json");
        fs::rename(tasks_path, &archive_tasks)?;
    }

    // Write metadata
    let (pending, completed, iterations, branch) = if let Some(ref p) = progress {
        let (pend, _, comp, _, _) = p.get_task_counts();
        // Use stored branch if available, otherwise try to get current branch
        let branch = p.last_branch.clone().or_else(get_current_branch);
        (pend, comp, p.iterations, branch)
    } else {
        (0, 0, 0, get_current_branch())
    };

    let metadata = ArchiveMetadata {
        archived_at: Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
        branch,
        reason: reason.to_string(),
        iterations,
        tasks_completed: completed,
        tasks_pending: pending,
    };
    let metadata_path = archive_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, metadata_json)?;

    Ok(Some(archive_dir))
}

/// Clear the current session (delete progress.json).
pub fn clear_session() -> Result<(), ProgressError> {
    let progress_path = Path::new(PROGRESS_FILE);
    if progress_path.exists() {
        fs::remove_file(progress_path)?;
    }
    Ok(())
}

/// List archived sessions.
///
/// Returns a list of (archive_name, metadata) pairs, sorted by date (newest first).
pub fn list_archives() -> Result<Vec<(String, ArchiveMetadata)>, ProgressError> {
    let archive_dir = Path::new(ARCHIVE_DIR);
    if !archive_dir.exists() {
        return Ok(Vec::new());
    }

    let mut archives = Vec::new();

    for entry in fs::read_dir(archive_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let metadata_path = path.join("metadata.json");
            if metadata_path.exists() {
                if let Ok(contents) = fs::read_to_string(&metadata_path) {
                    if let Ok(metadata) = serde_json::from_str::<ArchiveMetadata>(&contents) {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        archives.push((name, metadata));
                    }
                }
            }
        }
    }

    // Sort by archived_at descending (newest first)
    archives.sort_by(|a, b| b.1.archived_at.cmp(&a.1.archived_at));

    Ok(archives)
}

/// Result of branch change detection.
#[derive(Debug, Clone)]
pub struct BranchChangeInfo {
    /// The branch stored in the previous session.
    pub previous_branch: String,
    /// The current git branch.
    pub current_branch: String,
}

/// Check if the git branch has changed since the last session.
///
/// Returns `Some(BranchChangeInfo)` if a branch change is detected,
/// `None` if no change or if there's no previous session/branch info.
pub fn check_branch_change() -> Option<BranchChangeInfo> {
    // Load existing progress
    let progress = SessionProgress::load(None).ok()?;

    // Get stored branch from previous session
    let previous_branch = progress.last_branch.as_ref()?;

    // Get current git branch
    let current_branch = get_current_branch()?;

    // Check if they differ
    if previous_branch != &current_branch {
        Some(BranchChangeInfo {
            previous_branch: previous_branch.clone(),
            current_branch,
        })
    } else {
        None
    }
}

/// Update the stored branch in the progress file.
///
/// This should be called after the user decides not to archive,
/// or at the start of a new session.
pub fn update_stored_branch() -> Result<(), ProgressError> {
    let mut progress = SessionProgress::load(None)?;
    progress.set_branch(get_current_branch());
    progress.save(None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_archive_metadata_serialisation() {
        let metadata = ArchiveMetadata {
            archived_at: "2024-01-15T10:30:00.000000".to_string(),
            branch: Some("feature/add-dark-mode".to_string()),
            reason: "completed".to_string(),
            iterations: 10,
            tasks_completed: 5,
            tasks_pending: 3,
        };

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        assert!(json.contains("\"archived_at\""));
        assert!(json.contains("\"reason\""));
        assert!(json.contains("\"iterations\""));
        assert!(json.contains("\"tasks_completed\""));
        assert!(json.contains("\"tasks_pending\""));

        // Round-trip
        let parsed: ArchiveMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.archived_at, metadata.archived_at);
        assert_eq!(parsed.branch, Some("feature/add-dark-mode".to_string()));
        assert_eq!(parsed.reason, metadata.reason);
        assert_eq!(parsed.iterations, metadata.iterations);
        assert_eq!(parsed.tasks_completed, metadata.tasks_completed);
        assert_eq!(parsed.tasks_pending, metadata.tasks_pending);
    }

    #[test]
    fn test_archive_metadata_with_all_complete() {
        let metadata = ArchiveMetadata {
            archived_at: "2024-01-15T10:30:00.000000".to_string(),
            branch: None,
            reason: "completed".to_string(),
            iterations: 5,
            tasks_completed: 5,
            tasks_pending: 0,
        };

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        let parsed: ArchiveMetadata = serde_json::from_str(&json).unwrap();
        assert!(parsed.branch.is_none());
        assert_eq!(parsed.reason, "completed");
        assert_eq!(parsed.tasks_pending, 0);
    }

    #[test]
    fn test_clear_session_nonexistent() {
        // Should not error when progress.json doesn't exist
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // This should succeed silently
        let result = clear_session();
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_archives_empty() {
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let archives = list_archives().unwrap();
        assert!(archives.is_empty());
    }

    #[test]
    fn test_branch_change_info_struct() {
        let info = BranchChangeInfo {
            previous_branch: "main".to_string(),
            current_branch: "feature/new-feature".to_string(),
        };

        assert_eq!(info.previous_branch, "main");
        assert_eq!(info.current_branch, "feature/new-feature");
    }

    #[test]
    fn test_check_branch_change_no_progress_file() {
        // With no progress file, check_branch_change should return None
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = check_branch_change();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_branch_change_no_stored_branch() {
        // With progress file but no stored branch, should return None
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Create progress file without last_branch
        let afk_dir = temp.path().join(".afk");
        std::fs::create_dir_all(&afk_dir).unwrap();
        let progress = SessionProgress::new();
        progress.save(None).unwrap();

        let result = check_branch_change();
        assert!(result.is_none());
    }

    #[test]
    fn test_archive_metadata_with_branch() {
        let metadata = ArchiveMetadata {
            archived_at: "2024-01-15T10:30:00.000000".to_string(),
            branch: Some("feature/branch-archiving".to_string()),
            reason: "branch_change".to_string(),
            iterations: 3,
            tasks_completed: 2,
            tasks_pending: 1,
        };

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        assert!(json.contains("\"branch\": \"feature/branch-archiving\""));
        assert!(json.contains("\"reason\": \"branch_change\""));

        let parsed: ArchiveMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.branch, Some("feature/branch-archiving".to_string()));
        assert_eq!(parsed.reason, "branch_change");
    }
}
