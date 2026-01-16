//! Configuration models for .afk/config.json.
//!
//! This module contains Serde models for the afk configuration,
//! mirroring the Python Pydantic models in src/afk/config.py.

pub mod field;
pub mod metadata;
mod validation;

use serde::{Deserialize, Serialize};

pub use field::{ConfigField, FieldError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Default config directory path.
pub const AFK_DIR: &str = ".afk";
/// Default config file path.
pub const CONFIG_FILE: &str = ".afk/config.json";
/// Default progress file path.
pub const PROGRESS_FILE: &str = ".afk/progress.json";
/// Default tasks file path.
pub const TASKS_FILE: &str = ".afk/tasks.json";
/// Default archive directory path.
pub const ARCHIVE_DIR: &str = ".afk/archive";

/// Source types supported by afk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// Beads (bd) issue tracker integration.
    Beads,
    /// JSON file containing tasks.
    Json,
    /// Markdown file with checklist items.
    Markdown,
    /// GitHub issues via gh CLI.
    Github,
}

/// Configuration for a task source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Type of source (beads, json, markdown, github).
    #[serde(rename = "type")]
    pub source_type: SourceType,
    /// Path to source file (for json/markdown sources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// GitHub-specific: repository in "owner/repo" format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// GitHub-specific: labels to filter issues.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

impl SourceConfig {
    /// Create a new beads source.
    pub fn beads() -> Self {
        Self {
            source_type: SourceType::Beads,
            path: None,
            repo: None,
            labels: Vec::new(),
        }
    }

    /// Create a new JSON source with a path.
    pub fn json(path: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::Json,
            path: Some(path.into()),
            repo: None,
            labels: Vec::new(),
        }
    }

    /// Create a new markdown source with a path.
    pub fn markdown(path: impl Into<String>) -> Self {
        Self {
            source_type: SourceType::Markdown,
            path: Some(path.into()),
            repo: None,
            labels: Vec::new(),
        }
    }

    /// Create a new GitHub source.
    pub fn github(repo: impl Into<String>, labels: Vec<String>) -> Self {
        Self {
            source_type: SourceType::Github,
            path: None,
            repo: Some(repo.into()),
            labels,
        }
    }
}

/// Configuration for feedback loop commands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackLoopsConfig {
    /// Type checker command (e.g., "mypy .").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,
    /// Linter command (e.g., "ruff check .").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<String>,
    /// Test command (e.g., "pytest").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// Build command (e.g., "pip wheel .").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// Custom commands with name => command mapping.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

/// Configuration for iteration limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Maximum number of iterations before stopping.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Maximum failures per task before skipping.
    #[serde(default = "default_max_task_failures")]
    pub max_task_failures: u32,
    /// Maximum time in minutes before timeout.
    #[serde(default = "default_timeout_minutes")]
    pub timeout_minutes: u32,
}

fn default_max_iterations() -> u32 {
    200
}

fn default_max_task_failures() -> u32 {
    50
}

fn default_timeout_minutes() -> u32 {
    120
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            max_task_failures: default_max_task_failures(),
            timeout_minutes: default_timeout_minutes(),
        }
    }
}

/// Output mode for prompts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Copy prompt to clipboard.
    Clipboard,
    /// Write prompt to file.
    File,
    /// Print prompt to stdout (default).
    #[default]
    Stdout,
}

/// Configuration for output modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Default output mode.
    #[serde(default)]
    pub default: OutputMode,
    /// Path for file output.
    #[serde(default = "default_file_path")]
    pub file_path: String,
}

fn default_file_path() -> String {
    ".afk/prompt.md".to_string()
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default: OutputMode::default(),
            file_path: default_file_path(),
        }
    }
}

/// Output format for AI CLI streaming.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiOutputFormat {
    /// Plain text output (default for backwards compatibility).
    Text,
    /// JSON output at completion.
    Json,
    /// NDJSON streaming output (recommended for real-time feedback).
    #[default]
    StreamJson,
}

/// Configuration for AI CLI integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiCliConfig {
    /// CLI command to run (e.g., "claude", "cursor", "aider").
    #[serde(default = "default_ai_command")]
    pub command: String,
    /// Arguments to pass to the CLI.
    #[serde(default = "default_ai_args")]
    pub args: Vec<String>,
    /// Output format for streaming (stream-json recommended).
    #[serde(default)]
    pub output_format: AiOutputFormat,
    /// Whether to include partial/character-level streaming.
    #[serde(default)]
    pub stream_partial: bool,
    /// List of models to rotate between. When multiple models are configured,
    /// one is selected pseudo-randomly with equal distribution each iteration.
    /// This brings different perspectives and avoids local optima.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

