//! Git operations.
//!
//! This module handles branching, committing, and status checks.

use std::process::Command;

/// Check if the current directory is a git repository.
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the current branch name.
pub fn get_current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() || branch == "HEAD" {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

/// Create and checkout a new branch, or checkout if it exists.
///
/// Returns true if successful.
pub fn create_branch(name: &str) -> bool {
    // First try to checkout existing branch
    let checkout = Command::new("git").args(["checkout", name]).output();

    if let Ok(output) = checkout {
        if output.status.success() {
            return true;
        }
    }

    // If checkout failed, create new branch
    let create = Command::new("git").args(["checkout", "-b", name]).output();

    create.map(|o| o.status.success()).unwrap_or(false)
}

/// Check if there are uncommitted changes.
pub fn has_uncommitted_changes() -> bool {
    let output = Command::new("git").args(["status", "--porcelain"]).output();

    match output {
        Ok(o) if o.status.success() => !o.stdout.is_empty(),
        _ => false,
    }
}

/// Get list of staged files.
pub fn get_staged_files() -> Vec<String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Stage all changes.
pub fn stage_all() -> bool {
    Command::new("git")
        .args(["add", "-A"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Commit staged changes with a message.
///
/// Returns true if successful.
pub fn commit(message: &str) -> bool {
    Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Auto-commit with a conventional commit message.
///
/// Format: `feat: [task_id] - message`
pub fn auto_commit(task_id: &str, message: &str) -> bool {
    let commit_msg = if message.is_empty() {
        format!("feat: {task_id}")
    } else {
        format!("feat: {task_id} - {message}")
    };

    // Stage all changes first
    if !stage_all() {
        return false;
    }

    // Check if there's anything to commit
    if get_staged_files().is_empty() {
        return true; // Nothing to commit is still success
    }

    commit(&commit_msg)
}

/// Get the short hash of the current commit.
pub fn get_current_commit_short() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the repository root path.
pub fn get_repo_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo_true() {
        // Current afk directory should be a git repo
        assert!(is_git_repo());
    }

    #[test]
    fn test_get_current_branch() {
        // Should return Some branch name in a git repo
        let branch = get_current_branch();
        assert!(branch.is_some());
    }

    #[test]
    fn test_get_current_branch_not_empty() {
        let branch = get_current_branch();
        if let Some(b) = branch {
            assert!(!b.is_empty());
        }
    }

    #[test]
    fn test_has_uncommitted_changes() {
        // This test just verifies the function runs without error
        let _ = has_uncommitted_changes();
    }

    #[test]
    fn test_get_staged_files() {
        // This test just verifies the function runs and returns a vec
        let files = get_staged_files();
        // Result should be a valid vec (may be empty) - just verify it's callable
        let _ = files.len();
    }

    #[test]
    fn test_get_current_commit_short() {
        let commit = get_current_commit_short();
        // Should have a commit in this repo
        assert!(commit.is_some());
        if let Some(c) = commit {
            // Short hash is typically 7 characters
            assert!(c.len() >= 7);
        }
    }

    #[test]
    fn test_get_repo_root() {
        let root = get_repo_root();
        assert!(root.is_some());
        if let Some(r) = root {
            assert!(r.contains("afk"));
        }
    }

    // Note: Tests that modify git state (create_branch, commit, etc.)
    // would need a temporary test repository to avoid affecting the real repo.
}
