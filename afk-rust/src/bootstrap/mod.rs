//! Project analysis and auto-configuration.
//!
//! This module detects project type, available tools, and generates config.

use crate::config::{AfkConfig, FeedbackLoopsConfig, SourceConfig};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Detected project type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Python,
    Node,
    Go,
    Unknown,
}

/// Result of project analysis.
#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    /// Detected project type.
    pub project_type: ProjectType,
    /// Name of the project (from Cargo.toml, pyproject.toml, package.json, etc.).
    pub name: Option<String>,
    /// Detected package manager.
    pub package_manager: Option<String>,
    /// Whether the project has tests configured.
    pub has_tests: bool,
    /// Whether the project has a linter configured.
    pub has_linter: bool,
    /// Whether the project has types (type checking) configured.
    pub has_types: bool,
    /// Suggested feedback loop commands.
    pub suggested_feedback: FeedbackLoopsConfig,
}

impl Default for ProjectAnalysis {
    fn default() -> Self {
        Self {
            project_type: ProjectType::Unknown,
            name: None,
            package_manager: None,
            has_tests: false,
            has_linter: false,
            has_types: false,
            suggested_feedback: FeedbackLoopsConfig::default(),
        }
    }
}

/// Analyse a project and detect its type and configuration.
pub fn analyse_project(root: Option<&Path>) -> ProjectAnalysis {
    let root = root.unwrap_or(Path::new("."));
    let mut analysis = ProjectAnalysis::default();

    // Detect Rust project
    if root.join("Cargo.toml").exists() {
        analysis.project_type = ProjectType::Rust;
        analysis.name = extract_cargo_name(root);
        analysis.package_manager = Some("cargo".to_string());
        analysis.has_tests = true;
        analysis.has_linter = true;
        analysis.has_types = true; // Rust has built-in types
        analysis.suggested_feedback = FeedbackLoopsConfig {
            types: Some("cargo check".to_string()),
            lint: Some("cargo clippy".to_string()),
            test: Some("cargo test".to_string()),
            build: Some("cargo build --release".to_string()),
            custom: HashMap::new(),
        };
        return analysis;
    }

    // Detect Python project
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        analysis.project_type = ProjectType::Python;
        analysis.name = extract_python_name(root);

        // Detect package manager
        if root.join("uv.lock").exists() {
            analysis.package_manager = Some("uv".to_string());
        } else if root.join("poetry.lock").exists() {
            analysis.package_manager = Some("poetry".to_string());
        } else if root.join("Pipfile.lock").exists() {
            analysis.package_manager = Some("pipenv".to_string());
        } else {
            analysis.package_manager = Some("pip".to_string());
        }

        // Check for linting tools
        let has_ruff = root.join("ruff.toml").exists()
            || root.join(".ruff.toml").exists()
            || file_contains(root.join("pyproject.toml"), "[tool.ruff]");
        
        // Check for type checking
        let has_mypy = root.join("mypy.ini").exists()
            || file_contains(root.join("pyproject.toml"), "[tool.mypy]");
        let has_pyright = root.join("pyrightconfig.json").exists()
            || file_contains(root.join("pyproject.toml"), "[tool.pyright]");

        // Check for tests
        let has_pytest = root.join("pytest.ini").exists()
            || root.join("tests").exists()
            || file_contains(root.join("pyproject.toml"), "[tool.pytest");

        analysis.has_linter = has_ruff;
        analysis.has_types = has_mypy || has_pyright;
        analysis.has_tests = has_pytest;

        analysis.suggested_feedback = FeedbackLoopsConfig {
            types: if has_mypy {
                Some("mypy .".to_string())
            } else if has_pyright {
                Some("pyright".to_string())
            } else {
                None
            },
            lint: if has_ruff {
                Some("ruff check .".to_string())
            } else {
                None
            },
            test: if has_pytest {
                Some("pytest".to_string())
            } else {
                None
            },
            build: None,
            custom: HashMap::new(),
        };
        return analysis;
    }

    // Detect Node project
    if root.join("package.json").exists() {
        analysis.project_type = ProjectType::Node;
        analysis.name = extract_node_name(root);

        // Detect package manager
        if root.join("pnpm-lock.yaml").exists() {
            analysis.package_manager = Some("pnpm".to_string());
        } else if root.join("yarn.lock").exists() {
            analysis.package_manager = Some("yarn".to_string());
        } else if root.join("bun.lockb").exists() {
            analysis.package_manager = Some("bun".to_string());
        } else {
            analysis.package_manager = Some("npm".to_string());
        }

        let is_typescript = root.join("tsconfig.json").exists();
        analysis.has_types = is_typescript;
        analysis.has_linter = root.join(".eslintrc.json").exists()
            || root.join(".eslintrc.js").exists()
            || root.join("eslint.config.js").exists();
        analysis.has_tests = root.join("jest.config.js").exists()
            || root.join("vitest.config.ts").exists()
            || root.join("vitest.config.js").exists();

        let pm = analysis.package_manager.clone().unwrap_or_default();
        let run_prefix = if pm == "npm" { "npm run" } else { &pm };

        analysis.suggested_feedback = FeedbackLoopsConfig {
            types: if is_typescript {
                Some(format!("{run_prefix} typecheck"))
            } else {
                None
            },
            lint: if analysis.has_linter {
                Some(format!("{run_prefix} lint"))
            } else {
                None
            },
            test: if analysis.has_tests {
                Some(format!("{run_prefix} test"))
            } else {
                None
            },
            build: Some(format!("{run_prefix} build")),
            custom: HashMap::new(),
        };
        return analysis;
    }

    // Detect Go project
    if root.join("go.mod").exists() {
        analysis.project_type = ProjectType::Go;
        analysis.name = extract_go_name(root);
        analysis.package_manager = Some("go".to_string());
        analysis.has_tests = true;
        analysis.has_linter = command_exists("golangci-lint");
        analysis.has_types = true; // Go is statically typed

        analysis.suggested_feedback = FeedbackLoopsConfig {
            types: Some("go build ./...".to_string()),
            lint: if analysis.has_linter {
                Some("golangci-lint run".to_string())
            } else {
                Some("go vet ./...".to_string())
            },
            test: Some("go test ./...".to_string()),
            build: Some("go build ./...".to_string()),
            custom: HashMap::new(),
        };
        return analysis;
    }

    analysis
}

