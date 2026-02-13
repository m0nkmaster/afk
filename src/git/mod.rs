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

/// Get the GitHub repository in `owner/repo` format from the remote origin.
///
/// Parses common GitHub remote URL formats:
/// - `git@github.com:owner/repo.git`
/// - `https://github.com/owner/repo.git`
/// - `https://github.com/owner/repo`
///
/// # Returns
///
/// The repository in `owner/repo` format, or None if not a GitHub remote.
pub fn get_github_remote() -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_url(&url)
}

/// Parse a GitHub URL into `owner/repo` format.
///
/// Supports SSH and HTTPS formats.
fn parse_github_url(url: &str) -> Option<String> {
    // SSH format: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git");
        if repo.contains('/') && !repo.is_empty() {
            return Some(repo.to_string());
        }
    }

    // HTTPS format: https://github.com/owner/repo.git
    if url.contains("github.com/") {
        if let Some(idx) = url.find("github.com/") {
            let rest = &url[idx + 11..]; // Skip "github.com/"
            let repo = rest.trim_end_matches(".git").trim_end_matches('/');
            if repo.contains('/') && !repo.is_empty() {
                return Some(repo.to_string());
            }
        }
    }

    None
}

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub enum MergeResult {
    /// Merge completed cleanly.
    Clean,
    /// Merge had conflicts in the listed files.
    Conflict(Vec<String>),
    /// Merge failed for a non-conflict reason.
    Failed(String),
}

/// Create a git worktree at the given path on a new branch.
///
/// Equivalent to: `git worktree add <path> -b <branch> <start_point>`
///
/// # Arguments
///
/// * `path` - Directory for the new worktree
/// * `branch` - New branch name to create
/// * `start_point` - Commit/branch to start from (e.g., "HEAD")
///
/// # Returns
///
/// True if the worktree was created successfully.
pub fn create_worktree(path: &str, branch: &str, start_point: &str) -> bool {
    Command::new("git")
        .args(["worktree", "add", path, "-b", branch, start_point])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Remove a git worktree.
///
/// Equivalent to: `git worktree remove <path> --force`
///
/// # Returns
///
/// True if the worktree was removed successfully.
pub fn remove_worktree(path: &str) -> bool {
    Command::new("git")
        .args(["worktree", "remove", path, "--force"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List all git worktrees.
///
/// Returns a list of worktree paths.
pub fn list_worktrees() -> Vec<String> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

/// Merge a branch into the current branch.
///
/// Attempts a no-fast-forward merge. If there are conflicts, aborts the
/// merge and returns the list of conflicting files.
///
/// # Arguments
///
/// * `branch` - The branch to merge in
///
/// # Returns
///
/// `MergeResult::Clean` if the merge succeeded,
/// `MergeResult::Conflict` with conflicting file list if there were conflicts,
/// `MergeResult::Failed` if the merge failed for another reason.
pub fn merge_branch(branch: &str) -> MergeResult {
    let output = Command::new("git")
        .args(["merge", "--no-ff", branch, "-m", &format!("Merge {branch}")])
        .output();

    match output {
        Ok(o) if o.status.success() => MergeResult::Clean,
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();

            // Check for merge conflicts
            if stderr.contains("CONFLICT") || stderr.contains("Automatic merge failed") {
                // Get list of conflicting files
                let conflicts = get_conflict_files();

                // Abort the merge to leave working directory clean
                let _ = Command::new("git").args(["merge", "--abort"]).output();

                if conflicts.is_empty() {
                    MergeResult::Conflict(vec!["(unknown files)".to_string()])
                } else {
                    MergeResult::Conflict(conflicts)
                }
            } else {
                MergeResult::Failed(stderr)
            }
        }
        Err(e) => MergeResult::Failed(e.to_string()),
    }
}

/// Get list of files with merge conflicts.
fn get_conflict_files() -> Vec<String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
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

/// Delete a local branch.
///
/// Uses `-D` (force delete) since worktree branches may not be fully merged.
pub fn delete_branch(branch: &str) -> bool {
    Command::new("git")
        .args(["branch", "-D", branch])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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

    #[test]
    fn test_parse_github_url_ssh() {
        assert_eq!(
            parse_github_url("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            parse_github_url("git@github.com:some-user/my-project.git"),
            Some("some-user/my-project".to_string())
        );
        assert_eq!(
            parse_github_url("git@github.com:owner/repo"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn test_parse_github_url_https() {
        assert_eq!(
            parse_github_url("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            parse_github_url("https://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            parse_github_url("https://github.com/some-user/my-project.git"),
            Some("some-user/my-project".to_string())
        );
        assert_eq!(
            parse_github_url("https://github.com/owner/repo/"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn test_parse_github_url_invalid() {
        // Not GitHub URLs
        assert!(parse_github_url("git@gitlab.com:owner/repo.git").is_none());
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_none());
        assert!(parse_github_url("not a url").is_none());
        assert!(parse_github_url("").is_none());
    }

    #[test]
    fn test_get_github_remote_returns_result() {
        // This test just verifies the function runs
        // In the afk repo, it should return Some value with the repo
        let remote = get_github_remote();
        // If running in the afk repo, we should get a result
        // (may be None in CI environments without git remote)
        if let Some(r) = remote {
            assert!(r.contains('/'), "Remote should be in owner/repo format");
        }
    }

    // Note: Tests that modify git state (create_branch, commit, etc.)
    // would need a temporary test repository to avoid affecting the real repo.
}
