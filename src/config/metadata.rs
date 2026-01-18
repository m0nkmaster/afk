//! Config key metadata for documentation and help.
//!
//! This module provides descriptions, types, defaults, and examples
//! for all config keys, used by `afk config explain`.

/// Metadata for a single config key.
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// The full dot-notation key (e.g., "limits.max_iterations").
    pub key: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Type description (e.g., "u32", "bool", "string").
    pub value_type: &'static str,
    /// Default value as string.
    pub default: &'static str,
    /// Example values.
    pub examples: &'static [&'static str],
}

/// All config key metadata.
pub static METADATA: &[KeyMetadata] = &[
    // limits section
    KeyMetadata {
        key: "limits.max_iterations",
        description: "Maximum number of iterations before stopping the loop. The loop will \
                      terminate after this many iterations even if tasks remain.",
        value_type: "positive integer",
        default: "200",
        examples: &["50", "100", "500"],
    },
    KeyMetadata {
        key: "limits.max_task_failures",
        description: "Maximum failures allowed per task before skipping it. When a task fails \
                      this many times, it's marked as skipped and the next task is attempted.",
        value_type: "positive integer",
        default: "50",
        examples: &["3", "10", "20"],
    },
    KeyMetadata {
        key: "limits.timeout_minutes",
        description: "Maximum time in minutes before the session times out. The loop will \
                      stop after this duration regardless of progress.",
        value_type: "positive integer",
        default: "120",
        examples: &["30", "60", "240"],
    },
    // output section
    KeyMetadata {
        key: "output.default",
        description: "Default output mode for prompts. Controls where generated prompts are sent.",
        value_type: "clipboard | file | stdout",
        default: "stdout",
        examples: &["clipboard", "file", "stdout"],
    },
    KeyMetadata {
        key: "output.file_path",
        description: "Path to write prompts when output mode is 'file'.",
        value_type: "file path",
        default: ".afk/prompt.md",
        examples: &["prompt.txt", ".afk/current-prompt.md"],
    },
    // ai_cli section
    KeyMetadata {
        key: "ai_cli.command",
        description: "The AI CLI command to execute. This is the binary name or path.",
        value_type: "string",
        default: "claude",
        examples: &["claude", "cursor", "aider", "/usr/local/bin/claude"],
    },
    KeyMetadata {
        key: "ai_cli.args",
        description: "Arguments to pass to the AI CLI. Comma-separated list of arguments.",
        value_type: "comma-separated strings",
        default: "--dangerously-skip-permissions, -p",
        examples: &["-p, --force", "--print", "-m, --no-confirm"],
    },
    KeyMetadata {
        key: "ai_cli.output_format",
        description: "Output format for AI CLI streaming. 'stream-json' provides real-time \
                      progress tracking; 'text' is plain output; 'json' is structured at completion.",
        value_type: "text | json | stream-json",
        default: "stream-json",
        examples: &["stream-json", "text", "json"],
    },
    KeyMetadata {
        key: "ai_cli.stream_partial",
        description: "Whether to include partial/character-level streaming. Enables more \
                      granular progress updates but increases output volume.",
        value_type: "bool",
        default: "false",
        examples: &["true", "false"],
    },
    // prompt section
    KeyMetadata {
        key: "prompt.template",
        description: "Template name for prompt generation. Use 'default' for the built-in \
                      template or specify a custom template name.",
        value_type: "string",
        default: "default",
        examples: &["default", "minimal", "verbose"],
    },
    KeyMetadata {
        key: "prompt.custom_path",
        description: "Path to a custom Tera template file for prompt generation. When set, \
                      this template is used instead of the built-in one.",
        value_type: "file path (optional)",
        default: "(not set)",
        examples: &[".afk/prompt.jinja2", "templates/custom.tera"],
    },
    KeyMetadata {
        key: "prompt.context_files",
        description: "Additional files to include as context in prompts. Their contents are \
                      appended to provide extra information to the AI.",
        value_type: "comma-separated file paths",
        default: "(none)",
        examples: &["AGENTS.md", "README.md, CONTRIBUTING.md", "docs/architecture.md"],
    },
    KeyMetadata {
        key: "prompt.instructions",
        description: "Additional instructions to include in every prompt. These are appended \
                      to the generated prompt as extra guidance.",
        value_type: "comma-separated strings",
        default: "(none)",
        examples: &["Always run tests", "Use British English, Follow TDD"],
    },
    KeyMetadata {
        key: "prompt.has_frontend",
        description: "Enable browser testing instructions for UI stories. When true, prompts \
                      include requirements for visual verification of frontend changes. \
                      Auto-detected during init by checking for React, Vue, Svelte, Next.js, etc.",
        value_type: "bool",
        default: "false (auto-detected during init)",
        examples: &["true", "false"],
    },
    // git section
    KeyMetadata {
        key: "git.auto_commit",
        description: "Whether to automatically commit after task completion. When true, \
                      successful task completions trigger a git commit.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "git.commit_message_template",
        description: "Template for auto-commit messages. Available placeholders: {task_id}, \
                      {message}.",
        value_type: "string with placeholders",
        default: "afk: {task_id} - {message}",
        examples: &["[{task_id}] {message}", "feat({task_id}): {message}"],
    },
    // archive section
    KeyMetadata {
        key: "archive.enabled",
        description: "Whether session archiving is enabled. When true, completed sessions \
                      can be archived for later reference.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "archive.directory",
        description: "Directory to store archived sessions.",
        value_type: "directory path",
        default: ".afk/archive",
        examples: &[".afk/archive", "archives", ".archive"],
    },
    // feedback section
    KeyMetadata {
        key: "feedback.enabled",
        description: "Whether feedback display is enabled during loop execution.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "feedback.mode",
        description: "Display mode for feedback. 'full' shows all details, 'minimal' shows \
                      progress only, 'off' disables display.",
        value_type: "full | minimal | off",
        default: "full",
        examples: &["full", "minimal", "off"],
    },
    KeyMetadata {
        key: "feedback.show_files",
        description: "Whether to show modified files in feedback display.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "feedback.show_metrics",
        description: "Whether to show metrics (tokens, duration) in feedback display.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "feedback.show_mascot",
        description: "Whether to show the ASCII mascot in feedback display.",
        value_type: "bool",
        default: "true",
        examples: &["true", "false"],
    },
    KeyMetadata {
        key: "feedback.refresh_rate",
        description: "Refresh rate for feedback display in seconds.",
        value_type: "decimal number",
        default: "0.1",
        examples: &["0.1", "0.5", "1.0"],
    },
    KeyMetadata {
        key: "feedback.max_output_lines",
        description: "Maximum number of output lines to keep in the TUI buffer. Older lines \
                      are discarded when this limit is reached.",
        value_type: "positive integer",
        default: "500",
        examples: &["250", "500", "1000"],
    },
    KeyMetadata {
        key: "feedback.active_threshold_secs",
        description: "Seconds of inactivity before transitioning from 'active' to 'thinking' \
                      state. Lower values make the UI more responsive to brief pauses.",
        value_type: "positive integer",
        default: "2",
        examples: &["1", "2", "5"],
    },
    KeyMetadata {
        key: "feedback.thinking_threshold_secs",
        description: "Seconds of inactivity before transitioning from 'thinking' to 'stalled' \
                      state. This indicates potential connection issues or long processing.",
        value_type: "positive integer",
        default: "10",
        examples: &["5", "10", "30"],
    },
    // feedback_loops section
    KeyMetadata {
        key: "feedback_loops.types",
        description: "Type checker command to run as a quality gate (e.g., mypy, tsc).",
        value_type: "shell command (optional)",
        default: "(not set)",
        examples: &["mypy .", "tsc --noEmit", "cargo check"],
    },
    KeyMetadata {
        key: "feedback_loops.lint",
        description: "Linter command to run as a quality gate.",
        value_type: "shell command (optional)",
        default: "(not set)",
        examples: &["ruff check .", "eslint .", "cargo clippy"],
    },
    KeyMetadata {
        key: "feedback_loops.test",
        description: "Test command to run as a quality gate.",
        value_type: "shell command (optional)",
        default: "(not set)",
        examples: &["pytest", "npm test", "cargo test"],
    },
    KeyMetadata {
        key: "feedback_loops.build",
        description: "Build command to run as a quality gate.",
        value_type: "shell command (optional)",
        default: "(not set)",
        examples: &["pip wheel .", "npm run build", "cargo build"],
    },
];

