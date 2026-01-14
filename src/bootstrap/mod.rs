//! Project analysis and auto-configuration.
//!
//! This module detects project type, available tools, and generates config.
//! Also handles the first-run experience for AI CLI selection.

use crate::config::{
    AfkConfig, AiCliConfig, FeedbackLoopsConfig, SourceConfig, AFK_DIR, CONFIG_FILE,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Information about an AI CLI tool.
#[derive(Debug, Clone)]
pub struct AiCliInfo {
    /// Command to run (e.g., "claude", "agent").
    pub command: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Default arguments for autonomous operation.
    pub args: &'static [&'static str],
    /// Short description.
    pub description: &'static str,
    /// URL for installation instructions.
    pub install_url: &'static str,
}

/// Known AI CLI tools with their configurations.
/// Priority order for auto-detection: claude > agent > codex > kiro > aider > amp
pub const AI_CLIS: &[AiCliInfo] = &[
    AiCliInfo {
        command: "claude",
        name: "Claude Code",
        args: &["--dangerously-skip-permissions", "-p"],
        description: "Anthropic's Claude CLI for autonomous terminal-based AI coding",
        install_url: "https://docs.anthropic.com/en/docs/claude-code",
    },
    AiCliInfo {
        command: "agent",
        name: "Cursor Agent",
        args: &["-p", "--force"],
        description: "Cursor's CLI agent for autonomous terminal-based AI coding",
        install_url: "https://docs.cursor.com/cli",
    },
    AiCliInfo {
        command: "codex",
        name: "Codex",
        args: &["--approval-mode", "full-auto", "-q"],
        description: "OpenAI's Codex CLI for terminal-based AI coding",
        install_url: "https://github.com/openai/codex",
    },
    AiCliInfo {
        command: "aider",
        name: "Aider",
        args: &["--yes", "--message"],
        description: "AI pair programming in your terminal",
        install_url: "https://aider.chat",
    },
    AiCliInfo {
        command: "amp",
        name: "Amp",
        args: &["--dangerously-allow-all"],
        description: "Sourcegraph's agentic coding tool",
        install_url: "https://sourcegraph.com/amp",
    },
    AiCliInfo {
        command: "kiro",
        name: "Kiro",
        args: &["--auto"],
        description: "Amazon's AI-powered development CLI for terminal-based coding",
        install_url: "https://kiro.dev",
    },
];

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
    AfkConfig {
        feedback_loops: analysis.suggested_feedback.clone(),
        ..AfkConfig::default()
    }
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
            return module_path.split('/').next_back().map(|s| s.to_string());
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
    #[cfg(windows)]
    {
        // On Windows, use cmd.exe /c to properly resolve .cmd/.bat extensions via PATH
        Command::new("cmd")
            .args(["/c", cmd, "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

// ============================================================================
// First-run AI CLI selection experience
// ============================================================================

/// Detect which AI CLI tools are installed on the system.
///
/// Returns a list of `AiCliInfo` for each installed AI CLI tool.
pub fn detect_available_ai_clis() -> Vec<&'static AiCliInfo> {
    AI_CLIS
        .iter()
        .filter(|cli| command_exists(cli.command))
        .collect()
}

/// Result of the AI CLI selection prompt.
#[derive(Debug, Clone)]
pub enum AiCliSelectionResult {
    /// User selected an AI CLI.
    Selected(AiCliConfig),
    /// No AI CLIs are available.
    NoneAvailable,
    /// User cancelled the selection.
    Cancelled,
}

/// Interactively prompt the user to select an AI CLI.
///
/// # Arguments
///
/// * `available` - List of available AI CLI tools
///
/// # Returns
///
/// The selected `AiCliConfig`, or an error if no selection was made.
pub fn prompt_ai_cli_selection(available: &[&AiCliInfo]) -> AiCliSelectionResult {
    if available.is_empty() {
        println!();
        println!("\x1b[31mNo AI CLI tools found.\x1b[0m");
        println!();
        println!("Install one of the following:");
        for cli in AI_CLIS {
            println!("  • \x1b[36m{}\x1b[0m: {}", cli.name, cli.install_url);
        }
        println!();
        return AiCliSelectionResult::NoneAvailable;
    }

    println!();
    println!("\x1b[1mWelcome to afk!\x1b[0m");
    println!();
    println!("Detected AI tools:");
    for (i, cli) in available.iter().enumerate() {
        println!(
            "  \x1b[36m{}\x1b[0m. {} \x1b[2m({})\x1b[0m",
            i + 1,
            cli.name,
            cli.description
        );
    }
    println!();

    // Prompt for selection
    print!(
        "Which AI CLI should afk use? [1-{}, default=1]: ",
        available.len()
    );
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return AiCliSelectionResult::Cancelled;
    }

    let input = input.trim();

    // Handle empty input (default to 1)
    let choice: usize = if input.is_empty() {
        1
    } else {
        match input.parse() {
            Ok(n) if n >= 1 && n <= available.len() => n,
            _ => {
                println!("\x1b[31mInvalid selection.\x1b[0m");
                return AiCliSelectionResult::Cancelled;
            }
        }
    };

    let selected = &available[choice - 1];
    let config = AiCliConfig {
        command: selected.command.to_string(),
        args: selected.args.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };

    AiCliSelectionResult::Selected(config)
}

/// Ensure an AI CLI is configured, prompting the user if needed.
///
/// This is the main entry point for the first-run experience.
/// If config exists with ai_cli set, returns it.
/// Otherwise, detects available CLIs and prompts user to choose.
///
/// # Arguments
///
/// * `config` - Optional existing config (loads from file if None)
///
/// # Returns
///
/// The configured `AiCliConfig`, or None if the user needs to install a CLI.
pub fn ensure_ai_cli_configured(config: Option<&mut AfkConfig>) -> Option<AiCliConfig> {
    let config_path = std::path::Path::new(CONFIG_FILE);

    // Check if config file exists - if so, use what's there
    if config_path.exists() {
        if let Some(cfg) = config {
            return Some(cfg.ai_cli.clone());
        }
        if let Ok(cfg) = AfkConfig::load(Some(config_path)) {
            return Some(cfg.ai_cli);
        }
    }

    // First run - need to prompt
    let available = detect_available_ai_clis();
    let result = prompt_ai_cli_selection(&available);

    match result {
        AiCliSelectionResult::Selected(ai_cli) => {
            // Save the selection to config
            let mut new_config = config
                .map(|c| c.clone())
                .or_else(|| AfkConfig::load(None).ok())
                .unwrap_or_default();

            new_config.ai_cli = ai_cli.clone();

            // Ensure .afk directory exists
            let afk_dir = std::path::Path::new(AFK_DIR);
            if let Err(e) = std::fs::create_dir_all(afk_dir) {
                eprintln!("\x1b[33mWarning:\x1b[0m Could not create .afk directory: {e}");
            }

            // Save config
            if let Err(e) = new_config.save(Some(config_path)) {
                eprintln!("\x1b[33mWarning:\x1b[0m Could not save config: {e}");
            } else {
                println!();
                println!(
                    "\x1b[32m✓\x1b[0m Saved AI CLI choice: \x1b[36m{}\x1b[0m",
                    ai_cli.command
                );
                println!("  Config: \x1b[2m{}\x1b[0m", CONFIG_FILE);
                println!();
            }

            Some(ai_cli)
        }
        AiCliSelectionResult::NoneAvailable | AiCliSelectionResult::Cancelled => None,
    }
}

/// Detect the best available AI CLI tool without prompting.
///
/// This is used for auto-detection when prompting is not desired.
/// Priority order: claude > agent > codex > kiro > aider > amp
pub fn detect_ai_cli() -> Option<AiCliConfig> {
    let available = detect_available_ai_clis();
    available.first().map(|cli| AiCliConfig {
        command: cli.command.to_string(),
        args: cli.args.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    })
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
        assert_eq!(config.feedback_loops.types, Some("cargo check".to_string()));
    }

    // ========================================================================
    // Tests for first-run AI CLI selection experience
    // ========================================================================

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_ai_cli_info_structure() {
        // Verify AI_CLIS has expected entries
        assert!(!AI_CLIS.is_empty());

        // First entry should be claude (highest priority)
        assert_eq!(AI_CLIS[0].command, "claude");
        assert_eq!(AI_CLIS[0].name, "Claude Code");
        assert!(!AI_CLIS[0].args.is_empty());
        assert!(!AI_CLIS[0].description.is_empty());
        assert!(!AI_CLIS[0].install_url.is_empty());
    }

    #[test]
    fn test_ai_cli_priority_order() {
        // Verify priority order: claude > agent > codex > aider > amp > kiro
        let commands: Vec<&str> = AI_CLIS.iter().map(|c| c.command).collect();
        assert_eq!(commands[0], "claude");
        assert_eq!(commands[1], "agent");
        assert_eq!(commands[2], "codex");
        // Remaining order may vary but all should be present
        assert!(commands.contains(&"aider"));
        assert!(commands.contains(&"amp"));
        assert!(commands.contains(&"kiro"));
    }

    #[test]
    fn test_ai_cli_args_are_valid() {
        // Verify each CLI has appropriate args for autonomous operation
        for cli in AI_CLIS {
            // Each CLI should have at least one arg
            // (some may have empty args if the CLI defaults to autonomous mode)
            match cli.command {
                "claude" => {
                    assert!(cli.args.contains(&"--dangerously-skip-permissions"));
                }
                "agent" => {
                    assert!(cli.args.contains(&"--force"));
                }
                "codex" => {
                    assert!(cli.args.contains(&"--approval-mode"));
                }
                "aider" => {
                    assert!(cli.args.contains(&"--yes"));
                }
                "amp" => {
                    assert!(cli.args.contains(&"--dangerously-allow-all"));
                }
                "kiro" => {
                    assert!(cli.args.contains(&"--auto"));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_detect_available_ai_clis_returns_vec() {
        // This test just verifies the function returns a valid Vec
        // It may be empty if no AI CLIs are installed
        let available = detect_available_ai_clis();
        // The result should be a subset of AI_CLIS
        for cli in &available {
            assert!(AI_CLIS.iter().any(|c| c.command == cli.command));
        }
    }

    #[test]
    fn test_ai_cli_selection_result_none_available() {
        // Test with empty available list
        let result = prompt_ai_cli_selection(&[]);
        assert!(matches!(result, AiCliSelectionResult::NoneAvailable));
    }

    #[test]
    fn test_detect_ai_cli_uses_priority() {
        // If detect_ai_cli returns Some, it should be from available CLIs
        if let Some(config) = detect_ai_cli() {
            // The command should be from our AI_CLIS list
            assert!(AI_CLIS.iter().any(|c| c.command == config.command));
            // Args should be non-empty for most CLIs
            // (detect_ai_cli uses the first available, which has priority)
        }
        // If None, that's fine too - means no AI CLIs installed
    }

    #[test]
    fn test_ai_cli_info_all_have_install_urls() {
        for cli in AI_CLIS {
            assert!(
                cli.install_url.starts_with("http"),
                "CLI {} should have a valid install URL",
                cli.command
            );
        }
    }

    #[test]
    fn test_ai_cli_info_all_have_descriptions() {
        for cli in AI_CLIS {
            assert!(
                !cli.description.is_empty(),
                "CLI {} should have a description",
                cli.command
            );
            // Descriptions should be meaningful (at least 10 chars)
            assert!(
                cli.description.len() >= 10,
                "CLI {} description should be meaningful",
                cli.command
            );
        }
    }
}
