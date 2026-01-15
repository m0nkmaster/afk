//! Init command implementation.
//!
//! This module implements the `afk init` command for project initialisation.

use std::fs;
use std::path::Path;

use crate::bootstrap::{
    analyse_project, detect_ai_cli, ensure_ai_cli_configured, generate_config, infer_sources,
};

/// Result type for init command operations.
pub type InitCommandResult = Result<(), InitCommandError>;

/// Error type for init command operations.
#[derive(Debug, thiserror::Error)]
pub enum InitCommandError {
    /// Project is already initialised with .afk directory.
    #[error("Already initialised. Use --force to reinitialise.")]
    AlreadyInitialised,
    /// Failed to create the .afk directory.
    #[error("Failed to create .afk directory: {0}")]
    CreateDirError(std::io::Error),
    /// Failed to save configuration file.
    #[error("Failed to save config: {0}")]
    SaveConfigError(#[from] crate::config::ConfigError),
    /// Failed to create tasks.json file.
    #[error("Failed to create tasks.json: {0}")]
    CreateTasksError(std::io::Error),
    /// No AI CLI tool is configured or available.
    #[error("No AI CLI configured")]
    NoAiCli,
}

/// Options for the init command.
pub struct InitOptions {
    /// Show what would be configured without writing.
    pub dry_run: bool,
    /// Re-initialise existing project.
    pub force: bool,
    /// Accept all defaults without prompting.
    pub yes: bool,
}

/// Execute the init command.
pub fn init(options: InitOptions) -> InitCommandResult {
    let afk_dir = Path::new(".afk");
    let config_path = afk_dir.join("config.json");

    // Check if already initialised
    if config_path.exists() && !options.force {
        return Err(InitCommandError::AlreadyInitialised);
    }

    // Analyse project
    println!("\x1b[1mAnalysing project...\x1b[0m");
    let analysis = analyse_project(None);

    println!("  Project type: {:?}", analysis.project_type);
    if let Some(ref name) = analysis.name {
        println!("  Project name: {name}");
    }
    if let Some(ref pm) = analysis.package_manager {
        println!("  Package manager: {pm}");
    }

    // Generate config
    let mut config = generate_config(&analysis);
    config.sources = infer_sources(None);

    // Handle AI CLI selection
    if options.dry_run {
        if let Some(ai_cli) = detect_ai_cli() {
            config.ai_cli = ai_cli;
        }
    } else if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config), options.force) {
        config.ai_cli = ai_cli;
    } else {
        return Err(InitCommandError::NoAiCli);
    }

    // Show what would be written
    println!("\n\x1b[1mConfiguration:\x1b[0m");
    println!(
        "  AI CLI: {} {}",
        config.ai_cli.command,
        config.ai_cli.args.join(" ")
    );
    println!(
        "  Sources: {:?}",
        config
            .sources
            .iter()
            .map(|s| &s.source_type)
            .collect::<Vec<_>>()
    );
    if let Some(ref cmd) = config.feedback_loops.test {
        println!("  Test: {cmd}");
    }
    if let Some(ref cmd) = config.feedback_loops.lint {
        println!("  Lint: {cmd}");
    }

    // Dry run mode
    if options.dry_run {
        println!("\n\x1b[2mDry run - no files written.\x1b[0m");
        return Ok(());
    }

    // Create .afk directory
    fs::create_dir_all(afk_dir).map_err(InitCommandError::CreateDirError)?;

    // Write config
    config.save(Some(&config_path))?;

    // Create empty tasks.json
    let tasks_path = afk_dir.join("tasks.json");
    if !tasks_path.exists() {
        let empty_tasks = r#"{
  "project": "",
  "branchName": "",
  "description": "",
  "userStories": []
}"#;
        fs::write(&tasks_path, empty_tasks).map_err(InitCommandError::CreateTasksError)?;
    }

    println!("\n\x1b[32m✓ Initialised afk\x1b[0m");
    println!("  Config: {}", config_path.display());

    // Suggest next steps
    println!("\n\x1b[1mNext steps:\x1b[0m");
    if config.sources.is_empty() {
        println!("  1. Add a task source:");
        println!("     afk source add beads      # Use beads issues");
        println!("     afk import spec.md        # Import a requirements doc");
    } else {
        println!("  1. afk go   # Start working through tasks");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_command_error_display() {
        let err = InitCommandError::AlreadyInitialised;
        assert!(err.to_string().contains("Already initialised"));

        let err = InitCommandError::NoAiCli;
        assert_eq!(err.to_string(), "No AI CLI configured");
    }
}
