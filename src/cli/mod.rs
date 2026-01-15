//! CLI commands and argument handling.
//!
//! This module contains the clap CLI definitions. Command implementations
//! are in the `commands` submodule.

pub mod commands;
pub mod output;
pub mod update;

use clap::{Args, Parser, Subcommand};
use std::fmt;

// ============================================================================
// Exit codes and error types for testable command execution
// ============================================================================

/// Exit code for CLI commands.
///
/// Wraps a u8 exit code for returning from execute() methods instead of
/// calling `std::process::exit()` directly. This improves testability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(pub u8);

impl ExitCode {
    /// Success exit code (0).
    pub const SUCCESS: ExitCode = ExitCode(0);
    /// Failure exit code (1).
    pub const FAILURE: ExitCode = ExitCode(1);
    /// User interrupt exit code (130 = 128 + SIGINT).
    pub const INTERRUPT: ExitCode = ExitCode(130);
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> Self {
        std::process::ExitCode::from(code.0)
    }
}

/// Unified error type for CLI command execution.
///
/// Wraps command-specific errors and provides consistent error handling
/// across all CLI commands.
#[derive(Debug)]
pub enum CliError {
    /// General command error with message.
    Command(String),
    /// No task sources found (special case for `go` command).
    NoSources,
    /// Already initialised (special case for `init` command).
    AlreadyInitialised,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Command(msg) => write!(f, "{msg}"),
            CliError::NoSources => write!(f, "No task sources found"),
            CliError::AlreadyInitialised => write!(f, "Already initialised"),
        }
    }
}

impl std::error::Error for CliError {}

/// Type alias for CLI command results.
pub type CliResult = Result<ExitCode, CliError>;

/// Handle a CLI result by printing errors and returning the appropriate exit code.
///
/// This centralises the error handling that was previously scattered across
/// individual execute() methods.
pub fn handle_result(result: CliResult) -> std::process::ExitCode {
    match result {
        Ok(code) => code.into(),
        Err(CliError::NoSources) => {
            commands::go::print_no_sources_help();
            std::process::ExitCode::from(1)
        }
        Err(CliError::AlreadyInitialised) => {
            eprintln!("\x1b[33mAlready initialised.\x1b[0m Use --force to reinitialise.");
            std::process::ExitCode::from(0) // Not a failure, just informational
        }
        Err(CliError::Command(msg)) => {
            eprintln!("\x1b[31mError:\x1b[0m {msg}");
            std::process::ExitCode::from(1)
        }
    }
}