/// Get metadata for a specific key.
pub fn get_metadata(key: &str) -> Option<&'static KeyMetadata> {
    METADATA.iter().find(|m| m.key == key)
}

/// Get all keys in the metadata table.
pub fn all_keys() -> impl Iterator<Item = &'static str> {
    METADATA.iter().map(|m| m.key)
}

/// Search for keys matching a query (prefix or contains).
pub fn search_keys(query: &str) -> Vec<&'static KeyMetadata> {
    let query_lower = query.to_lowercase();
    METADATA
        .iter()
        .filter(|m| {
            m.key.to_lowercase().contains(&query_lower)
                || m.description.to_lowercase().contains(&query_lower)
        })
        .collect()
}

/// Get all keys for a specific section.
pub fn keys_for_section(section: &str) -> Vec<&'static KeyMetadata> {
    let prefix = format!("{section}.");
    METADATA
        .iter()
        .filter(|m| m.key.starts_with(&prefix))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_metadata_existing() {
        let meta = get_metadata("limits.max_iterations");
        assert!(meta.is_some());
        let meta = meta.unwrap();
        assert_eq!(meta.key, "limits.max_iterations");
        assert_eq!(meta.default, "200");
        assert!(meta.description.contains("iterations"));
    }

    #[test]
    fn test_get_metadata_missing() {
        let meta = get_metadata("nonexistent.key");
        assert!(meta.is_none());
    }

    #[test]
    fn test_all_keys() {
        let keys: Vec<_> = all_keys().collect();
        assert!(keys.contains(&"limits.max_iterations"));
        assert!(keys.contains(&"ai_cli.command"));
        assert!(keys.contains(&"git.auto_commit"));
    }

    #[test]
    fn test_search_keys_by_key() {
        let results = search_keys("max");
        assert!(!results.is_empty());
        assert!(results.iter().any(|m| m.key == "limits.max_iterations"));
        assert!(results.iter().any(|m| m.key == "limits.max_task_failures"));
    }

    #[test]
    fn test_search_keys_by_description() {
        let results = search_keys("timeout");
        assert!(!results.is_empty());
        assert!(results.iter().any(|m| m.key == "limits.timeout_minutes"));
    }

    #[test]
    fn test_keys_for_section() {
        let limits_keys = keys_for_section("limits");
        assert_eq!(limits_keys.len(), 3);
        assert!(limits_keys.iter().all(|m| m.key.starts_with("limits.")));

        let git_keys = keys_for_section("git");
        assert_eq!(git_keys.len(), 2);
    }

    #[test]
    fn test_all_metadata_has_required_fields() {
        for meta in METADATA {
            assert!(!meta.key.is_empty(), "Key should not be empty");
            assert!(
                !meta.description.is_empty(),
                "Description should not be empty"
            );
            assert!(
                !meta.value_type.is_empty(),
                "Value type should not be empty"
            );
            assert!(!meta.default.is_empty(), "Default should not be empty");
            assert!(!meta.examples.is_empty(), "Examples should not be empty");
        }
    }

    #[test]
    fn test_metadata_keys_are_valid_paths() {
        for meta in METADATA {
            assert!(
                meta.key.contains('.'),
                "Key '{}' should be in section.field format",
                meta.key
            );
            let parts: Vec<_> = meta.key.split('.').collect();
            assert_eq!(
                parts.len(),
                2,
                "Key '{}' should have exactly one dot",
                meta.key
            );
        }
    }
}
