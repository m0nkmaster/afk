//! ConfigField trait implementations for config sections.
//!
//! This module contains the `ConfigField` implementations for each config type,
//! providing dynamic get/set access with validation for the `afk config` CLI.

use super::field::{format_optional, format_vec, parse_bool, parse_vec, ConfigField, FieldError};
use super::{
    AiCliConfig, AiOutputFormat, ArchiveConfig, FeedbackConfig, FeedbackLoopsConfig, FeedbackMode,
    GitConfig, LimitsConfig, OutputConfig, OutputMode, PromptConfig,
};

impl ConfigField for LimitsConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "max_iterations" => Some(self.max_iterations.to_string()),
            "max_task_failures" => Some(self.max_task_failures.to_string()),
            "timeout_minutes" => Some(self.timeout_minutes.to_string()),
            "prevent_sleep" => Some(self.prevent_sleep.to_string()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "max_iterations" => {
                self.max_iterations = value.parse().map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "positive integer".into(),
                })?;
                Ok(())
            }
            "max_task_failures" => {
                self.max_task_failures = value.parse().map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "positive integer".into(),
                })?;
                Ok(())
            }
            "timeout_minutes" => {
                self.timeout_minutes = value.parse().map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "positive integer".into(),
                })?;
                Ok(())
            }
            "prevent_sleep" => {
                self.prevent_sleep = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &[
            "max_iterations",
            "max_task_failures",
            "timeout_minutes",
            "prevent_sleep",
        ]
    }

    fn section_name() -> &'static str {
        "limits"
    }
}

impl ConfigField for OutputConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "default" => Some(
                match self.default {
                    OutputMode::Clipboard => "clipboard",
                    OutputMode::File => "file",
                    OutputMode::Stdout => "stdout",
                }
                .to_string(),
            ),
            "file_path" => Some(self.file_path.clone()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "default" => {
                self.default = match value.to_lowercase().as_str() {
                    "clipboard" => OutputMode::Clipboard,
                    "file" => OutputMode::File,
                    "stdout" => OutputMode::Stdout,
                    _ => {
                        return Err(FieldError::InvalidValue {
                            key: key.into(),
                            expected: "clipboard, file, or stdout".into(),
                        })
                    }
                };
                Ok(())
            }
            "file_path" => {
                self.file_path = value.to_string();
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &["default", "file_path"]
    }

    fn section_name() -> &'static str {
        "output"
    }
}

impl ConfigField for AiCliConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "command" => Some(self.command.clone()),
            "args" => Some(format_vec(&self.args)),
            "output_format" => Some(
                match self.output_format {
                    AiOutputFormat::Text => "text",
                    AiOutputFormat::Json => "json",
                    AiOutputFormat::StreamJson => "stream-json",
                }
                .to_string(),
            ),
            "stream_partial" => Some(self.stream_partial.to_string()),
            "models" => Some(format_vec(&self.models)),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "command" => {
                self.command = value.to_string();
                Ok(())
            }
            "args" => {
                self.args = parse_vec(value);
                Ok(())
            }
            "output_format" => {
                self.output_format = match value.to_lowercase().as_str() {
                    "text" => AiOutputFormat::Text,
                    "json" => AiOutputFormat::Json,
                    "stream-json" | "streamjson" => AiOutputFormat::StreamJson,
                    _ => {
                        return Err(FieldError::InvalidValue {
                            key: key.into(),
                            expected: "text, json, or stream-json".into(),
                        })
                    }
                };
                Ok(())
            }
            "stream_partial" => {
                self.stream_partial = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "models" => {
                self.models = parse_vec(value);
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &[
            "command",
            "args",
            "output_format",
            "stream_partial",
            "models",
        ]
    }

    fn section_name() -> &'static str {
        "ai_cli"
    }
}

impl ConfigField for PromptConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "template" => Some(self.template.clone()),
            "custom_path" => Some(format_optional(&self.custom_path)),
            "context_files" => Some(format_vec(&self.context_files)),
            "instructions" => Some(format_vec(&self.instructions)),
            "has_frontend" => Some(self.has_frontend.to_string()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "template" => {
                self.template = value.to_string();
                Ok(())
            }
            "custom_path" => {
                self.custom_path = if value.is_empty() || value == "(not set)" {
                    None
                } else {
                    Some(value.to_string())
                };
                Ok(())
            }
            "context_files" => {
                self.context_files = parse_vec(value);
                Ok(())
            }
            "instructions" => {
                self.instructions = parse_vec(value);
                Ok(())
            }
            "has_frontend" => {
                self.has_frontend = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: "has_frontend".into(),
                    expected: "true/false, yes/no, 1/0, on/off".into(),
                })?;
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &[
            "template",
            "custom_path",
            "context_files",
            "instructions",
            "has_frontend",
        ]
    }

    fn section_name() -> &'static str {
        "prompt"
    }
}