/// Autonomous AI coding loops - Ralph Wiggum style.
///
/// Run AI coding tasks in a loop with fresh context each iteration.
/// Memory persists via git history, progress.json, and task sources.
#[derive(Parser, Debug)]
#[command(name = "afk")]
#[command(author, version = crate::VERSION, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Top-level commands for afk.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the loop with zero config.
    ///
    /// Auto-detects project type, available tools, and task sources.
    /// On first run, prompts to confirm AI CLI selection.
    /// Always continues from previous session if one exists.
    ///
    /// Examples:
    ///   afk go                 # Auto-detect, run 10 iterations
    ///   afk go 20              # Run 20 iterations
    ///   afk go -u              # Run until all tasks complete
    ///   afk go TODO.md 5       # Use TODO.md as source, run 5 iterations
    ///   afk go --init          # Re-run setup, then run
    Go(GoCommand),

    /// Initialise afk by analysing the project.
    ///
    /// Detects project type, available tools, task sources, and context files
    /// to generate a sensible configuration.
    Init(InitCommand),

    /// Show current status and tasks.
    ///
    /// Use -v for verbose output including learnings.
    Status(StatusCommand),

    /// Show details of a specific task.
    ///
    /// Displays full task information including acceptance criteria and learnings.
    Task(TaskCommand),

    /// Preview the prompt for the next iteration.
    ///
    /// Shows what will be sent to the AI CLI on the next iteration.
    Prompt(PromptCommand),

    /// Run quality gates and report results.
    ///
    /// Runs all configured feedback loops (types, lint, test, build) and reports
    /// pass/fail status. Use this before marking a story as complete.
    Verify(VerifyCommand),

    /// Mark a task as complete.
    Done(DoneCommand),

    /// Mark a task as failed.
    Fail(FailCommand),

    /// Reset a stuck task to pending state.
    ///
    /// Clears failure count and sets status back to pending.
    Reset(ResetCommand),

    /// Manage task sources.
    #[command(subcommand)]
    Source(SourceCommands),

    /// Import a requirements document into structured JSON tasks.
    ///
    /// Takes a product requirements document (markdown, text, etc.) and runs the
    /// configured AI CLI to convert it into structured JSON format in .afk/tasks.json.
    ///
    /// By default, runs the AI CLI directly. Use --stdout, --copy, or --file
    /// to output the prompt for manual use instead.
    Import(ImportCommand),

    /// List and manage tasks.
    ///
    /// Shows tasks from .afk/tasks.json. Use `afk tasks sync` to aggregate from sources.
    #[command(subcommand_required = false, args_conflicts_with_subcommands = true)]
    Tasks {
        /// Tasks subcommand (sync) or list tasks if omitted.
        #[command(subcommand)]
        command: Option<TasksCommands>,

        /// Show only pending tasks.
        #[arg(short = 'p', long)]
        pending: bool,

        /// Show only completed tasks.
        #[arg(long)]
        complete: bool,

        /// Maximum number of tasks to show.
        #[arg(short = 'l', long, default_value = "50")]
        limit: usize,
    },

    /// Sync tasks from configured sources.
    ///
    /// Alias for `afk tasks sync`. Aggregates tasks from all sources
    /// into .afk/tasks.json.
    Sync,

    /// Archive and clear current session.
    ///
    /// Moves tasks.json and progress.json to a timestamped archive directory,
    /// clearing the session ready for fresh work. Use `afk archive list` to
    /// view archived sessions.
    #[command(subcommand_required = false, args_conflicts_with_subcommands = true)]
    Archive {
        /// Archive subcommand (list) or archive now if omitted.
        #[command(subcommand)]
        command: Option<ArchiveCommands>,

        /// Reason for archiving.
        #[arg(short = 'r', long, default_value = "manual")]
        reason: String,

        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Manage afk configuration.
    ///
    /// View, set, and understand config parameters without editing JSON directly.
    /// Running `afk config` without a subcommand shows all current settings.
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Update afk to the latest version.
    ///
    /// Downloads and installs the latest release from GitHub.
    Update(UpdateCommand),

    /// Generate shell completions.
    ///
    /// Outputs completion script to stdout for bash, zsh, or fish.
    Completions(CompletionsCommand),

    /// Switch the AI CLI used by afk.
    ///
    /// Quickly switch between AI CLI tools (claude, cursor, codex, etc.)
    /// with appropriate default arguments.
    ///
    /// Examples:
    ///   afk use claude    # Switch to Claude Code
    ///   afk use cursor    # Switch to Cursor agent
    ///   afk use           # Interactive selection
    ///   afk use --list    # Show available CLIs
    Use(UseCommand),
}

/// Arguments for the 'go' command.
#[derive(Args, Debug)]
pub struct GoCommand {
    /// Number of iterations to run, or path to a source file.
    ///
    /// If a number, sets the iteration limit.
    /// If a path (e.g., TODO.md), uses it as the task source.
    #[arg(value_name = "ITERATIONS_OR_SOURCE")]
    pub iterations_or_source: Option<String>,

    /// Number of iterations when first argument is a source path.
    #[arg(value_name = "ITERATIONS")]
    pub iterations_if_source: Option<u32>,

    /// Show what would run without running.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Run until all tasks complete.
    #[arg(short = 'u', long)]
    pub until_complete: bool,

    /// Re-run setup (re-prompts for AI CLI selection).
    ///
    /// Deletes existing .afk/config.json and prompts for full reconfiguration
    /// including AI CLI selection.
    #[arg(long)]
    pub init: bool,

    /// Start fresh by clearing any existing session progress.
    ///
    /// Deletes .afk/progress.json before running, giving a clean slate.
    #[arg(long)]
    pub fresh: bool,

    /// Override timeout in minutes.
    #[arg(short = 't', long)]
    pub timeout: Option<u32>,

    /// Feedback display mode.
    ///
    /// Options: tui (rich dashboard), full, minimal, off
    #[arg(long, value_parser = ["tui", "full", "minimal", "off"], default_value = "tui")]
    pub feedback: Option<String>,

    /// Disable ASCII mascot in feedback display.
    #[arg(long)]
    pub no_mascot: bool,
}

