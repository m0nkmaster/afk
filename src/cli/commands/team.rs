//! Team command implementation.
//!
//! This module implements the `afk team` command for running multiple
//! AI agents in parallel with supervised team mode.

use crate::bootstrap::ensure_ai_cli_configured;
use crate::config::AfkConfig;
use crate::runner::team::{TeamOptions, TeamRunner};

/// Result type for team command operations.
pub type TeamCommandResult = Result<(), TeamCommandError>;

/// Error type for team command operations.
#[derive(Debug, thiserror::Error)]
pub enum TeamCommandError {
    /// No AI CLI configured.
    #[error("No AI CLI configured")]
    NoAiCli,
    /// Team runner error.
    #[error("{0}")]
    TeamError(#[from] crate::runner::team::TeamError),
    /// Config error.
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

/// Options for the team command.
pub struct TeamCommandOptions {
    /// Number of parallel agents.
    pub num_agents: u32,
    /// Optional natural language prompt (quick mode).
    pub prompt: Option<String>,
    /// Max iterations per agent per task.
    pub max_iterations: Option<u32>,
    /// Use TUI dashboard (default: true).
    pub use_tui: bool,
}

/// Execute the team command.
pub fn team(options: TeamCommandOptions) -> TeamCommandResult {
    // Load config
    let mut config = AfkConfig::load(None).unwrap_or_default();

    // Ensure AI CLI is configured
    if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config), false) {
        config.ai_cli = ai_cli;
    } else {
        return Err(TeamCommandError::NoAiCli);
    }

    let max_iterations = options
        .max_iterations
        .unwrap_or(config.limits.max_iterations);

    let team_options = TeamOptions {
        num_agents: options.num_agents,
        max_iterations,
        prompt: options.prompt,
    };

    let mut runner = TeamRunner::new(config, team_options);

    if options.use_tui {
        runner.run_with_tui()?;
    } else {
        println!(
            "\x1b[1m◉ afk team\x1b[0m │ {} agents │ {} iterations/task\n",
            options.num_agents,
            options.max_iterations.unwrap_or(5)
        );
        runner.run()?;
    }

    Ok(())
}
