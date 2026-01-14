//! Config command implementations.
//!
//! This module implements the `afk config` subcommands for managing
//! configuration without editing JSON directly.

use crate::config::{
    metadata::{self, KeyMetadata},
    AfkConfig, FieldError,
};

/// Result type for config command operations.
pub type ConfigCommandResult = Result<(), ConfigCommandError>;

/// Error type for config command operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigCommandError {
    #[error("{0}")]
    FieldError(#[from] FieldError),

    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),

    #[error("Failed to open editor: {0}")]
    EditorError(String),

    #[error("No EDITOR environment variable set")]
    NoEditor,

    #[error("Unknown section: {0}")]
    UnknownSection(String),
}

/// Show all config values in a human-readable format.
pub fn config_show(section_filter: Option<&str>) -> ConfigCommandResult {
    let config = AfkConfig::load(None)?;

    println!("\x1b[1m=== afk config ===\x1b[0m");
    println!();

    // Determine which sections to show
    let sections: Vec<&str> = if let Some(filter) = section_filter {
        if AfkConfig::fields_for_section(filter).is_some() || filter == "sources" {
            vec![filter]
        } else {
            return Err(ConfigCommandError::UnknownSection(filter.into()));
        }
    } else {
        AfkConfig::section_names().to_vec()
    };

    for section in sections {
        println!("\x1b[36m{section}\x1b[0m");

        if section == "sources" {
            // Handle sources specially
            if config.sources.is_empty() {
                println!("  \x1b[2m(none configured)\x1b[0m");
            } else {
                for (i, src) in config.sources.iter().enumerate() {
                    let type_str = format!("{:?}", src.source_type).to_lowercase();
                    let path_info = src
                        .path
                        .as_ref()
                        .or(src.repo.as_ref())
                        .map(|p| format!(" ({p})"))
                        .unwrap_or_default();
                    println!("  {}. {}{}", i + 1, type_str, path_info);
                }
            }
        } else if let Some(fields) = AfkConfig::fields_for_section(section) {
            for field in fields {
                let path = format!("{section}.{field}");
                let value = config.get_by_path(&path).unwrap_or_else(|_| "?".into());
                println!("  {:<24} {}", field, value);
            }
        }
        println!();
    }

    Ok(())
}

/// Get a specific config value.
pub fn config_get(key: &str) -> ConfigCommandResult {
    let config = AfkConfig::load(None)?;
    let value = config.get_by_path(key)?;
    println!("{value}");
    Ok(())
}

/// Set a config value.
pub fn config_set(key: &str, value: &str) -> ConfigCommandResult {
    let mut config = AfkConfig::load(None)?;
    let old_value = config.get_by_path(key).ok();

    config.set_by_path(key, value)?;
    config.save(None)?;

    // Show what changed
    let new_value = config.get_by_path(key)?;
    if let Some(old) = old_value {
        if old != new_value {
            println!("\x1b[32m✓\x1b[0m {key}: {old} → {new_value}");
        } else {
            println!("\x1b[33m⚠\x1b[0m {key} unchanged: {new_value}");
        }
    } else {
        println!("\x1b[32m✓\x1b[0m {key} = {new_value}");
    }

    Ok(())
}

/// Reset config to defaults.
pub fn config_reset(key: Option<&str>) -> ConfigCommandResult {
    let mut config = AfkConfig::load(None)?;

    match key {
        Some(k) => {
            // Check if it's a section or a field
            if AfkConfig::fields_for_section(k).is_some() {
                config.reset_section(k)?;
                println!("\x1b[32m✓\x1b[0m Reset section '{k}' to defaults");
            } else if k.contains('.') {
                config.reset_field(k)?;
                let new_value = config.get_by_path(k)?;
                println!("\x1b[32m✓\x1b[0m Reset {k} to default: {new_value}");
            } else {
                return Err(ConfigCommandError::UnknownSection(k.into()));
            }
        }
        None => {
            config = AfkConfig::default();
            println!("\x1b[32m✓\x1b[0m Reset all config to defaults");
        }
    }

    config.save(None)?;
    Ok(())
}

/// Open config file in editor.
pub fn config_edit() -> ConfigCommandResult {
    use std::process::Command;

    let editor = std::env::var("EDITOR").map_err(|_| ConfigCommandError::NoEditor)?;
    let config_path = AfkConfig::config_file();

    // Ensure config file exists
    if !config_path.exists() {
        let config = AfkConfig::default();
        config.save(None)?;
        println!(
            "\x1b[2mCreated default config at {}\x1b[0m",
            config_path.display()
        );
    }

    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| ConfigCommandError::EditorError(e.to_string()))?;

    if status.success() {
        // Validate the edited config
        match AfkConfig::load(None) {
            Ok(_) => println!("\x1b[32m✓\x1b[0m Config saved and validated"),
            Err(e) => {
                eprintln!("\x1b[31mWarning:\x1b[0m Config may be invalid: {e}");
            }
        }
    } else {
        eprintln!("\x1b[31mEditor exited with error\x1b[0m");
    }

    Ok(())
}

