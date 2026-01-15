//! GitHub Issues task source adapter.
//!
//! Uses the `gh` CLI to fetch issues and convert them to UserStory.

use crate::prd::UserStory;
use serde::Deserialize;
use std::process::Command;

/// A GitHub issue as returned by `gh issue list --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct GhIssue {
    /// Issue number.
    pub number: i64,
    /// Issue title.
    pub title: String,
    /// Issue body/description (optional).
    #[serde(default)]
    pub body: Option<String>,
    /// Labels attached to the issue.
    #[serde(default)]
    pub labels: Vec<GhLabel>,
    /// Issue state (open/closed).
    #[serde(default)]
    pub state: String,
}

/// A GitHub label.
#[derive(Debug, Clone, Deserialize)]
pub struct GhLabel {
    /// Label name.
    pub name: String,
}

/// Load tasks from GitHub Issues via the gh CLI.
///
/// # Arguments
///
/// * `repo` - Optional repository in "owner/repo" format. If empty, uses current repo.
/// * `labels` - Optional list of labels to filter by.
///
/// # Returns
///
/// Vector of UserStory items converted from GitHub issues.
pub fn load_github_tasks(repo: Option<&str>, labels: &[String]) -> Vec<UserStory> {
    // Check if gh is available
    if !gh_available() {
        eprintln!("Warning: gh CLI not available. Skipping GitHub source.");
        return Vec::new();
    }

    // Build command
    let mut args = vec![
        "issue",
        "list",
        "--state",
        "open",
        "--json",
        "number,title,body,labels,state",
    ];

    // Add repo if specified
    let repo_arg;
    if let Some(r) = repo {
        if !r.is_empty() {
            repo_arg = format!("--repo={r}");
            args.push(&repo_arg);
        }
    }

    // Add label filter if specified
    let label_args: Vec<String> = labels.iter().map(|l| format!("--label={l}")).collect();
    for arg in &label_args {
        args.push(arg);
    }

    // Run gh command
    let output = match Command::new("gh").args(&args).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Warning: Failed to run gh: {e}");
            return Vec::new();
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: gh command failed: {stderr}");
        return Vec::new();
    }

    // Parse JSON output
    let json_str = String::from_utf8_lossy(&output.stdout);
    let issues: Vec<GhIssue> = match serde_json::from_str(&json_str) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Warning: Failed to parse gh output: {e}");
            return Vec::new();
        }
    };

    // Convert to UserStory
    issues.into_iter().map(issue_to_story).collect()
}

/// Convert a GitHub issue to a UserStory.
fn issue_to_story(issue: GhIssue) -> UserStory {
    let id = format!("gh-{}", issue.number);
    let priority = infer_priority(&issue.labels);
    let acceptance_criteria = extract_acceptance_criteria(issue.body.as_deref());

    UserStory {
        id,
        title: issue.title,
        description: issue.body.clone().unwrap_or_default(),
        acceptance_criteria,
        priority,
        passes: false,
        source: format!("github:#{}", issue.number),
        notes: String::new(),
    }
}

/// Infer priority from GitHub labels.
fn infer_priority(labels: &[GhLabel]) -> i32 {
    for label in labels {
        let name = label.name.to_lowercase();
        if name.contains("p0") || name.contains("critical") || name.contains("urgent") {
            return 0;
        }
        if name.contains("p1") || name.contains("high") {
            return 1;
        }
        if name.contains("p2") || name.contains("medium") {
            return 2;
        }
        if name.contains("p3") || name.contains("low") {
            return 3;
        }
    }
    // Default priority
    2
}

/// Extract acceptance criteria from issue body.
fn extract_acceptance_criteria(body: Option<&str>) -> Vec<String> {
    let body = match body {
        Some(b) => b,
        None => return Vec::new(),
    };

    let mut criteria = Vec::new();
    let mut in_ac_section = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // Check for AC section header
        if trimmed.to_lowercase().contains("acceptance criteria")
            || trimmed.to_lowercase().contains("ac:")
            || trimmed.to_lowercase().contains("requirements:")
        {
            in_ac_section = true;
            continue;
        }

        // Check for end of section (next heading)
        if in_ac_section && trimmed.starts_with('#') {
            in_ac_section = false;
            continue;
        }

        // Extract list items
        if in_ac_section {
            if let Some(item) = extract_list_item(trimmed) {
                criteria.push(item);
            }
        }

        // Also extract checkboxes anywhere in body
        if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
            let item = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string();
            if !item.is_empty() && !criteria.contains(&item) {
                criteria.push(item);
            }
        }
    }

    criteria
}