/// Arguments for the 'init' command.
#[derive(Args, Debug)]
pub struct InitCommand {
    /// Show what would be configured without writing.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Re-initialise existing project (re-prompts for AI CLI).
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Accept all defaults without prompting.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for the 'status' command.
#[derive(Args, Debug)]
pub struct StatusCommand {
    /// Show verbose output including learnings.
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

/// Arguments for the 'task' command.
#[derive(Args, Debug)]
pub struct TaskCommand {
    /// Task ID to show details for.
    pub task_id: String,
}

/// Subcommands for source management.
#[derive(Subcommand, Debug)]
pub enum SourceCommands {
    /// Add a task source.
    Add(SourceAddCommand),

    /// List configured task sources.
    List(SourceListCommand),

    /// Remove a task source by index (1-based).
    Remove(SourceRemoveCommand),
}

/// Arguments for 'source add' command.
#[derive(Args, Debug)]
pub struct SourceAddCommand {
    /// Type of source to add.
    #[arg(value_parser = ["beads", "json", "markdown", "github"])]
    pub source_type: String,

    /// Path to the source file (for json/markdown types).
    pub path: Option<String>,
}

/// Arguments for 'source list' command.
#[derive(Args, Debug)]
pub struct SourceListCommand {}

/// Arguments for 'source remove' command.
#[derive(Args, Debug)]
pub struct SourceRemoveCommand {
    /// Index of source to remove (1-based).
    pub index: usize,
}

/// Subcommands for task list management.
#[derive(Subcommand, Debug)]
pub enum TasksCommands {
    /// Sync tasks from all configured sources.
    ///
    /// Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
    /// .afk/tasks.json file.
    Sync(TasksSyncCommand),
}

/// Subcommands for config management.
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show all configuration values.
    ///
    /// Displays all config sections and their current values.
    Show(ConfigShowCommand),

    /// Get a specific config value.
    ///
    /// Use dot notation for nested keys (e.g., limits.max_iterations).
    Get(ConfigGetCommand),

    /// Set a config value.
    ///
    /// Use dot notation for keys. Values are validated before saving.
    Set(ConfigSetCommand),

    /// Reset config to defaults.
    ///
    /// Can reset a specific key, a section, or all config.
    Reset(ConfigResetCommand),

    /// Open config file in your editor.
    ///
    /// Uses $EDITOR environment variable.
    Edit(ConfigEditCommand),

    /// Show documentation for config keys.
    ///
    /// Displays description, type, default value, and examples.
    Explain(ConfigExplainCommand),

    /// List all valid config keys.
    Keys(ConfigKeysCommand),
}

/// Arguments for 'config show' command.
#[derive(Args, Debug)]
pub struct ConfigShowCommand {
    /// Filter to a specific section (e.g., 'limits', 'ai_cli').
    #[arg(short = 's', long)]
    pub section: Option<String>,
}

/// Arguments for 'config get' command.
#[derive(Args, Debug)]
pub struct ConfigGetCommand {
    /// Config key in dot notation (e.g., limits.max_iterations).
    pub key: String,
}

/// Arguments for 'config set' command.
#[derive(Args, Debug)]
pub struct ConfigSetCommand {
    /// Config key in dot notation (e.g., limits.max_iterations).
    pub key: String,

    /// Value to set.
    pub value: String,
}

/// Arguments for 'config reset' command.
#[derive(Args, Debug)]
pub struct ConfigResetCommand {
    /// Key or section to reset. If omitted, resets all config.
    pub key: Option<String>,

    /// Skip confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for 'config edit' command.
#[derive(Args, Debug)]
pub struct ConfigEditCommand {}

/// Arguments for 'config explain' command.
#[derive(Args, Debug)]
pub struct ConfigExplainCommand {
    /// Config key to explain. If omitted, lists all keys with brief descriptions.
    pub key: Option<String>,
}

/// Arguments for 'config keys' command.
#[derive(Args, Debug)]
pub struct ConfigKeysCommand {}

/// Arguments for 'import' command.
#[derive(Args, Debug)]
pub struct ImportCommand {
    /// Input file to import.
    pub input_file: String,

    /// Output JSON path.
    #[arg(short = 'o', long, default_value = ".afk/tasks.json")]
    pub output: String,

    /// Copy to clipboard.
    #[arg(short = 'c', long)]
    pub copy: bool,