/// Show documentation for config keys.
pub fn config_explain(key: Option<&str>) -> ConfigCommandResult {
    match key {
        Some(k) => explain_key(k),
        None => explain_all(),
    }
}

fn explain_key(key: &str) -> ConfigCommandResult {
    // Try exact match first
    if let Some(meta) = metadata::get_metadata(key) {
        print_key_help(meta)?;
        return Ok(());
    }

    // Check if it's a section
    if AfkConfig::fields_for_section(key).is_some() {
        let section_keys = metadata::keys_for_section(key);
        println!("\x1b[1m{key}\x1b[0m section");
        println!();
        for meta in section_keys {
            let field = meta
                .key
                .strip_prefix(&format!("{key}."))
                .unwrap_or(meta.key);
            println!("  \x1b[36m{field}\x1b[0m");
            println!("    {}", meta.description.lines().next().unwrap_or(""));
            println!();
        }
        return Ok(());
    }

    // Try fuzzy search
    let matches = metadata::search_keys(key);
    if matches.is_empty() {
        eprintln!("\x1b[31mUnknown key:\x1b[0m {key}");
        eprintln!();
        suggest_similar_keys(key);
        return Err(ConfigCommandError::FieldError(FieldError::UnknownKey(
            key.into(),
        )));
    }

    println!("\x1b[33mDid you mean:\x1b[0m");
    for meta in matches.iter().take(5) {
        println!("  {}", meta.key);
    }
    Ok(())
}

fn explain_all() -> ConfigCommandResult {
    println!("\x1b[1m=== afk config keys ===\x1b[0m");
    println!();
    println!("Use \x1b[36mafk config explain <key>\x1b[0m for details.");
    println!();

    for &section in AfkConfig::section_names() {
        if section == "sources" {
            println!("\x1b[1m{section}\x1b[0m");
            println!("  (managed via \x1b[36mafk source\x1b[0m commands)");
            println!();
            continue;
        }

        println!("\x1b[1m{section}\x1b[0m");
        if let Some(fields) = AfkConfig::fields_for_section(section) {
            for field in fields {
                let path = format!("{section}.{field}");
                if let Some(meta) = metadata::get_metadata(&path) {
                    // Truncate description to first sentence or 60 chars
                    let desc = meta
                        .description
                        .split('.')
                        .next()
                        .unwrap_or(meta.description);
                    let desc = if desc.len() > 55 {
                        format!("{}...", &desc[..52])
                    } else {
                        desc.to_string()
                    };
                    println!("  \x1b[36m{field}\x1b[0m");
                    println!("    {desc}");
                } else {
                    println!("  \x1b[36m{field}\x1b[0m");
                }
            }
        }
        println!();
    }

    Ok(())
}

fn print_key_help(meta: &KeyMetadata) -> ConfigCommandResult {
    let config = AfkConfig::load(None).ok();
    let current = config.as_ref().and_then(|c| c.get_by_path(meta.key).ok());

    println!("\x1b[1m{}\x1b[0m", meta.key);
    println!();
    println!("  {}", meta.description);
    println!();
    println!("  \x1b[2mType:\x1b[0m     {}", meta.value_type);
    println!("  \x1b[2mDefault:\x1b[0m  {}", meta.default);
    if let Some(curr) = current {
        let is_default = curr == meta.default;
        if is_default {
            println!("  \x1b[2mCurrent:\x1b[0m  {} \x1b[2m(default)\x1b[0m", curr);
        } else {
            println!("  \x1b[2mCurrent:\x1b[0m  \x1b[33m{}\x1b[0m", curr);
        }
    }
    println!();
    println!("  \x1b[2mExamples:\x1b[0m");
    for example in meta.examples {
        println!("    afk config set {} {}", meta.key, example);
    }

    Ok(())
}

fn suggest_similar_keys(key: &str) {
    // Simple suggestion: find keys that share a common prefix or contain parts
    let parts: Vec<&str> = key.split('.').collect();
    let suggestions: Vec<_> = metadata::all_keys()
        .filter(|k| {
            parts.iter().any(|p| k.contains(p))
                || k.split('.').any(|kp| parts.iter().any(|p| kp.contains(p)))
        })
        .take(5)
        .collect();

    if !suggestions.is_empty() {
        eprintln!("\x1b[33mSimilar keys:\x1b[0m");
        for s in suggestions {
            eprintln!("  {s}");
        }
    } else {
        eprintln!("Run \x1b[36mafk config explain\x1b[0m to see all available keys.");
    }
}