/// Generate a config from project analysis.
pub fn generate_config(analysis: &ProjectAnalysis) -> AfkConfig {
    let mut config = AfkConfig::default();
    config.feedback_loops = analysis.suggested_feedback.clone();
    config
}

/// Infer sources from the current directory.
pub fn infer_sources(root: Option<&Path>) -> Vec<SourceConfig> {
    let root = root.unwrap_or(Path::new("."));
    let mut sources = Vec::new();

    // Check for TODO.md or similar
    for name in ["TODO.md", "TASKS.md", "tasks.md", "todo.md"] {
        if root.join(name).exists() {
            sources.push(SourceConfig::markdown(name));
            break;
        }
    }

    // Check for beads (.beads directory)
    if root.join(".beads").exists() && command_exists("bd") {
        sources.push(SourceConfig::beads());
    }

    // Check for GitHub issues (.github directory and gh CLI)
    if root.join(".github").exists() && command_exists("gh") {
        // Only add if we can verify gh is authenticated
        let gh_auth = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if gh_auth {
            // Use default repo detection (empty string means current repo)
            sources.push(SourceConfig::github("", Vec::new()));
        }
    }

    sources
}

/// Infer config from the current directory.
///
/// If .afk/config.json exists, loads and returns it.
/// Otherwise, analyses the project and generates a config.
pub fn infer_config(root: Option<&Path>) -> AfkConfig {
    // Try to load existing config
    let config_path = root
        .map(|p| p.join(".afk/config.json"))
        .unwrap_or_else(|| Path::new(".afk/config.json").to_path_buf());

    if let Ok(config) = AfkConfig::load(Some(config_path.as_path())) {
        // Only return if the config file actually exists
        if config_path.exists() {
            return config;
        }
    }

    // Analyse project and generate config
    let analysis = analyse_project(root);
    let mut config = generate_config(&analysis);

    // Infer sources
    config.sources = infer_sources(root);

    config
}