    /// Write prompt to file.
    #[arg(short = 'f', long)]
    pub file: bool,

    /// Print prompt to stdout.
    #[arg(short = 's', long)]
    pub stdout: bool,
}

/// Arguments for 'tasks sync' command.
#[derive(Args, Debug)]
pub struct TasksSyncCommand {
    /// Archive completed tasks before syncing (fresh start).
    #[arg(short = 'r', long)]
    pub reset: bool,
}

/// Arguments for the 'prompt' command.
#[derive(Args, Debug)]
pub struct PromptCommand {
    /// Copy to clipboard.
    #[arg(short = 'c', long)]
    pub copy: bool,

    /// Write to file.
    #[arg(short = 'f', long)]
    pub file: bool,

    /// Print to stdout.
    #[arg(short = 's', long)]
    pub stdout: bool,

    /// Include afk command instructions for AI.
    #[arg(short = 'b', long)]
    pub bootstrap: bool,

    /// Override max iterations.
    #[arg(short = 'l', long)]
    pub limit: Option<u32>,
}

/// Arguments for the 'verify' command.
#[derive(Args, Debug)]
pub struct VerifyCommand {
    /// Show full output from failed gates.
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

/// Arguments for the 'done' command.
#[derive(Args, Debug)]
pub struct DoneCommand {
    /// Task ID to mark as complete.
    pub task_id: String,

    /// Completion message.
    #[arg(short = 'm', long)]
    pub message: Option<String>,
}

/// Arguments for the 'fail' command.
#[derive(Args, Debug)]
pub struct FailCommand {
    /// Task ID to mark as failed.
    pub task_id: String,

    /// Failure reason.
    #[arg(short = 'm', long)]
    pub message: Option<String>,
}

/// Arguments for the 'reset' command.
#[derive(Args, Debug)]
pub struct ResetCommand {
    /// Task ID to reset.
    pub task_id: String,
}

/// Subcommands for archive management.
#[derive(Subcommand, Debug)]
pub enum ArchiveCommands {
    /// List archived sessions.
    List,
}

/// Arguments for the 'update' command.
#[derive(Args, Debug)]
pub struct UpdateCommand {
    /// Update to beta channel (pre-releases).
    #[arg(long)]
    pub beta: bool,

    /// Check for updates without installing.
    #[arg(long)]
    pub check: bool,
}

/// Arguments for the 'completions' command.
#[derive(Args, Debug)]
pub struct CompletionsCommand {
    /// Shell to generate completions for.
    #[arg(value_parser = ["bash", "zsh", "fish"])]
    pub shell: String,
}

/// Arguments for the 'use' command.
#[derive(Args, Debug)]
pub struct UseCommand {
    /// AI CLI to switch to (claude, cursor, codex, aider, amp, kiro).
    ///
    /// If omitted, prompts for interactive selection.
    pub cli: Option<String>,

    /// List all known AI CLIs with installation status.
    #[arg(short = 'l', long)]
    pub list: bool,
}

// ============================================================================
// Command implementations - thin wrappers around commands/ modules
// ============================================================================

impl GoCommand {
    /// Execute the go command.
    pub fn execute(&self) -> CliResult {
        use crate::runner::StopReason;
        use commands::go::GoOptions;

        let (iterations, source_path) = self.parse_args();

        let options = GoOptions {
            iterations,
            source_path,
            init: self.init,
            fresh: self.fresh,
            until_complete: self.until_complete,
            timeout: self.timeout,
            feedback: self.feedback.clone(),
            no_mascot: self.no_mascot,
            dry_run: self.dry_run,
        };

        match commands::go::go(options) {
            Ok(outcome) => match outcome.stop_reason {
                StopReason::Complete => Ok(ExitCode::SUCCESS),
                StopReason::MaxIterations => Ok(ExitCode::SUCCESS),
                StopReason::UserInterrupt => Ok(ExitCode::INTERRUPT),
                _ => Ok(ExitCode::FAILURE),
            },
            Err(commands::go::GoCommandError::NoSources) => Err(CliError::NoSources),
            Err(e) => Err(CliError::Command(e.to_string())),
        }
    }