/// Extract a list item from a line.
fn extract_list_item(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Bullet list
    if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
        let item = trimmed[1..].trim().to_string();
        if !item.is_empty() {
            return Some(item);
        }
    }

    // Numbered list
    if let Some(idx) = trimmed.find('.') {
        if trimmed[..idx].chars().all(|c| c.is_ascii_digit()) {
            let item = trimmed[idx + 1..].trim().to_string();
            if !item.is_empty() {
                return Some(item);
            }
        }
    }

    None
}

/// Check if gh CLI is available.
fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_priority_p0() {
        let labels = vec![GhLabel {
            name: "P0".to_string(),
        }];
        assert_eq!(infer_priority(&labels), 0);
    }

    #[test]
    fn test_infer_priority_critical() {
        let labels = vec![GhLabel {
            name: "critical".to_string(),
        }];
        assert_eq!(infer_priority(&labels), 0);
    }

    #[test]
    fn test_infer_priority_high() {
        let labels = vec![GhLabel {
            name: "high-priority".to_string(),
        }];
        assert_eq!(infer_priority(&labels), 1);
    }

    #[test]
    fn test_infer_priority_default() {
        let labels = vec![GhLabel {
            name: "bug".to_string(),
        }];
        assert_eq!(infer_priority(&labels), 2);
    }

    #[test]
    fn test_infer_priority_empty() {
        let labels: Vec<GhLabel> = vec![];
        assert_eq!(infer_priority(&labels), 2);
    }

    #[test]
    fn test_extract_acceptance_criteria_checkboxes() {
        let body = r#"
## Description
Something

## Requirements
- [ ] First requirement
- [x] Second requirement (done)
- [ ] Third requirement
"#;
        let criteria = extract_acceptance_criteria(Some(body));
        assert_eq!(criteria.len(), 3);
        assert!(criteria.contains(&"First requirement".to_string()));
        assert!(criteria.contains(&"Second requirement (done)".to_string()));
    }

    #[test]
    fn test_extract_acceptance_criteria_section() {
        let body = r#"
## Acceptance Criteria
- First item
- Second item

## Notes
Something else
"#;
        let criteria = extract_acceptance_criteria(Some(body));
        assert_eq!(criteria.len(), 2);
        assert!(criteria.contains(&"First item".to_string()));
        assert!(criteria.contains(&"Second item".to_string()));
    }

    #[test]
    fn test_extract_acceptance_criteria_empty() {
        assert!(extract_acceptance_criteria(None).is_empty());
        assert!(extract_acceptance_criteria(Some("")).is_empty());
    }

    #[test]
    fn test_extract_list_item_bullet() {
        assert_eq!(extract_list_item("- item"), Some("item".to_string()));
        assert_eq!(extract_list_item("* item"), Some("item".to_string()));
        assert_eq!(extract_list_item("+ item"), Some("item".to_string()));
    }

    #[test]
    fn test_extract_list_item_numbered() {
        assert_eq!(extract_list_item("1. item"), Some("item".to_string()));
        assert_eq!(extract_list_item("10. item"), Some("item".to_string()));
    }

    #[test]
    fn test_extract_list_item_not_list() {
        assert!(extract_list_item("regular text").is_none());
        assert!(extract_list_item("").is_none());
    }

    #[test]
    fn test_issue_to_story() {
        let issue = GhIssue {
            number: 42,
            title: "Fix the bug".to_string(),
            body: Some("Description\n- [ ] Fix it".to_string()),
            labels: vec![GhLabel {
                name: "P1".to_string(),
            }],
            state: "open".to_string(),
        };

        let story = issue_to_story(issue);
        assert_eq!(story.id, "gh-42");
        assert_eq!(story.title, "Fix the bug");
        assert_eq!(story.priority, 1);
        assert_eq!(story.source, "github:#42");
        assert!(!story.passes);
    }

    #[test]
    fn test_gh_issue_deserialization() {
        let json = r#"{
            "number": 123,
            "title": "Test issue",
            "body": "Body text",
            "labels": [{"name": "bug"}],
            "state": "open"
        }"#;

        let issue: GhIssue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 123);
        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.labels.len(), 1);
    }

    #[test]
    fn test_gh_issue_deserialization_minimal() {
        let json = r#"{
            "number": 1,
            "title": "Minimal"
        }"#;

        let issue: GhIssue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 1);
        assert!(issue.body.is_none());
        assert!(issue.labels.is_empty());
    }
}
