//! Source command implementations.
//!
//! This module implements the `afk source add/list/remove` commands
//! for managing task sources in the configuration.

use std::path::Path;

use crate::config::{AfkConfig, SourceConfig, SourceType};

/// Result type for source command operations.
pub type SourceCommandResult = Result<(), SourceCommandError>;

/// Error type for source command operations.
#[derive(Debug, thiserror::Error)]
pub enum SourceCommandError {
    /// Source file was not found at the specified path.
    #[error("File not found: {0}")]
    FileNotFound(String),
    /// Invalid source type name provided.
    #[error("Invalid source type: {0}")]
    InvalidSourceType(String),
    /// Index out of range for source list.
    #[error("Invalid index: {index}. Must be 1-{max}")]
    InvalidIndex {
        /// The invalid index that was provided.
        index: usize,
        /// Maximum valid index.
        max: usize,
    },
    /// No sources are configured.
    #[error("No sources configured")]
    NoSources,
    /// Configuration error.
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

/// Add a task source to the configuration.
///
/// # Arguments
///
/// * `source_type` - The type of source to add (beads, json, markdown, github).
/// * `path` - Optional path for file-based sources (json, markdown).
///
/// # Returns
///
/// Ok(()) on success, or an error if validation fails.
///
/// # Example
///
/// ```ignore
/// source_add("json", Some("tasks.json"))?;
/// source_add("beads", None)?;
/// ```
pub fn source_add(source_type: &str, path: Option<&str>) -> SourceCommandResult {
    source_add_impl(source_type, path, None)
}

/// Internal implementation of source_add with optional config path for testing.
fn source_add_impl(
    source_type: &str,
    path: Option<&str>,
    config_path: Option<&Path>,
) -> SourceCommandResult {
    let mut config = AfkConfig::load(config_path)?;

    // Parse and validate source type
    let source_type_enum = parse_source_type(source_type)?;

    // Validate path exists for file-based sources
    if matches!(source_type_enum, SourceType::Json | SourceType::Markdown) {
        if let Some(p) = path {
            if !Path::new(p).exists() {
                return Err(SourceCommandError::FileNotFound(p.to_string()));
            }
        }
    }

    // Create the source configuration
    let new_source = match source_type_enum {
        SourceType::Beads => SourceConfig::beads(),
        SourceType::Json => SourceConfig::json(path.unwrap_or(".afk/tasks.json")),
        SourceType::Markdown => SourceConfig::markdown(path.unwrap_or("TODO.md")),
        SourceType::Github => {
            // GitHub requires special handling - repo and labels
            // For now, create with empty values; user would typically use
            // a more complex CLI or edit config directly
            SourceConfig::github(path.unwrap_or(""), vec![])
        }
    };

    config.sources.push(new_source);
    config.save(config_path)?;

    // Print success message
    let path_info = path.map(|p| format!(" ({p})")).unwrap_or_default();
    println!("\x1b[32mAdded source:\x1b[0m {source_type}{path_info}");

    Ok(())
}

/// List all configured task sources.
///
/// Prints each source with its 1-based index for easy removal.
pub fn source_list() -> SourceCommandResult {
    source_list_impl(None)
}

/// Internal implementation of source_list with optional config path for testing.
fn source_list_impl(config_path: Option<&Path>) -> SourceCommandResult {
    let config = AfkConfig::load(config_path)?;

    if config.sources.is_empty() {
        println!("\x1b[2mNo sources configured.\x1b[0m Use \x1b[36mafk source add\x1b[0m");
        return Ok(());
    }

    for (i, src) in config.sources.iter().enumerate() {
        let path_info = match &src.source_type {
            SourceType::Github => src
                .repo
                .as_ref()
                .map(|r| format!(" ({r})"))
                .unwrap_or_default(),
            _ => src
                .path
                .as_ref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default(),
        };
        let type_str = source_type_to_str(&src.source_type);
        println!("  {}. \x1b[36m{}\x1b[0m{}", i + 1, type_str, path_info);
    }

    Ok(())
}

/// Remove a task source by 1-based index.
///
/// # Arguments
///
/// * `index` - The 1-based index of the source to remove.
///
/// # Returns
///
/// Ok(()) on success, or an error if the index is invalid.
pub fn source_remove(index: usize) -> SourceCommandResult {
    source_remove_impl(index, None)
}

/// Internal implementation of source_remove with optional config path for testing.
fn source_remove_impl(index: usize, config_path: Option<&Path>) -> SourceCommandResult {
    let mut config = AfkConfig::load(config_path)?;

    if config.sources.is_empty() {
        return Err(SourceCommandError::NoSources);
    }

    if index < 1 || index > config.sources.len() {
        return Err(SourceCommandError::InvalidIndex {
            index,
            max: config.sources.len(),
        });
    }

    let removed = config.sources.remove(index - 1);
    config.save(config_path)?;

    let type_str = source_type_to_str(&removed.source_type);
    println!("\x1b[32mRemoved source:\x1b[0m {type_str}");

    Ok(())
}

/// Parse a source type string into a SourceType enum.
fn parse_source_type(s: &str) -> Result<SourceType, SourceCommandError> {
    match s.to_lowercase().as_str() {
        "beads" => Ok(SourceType::Beads),
        "json" => Ok(SourceType::Json),
        "markdown" => Ok(SourceType::Markdown),
        "github" => Ok(SourceType::Github),
        _ => Err(SourceCommandError::InvalidSourceType(s.to_string())),
    }
}

/// Convert a SourceType enum to its string representation.
fn source_type_to_str(st: &SourceType) -> &'static str {
    match st {
        SourceType::Beads => "beads",
        SourceType::Json => "json",
        SourceType::Markdown => "markdown",
        SourceType::Github => "github",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to set up a temp directory with .afk subdirectory and return config path.
    fn setup_temp_config() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let config_path = afk_dir.join("config.json");
        (temp, config_path)
    }

