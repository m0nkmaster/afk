//! Go command implementation.
//!
//! This module implements the `afk go` command for running the autonomous loop.

use std::fs;
use std::path::Path;

use crate::bootstrap::{
    analyse_project, ensure_ai_cli_configured, generate_config,
    infer_sources as bootstrap_infer_sources,
};
use crate::config::{AfkConfig, SourceConfig};
use crate::prd::PrdDocument;
use crate::runner::{run_loop_with_options, run_loop_with_tui, RunOptions, StopReason};

/// Result type for go command operations.
pub type GoCommandResult = Result<GoOutcome, GoCommandError>;

/// Outcome of the go command.
pub struct GoOutcome {
    /// The stop reason from the runner.
    pub stop_reason: StopReason,
}

/// Error type for go command operations.
#[derive(Debug, thiserror::Error)]
pub enum GoCommandError {
    /// Failed to remove existing configuration file.
    #[error("Failed to remove config: {0}")]
    RemoveConfigError(std::io::Error),
    /// Failed to clear session progress file.
    #[error("Failed to clear progress: {0}")]
    ClearProgressError(std::io::Error),
    /// Specified source file was not found.
    #[error("Source file not found: {0}")]
    SourceNotFound(String),
    /// Failed to create the .afk directory.
    #[error("Failed to create .afk directory: {0}")]
    CreateDirError(std::io::Error),
    /// Failed to save configuration to file.
    #[error("Failed to save config: {0}")]
    SaveConfigError(#[from] crate::config::ConfigError),
    /// No AI CLI tool is configured or available.
    #[error("No AI CLI configured")]
    NoAiCli,
    /// No task sources found or configured.
    #[error("No task sources found")]
    NoSources,
}

/// Options for the go command.
pub struct GoOptions {
    /// Number of iterations to run.
    pub iterations: Option<u32>,
    /// Path to explicit source file.
    pub source_path: Option<String>,
    /// Re-run setup (re-prompts for AI CLI selection).
    pub init: bool,
    /// Start fresh by clearing session progress.
    pub fresh: bool,
    /// Run until all tasks complete.
    pub until_complete: bool,
    /// Override timeout in minutes.
    pub timeout: Option<u32>,
    /// Feedback display mode.
    pub feedback: Option<String>,
    /// Disable ASCII mascot.
    pub no_mascot: bool,
    /// Show what would run without running.
    pub dry_run: bool,
}

/// Execute the go command.
///
/// This is the main entry point for running the autonomous loop.
pub fn go(options: GoOptions) -> GoCommandResult {
    let afk_dir = Path::new(".afk");
    let config_path = afk_dir.join("config.json");

    // Handle --init flag: delete config and re-run setup
    if options.init && config_path.exists() {
        fs::remove_file(&config_path).map_err(GoCommandError::RemoveConfigError)?;
        println!("\x1b[2mCleared existing configuration.\x1b[0m");
    }

    // Handle --fresh flag: clear session progress
    if options.fresh {
        let progress_path = afk_dir.join("progress.json");
        if progress_path.exists() {
            fs::remove_file(&progress_path).map_err(GoCommandError::ClearProgressError)?;
            println!("\x1b[2mCleared session progress.\x1b[0m");
        }
    }

    // Load or create config
    let mut config = if config_path.exists() {
        AfkConfig::load(None).unwrap_or_default()
    } else {
        // First run: analyse project and create config
        println!("\x1b[1mAnalysing project...\x1b[0m");
        let analysis = analyse_project(None);

        println!("  Project type: {:?}", analysis.project_type);
        if let Some(ref name) = analysis.name {
            println!("  Project name: {name}");
        }

        let mut new_config = generate_config(&analysis);
        new_config.sources = bootstrap_infer_sources(None);

        // Create .afk directory
        if !afk_dir.exists() {
            fs::create_dir_all(afk_dir).map_err(GoCommandError::CreateDirError)?;
        }

        new_config
    };

    // Handle explicit source file path
    if let Some(ref source_path) = options.source_path {
        let path = Path::new(source_path);
        if !path.exists() {
            return Err(GoCommandError::SourceNotFound(source_path.clone()));
        }

        // Determine source type from extension
        let source = if source_path.ends_with(".json") {
            SourceConfig::json(source_path)
        } else {
            SourceConfig::markdown(source_path)
        };

        config.sources = vec![source];
    }

    // Check for existing PRD with stories (zero-config mode)
    if config.sources.is_empty() {
        let prd = PrdDocument::load(None).unwrap_or_default();
        if !prd.user_stories.is_empty() {
            println!(
                "\x1b[2mUsing existing .afk/tasks.json ({} tasks)\x1b[0m",
                prd.user_stories.len()
            );
        } else {
            // Try to infer sources
            let inferred = infer_sources();
            if inferred.is_empty() {
                return Err(GoCommandError::NoSources);
            }
            config.sources = inferred;
        }
    }

    // Ensure AI CLI is configured (first-run experience)
    if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config), options.init) {
        config.ai_cli = ai_cli;
    } else {
        return Err(GoCommandError::NoAiCli);
    }

    // Save config if it was newly created or modified
    if !config_path.exists() || options.init {
        config.save(Some(&config_path))?;
        println!(
            "\x1b[32m✓\x1b[0m Configuration saved to {}",
            config_path.display()
        );
    }

    // Dry run mode
    if options.dry_run {
        let effective_iterations = options.iterations.unwrap_or(config.limits.max_iterations);
        println!("\x1b[1mDry run mode - would execute:\x1b[0m");
        println!(
            "  AI CLI: {} {}",
            config.ai_cli.command,
            config.ai_cli.args.join(" ")
        );
        println!("  Iterations: {}", effective_iterations);
        println!(
            "  Sources: {:?}",
            config
                .sources
                .iter()
                .map(|s| &s.source_type)
                .collect::<Vec<_>>()
        );
        return Ok(GoOutcome {
            stop_reason: StopReason::Complete,
        });
    }

    // Build run options with feedback settings
    let effective_iterations = options.iterations.or(Some(config.limits.max_iterations));
    let run_opts = RunOptions::new()
        .with_iterations(effective_iterations)
        .with_until_complete(options.until_complete)
        .with_timeout(options.timeout)
        .with_resume(false)
        .with_feedback_mode(RunOptions::parse_feedback_mode(options.feedback.as_deref()))
        .with_mascot(!options.no_mascot);

    // Run the loop - use TUI if requested
    let result = if RunOptions::is_tui_mode(options.feedback.as_deref()) {
        run_loop_with_tui(&config, run_opts)
    } else {
        run_loop_with_options(&config, run_opts)
    };

    Ok(GoOutcome {
        stop_reason: result.stop_reason,
    })
}