fn default_ai_command() -> String {
    "claude".to_string()
}

fn default_ai_args() -> Vec<String> {
    vec![
        "--dangerously-skip-permissions".to_string(),
        "-p".to_string(),
    ]
}

impl Default for AiCliConfig {
    fn default() -> Self {
        Self {
            command: default_ai_command(),
            args: default_ai_args(),
            output_format: AiOutputFormat::default(),
            stream_partial: false,
            models: Vec::new(),
        }
    }
}

impl AiCliConfig {
    /// Get the full command arguments including output format flags.
    ///
    /// This applies the appropriate streaming flags based on the detected CLI type.
    /// If multiple models are configured, one is selected pseudo-randomly.
    pub fn full_args(&self) -> Vec<String> {
        self.full_args_with_model(self.select_model())
    }

    /// Get the full command arguments with a specific model selection.
    ///
    /// This is useful when you want to control which model is used, or when
    /// you've already selected a model and want to reuse it.
    pub fn full_args_with_model(&self, model: Option<&str>) -> Vec<String> {
        let mut args = self.args.clone();

        // Add model flag if a model is specified
        if let Some(model_name) = model {
            args.extend(self.get_model_args(model_name));
        }

        // Only add output format args if using streaming
        if self.output_format != AiOutputFormat::Text {
            let format_args = self.get_output_format_args();
            args.extend(format_args);
        }

        args
    }

    /// Select a model from the configured list using pseudo-random selection.
    ///
    /// Returns `None` if no models are configured, allowing the CLI to use its default.
    /// When multiple models are configured, selects one with equal probability distribution.
    #[must_use]
    pub fn select_model(&self) -> Option<&str> {
        if self.models.is_empty() {
            return None;
        }
        if self.models.len() == 1 {
            return Some(&self.models[0]);
        }

        // Use system time for pseudo-random selection with equal distribution
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        let index = (nanos as usize) % self.models.len();
        Some(&self.models[index])
    }

    /// Get the model flag arguments for the detected CLI.
    fn get_model_args(&self, model: &str) -> Vec<String> {
        // All supported CLIs use --model flag:
        // - Claude Code: --model <alias|name>
        // - Cursor CLI: --model <model-name>
        // - Codex: --model
        // - Aider: --model
        vec!["--model".to_string(), model.to_string()]
    }

    /// Get the output format arguments for the detected CLI.
    fn get_output_format_args(&self) -> Vec<String> {
        if self.output_format == AiOutputFormat::Text {
            return Vec::new();
        }

        let cmd_lower = self.command.to_lowercase();
        let mut args = vec![
            "--output-format".to_string(),
            match self.output_format {
                AiOutputFormat::Json => "json".to_string(),
                AiOutputFormat::StreamJson => "stream-json".to_string(),
                AiOutputFormat::Text => unreachable!(),
            },
        ];

        // Add partial streaming flag for stream-json mode
        if self.output_format == AiOutputFormat::StreamJson && self.stream_partial {
            if cmd_lower.contains("cursor") {
                args.push("--stream-partial-output".to_string());
            } else if cmd_lower.contains("claude") {
                args.push("--include-partial-messages".to_string());
            }
        }

        args
    }

    /// Check if this CLI is configured for NDJSON streaming.
    #[must_use]
    pub fn uses_stream_json(&self) -> bool {
        self.output_format == AiOutputFormat::StreamJson
    }

    /// Detect which CLI format parser to use based on the command.
    pub fn detect_cli_format(&self) -> crate::parser::CliFormat {
        let cmd_lower = self.command.to_lowercase();
        if cmd_lower.contains("cursor") {
            crate::parser::CliFormat::Cursor
        } else if cmd_lower.contains("claude") {
            crate::parser::CliFormat::Claude
        } else {
            crate::parser::CliFormat::Auto
        }
    }
}

/// Configuration for prompt generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Template name or "default".
    #[serde(default = "default_template")]
    pub template: String,
    /// Path to custom template file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_path: Option<String>,
    /// Additional context files to include.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_files: Vec<String>,
    /// Additional instructions to include.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<String>,
}

fn default_template() -> String {
    "default".to_string()
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            template: default_template(),
            custom_path: None,
            context_files: Vec::new(),
            instructions: Vec::new(),
        }
    }
}

/// Configuration for git integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    /// Whether to auto-commit after task completion.
    #[serde(default = "default_true")]
    pub auto_commit: bool,
    /// Template for commit messages.
    #[serde(default = "default_commit_template")]
    pub commit_message_template: String,
}

fn default_true() -> bool {
    true
}

