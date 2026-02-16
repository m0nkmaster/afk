//! Gherkin/BDD feature file task source adapter.
//!
//! Loads job stories from .feature files (Given/When/Then). Each Scenario or
//! Scenario Outline becomes one UserStory with steps as acceptance_criteria.

use crate::prd::UserStory;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// Default paths to check when no path is specified.
const DEFAULT_PATHS: &[&str] = &["features", "spec/features", "tests/features", "acceptance"];

/// Regex for Feature: line.
static FEATURE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*Feature\s*:\s*(.+)$").expect("FEATURE_RE regex is valid"));

/// Regex for Scenario:, Scenario Outline:, or Scenario Template: line.
static SCENARIO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*Scenario(?:\s+(?:Outline|Template))?\s*:\s*(.+)$")
        .expect("SCENARIO_RE regex is valid")
});

/// Regex for step lines (Given, When, Then, And, But) with optional colon.
static STEP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(Given|When|Then|And|But)\s+(.+)$").expect("STEP_RE regex is valid")
});

/// Load tasks from Gherkin .feature file(s).
///
/// If path is None, tries default directories (features/, spec/features/, etc.).
/// If path is a file, parses that file. If path is a directory, collects all
/// .feature files recursively and parses them.
///
/// Each Scenario or Scenario Outline becomes one UserStory with:
/// - id: slug from feature + scenario name (unique)
/// - title: scenario name
/// - description: full scenario text (feature + scenario + steps)
/// - acceptance_criteria: list of step strings (Given/When/Then/And/But)
/// - source: gherkin:<path>
///
/// # Returns
///
/// A vector of UserStory items. Returns empty on read/parse errors (graceful).
pub fn load_gherkin_tasks(path: Option<&str>) -> Vec<UserStory> {
    let paths_to_parse = match path {
        Some(p) => {
            let path = Path::new(p);
            if !path.exists() {
                return Vec::new();
            }
            if path.is_file() {
                if path.extension().is_some_and(|e| e == "feature") {
                    vec![path.to_path_buf()]
                } else {
                    // Not a .feature file; do not attempt to parse.
                    Vec::new()
                }
            } else {
                collect_feature_files(path)
            }
        }
        None => {
            let found = DEFAULT_PATHS
                .iter()
                .map(Path::new)
                .find(|p| p.exists() && p.is_dir());
            match found {
                Some(dir) => collect_feature_files(dir),
                None => return Vec::new(),
            }
        }
    };

    let mut all_stories = Vec::new();
    let mut slug_counts: HashMap<String, u32> = HashMap::new();

    for file_path in paths_to_parse {
        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let source_str = format!("gherkin:{}", file_path.display());
        let stories = parse_feature_content(&contents, &source_str);

        for mut story in stories {
            let base_slug = story.id.clone();
            let count = slug_counts.entry(base_slug.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                story.id = format!("{}-{}", base_slug, *count - 1);
            }
            all_stories.push(story);
        }
    }

    all_stories
}

/// Collect all .feature files under a directory recursively.
///
/// Does not follow symbolic links when recursing (uses `entry.file_type().is_dir()`)
/// to avoid infinite loops on symlink cycles.
fn collect_feature_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    out.extend(collect_feature_files(&path));
                } else if path.extension().is_some_and(|e| e == "feature") {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

/// Parse feature file content into UserStory list.
fn parse_feature_content(content: &str, source: &str) -> Vec<UserStory> {
    let mut stories = Vec::new();
    let mut feature_name = String::new();
    let mut scenario_name = String::new();
    let mut scenario_lines = Vec::new();
    let mut steps = Vec::new();
    let mut in_scenario = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines for keyword detection
        if trimmed.starts_with('#') || trimmed.is_empty() {
            if in_scenario {
                scenario_lines.push(line.to_string());
            }
            continue;
        }

        // Doc string start/end: skip but keep in scenario text if desired
        if trimmed.starts_with("\"\"\"") {
            if in_scenario {
                scenario_lines.push(line.to_string());
            }
            continue;
        }

        if let Some(cap) = FEATURE_RE.captures(line) {
            if in_scenario {
                flush_scenario(
                    &feature_name,
                    &scenario_name,
                    &scenario_lines,
                    &steps,
                    source,
                    &mut stories,
                );
                scenario_name.clear();
                scenario_lines.clear();
                steps.clear();
                in_scenario = false;
            }
            feature_name = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            scenario_lines.push(line.to_string());
            continue;
        }

        if let Some(cap) = SCENARIO_RE.captures(line) {
            if in_scenario {
                flush_scenario(
                    &feature_name,
                    &scenario_name,
                    &scenario_lines,
                    &steps,
                    source,
                    &mut stories,
                );
                scenario_name.clear();
                scenario_lines.clear();
                steps.clear();
            }
            scenario_name = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            scenario_lines = vec![line.to_string()];
            steps = Vec::new();
            in_scenario = true;
            continue;
        }

        if in_scenario {
            if let Some(cap) = STEP_RE.captures(line) {
                let keyword = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let step_text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let full_step = format!("{} {}", keyword, step_text);
                steps.push(full_step);
            }
            scenario_lines.push(line.to_string());
        }
    }

    if in_scenario {
        flush_scenario(
            &feature_name,
            &scenario_name,
            &scenario_lines,
            &steps,
            source,
            &mut stories,
        );
    }

    stories
}

