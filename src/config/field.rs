//! ConfigField trait for dynamic get/set access to config sections.
//!
//! This module provides a trait-based abstraction for accessing config
//! fields by name, enabling the `afk config` CLI commands.

/// Error type for config field operations.
#[derive(Debug, thiserror::Error)]
pub enum FieldError {
    /// The specified config key is not recognised.
    #[error("Unknown config key: {0}")]
    UnknownKey(String),

    /// The value provided is invalid for the specified key.
    #[error("Invalid value for {key}: expected {expected}")]
    InvalidValue {
        /// The config key that was being set.
        key: String,
        /// Description of the expected value format.
        expected: String,
    },

    /// The config path format is invalid.
    #[error("Invalid path format: {0}")]
    InvalidPath(String),
}

/// Trait for config sections that support dynamic get/set access.
///
/// Each config section (LimitsConfig, AiCliConfig, etc.) implements this trait
/// to enable field access by name. This powers the `afk config get/set` commands.
pub trait ConfigField {
    /// Get a field value by name, returning serialised string.
    ///
    /// Returns `None` if the key is not recognised.
    fn get_field(&self, key: &str) -> Option<String>;

    /// Set a field value by name from string input.
    ///
    /// Returns an error if the key is not recognised or the value is invalid.
    fn set_field(&mut self, key: &str, value: &str) -> Result<(), FieldError>;

    /// List all field names in this section.
    fn field_names() -> &'static [&'static str]
    where
        Self: Sized;

    /// Get the section name for this config type.
    fn section_name() -> &'static str
    where
        Self: Sized;
}

/// Split a dot-notation path into section and field.
///
/// # Examples
///
/// ```ignore
/// split_path("limits.max_iterations") // Ok(("limits", "max_iterations"))
/// split_path("ai_cli.command") // Ok(("ai_cli", "command"))
/// split_path("invalid") // Err(InvalidPath)
/// ```
pub fn split_path(path: &str) -> Result<(&str, &str), FieldError> {
    path.split_once('.')
        .ok_or_else(|| FieldError::InvalidPath(format!("{path} (expected section.field)")))
}

/// Format an optional string value for display.
pub fn format_optional(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "(not set)".to_string())
}

/// Format a vector of strings for display.
pub fn format_vec(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}

/// Parse a boolean from various string representations.
pub fn parse_bool(value: &str) -> Result<bool, FieldError> {
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => Err(FieldError::InvalidValue {
            key: "bool".into(),
            expected: "true/false, yes/no, 1/0, on/off".into(),
        }),
    }
}

/// Parse a comma-separated string into a vector.
pub fn parse_vec(value: &str) -> Vec<String> {
    if value.is_empty() || value == "(none)" {
        Vec::new()
    } else {
        value.split(',').map(|s| s.trim().to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path_valid() {
        let (section, field) = split_path("limits.max_iterations").unwrap();
        assert_eq!(section, "limits");
        assert_eq!(field, "max_iterations");
    }

    #[test]
    fn test_split_path_nested() {
        let (section, field) = split_path("ai_cli.command").unwrap();
        assert_eq!(section, "ai_cli");
        assert_eq!(field, "command");
    }

    #[test]
    fn test_split_path_invalid() {
        let result = split_path("invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FieldError::InvalidPath(_)));
    }

    #[test]
    fn test_format_optional_some() {
        let opt = Some("value".to_string());
        assert_eq!(format_optional(&opt), "value");
    }

    #[test]
    fn test_format_optional_none() {
        let opt: Option<String> = None;
        assert_eq!(format_optional(&opt), "(not set)");
    }

    #[test]
    fn test_format_vec_empty() {
        let vec: Vec<String> = vec![];
        assert_eq!(format_vec(&vec), "(none)");
    }

    #[test]
    fn test_format_vec_values() {
        let vec = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(format_vec(&vec), "a, b, c");
    }

    #[test]
    fn test_parse_bool_true_variants() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("True").unwrap());
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("on").unwrap());
    }

    #[test]
    fn test_parse_bool_false_variants() {
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("False").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("off").unwrap());
    }

    #[test]
    fn test_parse_bool_invalid() {
        let result = parse_bool("maybe");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_vec_empty() {
        assert!(parse_vec("").is_empty());
        assert!(parse_vec("(none)").is_empty());
    }

    #[test]
    fn test_parse_vec_values() {
        let result = parse_vec("a, b, c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_vec_single() {
        let result = parse_vec("single");
        assert_eq!(result, vec!["single"]);
    }

    #[test]
    fn test_field_error_display() {
        let err = FieldError::UnknownKey("foo.bar".into());
        assert_eq!(err.to_string(), "Unknown config key: foo.bar");

        let err = FieldError::InvalidValue {
            key: "max_iterations".into(),
            expected: "positive integer".into(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value for max_iterations: expected positive integer"
        );

        let err = FieldError::InvalidPath("invalid".into());
        assert_eq!(err.to_string(), "Invalid path format: invalid");
    }
}