// Helper functions

fn extract_cargo_name(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        if line.starts_with("name") && line.contains('=') {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() >= 2 {
                return Some(parts[1].trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_python_name(root: &Path) -> Option<String> {
    if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
        for line in content.lines() {
            if line.starts_with("name") && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    return Some(parts[1].trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

fn extract_node_name(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("package.json")).ok()?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        return json.get("name")?.as_str().map(|s| s.to_string());
    }
    None
}

fn extract_go_name(root: &Path) -> Option<String> {
    let content = std::fs::read_to_string(root.join("go.mod")).ok()?;
    let first_line = content.lines().next()?;
    if first_line.starts_with("module") {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() >= 2 {
            // Get the last segment of the module path
            let module_path = parts[1];
            return module_path.split('/').last().map(|s| s.to_string());
        }
    }
    None
}

fn file_contains(path: impl AsRef<Path>, pattern: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|content| content.contains(pattern))
        .unwrap_or(false)
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SourceType;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_analyse_project_rust() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        let analysis = analyse_project(Some(temp.path()));
        assert_eq!(analysis.project_type, ProjectType::Rust);
        assert_eq!(analysis.name, Some("test-project".to_string()));
        assert_eq!(analysis.package_manager, Some("cargo".to_string()));
        assert!(analysis.suggested_feedback.test.is_some());
    }

    #[test]
    fn test_analyse_project_python() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("pyproject.toml"),
            r#"[project]
name = "myproject"

[tool.ruff]
line-length = 100

[tool.pytest.ini_options]
"#,
        )
        .unwrap();

        let analysis = analyse_project(Some(temp.path()));
        assert_eq!(analysis.project_type, ProjectType::Python);
        assert!(analysis.has_linter);
        assert!(analysis.has_tests);
    }

    #[test]
    fn test_analyse_project_node() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"name": "my-app", "version": "1.0.0"}"#,
        )
        .unwrap();
        fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();

        let analysis = analyse_project(Some(temp.path()));
        assert_eq!(analysis.project_type, ProjectType::Node);
        assert_eq!(analysis.name, Some("my-app".to_string()));
        assert!(analysis.has_types);
    }

    #[test]
    fn test_analyse_project_go() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.21\n",
        )
        .unwrap();

        let analysis = analyse_project(Some(temp.path()));
        assert_eq!(analysis.project_type, ProjectType::Go);
        assert_eq!(analysis.name, Some("myapp".to_string()));
    }

    #[test]
    fn test_analyse_project_unknown() {
        let temp = TempDir::new().unwrap();

        let analysis = analyse_project(Some(temp.path()));
        assert_eq!(analysis.project_type, ProjectType::Unknown);
        assert!(analysis.name.is_none());
    }

    #[test]
    fn test_infer_sources_markdown() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("TODO.md"), "# Tasks\n- [ ] Task 1").unwrap();

        let sources = infer_sources(Some(temp.path()));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_type, SourceType::Markdown);
    }

    #[test]
    fn test_infer_sources_empty() {
        let temp = TempDir::new().unwrap();

        let sources = infer_sources(Some(temp.path()));
        assert!(sources.is_empty());
    }

    #[test]
    fn test_infer_config_generates_new() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();

        let config = infer_config(Some(temp.path()));
        assert!(config.feedback_loops.test.is_some());
    }

    #[test]
    fn test_generate_config_from_analysis() {
        let analysis = ProjectAnalysis {
            project_type: ProjectType::Rust,
            name: Some("test".to_string()),
            package_manager: Some("cargo".to_string()),
            has_tests: true,
            has_linter: true,
            has_types: true,
            suggested_feedback: FeedbackLoopsConfig {
                types: Some("cargo check".to_string()),
                lint: Some("cargo clippy".to_string()),
                test: Some("cargo test".to_string()),
                build: None,
                custom: HashMap::new(),
            },
        };

        let config = generate_config(&analysis);
        assert_eq!(
            config.feedback_loops.types,
            Some("cargo check".to_string())
        );
    }
}