    #[test]
    fn test_parse_source_type_valid() {
        assert_eq!(parse_source_type("beads").unwrap(), SourceType::Beads);
        assert_eq!(parse_source_type("json").unwrap(), SourceType::Json);
        assert_eq!(parse_source_type("markdown").unwrap(), SourceType::Markdown);
        assert_eq!(parse_source_type("github").unwrap(), SourceType::Github);
    }

    #[test]
    fn test_parse_source_type_case_insensitive() {
        assert_eq!(parse_source_type("BEADS").unwrap(), SourceType::Beads);
        assert_eq!(parse_source_type("Json").unwrap(), SourceType::Json);
        assert_eq!(parse_source_type("MARKDOWN").unwrap(), SourceType::Markdown);
        assert_eq!(parse_source_type("GitHub").unwrap(), SourceType::Github);
    }

    #[test]
    fn test_parse_source_type_invalid() {
        let result = parse_source_type("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SourceCommandError::InvalidSourceType(_)
        ));
    }

    #[test]
    fn test_source_type_to_str() {
        assert_eq!(source_type_to_str(&SourceType::Beads), "beads");
        assert_eq!(source_type_to_str(&SourceType::Json), "json");
        assert_eq!(source_type_to_str(&SourceType::Markdown), "markdown");
        assert_eq!(source_type_to_str(&SourceType::Github), "github");
    }

    #[test]
    fn test_source_add_beads() {
        let (_temp, config_path) = setup_temp_config();

        let result = source_add_impl("beads", None, Some(&config_path));
        assert!(result.is_ok());

        // Verify config was updated
        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::Beads);
    }

    #[test]
    fn test_source_add_json_with_path() {
        let (temp, config_path) = setup_temp_config();

        // Create the JSON file
        let json_path = temp.path().join("tasks.json");
        fs::write(&json_path, r#"[]"#).unwrap();

        let result = source_add_impl(
            "json",
            Some(json_path.to_str().unwrap()),
            Some(&config_path),
        );
        assert!(result.is_ok());

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::Json);
        assert!(config.sources[0].path.is_some());
    }

    #[test]
    fn test_source_add_json_file_not_found() {
        let (_temp, config_path) = setup_temp_config();

        let result = source_add_impl("json", Some("/nonexistent/path.json"), Some(&config_path));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SourceCommandError::FileNotFound(_)
        ));
    }

    #[test]
    fn test_source_add_markdown_file_not_found() {
        let (_temp, config_path) = setup_temp_config();

        let result = source_add_impl(
            "markdown",
            Some("/nonexistent/tasks.md"),
            Some(&config_path),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SourceCommandError::FileNotFound(_)
        ));
    }

    #[test]
    fn test_source_add_multiple_sources() {
        let (temp, config_path) = setup_temp_config();

        // Create files
        let json_path = temp.path().join("tasks.json");
        fs::write(&json_path, r#"[]"#).unwrap();
        let md_path = temp.path().join("TODO.md");
        fs::write(&md_path, "").unwrap();

        source_add_impl("beads", None, Some(&config_path)).unwrap();
        source_add_impl(
            "json",
            Some(json_path.to_str().unwrap()),
            Some(&config_path),
        )
        .unwrap();
        source_add_impl(
            "markdown",
            Some(md_path.to_str().unwrap()),
            Some(&config_path),
        )
        .unwrap();

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 3);
        assert_eq!(config.sources[0].source_type, SourceType::Beads);
        assert_eq!(config.sources[1].source_type, SourceType::Json);
        assert_eq!(config.sources[2].source_type, SourceType::Markdown);
    }

    #[test]
    fn test_source_list_empty() {
        let (_temp, config_path) = setup_temp_config();

        // Should not error even with no sources
        let result = source_list_impl(Some(&config_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_source_list_with_sources() {
        let (_temp, config_path) = setup_temp_config();

        // Add some sources directly to config
        let config = AfkConfig {
            sources: vec![SourceConfig::beads(), SourceConfig::json("tasks.json")],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = source_list_impl(Some(&config_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_source_remove_valid_index() {
        let (_temp, config_path) = setup_temp_config();

        // Add sources
        let config = AfkConfig {
            sources: vec![SourceConfig::beads(), SourceConfig::json("tasks.json")],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = source_remove_impl(1, Some(&config_path));
        assert!(result.is_ok());

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::Json);
    }

    #[test]
    fn test_source_remove_last_item() {
        let (_temp, config_path) = setup_temp_config();

        let config = AfkConfig {
            sources: vec![SourceConfig::beads(), SourceConfig::json("tasks.json")],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = source_remove_impl(2, Some(&config_path));
        assert!(result.is_ok());

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::Beads);
    }

    #[test]
    fn test_source_remove_index_too_high() {
        let (_temp, config_path) = setup_temp_config();

        let config = AfkConfig {
            sources: vec![SourceConfig::beads()],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = source_remove_impl(5, Some(&config_path));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SourceCommandError::InvalidIndex { index: 5, max: 1 }
        ));
    }

    #[test]
    fn test_source_remove_index_zero() {
        let (_temp, config_path) = setup_temp_config();

        let config = AfkConfig {
            sources: vec![SourceConfig::beads()],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        let result = source_remove_impl(0, Some(&config_path));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SourceCommandError::InvalidIndex { index: 0, max: 1 }
        ));
    }

    #[test]
    fn test_source_remove_no_sources() {
        let (_temp, config_path) = setup_temp_config();

        let result = source_remove_impl(1, Some(&config_path));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceCommandError::NoSources));
    }

    #[test]
    fn test_source_add_github() {
        let (_temp, config_path) = setup_temp_config();

        let result = source_add_impl("github", Some("owner/repo"), Some(&config_path));
        assert!(result.is_ok());

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::Github);
    }

    #[test]
    fn test_source_error_display() {
        let err = SourceCommandError::FileNotFound("/path/to/file.json".to_string());
        assert_eq!(err.to_string(), "File not found: /path/to/file.json");

        let err = SourceCommandError::InvalidSourceType("invalid".to_string());
        assert_eq!(err.to_string(), "Invalid source type: invalid");

        let err = SourceCommandError::InvalidIndex { index: 5, max: 3 };
        assert_eq!(err.to_string(), "Invalid index: 5. Must be 1-3");

        let err = SourceCommandError::NoSources;
        assert_eq!(err.to_string(), "No sources configured");
    }

    #[test]
    fn test_source_add_json_without_path_uses_default() {
        let (temp, config_path) = setup_temp_config();

        // Create the default PRD file path
        let prd_path = temp.path().join(".afk/tasks.json");
        fs::write(&prd_path, r#"[]"#).unwrap();

        // Note: Without changing cwd, the default path check won't find the file.
        // In this test, we just verify that when no path is given,
        // the source is created with the default path value.
        let result = source_add_impl("json", None, Some(&config_path));
        assert!(result.is_ok());

        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].path, Some(".afk/tasks.json".to_string()));
    }

    #[test]
    fn test_source_list_github_shows_repo() {
        let (_temp, config_path) = setup_temp_config();

        let config = AfkConfig {
            sources: vec![SourceConfig::github("owner/repo", vec!["bug".to_string()])],
            ..Default::default()
        };
        config.save(Some(&config_path)).unwrap();

        // This should not error and should print the repo
        let result = source_list_impl(Some(&config_path));
        assert!(result.is_ok());
    }
}