fn default_commit_template() -> String {
    "afk: {task_id} - {message}".to_string()
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            auto_commit: default_true(),
            commit_message_template: default_commit_template(),
        }
    }
}

/// Configuration for session archiving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// Whether archiving is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Directory to store archives.
    #[serde(default = "default_archive_directory")]
    pub directory: String,
}

fn default_archive_directory() -> String {
    ".afk/archive".to_string()
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            directory: default_archive_directory(),
        }
    }
}

/// Feedback display mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackMode {
    /// Full TUI with all panels (default).
    #[default]
    Full,
    /// Minimal output display.
    Minimal,
    /// No feedback display.
    Off,
}

/// Configuration for feedback display settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackConfig {
    /// Whether feedback display is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Display mode.
    #[serde(default)]
    pub mode: FeedbackMode,
    /// Whether to show modified files.
    #[serde(default = "default_true")]
    pub show_files: bool,
    /// Whether to show metrics.
    #[serde(default = "default_true")]
    pub show_metrics: bool,
    /// Whether to show mascot.
    #[serde(default = "default_true")]
    pub show_mascot: bool,
    /// Refresh rate in seconds.
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate: f64,
    /// Maximum output lines to keep in TUI buffer.
    #[serde(default = "default_max_output_lines")]
    pub max_output_lines: usize,
    /// Seconds of inactivity before "thinking" state (was actively receiving output).
    #[serde(default = "default_active_threshold_secs")]
    pub active_threshold_secs: u64,
    /// Seconds of inactivity before "stalled" state (no output for extended period).
    #[serde(default = "default_thinking_threshold_secs")]
    pub thinking_threshold_secs: u64,
}

fn default_refresh_rate() -> f64 {
    0.1
}

fn default_max_output_lines() -> usize {
    500
}

fn default_active_threshold_secs() -> u64 {
    2
}

fn default_thinking_threshold_secs() -> u64 {
    10
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            mode: FeedbackMode::default(),
            show_files: default_true(),
            show_metrics: default_true(),
            show_mascot: default_true(),
            refresh_rate: default_refresh_rate(),
            max_output_lines: default_max_output_lines(),
            active_threshold_secs: default_active_threshold_secs(),
            thinking_threshold_secs: default_thinking_threshold_secs(),
        }
    }
}

/// Main configuration for afk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AfkConfig {
    /// Task sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceConfig>,
    /// Feedback loop commands.
    #[serde(default)]
    pub feedback_loops: FeedbackLoopsConfig,
    /// Iteration limits.
    #[serde(default)]
    pub limits: LimitsConfig,
    /// Output settings.
    #[serde(default)]
    pub output: OutputConfig,
    /// AI CLI settings.
    #[serde(default)]
    pub ai_cli: AiCliConfig,
    /// Prompt settings.
    #[serde(default)]
    pub prompt: PromptConfig,
    /// Git integration settings.
    #[serde(default)]
    pub git: GitConfig,
    /// Archive settings.
    #[serde(default)]
    pub archive: ArchiveConfig,
    /// Feedback display settings.
    #[serde(default)]
    pub feedback: FeedbackConfig,
}