/// Emit one UserStory for the current scenario.
fn flush_scenario(
    feature_name: &str,
    scenario_name: &str,
    scenario_lines: &[String],
    steps: &[String],
    source: &str,
    stories: &mut Vec<UserStory>,
) {
    if scenario_name.is_empty() {
        return;
    }

    let id = make_slug(feature_name, scenario_name);
    let description = if !feature_name.is_empty() {
        format!("Feature: {}\n\n{}", feature_name, scenario_lines.join("\n"))
    } else {
        scenario_lines.join("\n")
    };
    let title = scenario_name.to_string();

    stories.push(UserStory {
        id,
        title,
        description,
        acceptance_criteria: steps.to_vec(),
        priority: 3,
        passes: false,
        source: source.to_string(),
        notes: String::new(),
    });
}

/// Build a slug from feature and scenario names.
fn make_slug(feature: &str, scenario: &str) -> String {
    let feat = slug_part(feature);
    let scen = slug_part(scenario);
    if feat.is_empty() {
        scen
    } else if scen.is_empty() {
        feat
    } else {
        format!("{}-{}", feat, scen)
    }
}

fn slug_part(s: &str) -> String {
    let clean: String = s
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect();
    clean
        .split_whitespace()
        .map(|w| w.trim_matches('-'))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_feature_single_scenario() {
        let content = r#"
Feature: User login

  As a user I want to log in so that I can access the app.

  Scenario: User logs in with valid credentials
    Given the user is on the login page
    When the user enters valid credentials
    Then the user is redirected to the dashboard
"#;
        let stories = parse_feature_content(content, "gherkin:test.feature");
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].title, "User logs in with valid credentials");
        assert_eq!(
            stories[0].id,
            "user-login-user-logs-in-with-valid-credentials"
        );
        assert_eq!(stories[0].acceptance_criteria.len(), 3);
        assert!(stories[0].acceptance_criteria[0].starts_with("Given"));
        assert!(stories[0].acceptance_criteria[1].starts_with("When"));
        assert!(stories[0].acceptance_criteria[2].starts_with("Then"));
        assert_eq!(stories[0].source, "gherkin:test.feature");
    }

    #[test]
    fn test_parse_feature_multiple_scenarios() {
        let content = r#"
Feature: Checkout

  Scenario: Guest checkout
    Given the user has items in the cart
    When the user proceeds to checkout
    Then the user can enter shipping details

  Scenario: Logged-in user checkout
    Given the user is logged in
    And the user has items in the cart
    When the user proceeds to checkout
    Then the saved address is pre-filled
"#;
        let stories = parse_feature_content(content, "gherkin:checkout.feature");
        assert_eq!(stories.len(), 2);
        assert_eq!(stories[0].title, "Guest checkout");
        assert_eq!(stories[0].acceptance_criteria.len(), 3);
        assert_eq!(stories[1].title, "Logged-in user checkout");
        assert_eq!(stories[1].acceptance_criteria.len(), 4);
        assert!(stories[1].acceptance_criteria[1].starts_with("And"));
    }

    #[test]
    fn test_parse_scenario_outline() {
        let content = r#"
Feature: Search

  Scenario Outline: Search by keyword
    Given the user is on the search page
    When the user searches for "<keyword>"
    Then results contain "<keyword>"

    Examples:
      | keyword |
      | foo     |
      | bar     |
"#;
        let stories = parse_feature_content(content, "gherkin:search.feature");
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].title, "Search by keyword");
        assert!(stories[0].acceptance_criteria.len() >= 3);
    }

    #[test]
    fn test_parse_scenario_template() {
        let content = r#"
Feature: Search

  Scenario Template: Search by keyword
    Given the user is on the search page
    When the user searches for "<keyword>"
    Then results contain "<keyword>"

    Examples:
      | keyword |
      | foo     |
      | bar     |
"#;
        let stories = parse_feature_content(content, "gherkin:search.feature");
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].title, "Search by keyword");
        assert!(stories[0].acceptance_criteria.len() >= 3);
    }

    #[test]
    fn test_load_gherkin_tasks_from_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("login.feature");
        let content = r#"
Feature: Login

  Scenario: Successful login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
"#;
        fs::write(&path, content).unwrap();

        let tasks = load_gherkin_tasks(Some(path.to_str().unwrap()));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Successful login");
        assert_eq!(tasks[0].acceptance_criteria.len(), 3);
    }

    #[test]
    fn test_load_gherkin_tasks_missing_path_returns_empty() {
        let tasks = load_gherkin_tasks(Some("/nonexistent/path.feature"));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_make_slug() {
        assert_eq!(
            make_slug("User login", "Valid credentials"),
            "user-login-valid-credentials"
        );
        assert_eq!(make_slug("", "Only scenario"), "only-scenario");
    }
}