/// List all valid config keys.
pub fn config_keys() -> ConfigCommandResult {
    for key in AfkConfig::all_keys() {
        println!("{key}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[allow(dead_code)]
    fn setup_temp_config() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let config_path = afk_dir.join("config.json");
        (temp, config_path)
    }

    #[test]
    fn test_config_get_valid_key() {
        // This test uses the default config loading which may not find a file
        // In practice, this would use a temp config
        let config = AfkConfig::default();
        let value = config.get_by_path("limits.max_iterations").unwrap();
        assert_eq!(value, "200");
    }

    #[test]
    fn test_config_get_invalid_key() {
        let config = AfkConfig::default();
        let result = config.get_by_path("nonexistent.key");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_set_and_get() {
        let mut config = AfkConfig::default();
        config.set_by_path("limits.max_iterations", "50").unwrap();
        let value = config.get_by_path("limits.max_iterations").unwrap();
        assert_eq!(value, "50");
    }

    #[test]
    fn test_config_set_invalid_value() {
        let mut config = AfkConfig::default();
        let result = config.set_by_path("limits.max_iterations", "not-a-number");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_reset_section() {
        let mut config = AfkConfig::default();
        config.set_by_path("limits.max_iterations", "50").unwrap();
        config.reset_section("limits").unwrap();
        let value = config.get_by_path("limits.max_iterations").unwrap();
        assert_eq!(value, "200"); // Back to default
    }

    #[test]
    fn test_config_reset_field() {
        let mut config = AfkConfig::default();
        config.set_by_path("limits.max_iterations", "50").unwrap();
        config.reset_field("limits.max_iterations").unwrap();
        let value = config.get_by_path("limits.max_iterations").unwrap();
        assert_eq!(value, "200"); // Back to default
    }

    #[test]
    fn test_config_all_keys() {
        let keys = AfkConfig::all_keys();
        assert!(!keys.is_empty());
        assert!(keys.contains(&"limits.max_iterations".to_string()));
        assert!(keys.contains(&"ai_cli.command".to_string()));
    }

    #[test]
    fn test_config_sections() {
        let sections = AfkConfig::section_names();
        assert!(sections.contains(&"limits"));
        assert!(sections.contains(&"ai_cli"));
        assert!(sections.contains(&"git"));
    }

    #[test]
    fn test_config_fields_for_section() {
        let fields = AfkConfig::fields_for_section("limits").unwrap();
        assert!(fields.contains(&"max_iterations"));
        assert!(fields.contains(&"max_task_failures"));
        assert!(fields.contains(&"timeout_minutes"));
    }

    #[test]
    fn test_config_bool_values() {
        let mut config = AfkConfig::default();

        // Test various boolean representations
        config.set_by_path("git.auto_commit", "false").unwrap();
        assert_eq!(config.get_by_path("git.auto_commit").unwrap(), "false");

        config.set_by_path("git.auto_commit", "true").unwrap();
        assert_eq!(config.get_by_path("git.auto_commit").unwrap(), "true");

        config.set_by_path("git.auto_commit", "yes").unwrap();
        assert_eq!(config.get_by_path("git.auto_commit").unwrap(), "true");

        config.set_by_path("git.auto_commit", "no").unwrap();
        assert_eq!(config.get_by_path("git.auto_commit").unwrap(), "false");
    }

    #[test]
    fn test_config_enum_values() {
        let mut config = AfkConfig::default();

        config.set_by_path("output.default", "clipboard").unwrap();
        assert_eq!(config.get_by_path("output.default").unwrap(), "clipboard");

        config.set_by_path("output.default", "file").unwrap();
        assert_eq!(config.get_by_path("output.default").unwrap(), "file");

        config.set_by_path("output.default", "stdout").unwrap();
        assert_eq!(config.get_by_path("output.default").unwrap(), "stdout");
    }

    #[test]
    fn test_config_array_values() {
        let mut config = AfkConfig::default();

        config.set_by_path("ai_cli.args", "-p, --force").unwrap();
        let value = config.get_by_path("ai_cli.args").unwrap();
        assert!(value.contains("-p"));
        assert!(value.contains("--force"));

        config.set_by_path("ai_cli.args", "(none)").unwrap();
        let value = config.get_by_path("ai_cli.args").unwrap();
        assert_eq!(value, "(none)");
    }
}