/// Error type for config operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Error reading the config file from disk.
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    /// Error parsing the config file JSON.
    #[error("Failed to parse config JSON: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl AfkConfig {
    /// Load configuration from a file, or return defaults if file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to config file. Defaults to `.afk/config.json` if None.
    ///
    /// # Returns
    ///
    /// The loaded configuration, or defaults if the file doesn't exist.
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(CONFIG_FILE));

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let config: AfkConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save to. Defaults to `.afk/config.json` if None.
    ///
    /// Creates parent directories if they don't exist.
    pub fn save(&self, path: Option<&Path>) -> Result<(), ConfigError> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(CONFIG_FILE));

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Get the path for the afk directory.
    pub fn afk_dir() -> PathBuf {
        PathBuf::from(AFK_DIR)
    }

    /// Get the path for the config file.
    pub fn config_file() -> PathBuf {
        PathBuf::from(CONFIG_FILE)
    }

    /// Get the path for the progress file.
    pub fn progress_file() -> PathBuf {
        PathBuf::from(PROGRESS_FILE)
    }

    /// Get the path for the tasks file.
    pub fn tasks_file() -> PathBuf {
        PathBuf::from(TASKS_FILE)
    }

    /// Get the path for the archive directory.
    pub fn archive_dir() -> PathBuf {
        PathBuf::from(ARCHIVE_DIR)
    }

    /// Get a config value by dot-notation path (e.g., "limits.max_iterations").
    pub fn get_by_path(&self, path: &str) -> Result<String, FieldError> {
        let (section, key) = field::split_path(path)?;
        let value = match section {
            "limits" => self.limits.get_field(key),
            "output" => self.output.get_field(key),
            "ai_cli" => self.ai_cli.get_field(key),
            "prompt" => self.prompt.get_field(key),
            "git" => self.git.get_field(key),
            "archive" => self.archive.get_field(key),
            "feedback" => self.feedback.get_field(key),
            "feedback_loops" => self.feedback_loops.get_field(key),
            _ => None,
        };
        value.ok_or_else(|| FieldError::UnknownKey(path.into()))
    }

    /// Set a config value by dot-notation path.
    pub fn set_by_path(&mut self, path: &str, value: &str) -> Result<(), FieldError> {
        let (section, key) = field::split_path(path)?;
        match section {
            "limits" => self.limits.set_field(key, value),
            "output" => self.output.set_field(key, value),
            "ai_cli" => self.ai_cli.set_field(key, value),
            "prompt" => self.prompt.set_field(key, value),
            "git" => self.git.set_field(key, value),
            "archive" => self.archive.set_field(key, value),
            "feedback" => self.feedback.set_field(key, value),
            "feedback_loops" => self.feedback_loops.set_field(key, value),
            _ => Err(FieldError::UnknownKey(path.into())),
        }
    }

    /// List all section names.
    pub fn section_names() -> &'static [&'static str] {
        &[
            "limits",
            "output",
            "ai_cli",
            "prompt",
            "git",
            "archive",
            "feedback",
            "feedback_loops",
            "sources",
        ]
    }

    /// Get all field names for a section.
    pub fn fields_for_section(section: &str) -> Option<&'static [&'static str]> {
        match section {
            "limits" => Some(LimitsConfig::field_names()),
            "output" => Some(OutputConfig::field_names()),
            "ai_cli" => Some(AiCliConfig::field_names()),
            "prompt" => Some(PromptConfig::field_names()),
            "git" => Some(GitConfig::field_names()),
            "archive" => Some(ArchiveConfig::field_names()),
            "feedback" => Some(FeedbackConfig::field_names()),
            "feedback_loops" => Some(FeedbackLoopsConfig::field_names()),
            _ => None,
        }
    }

    /// List all valid config keys as dot-notation paths.
    pub fn all_keys() -> Vec<String> {
        let mut keys = Vec::new();
        for section in Self::section_names() {
            if *section == "sources" {
                // Sources handled separately
                continue;
            }
            if let Some(fields) = Self::fields_for_section(section) {
                for field in fields {
                    keys.push(format!("{section}.{field}"));
                }
            }
        }
        keys
    }

    /// Reset a section to its default values.
    pub fn reset_section(&mut self, section: &str) -> Result<(), FieldError> {
        match section {
            "limits" => {
                self.limits = LimitsConfig::default();
                Ok(())
            }
            "output" => {
                self.output = OutputConfig::default();
                Ok(())
            }
            "ai_cli" => {
                self.ai_cli = AiCliConfig::default();
                Ok(())
            }
            "prompt" => {
                self.prompt = PromptConfig::default();
                Ok(())
            }
            "git" => {
                self.git = GitConfig::default();
                Ok(())
            }
            "archive" => {
                self.archive = ArchiveConfig::default();
                Ok(())
            }
            "feedback" => {
                self.feedback = FeedbackConfig::default();
                Ok(())
            }
            "feedback_loops" => {
                self.feedback_loops = FeedbackLoopsConfig::default();
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(section.into())),
        }
    }

    /// Reset a specific field to its default value.
    pub fn reset_field(&mut self, path: &str) -> Result<(), FieldError> {
        let (section, key) = field::split_path(path)?;
        let default = Self::default();

        // Get default value and set it
        let default_value = match section {
            "limits" => default.limits.get_field(key),
            "output" => default.output.get_field(key),
            "ai_cli" => default.ai_cli.get_field(key),
            "prompt" => default.prompt.get_field(key),
            "git" => default.git.get_field(key),
            "archive" => default.archive.get_field(key),
            "feedback" => default.feedback.get_field(key),
            "feedback_loops" => default.feedback_loops.get_field(key),
            _ => return Err(FieldError::UnknownKey(path.into())),
        };

        match default_value {
            Some(val) => self.set_by_path(path, &val),
            None => Err(FieldError::UnknownKey(path.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_source_config_beads() {
        let source = SourceConfig::beads();
        assert_eq!(source.source_type, SourceType::Beads);
        assert!(source.path.is_none());
        assert!(source.repo.is_none());
        assert!(source.labels.is_empty());
    }

    #[test]
    fn test_source_config_json_with_path() {
        let source = SourceConfig::json("tasks.json");
        assert_eq!(source.source_type, SourceType::Json);
        assert_eq!(source.path, Some("tasks.json".to_string()));
    }

    #[test]
    fn test_source_config_github_with_options() {
        let source = SourceConfig::github(
            "owner/repo",
            vec!["bug".to_string(), "enhancement".to_string()],
        );
        assert_eq!(source.source_type, SourceType::Github);
        assert_eq!(source.repo, Some("owner/repo".to_string()));
        assert_eq!(source.labels, vec!["bug", "enhancement"]);
    }

    #[test]
    fn test_source_type_serialisation() {
        let source = SourceConfig::beads();
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains(r#""type":"beads""#));

        let parsed: SourceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source_type, SourceType::Beads);
    }

    #[test]
    fn test_feedback_loops_config_defaults() {
        let config = FeedbackLoopsConfig::default();
        assert!(config.types.is_none());
        assert!(config.lint.is_none());
        assert!(config.test.is_none());
        assert!(config.build.is_none());
        assert!(config.custom.is_empty());
    }

    #[test]
    fn test_feedback_loops_config_all_fields() {
        let mut custom = HashMap::new();
        custom.insert("format".to_string(), "ruff format .".to_string());

        let config = FeedbackLoopsConfig {
            types: Some("mypy .".to_string()),
            lint: Some("ruff check .".to_string()),
            test: Some("pytest".to_string()),
            build: Some("pip wheel .".to_string()),
            custom,
        };
        assert_eq!(config.types, Some("mypy .".to_string()));
        assert_eq!(config.lint, Some("ruff check .".to_string()));
        assert_eq!(config.test, Some("pytest".to_string()));
        assert_eq!(config.build, Some("pip wheel .".to_string()));
        assert_eq!(
            config.custom.get("format"),
            Some(&"ruff format .".to_string())
        );
    }

    #[test]
    fn test_limits_config_defaults() {
        let config = LimitsConfig::default();
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.max_task_failures, 50);
        assert_eq!(config.timeout_minutes, 120);
    }

    #[test]
    fn test_limits_config_custom_values() {
        let config = LimitsConfig {
            max_iterations: 5,
            max_task_failures: 1,
            timeout_minutes: 30,
        };
        assert_eq!(config.max_iterations, 5);
        assert_eq!(config.max_task_failures, 1);
        assert_eq!(config.timeout_minutes, 30);
    }

    #[test]
    fn test_output_config_defaults() {
        let config = OutputConfig::default();
        assert_eq!(config.default, OutputMode::Stdout);
        assert_eq!(config.file_path, ".afk/prompt.md");
    }

    #[test]
    fn test_output_config_clipboard() {
        let config = OutputConfig {
            default: OutputMode::Clipboard,
            file_path: ".afk/prompt.md".to_string(),
        };
        assert_eq!(config.default, OutputMode::Clipboard);
    }

    #[test]
    fn test_ai_cli_config_defaults() {
        let config = AiCliConfig::default();
        assert_eq!(config.command, "claude");
        assert_eq!(config.args, vec!["--dangerously-skip-permissions", "-p"]);
        assert_eq!(config.output_format, AiOutputFormat::StreamJson);
        assert!(!config.stream_partial);
    }

    #[test]
    fn test_ai_cli_config_custom() {
        let config = AiCliConfig {
            command: "aider".to_string(),
            args: vec!["--message".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: Vec::new(),
        };
        assert_eq!(config.command, "aider");
        assert_eq!(config.args, vec!["--message"]);
    }

    #[test]
    fn test_ai_cli_full_args_cursor() {
        let config = AiCliConfig {
            command: "cursor".to_string(),
            args: vec!["--print".to_string()],
            output_format: AiOutputFormat::StreamJson,
            stream_partial: false,
            models: Vec::new(),
        };
        let full_args = config.full_args();
        assert!(full_args.contains(&"--print".to_string()));
        assert!(full_args.contains(&"--output-format".to_string()));
        assert!(full_args.contains(&"stream-json".to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_cursor_with_partial() {
        let config = AiCliConfig {
            command: "cursor".to_string(),
            args: vec!["--print".to_string()],
            output_format: AiOutputFormat::StreamJson,
            stream_partial: true,
            models: Vec::new(),
        };
        let full_args = config.full_args();
        assert!(full_args.contains(&"--stream-partial-output".to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_claude() {
        let config = AiCliConfig {
            command: "claude".to_string(),
            args: vec!["-p".to_string()],
            output_format: AiOutputFormat::StreamJson,
            stream_partial: true,
            models: Vec::new(),
        };
        let full_args = config.full_args();
        assert!(full_args.contains(&"--output-format".to_string()));
        assert!(full_args.contains(&"stream-json".to_string()));
        assert!(full_args.contains(&"--include-partial-messages".to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_text_format() {
        let config = AiCliConfig {
            command: "claude".to_string(),
            args: vec!["-p".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: Vec::new(),
        };
        let full_args = config.full_args();
        assert_eq!(full_args, vec!["-p"]);
    }

    // ========================================================================
    // Multi-model rotation tests
    // ========================================================================

    #[test]
    fn test_ai_cli_select_model_empty() {
        let config = AiCliConfig::default();
        assert!(config.select_model().is_none());
    }

    #[test]
    fn test_ai_cli_select_model_single() {
        let config = AiCliConfig {
            models: vec!["sonnet".to_string()],
            ..Default::default()
        };
        assert_eq!(config.select_model(), Some("sonnet"));
    }

    #[test]
    fn test_ai_cli_select_model_multiple_returns_valid() {
        let config = AiCliConfig {
            models: vec![
                "sonnet".to_string(),
                "opus".to_string(),
                "haiku".to_string(),
            ],
            ..Default::default()
        };
        // Should return one of the configured models
        let selected = config.select_model();
        assert!(selected.is_some());
        assert!(config.models.contains(&selected.unwrap().to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_with_model_includes_flag() {
        let config = AiCliConfig {
            command: "claude".to_string(),
            args: vec!["-p".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: vec!["opus".to_string()],
        };
        let full_args = config.full_args();
        // Should include --model flag
        assert!(full_args.contains(&"--model".to_string()));
        assert!(full_args.contains(&"opus".to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_with_model_preserves_order() {
        let config = AiCliConfig {
            command: "claude".to_string(),
            args: vec!["-p".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: vec!["sonnet".to_string()],
        };
        let full_args = config.full_args();
        // -p should come first (from args), then --model
        assert_eq!(full_args[0], "-p");
        assert_eq!(full_args[1], "--model");
        assert_eq!(full_args[2], "sonnet");
    }

    #[test]
    fn test_ai_cli_full_args_with_model_explicit() {
        let config = AiCliConfig {
            command: "cursor".to_string(),
            args: vec!["--print".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: Vec::new(),
        };
        // Explicitly pass a model
        let full_args = config.full_args_with_model(Some("gpt-4"));
        assert!(full_args.contains(&"--model".to_string()));
        assert!(full_args.contains(&"gpt-4".to_string()));
    }

    #[test]
    fn test_ai_cli_full_args_with_model_none() {
        let config = AiCliConfig {
            command: "claude".to_string(),
            args: vec!["-p".to_string()],
            output_format: AiOutputFormat::Text,
            stream_partial: false,
            models: Vec::new(),
        };
        // No model means no --model flag
        let full_args = config.full_args_with_model(None);
        assert!(!full_args.contains(&"--model".to_string()));
    }

    #[test]
    fn test_ai_cli_models_serialisation() {
        let config = AiCliConfig {
            models: vec!["sonnet".to_string(), "opus".to_string()],
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(r#""models":["sonnet","opus"]"#));

        // Empty models should not be serialised
        let empty_config = AiCliConfig::default();
        let empty_json = serde_json::to_string(&empty_config).unwrap();
        assert!(!empty_json.contains("models"));
    }

    #[test]
    fn test_ai_cli_models_deserialisation() {
        let json = r#"{
            "command": "claude",
            "args": ["-p"],
            "models": ["sonnet", "opus", "haiku"]
        }"#;
        let config: AiCliConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.models, vec!["sonnet", "opus", "haiku"]);
    }

    #[test]
    fn test_ai_cli_models_deserialisation_missing() {
        // Models should default to empty if not present
        let json = r#"{"command": "claude", "args": ["-p"]}"#;
        let config: AiCliConfig = serde_json::from_str(json).unwrap();
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_ai_cli_uses_stream_json() {
        let config = AiCliConfig::default();
        assert!(config.uses_stream_json());

        let text_config = AiCliConfig {
            output_format: AiOutputFormat::Text,
            ..Default::default()
        };
        assert!(!text_config.uses_stream_json());
    }

    #[test]
    fn test_ai_cli_detect_cli_format() {
        use crate::parser::CliFormat;

        let cursor_config = AiCliConfig {
            command: "cursor".to_string(),
            ..Default::default()
        };
        assert_eq!(cursor_config.detect_cli_format(), CliFormat::Cursor);

        let claude_config = AiCliConfig {
            command: "claude".to_string(),
            ..Default::default()
        };
        assert_eq!(claude_config.detect_cli_format(), CliFormat::Claude);

        let unknown_config = AiCliConfig {
            command: "aider".to_string(),
            ..Default::default()
        };
        assert_eq!(unknown_config.detect_cli_format(), CliFormat::Auto);
    }

    #[test]
    fn test_prompt_config_defaults() {
        let config = PromptConfig::default();
        assert_eq!(config.template, "default");
        assert!(config.custom_path.is_none());
        assert!(config.context_files.is_empty());
        assert!(config.instructions.is_empty());
    }

    #[test]
    fn test_prompt_config_custom() {
        let config = PromptConfig {
            template: "minimal".to_string(),
            custom_path: Some(".afk/prompt.jinja2".to_string()),
            context_files: vec!["AGENTS.md".to_string(), "README.md".to_string()],
            instructions: vec![
                "Always run tests".to_string(),
                "Use British English".to_string(),
            ],
        };
        assert_eq!(config.template, "minimal");
        assert_eq!(config.custom_path, Some(".afk/prompt.jinja2".to_string()));
        assert_eq!(config.context_files, vec!["AGENTS.md", "README.md"]);
        assert_eq!(
            config.instructions,
            vec!["Always run tests", "Use British English"]
        );
    }

    #[test]
    fn test_git_config_defaults() {
        let config = GitConfig::default();
        assert!(config.auto_commit);
        assert_eq!(config.commit_message_template, "afk: {task_id} - {message}");
    }

    #[test]
    fn test_git_config_custom() {
        let config = GitConfig {
            auto_commit: true,
            commit_message_template: "[{task_id}] {message}".to_string(),
        };
        assert!(config.auto_commit);
        assert_eq!(config.commit_message_template, "[{task_id}] {message}");
    }

    #[test]
    fn test_archive_config_defaults() {
        let config = ArchiveConfig::default();
        assert!(config.enabled);
        assert_eq!(config.directory, ".afk/archive");
    }

    #[test]
    fn test_archive_config_disabled() {
        let config = ArchiveConfig {
            enabled: false,
            directory: ".archive".to_string(),
        };
        assert!(!config.enabled);
        assert_eq!(config.directory, ".archive");
    }

    #[test]
    fn test_feedback_config_defaults() {
        let config = FeedbackConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, FeedbackMode::Full);
        assert!(config.show_files);
        assert!(config.show_metrics);
        assert!(config.show_mascot);
        assert!((config.refresh_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_feedback_config_minimal_mode() {
        let config = FeedbackConfig {
            mode: FeedbackMode::Minimal,
            ..Default::default()
        };
        assert_eq!(config.mode, FeedbackMode::Minimal);
    }

    #[test]
    fn test_feedback_config_off_mode() {
        let config = FeedbackConfig {
            mode: FeedbackMode::Off,
            ..Default::default()
        };
        assert_eq!(config.mode, FeedbackMode::Off);
    }

    #[test]
    fn test_feedback_config_disabled() {
        let config = FeedbackConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!config.enabled);
    }

    #[test]
    fn test_feedback_config_custom_refresh_rate() {
        let config = FeedbackConfig {
            refresh_rate: 0.5,
            ..Default::default()
        };
        assert!((config.refresh_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_feedback_config_hide_panels() {
        let config = FeedbackConfig {
            show_files: false,
            show_metrics: false,
            show_mascot: false,
            ..Default::default()
        };
        assert!(!config.show_files);
        assert!(!config.show_metrics);
        assert!(!config.show_mascot);
    }

    #[test]
    fn test_afk_config_defaults() {
        let config = AfkConfig::default();
        assert!(config.sources.is_empty());
        assert_eq!(config.limits.max_iterations, 200);
        assert_eq!(config.output.default, OutputMode::Stdout);
        assert_eq!(config.ai_cli.command, "claude");
    }

    #[test]
    fn test_afk_config_load_missing_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".afk/config.json");
        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert!(config.sources.is_empty());
        assert_eq!(config.limits.max_iterations, 200);
    }

    #[test]
    fn test_afk_config_load_existing_file() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let config_path = afk_dir.join("config.json");

        let sample_config = r#"{
            "sources": [
                {"type": "beads"},
                {"type": "json", "path": "tasks.json"}
            ],
            "limits": {
                "max_iterations": 10
            }
        }"#;
        fs::write(&config_path, sample_config).unwrap();

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.sources[0].source_type, SourceType::Beads);
        assert_eq!(config.sources[1].source_type, SourceType::Json);
        assert_eq!(config.limits.max_iterations, 10);
    }

    #[test]
    fn test_afk_config_save_creates_directory() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".afk/config.json");

        let config = AfkConfig {
            sources: vec![SourceConfig::beads()],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        assert!(config_path.exists());
        let contents = fs::read_to_string(&config_path).unwrap();
        assert!(contents.contains(r#""type": "beads""#));
    }

    #[test]
    fn test_afk_config_round_trip() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".afk/config.json");

        let original = AfkConfig {
            sources: vec![SourceConfig::github("owner/repo", vec!["bug".to_string()])],
            feedback_loops: FeedbackLoopsConfig {
                lint: Some("ruff check .".to_string()),
                ..Default::default()
            },
            limits: LimitsConfig {
                max_iterations: 15,
                ..Default::default()
            },
            ..Default::default()
        };

        original.save(Some(&config_path)).unwrap();
        let loaded = AfkConfig::load(Some(&config_path)).unwrap();

        assert_eq!(loaded.sources[0].source_type, SourceType::Github);
        assert_eq!(loaded.sources[0].repo, Some("owner/repo".to_string()));
        assert_eq!(loaded.sources[0].labels, vec!["bug"]);
        assert_eq!(loaded.feedback_loops.lint, Some("ruff check .".to_string()));
        assert_eq!(loaded.limits.max_iterations, 15);
    }

    #[test]
    fn test_afk_config_with_real_config_format() {
        // Test with a typical config.json format
        let json = r#"{
            "sources": [],
            "feedback_loops": {
                "types": "mypy .",
                "lint": "ruff check .",
                "test": "pytest",
                "custom": {}
            },
            "limits": {
                "max_iterations": 200,
                "max_task_failures": 20,
                "timeout_minutes": 90
            },
            "output": {
                "default": "clipboard",
                "file_path": ".afk/prompt.md"
            },
            "ai_cli": {
                "command": "agent",
                "args": ["-p", "--force"]
            },
            "prompt": {
                "template": "default",
                "context_files": ["AGENTS.md", "README.md"],
                "instructions": []
            },
            "git": {
                "auto_commit": true,
                "commit_message_template": "afk: {task_id} - {message}"
            },
            "archive": {
                "enabled": true,
                "directory": ".afk/archive"
            }
        }"#;

        let config: AfkConfig = serde_json::from_str(json).unwrap();
        assert!(config.sources.is_empty());
        assert_eq!(config.feedback_loops.types, Some("mypy .".to_string()));
        assert_eq!(config.feedback_loops.lint, Some("ruff check .".to_string()));
        assert_eq!(config.feedback_loops.test, Some("pytest".to_string()));
        assert_eq!(config.limits.max_iterations, 200);
        assert_eq!(config.limits.max_task_failures, 20);
        assert_eq!(config.limits.timeout_minutes, 90);
        assert_eq!(config.output.default, OutputMode::Clipboard);
        assert_eq!(config.ai_cli.command, "agent");
        assert_eq!(config.ai_cli.args, vec!["-p", "--force"]);
        assert_eq!(config.prompt.context_files, vec!["AGENTS.md", "README.md"]);
        assert!(config.git.auto_commit);
        assert!(config.archive.enabled);
    }

    #[test]
    fn test_path_constants() {
        assert_eq!(AFK_DIR, ".afk");
        assert_eq!(CONFIG_FILE, ".afk/config.json");
        assert_eq!(PROGRESS_FILE, ".afk/progress.json");
        assert_eq!(TASKS_FILE, ".afk/tasks.json");
        assert_eq!(ARCHIVE_DIR, ".afk/archive");
    }

    #[test]
    fn test_path_helpers() {
        assert_eq!(AfkConfig::afk_dir(), PathBuf::from(".afk"));
        assert_eq!(AfkConfig::config_file(), PathBuf::from(".afk/config.json"));
        assert_eq!(
            AfkConfig::progress_file(),
            PathBuf::from(".afk/progress.json")
        );
        assert_eq!(AfkConfig::archive_dir(), PathBuf::from(".afk/archive"));
    }

    #[test]
    fn test_partial_config_with_defaults() {
        // Test that partial JSON gets merged with defaults
        let json = r#"{
            "limits": {
                "max_iterations": 50
            }
        }"#;

        let config: AfkConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.limits.max_iterations, 50);
        // Other limit fields should have defaults
        assert_eq!(config.limits.max_task_failures, 50);
        assert_eq!(config.limits.timeout_minutes, 120);
        // Other config sections should have defaults
        assert!(config.sources.is_empty());
        assert_eq!(config.ai_cli.command, "claude");
    }
}