    /// Parse iterations_or_source argument.
    fn parse_args(&self) -> (Option<u32>, Option<String>) {
        match &self.iterations_or_source {
            Some(arg) => {
                if let Ok(n) = arg.parse::<u32>() {
                    (Some(n), None)
                } else {
                    (self.iterations_if_source, Some(arg.clone()))
                }
            }
            None => (None, None),
        }
    }
}

impl InitCommand {
    /// Execute the init command.
    pub fn execute(&self) -> CliResult {
        use commands::init::InitOptions;

        let options = InitOptions {
            dry_run: self.dry_run,
            force: self.force,
            yes: self.yes,
        };

        match commands::init::init(options) {
            Ok(()) => Ok(ExitCode::SUCCESS),
            Err(commands::init::InitCommandError::AlreadyInitialised) => {
                Err(CliError::AlreadyInitialised)
            }
            Err(e) => Err(CliError::Command(e.to_string())),
        }
    }
}

impl StatusCommand {
    /// Execute the status command.
    pub fn execute(&self) -> CliResult {
        commands::status::status(self.verbose)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl TaskCommand {
    /// Execute the task command.
    pub fn execute(&self) -> CliResult {
        commands::task::task(&self.task_id)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl SourceAddCommand {
    /// Execute the source add command.
    pub fn execute(&self) -> CliResult {
        commands::source::source_add(&self.source_type, self.path.as_deref())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl SourceListCommand {
    /// Execute the source list command.
    pub fn execute(&self) -> CliResult {
        commands::source::source_list()
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl SourceRemoveCommand {
    /// Execute the source remove command.
    pub fn execute(&self) -> CliResult {
        commands::source::source_remove(self.index)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ImportCommand {
    /// Execute the import command.
    pub fn execute(&self) -> CliResult {
        commands::import::import(
            &self.input_file,
            &self.output,
            self.copy,
            self.file,
            self.stdout,
        )
        .map(|()| ExitCode::SUCCESS)
        .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl TasksSyncCommand {
    /// Execute the tasks sync command.
    pub fn execute(&self) -> CliResult {
        // If --reset, archive completed tasks first
        if self.reset {
            use crate::progress::archive_session;
            if let Ok(Some(path)) = archive_session("sync --reset") {
                println!(
                    "\x1b[32m✓\x1b[0m Archived completed tasks to: {}",
                    path.display()
                );
            }
        }

        commands::import::tasks_sync()
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

/// Execute the tasks command (list tasks).
pub fn execute_tasks(pending: bool, complete: bool, limit: usize) -> CliResult {
    commands::import::tasks_show(pending, complete, limit)
        .map(|()| ExitCode::SUCCESS)
        .map_err(|e| CliError::Command(e.to_string()))
}

impl PromptCommand {
    /// Execute the prompt command.
    pub fn execute(&self) -> CliResult {
        use commands::prompt::PromptOptions;

        let options = PromptOptions {
            copy: self.copy,
            file: self.file,
            stdout: self.stdout,
            bootstrap: self.bootstrap,
            limit: self.limit,
        };

        commands::prompt::prompt(options)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl VerifyCommand {
    /// Execute the verify command.
    pub fn execute(&self) -> CliResult {
        match commands::verify::verify(self.verbose) {
            Ok(outcome) => {
                if outcome.all_passed {
                    Ok(ExitCode::SUCCESS)
                } else {
                    Ok(ExitCode::FAILURE)
                }
            }
            Err(e) => Err(CliError::Command(e.to_string())),
        }
    }
}

impl DoneCommand {
    /// Execute the done command.
    pub fn execute(&self) -> CliResult {
        commands::progress_cmd::done(&self.task_id, self.message.clone())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl FailCommand {
    /// Execute the fail command.
    pub fn execute(&self) -> CliResult {
        commands::progress_cmd::fail(&self.task_id, self.message.clone())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ResetCommand {
    /// Execute the reset command.
    pub fn execute(&self) -> CliResult {
        commands::progress_cmd::reset(&self.task_id)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

/// Execute the archive command (archive and clear session).
pub fn execute_archive_now(reason: &str, yes: bool) -> CliResult {
    commands::archive::archive_now(reason, yes)
        .map(|()| ExitCode::SUCCESS)
        .map_err(|e| CliError::Command(e.to_string()))
}

/// Execute the archive list command.
pub fn execute_archive_list() -> CliResult {
    commands::archive::archive_list()
        .map(|()| ExitCode::SUCCESS)
        .map_err(|e| CliError::Command(e.to_string()))
}

impl ConfigShowCommand {
    /// Execute the config show command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_show(self.section.as_deref())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigGetCommand {
    /// Execute the config get command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_get(&self.key)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigSetCommand {
    /// Execute the config set command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_set(&self.key, &self.value)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigResetCommand {
    /// Execute the config reset command.
    pub fn execute(&self) -> CliResult {
        use std::io::{self, Write};

        // Confirm unless --yes (for resetting all)
        if self.key.is_none() && !self.yes {
            print!("Reset all config to defaults? [Y/n]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim().to_lowercase();
                if input == "n" || input == "no" {
                    println!("Cancelled.");
                    return Ok(ExitCode::SUCCESS);
                }
            }
        }

        commands::config::config_reset(self.key.as_deref())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigEditCommand {
    /// Execute the config edit command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_edit()
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigExplainCommand {
    /// Execute the config explain command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_explain(self.key.as_deref())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl ConfigKeysCommand {
    /// Execute the config keys command.
    pub fn execute(&self) -> CliResult {
        commands::config::config_keys()
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl UpdateCommand {
    /// Execute the update command.
    pub fn execute(&self) -> CliResult {
        update::execute_update(self.beta, self.check)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl CompletionsCommand {
    /// Execute the completions command.
    pub fn execute(&self) -> CliResult {
        commands::completions::completions(&self.shell)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

impl UseCommand {
    /// Execute the use command.
    pub fn execute(&self) -> CliResult {
        commands::use_cli::use_ai_cli(self.cli.as_deref(), self.list)
            .map(|()| ExitCode::SUCCESS)
            .map_err(|e| CliError::Command(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parses() {
        // Verify the CLI structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_version_flag() {
        let result = Cli::try_parse_from(["afk", "--version"]);
        // --version causes an early exit, which is expected
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_help_flag() {
        let result = Cli::try_parse_from(["afk", "--help"]);
        // --help causes an early exit, which is expected
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_go_command_default() {
        let cli = Cli::try_parse_from(["afk", "go"]).unwrap();
        match cli.command {
            Some(Commands::Go(cmd)) => {
                assert!(cmd.iterations_or_source.is_none());
                assert!(!cmd.dry_run);
                assert!(!cmd.until_complete);
                assert!(!cmd.init);
                assert!(cmd.timeout.is_none());
                assert_eq!(cmd.feedback, Some("tui".to_string()));
                assert!(!cmd.no_mascot);
            }
            _ => panic!("Expected Go command"),
        }
    }

    #[test]
    fn test_go_command_with_iterations() {
        let cli = Cli::try_parse_from(["afk", "go", "20"]).unwrap();
        match cli.command {
            Some(Commands::Go(cmd)) => {
                assert_eq!(cmd.iterations_or_source, Some("20".to_string()));
            }
            _ => panic!("Expected Go command"),
        }
    }

    #[test]
    fn test_go_command_with_source() {
        let cli = Cli::try_parse_from(["afk", "go", "TODO.md", "5"]).unwrap();
        match cli.command {
            Some(Commands::Go(cmd)) => {
                assert_eq!(cmd.iterations_or_source, Some("TODO.md".to_string()));
                assert_eq!(cmd.iterations_if_source, Some(5));
            }
            _ => panic!("Expected Go command"),
        }
    }

    #[test]
    fn test_go_command_with_flags() {
        let cli = Cli::try_parse_from([
            "afk",
            "go",
            "-n",
            "-u",
            "--init",
            "-t",
            "60",
            "--feedback",
            "minimal",
            "--no-mascot",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Go(cmd)) => {
                assert!(cmd.dry_run);
                assert!(cmd.until_complete);
                assert!(cmd.init);
                assert_eq!(cmd.timeout, Some(60));
                assert_eq!(cmd.feedback, Some("minimal".to_string()));
                assert!(cmd.no_mascot);
            }
            _ => panic!("Expected Go command"),
        }
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::try_parse_from(["afk", "init", "-n", "-f", "-y"]).unwrap();
        match cli.command {
            Some(Commands::Init(cmd)) => {
                assert!(cmd.dry_run);
                assert!(cmd.force);
                assert!(cmd.yes);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_status_command() {
        let cli = Cli::try_parse_from(["afk", "status"]).unwrap();
        match cli.command {
            Some(Commands::Status(cmd)) => {
                assert!(!cmd.verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_status_command_verbose() {
        let cli = Cli::try_parse_from(["afk", "status", "-v"]).unwrap();
        match cli.command {
            Some(Commands::Status(cmd)) => {
                assert!(cmd.verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_tasks_command_default() {
        // afk tasks (no subcommand) shows task list with defaults
        let cli = Cli::try_parse_from(["afk", "tasks"]).unwrap();
        match cli.command {
            Some(Commands::Tasks {
                command,
                pending,
                complete,
                limit,
            }) => {
                assert!(command.is_none()); // No subcommand = list tasks
                assert!(!pending);
                assert!(!complete);
                assert_eq!(limit, 50); // Default limit
            }
            _ => panic!("Expected Tasks command"),
        }
    }

    #[test]
    fn test_tasks_command_with_flags() {
        let cli = Cli::try_parse_from(["afk", "tasks", "-p", "-l", "10"]).unwrap();
        match cli.command {
            Some(Commands::Tasks {
                command,
                pending,
                complete,
                limit,
            }) => {
                assert!(command.is_none());
                assert!(pending);
                assert!(!complete);
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected Tasks command"),
        }
    }

    #[test]
    fn test_task_command() {
        let cli = Cli::try_parse_from(["afk", "task", "auth-001"]).unwrap();
        match cli.command {
            Some(Commands::Task(cmd)) => {
                assert_eq!(cmd.task_id, "auth-001");
            }
            _ => panic!("Expected Task command"),
        }
    }

    #[test]
    fn test_prompt_command() {
        let cli = Cli::try_parse_from(["afk", "prompt", "-c", "-b", "-l", "20"]).unwrap();
        match cli.command {
            Some(Commands::Prompt(cmd)) => {
                assert!(cmd.copy);
                assert!(cmd.bootstrap);
                assert_eq!(cmd.limit, Some(20));
            }
            _ => panic!("Expected Prompt command"),
        }
    }

    #[test]
    fn test_verify_command() {
        let cli = Cli::try_parse_from(["afk", "verify", "--verbose"]).unwrap();
        match cli.command {
            Some(Commands::Verify(cmd)) => {
                assert!(cmd.verbose);
            }
            _ => panic!("Expected Verify command"),
        }
    }

    #[test]
    fn test_done_command() {
        let cli = Cli::try_parse_from(["afk", "done", "task-123", "-m", "All tests pass"]).unwrap();
        match cli.command {
            Some(Commands::Done(cmd)) => {
                assert_eq!(cmd.task_id, "task-123");
                assert_eq!(cmd.message, Some("All tests pass".to_string()));
            }
            _ => panic!("Expected Done command"),
        }
    }

    #[test]
    fn test_fail_command() {
        let cli = Cli::try_parse_from(["afk", "fail", "task-456", "-m", "Tests failing"]).unwrap();
        match cli.command {
            Some(Commands::Fail(cmd)) => {
                assert_eq!(cmd.task_id, "task-456");
                assert_eq!(cmd.message, Some("Tests failing".to_string()));
            }
            _ => panic!("Expected Fail command"),
        }
    }

    #[test]
    fn test_source_add_command() {
        let cli = Cli::try_parse_from(["afk", "source", "add", "json", "tasks.json"]).unwrap();
        match cli.command {
            Some(Commands::Source(SourceCommands::Add(cmd))) => {
                assert_eq!(cmd.source_type, "json");
                assert_eq!(cmd.path, Some("tasks.json".to_string()));
            }
            _ => panic!("Expected Source Add command"),
        }
    }

    #[test]
    fn test_source_add_beads() {
        let cli = Cli::try_parse_from(["afk", "source", "add", "beads"]).unwrap();
        match cli.command {
            Some(Commands::Source(SourceCommands::Add(cmd))) => {
                assert_eq!(cmd.source_type, "beads");
                assert!(cmd.path.is_none());
            }
            _ => panic!("Expected Source Add command"),
        }
    }

    #[test]
    fn test_source_list_command() {
        let cli = Cli::try_parse_from(["afk", "source", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Source(SourceCommands::List(_)))
        ));
    }

    #[test]
    fn test_source_remove_command() {
        let cli = Cli::try_parse_from(["afk", "source", "remove", "1"]).unwrap();
        match cli.command {
            Some(Commands::Source(SourceCommands::Remove(cmd))) => {
                assert_eq!(cmd.index, 1);
            }
            _ => panic!("Expected Source Remove command"),
        }
    }

    #[test]
    fn test_import_command() {
        let cli = Cli::try_parse_from(["afk", "import", "requirements.md", "-c"]).unwrap();
        match cli.command {
            Some(Commands::Import(cmd)) => {
                assert_eq!(cmd.input_file, "requirements.md");
                assert!(cmd.copy);
            }
            _ => panic!("Expected Import command"),
        }
    }

    #[test]
    fn test_tasks_sync_command() {
        let cli = Cli::try_parse_from(["afk", "tasks", "sync"]).unwrap();
        match cli.command {
            Some(Commands::Tasks { command, .. }) => {
                assert!(matches!(command, Some(TasksCommands::Sync(_))));
            }
            _ => panic!("Expected Tasks command with sync subcommand"),
        }
    }

    #[test]
    fn test_reset_command() {
        let cli = Cli::try_parse_from(["afk", "reset", "stuck-task"]).unwrap();
        match cli.command {
            Some(Commands::Reset(cmd)) => {
                assert_eq!(cmd.task_id, "stuck-task");
            }
            _ => panic!("Expected Reset command"),
        }
    }

    #[test]
    fn test_archive_command_default() {
        // afk archive (no subcommand) should work with default args
        let cli = Cli::try_parse_from(["afk", "archive"]).unwrap();
        match cli.command {
            Some(Commands::Archive {
                command,
                reason,
                yes,
            }) => {
                assert!(command.is_none()); // No subcommand = archive now
                assert_eq!(reason, "manual"); // Default reason
                assert!(!yes);
            }
            _ => panic!("Expected Archive command"),
        }
    }

    #[test]
    fn test_archive_command_with_args() {
        let cli = Cli::try_parse_from(["afk", "archive", "-r", "completed", "-y"]).unwrap();
        match cli.command {
            Some(Commands::Archive {
                command,
                reason,
                yes,
            }) => {
                assert!(command.is_none());
                assert_eq!(reason, "completed");
                assert!(yes);
            }
            _ => panic!("Expected Archive command"),
        }
    }

    #[test]
    fn test_archive_list_command() {
        let cli = Cli::try_parse_from(["afk", "archive", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Archive {
                command: Some(ArchiveCommands::List),
                ..
            })
        ));
    }

    #[test]
    fn test_update_command() {
        let cli = Cli::try_parse_from(["afk", "update", "--beta", "--check"]).unwrap();
        match cli.command {
            Some(Commands::Update(cmd)) => {
                assert!(cmd.beta);
                assert!(cmd.check);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_completions_command() {
        let cli = Cli::try_parse_from(["afk", "completions", "zsh"]).unwrap();
        match cli.command {
            Some(Commands::Completions(cmd)) => {
                assert_eq!(cmd.shell, "zsh");
            }
            _ => panic!("Expected Completions command"),
        }
    }

    #[test]
    fn test_no_command_returns_none() {
        let cli = Cli::try_parse_from(["afk"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_invalid_source_type_rejected() {
        let result = Cli::try_parse_from(["afk", "source", "add", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_feedback_mode_rejected() {
        let result = Cli::try_parse_from(["afk", "go", "--feedback", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_shell_rejected() {
        let result = Cli::try_parse_from(["afk", "completions", "powershell"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_use_command_with_cli_name() {
        let cli = Cli::try_parse_from(["afk", "use", "claude"]).unwrap();
        match cli.command {
            Some(Commands::Use(cmd)) => {
                assert_eq!(cmd.cli, Some("claude".to_string()));
                assert!(!cmd.list);
            }
            _ => panic!("Expected Use command"),
        }
    }

    #[test]
    fn test_use_command_without_cli_name() {
        let cli = Cli::try_parse_from(["afk", "use"]).unwrap();
        match cli.command {
            Some(Commands::Use(cmd)) => {
                assert!(cmd.cli.is_none());
                assert!(!cmd.list);
            }
            _ => panic!("Expected Use command"),
        }
    }

    #[test]
    fn test_use_command_list_flag() {
        let cli = Cli::try_parse_from(["afk", "use", "--list"]).unwrap();
        match cli.command {
            Some(Commands::Use(cmd)) => {
                assert!(cmd.list);
            }
            _ => panic!("Expected Use command"),
        }
    }
}