impl ConfigField for GitConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "auto_commit" => Some(self.auto_commit.to_string()),
            "commit_message_template" => Some(self.commit_message_template.clone()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "auto_commit" => {
                self.auto_commit = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "commit_message_template" => {
                self.commit_message_template = value.to_string();
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &["auto_commit", "commit_message_template"]
    }

    fn section_name() -> &'static str {
        "git"
    }
}

impl ConfigField for ArchiveConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "enabled" => Some(self.enabled.to_string()),
            "directory" => Some(self.directory.clone()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "enabled" => {
                self.enabled = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "directory" => {
                self.directory = value.to_string();
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &["enabled", "directory"]
    }

    fn section_name() -> &'static str {
        "archive"
    }
}

impl ConfigField for FeedbackConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "enabled" => Some(self.enabled.to_string()),
            "mode" => Some(
                match self.mode {
                    FeedbackMode::Full => "full",
                    FeedbackMode::Minimal => "minimal",
                    FeedbackMode::Off => "off",
                }
                .to_string(),
            ),
            "show_files" => Some(self.show_files.to_string()),
            "show_metrics" => Some(self.show_metrics.to_string()),
            "show_mascot" => Some(self.show_mascot.to_string()),
            "refresh_rate" => Some(self.refresh_rate.to_string()),
            "max_output_lines" => Some(self.max_output_lines.to_string()),
            "active_threshold_secs" => Some(self.active_threshold_secs.to_string()),
            "thinking_threshold_secs" => Some(self.thinking_threshold_secs.to_string()),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        match key {
            "enabled" => {
                self.enabled = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "mode" => {
                self.mode = match value.to_lowercase().as_str() {
                    "full" => FeedbackMode::Full,
                    "minimal" => FeedbackMode::Minimal,
                    "off" => FeedbackMode::Off,
                    _ => {
                        return Err(FieldError::InvalidValue {
                            key: key.into(),
                            expected: "full, minimal, or off".into(),
                        })
                    }
                };
                Ok(())
            }
            "show_files" => {
                self.show_files = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "show_metrics" => {
                self.show_metrics = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "show_mascot" => {
                self.show_mascot = parse_bool(value).map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "true or false".into(),
                })?;
                Ok(())
            }
            "refresh_rate" => {
                self.refresh_rate = value.parse().map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "decimal number (e.g., 0.1)".into(),
                })?;
                Ok(())
            }
            "max_output_lines" => {
                self.max_output_lines = value.parse().map_err(|_| FieldError::InvalidValue {
                    key: key.into(),
                    expected: "positive integer (e.g., 500)".into(),
                })?;
                Ok(())
            }
            "active_threshold_secs" => {
                self.active_threshold_secs =
                    value.parse().map_err(|_| FieldError::InvalidValue {
                        key: key.into(),
                        expected: "positive integer (e.g., 2)".into(),
                    })?;
                Ok(())
            }
            "thinking_threshold_secs" => {
                self.thinking_threshold_secs =
                    value.parse().map_err(|_| FieldError::InvalidValue {
                        key: key.into(),
                        expected: "positive integer (e.g., 10)".into(),
                    })?;
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &[
            "enabled",
            "mode",
            "show_files",
            "show_metrics",
            "show_mascot",
            "refresh_rate",
            "max_output_lines",
            "active_threshold_secs",
            "thinking_threshold_secs",
        ]
    }

    fn section_name() -> &'static str {
        "feedback"
    }
}

impl ConfigField for FeedbackLoopsConfig {
    fn get_field(&self, key: &str) -> Option<String> {
        match key {
            "types" => Some(format_optional(&self.types)),
            "lint" => Some(format_optional(&self.lint)),
            "test" => Some(format_optional(&self.test)),
            "build" => Some(format_optional(&self.build)),
            _ => None,
        }
    }

    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError> {
        let opt_value = if value.is_empty() || value == "(not set)" {
            None
        } else {
            Some(value.to_string())
        };

        match key {
            "types" => {
                self.types = opt_value;
                Ok(())
            }
            "lint" => {
                self.lint = opt_value;
                Ok(())
            }
            "test" => {
                self.test = opt_value;
                Ok(())
            }
            "build" => {
                self.build = opt_value;
                Ok(())
            }
            _ => Err(FieldError::UnknownKey(key.into())),
        }
    }

    fn field_names() -> &'static [&'static str] {
        &["types", "lint", "test", "build"]
    }

    fn section_name() -> &'static str {
        "feedback_loops"
    }
}