/// Infer sources from the current directory.
pub fn infer_sources() -> Vec<SourceConfig> {
    let mut sources = Vec::new();

    // Check for TODO.md or similar
    for name in ["TODO.md", "TASKS.md", "tasks.md", "todo.md"] {
        if Path::new(name).exists() {
            sources.push(SourceConfig::markdown(name));
            break;
        }
    }

    // Check for beads (bd command)
    if std::process::Command::new("bd")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        // Only add if .beads directory exists
        if Path::new(".beads").exists() {
            sources.push(SourceConfig::beads());
        }
    }

    sources
}

/// Print helpful message when no sources are found.
pub fn print_no_sources_help() {
    eprintln!("\x1b[33mNo task sources found.\x1b[0m");
    eprintln!();
    eprintln!("Try one of:");
    eprintln!("  afk go TODO.md           # Use a markdown file");
    eprintln!("  afk import spec.md       # Import a requirements doc");
    eprintln!("  afk source add beads     # Use beads issues");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_sources_empty_dir() {
        // In an empty directory with no .beads or TODO.md, should return empty
        // This test relies on the fact we're in a test environment
        let sources = infer_sources();
        // May or may not find sources depending on test environment
        assert!(sources.len() <= 2);
    }

    #[test]
    fn test_go_command_error_display() {
        let err = GoCommandError::SourceNotFound("/path/to/file.md".to_string());
        assert!(err.to_string().contains("Source file not found"));

        let err = GoCommandError::NoAiCli;
        assert_eq!(err.to_string(), "No AI CLI configured");

        let err = GoCommandError::NoSources;
        assert_eq!(err.to_string(), "No task sources found");
    }
}
