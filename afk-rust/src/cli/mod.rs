//! CLI commands and argument handling.
//!
//! This module contains the clap CLI definitions and command implementations.

pub mod commands;
pub mod output;

use clap::{Args, Parser, Subcommand};

/// Autonomous AI coding loops - Ralph Wiggum style.
///
/// Run AI coding tasks in a loop with fresh context each iteration.
/// Memory persists via git history, progress.json, and task sources.
#[derive(Parser, Debug)]
#[command(name = "afk")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Top-level commands for afk.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Quick start with zero config.
    ///
    /// Auto-detects project type, available tools, and task sources.
    /// Runs the loop with sensible defaults.
    ///
    /// Examples:
    ///   afk go                 # Auto-detect, run 10 iterations
    ///   afk go 20              # Auto-detect, run 20 iterations
    ///   afk go -u              # Run until all tasks complete
    ///   afk go TODO.md 5       # Use TODO.md as source, run 5 iterations
    Go(GoCommand),

    /// Run multiple iterations using configured AI CLI.
    ///
    /// Implements the Ralph Wiggum pattern: each iteration spawns a fresh
    /// AI CLI instance with clean context.
    Run(RunCommand),

    /// Initialize afk by analysing the project.
    ///
    /// Detects project type, available tools, task sources, and context files
    /// to generate a sensible configuration.
    Init(InitCommand),

    /// Show current status and tasks.
    Status(StatusCommand),

    /// Manage task sources.
    #[command(subcommand)]
    Source(SourceCommands),

    /// Manage product requirements documents.
    #[command(subcommand)]
    Prd(PrdCommands),

    /// Sync PRD from all configured sources.
    ///
    /// Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
    /// .afk/prd.json file that the AI reads directly (Ralph pattern).
    Sync(SyncCommand),

    /// Generate prompt for next iteration.
    Next(NextCommand),

    /// Explain current loop state for debugging.
    ///
    /// Shows what task would be selected next, why, and session statistics.
    Explain(ExplainCommand),

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

    /// Manage session archives.
    #[command(subcommand)]
    Archive(ArchiveCommands),

    /// Update afk to the latest version.
    ///
    /// Downloads and installs the latest release from GitHub.
    Update(UpdateCommand),

    /// Generate shell completions.
    ///
    /// Outputs completion script to stdout for bash, zsh, or fish.
    Completions(CompletionsCommand),

    /// Quick start: init if needed, then run the loop.
    ///
    /// Convenience command that combines init and run with sensible defaults.
    Start(StartCommand),

    /// Resume from last session without archiving.
    ///
    /// Continues the loop from where it left off, preserving existing
    /// progress and iteration count.
    Resume(ResumeCommand),
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

    /// Feedback display mode.
    #[arg(long, value_parser = ["full", "minimal", "off"])]
    pub feedback: Option<String>,

    /// Disable ASCII mascot in feedback display.
    #[arg(long)]
    pub no_mascot: bool,
}

/// Arguments for the 'run' command.
#[derive(Args, Debug)]
pub struct RunCommand {
    /// Number of iterations to run.
    #[arg(default_value = "5")]
    pub iterations: u32,

    /// Run until all tasks complete.
    #[arg(short = 'u', long)]
    pub until_complete: bool,

    /// Override timeout in minutes.
    #[arg(short = 't', long)]
    pub timeout: Option<u32>,

    /// Create/checkout feature branch.
    #[arg(short = 'b', long)]
    pub branch: Option<String>,

    /// Continue from last session.
    #[arg(short = 'c', long = "continue")]
    pub resume_session: bool,

    /// Feedback display mode.
    #[arg(long, value_parser = ["full", "minimal", "off"])]
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

    /// Overwrite existing config.
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Accept all defaults without prompting.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for the 'status' command.
#[derive(Args, Debug)]
pub struct StatusCommand {}

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

/// Subcommands for PRD management.
#[derive(Subcommand, Debug)]
pub enum PrdCommands {
    /// Parse a PRD into a structured JSON feature list.
    ///
    /// Takes a product requirements document (markdown, text, etc.) and generates
    /// an AI prompt to convert it into the Anthropic-style JSON format.
    Parse(PrdParseCommand),

    /// Sync PRD from all configured sources.
    ///
    /// Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
    /// .afk/prd.json file.
    Sync(PrdSyncCommand),

    /// Show the current PRD state.
    ///
    /// Displays user stories from .afk/prd.json with their completion status.
    Show(PrdShowCommand),
}

/// Arguments for 'prd parse' command.
#[derive(Args, Debug)]
pub struct PrdParseCommand {
    /// Input file to parse.
    pub input_file: String,

    /// Output JSON path.
    #[arg(short = 'o', long, default_value = ".afk/prd.json")]
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

/// Arguments for 'prd sync' command.
#[derive(Args, Debug)]
pub struct PrdSyncCommand {
    /// Branch name for PRD.
    #[arg(short = 'b', long)]
    pub branch: Option<String>,
}

/// Arguments for 'prd show' command.
#[derive(Args, Debug)]
pub struct PrdShowCommand {
    /// Show only pending stories.
    #[arg(short = 'p', long)]
    pub pending: bool,
}

/// Arguments for the 'sync' command.
#[derive(Args, Debug)]
pub struct SyncCommand {
    /// Branch name for PRD.
    #[arg(short = 'b', long)]
    pub branch: Option<String>,
}

/// Arguments for the 'next' command.
#[derive(Args, Debug)]
pub struct NextCommand {
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

/// Arguments for the 'explain' command.
#[derive(Args, Debug)]
pub struct ExplainCommand {
    /// Show detailed information.
    #[arg(short = 'v', long)]
    pub verbose: bool,
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
    /// Archive current session.
    ///
    /// Saves progress.json to a timestamped directory for later reference or recovery.
    Create(ArchiveCreateCommand),

    /// List archived sessions.
    List(ArchiveListCommand),

    /// Clear current session progress.
    ///
    /// Removes progress.json to start fresh. Optionally archives first.
    Clear(ArchiveClearCommand),
}

/// Arguments for 'archive create' command.
#[derive(Args, Debug)]
pub struct ArchiveCreateCommand {
    /// Reason for archiving.
    #[arg(short = 'r', long, default_value = "manual")]
    pub reason: String,
}

/// Arguments for 'archive list' command.
#[derive(Args, Debug)]
pub struct ArchiveListCommand {}

/// Arguments for 'archive clear' command.
#[derive(Args, Debug)]
pub struct ArchiveClearCommand {
    /// Skip confirmation.
    #[arg(short = 'y', long)]
    pub yes: bool,
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

/// Arguments for the 'start' command.
#[derive(Args, Debug)]
pub struct StartCommand {
    /// Number of iterations to run.
    #[arg(default_value = "10")]
    pub iterations: u32,

    /// Create/checkout feature branch.
    #[arg(short = 'b', long)]
    pub branch: Option<String>,

    /// Skip confirmation prompts.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Arguments for the 'resume' command.
#[derive(Args, Debug)]
pub struct ResumeCommand {
    /// Number of iterations to run.
    #[arg(default_value = "10")]
    pub iterations: u32,

    /// Run until all tasks complete.
    #[arg(short = 'u', long)]
    pub until_complete: bool,

    /// Override timeout in minutes.
    #[arg(short = 't', long)]
    pub timeout: Option<u32>,
}

// ============================================================================
// Stub implementations - these print "not implemented" for now
// ============================================================================

impl GoCommand {
    /// Execute the go command.
    pub fn execute(&self) {
        use crate::config::{AfkConfig, SourceConfig};
        use crate::prd::PrdDocument;
        use crate::runner::run_loop;
        use std::path::Path;

        // Parse iterations_or_source
        let (iterations, explicit_source) = self.parse_args();

        // Try to load existing config, or create default
        let mut config = AfkConfig::load(None).unwrap_or_default();

        // Handle explicit source file path
        if let Some(ref source_path) = explicit_source {
            let path = Path::new(source_path);
            if !path.exists() {
                eprintln!("\x1b[31mError:\x1b[0m Source file not found: {source_path}");
                std::process::exit(1);
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
                println!("\x1b[2mUsing existing .afk/prd.json ({} stories)\x1b[0m", prd.user_stories.len());
            } else {
                // Try to infer sources
                let inferred = infer_sources();
                if inferred.is_empty() {
                    eprintln!("\x1b[33mNo task sources found.\x1b[0m");
                    eprintln!();
                    eprintln!("Try one of:");
                    eprintln!("  afk go TODO.md           # Use a markdown file");
                    eprintln!("  afk prd parse spec.md    # Parse a requirements doc");
                    eprintln!("  afk source add beads     # Use beads issues");
                    std::process::exit(1);
                }
                config.sources = inferred;
            }
        }

        // Ensure AI CLI is configured
        if config.ai_cli.command.is_empty() {
            config.ai_cli.command = "claude".to_string();
            config.ai_cli.args = vec!["--dangerously-skip-permissions".to_string(), "-p".to_string()];
        }

        // Dry run mode
        if self.dry_run {
            println!("\x1b[1mDry run mode - would execute:\x1b[0m");
            println!("  AI CLI: {} {}", config.ai_cli.command, config.ai_cli.args.join(" "));
            println!("  Iterations: {}", iterations.unwrap_or(10));
            println!("  Sources: {:?}", config.sources.iter().map(|s| &s.source_type).collect::<Vec<_>>());
            return;
        }

        // Run the loop
        let result = run_loop(
            &config,
            iterations,
            None, // branch
            self.until_complete,
            None, // timeout
            false, // resume
        );

        // Exit with appropriate code
        match result.stop_reason {
            crate::runner::StopReason::Complete => std::process::exit(0),
            crate::runner::StopReason::MaxIterations => std::process::exit(0),
            crate::runner::StopReason::UserInterrupt => std::process::exit(130),
            _ => std::process::exit(1),
        }
    }

    /// Parse iterations_or_source argument.
    fn parse_args(&self) -> (Option<u32>, Option<String>) {
        match &self.iterations_or_source {
            Some(arg) => {
                // Try to parse as number first
                if let Ok(n) = arg.parse::<u32>() {
                    (Some(n), None)
                } else {
                    // It's a path - use iterations_if_source or default 10
                    (self.iterations_if_source.or(Some(10)), Some(arg.clone()))
                }
            }
            None => (Some(10), None), // Default 10 iterations
        }
    }
}

/// Infer sources from the current directory.
fn infer_sources() -> Vec<crate::config::SourceConfig> {
    use crate::config::SourceConfig;
    use std::path::Path;

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

impl RunCommand {
    /// Execute the run command.
    pub fn execute(&self) {
        use crate::config::AfkConfig;
        use crate::runner::run_loop;

        // Load config
        let config = match AfkConfig::load(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m Failed to load config: {e}");
                eprintln!("\x1b[2mRun `afk init` to initialise the project.\x1b[0m");
                std::process::exit(1);
            }
        };

        // Run the loop
        let result = run_loop(
            &config,
            Some(self.iterations),
            self.branch.as_deref(),
            self.until_complete,
            self.timeout,
            self.resume_session,
        );

        // Exit with appropriate code
        match result.stop_reason {
            crate::runner::StopReason::Complete => std::process::exit(0),
            crate::runner::StopReason::MaxIterations => std::process::exit(0),
            crate::runner::StopReason::UserInterrupt => std::process::exit(130),
            _ => std::process::exit(1),
        }
    }
}

impl InitCommand {
    /// Execute the init command (stub).
    pub fn execute(&self) {
        println!("afk init: not implemented");
        println!("  dry_run: {}", self.dry_run);
        println!("  force: {}", self.force);
        println!("  yes: {}", self.yes);
    }
}

impl StatusCommand {
    /// Execute the status command (stub).
    pub fn execute(&self) {
        println!("afk status: not implemented");
    }
}

impl SourceAddCommand {
    /// Execute the source add command.
    pub fn execute(&self) {
        match commands::source::source_add(&self.source_type, self.path.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl SourceListCommand {
    /// Execute the source list command.
    pub fn execute(&self) {
        match commands::source::source_list() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl SourceRemoveCommand {
    /// Execute the source remove command.
    pub fn execute(&self) {
        match commands::source::source_remove(self.index) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl PrdParseCommand {
    /// Execute the prd parse command.
    pub fn execute(&self) {
        match commands::prd::prd_parse(
            &self.input_file,
            &self.output,
            self.copy,
            self.file,
            self.stdout,
        ) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl PrdSyncCommand {
    /// Execute the prd sync command.
    pub fn execute(&self) {
        match commands::prd::prd_sync(self.branch.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl PrdShowCommand {
    /// Execute the prd show command.
    pub fn execute(&self) {
        match commands::prd::prd_show(self.pending) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl SyncCommand {
    /// Execute the sync command (stub).
    pub fn execute(&self) {
        println!("afk sync: not implemented");
        println!("  branch: {:?}", self.branch);
    }
}

impl NextCommand {
    /// Execute the next command (stub).
    pub fn execute(&self) {
        println!("afk next: not implemented");
        println!("  copy: {}", self.copy);
        println!("  file: {}", self.file);
        println!("  stdout: {}", self.stdout);
        println!("  bootstrap: {}", self.bootstrap);
        println!("  limit: {:?}", self.limit);
    }
}

impl ExplainCommand {
    /// Execute the explain command (stub).
    pub fn execute(&self) {
        println!("afk explain: not implemented");
        println!("  verbose: {}", self.verbose);
    }
}

impl VerifyCommand {
    /// Execute the verify command.
    pub fn execute(&self) {
        use crate::config::AfkConfig;
        use crate::runner::{has_configured_gates, run_quality_gates};

        // Load config
        let config = match AfkConfig::load(None) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m Failed to load config: {e}");
                std::process::exit(1);
            }
        };

        // Check if any gates are configured
        if !has_configured_gates(&config.feedback_loops) {
            println!("\x1b[33mNo quality gates configured.\x1b[0m");
            println!();
            println!("Configure gates in .afk/config.json:");
            println!("  {{");
            println!("    \"feedbackLoops\": {{");
            println!("      \"lint\": \"cargo clippy\",");
            println!("      \"test\": \"cargo test\"");
            println!("    }}");
            println!("  }}");
            std::process::exit(0);
        }

        // Run quality gates
        let result = run_quality_gates(&config.feedback_loops, self.verbose);

        // Exit with appropriate code
        if result.all_passed {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }
}

impl DoneCommand {
    /// Execute the done command (stub).
    pub fn execute(&self) {
        println!("afk done: not implemented");
        println!("  task_id: {}", self.task_id);
        println!("  message: {:?}", self.message);
    }
}

impl FailCommand {
    /// Execute the fail command (stub).
    pub fn execute(&self) {
        println!("afk fail: not implemented");
        println!("  task_id: {}", self.task_id);
        println!("  message: {:?}", self.message);
    }
}

impl ResetCommand {
    /// Execute the reset command (stub).
    pub fn execute(&self) {
        println!("afk reset: not implemented");
        println!("  task_id: {}", self.task_id);
    }
}

impl ArchiveCreateCommand {
    /// Execute the archive create command (stub).
    pub fn execute(&self) {
        println!("afk archive create: not implemented");
        println!("  reason: {}", self.reason);
    }
}

impl ArchiveListCommand {
    /// Execute the archive list command (stub).
    pub fn execute(&self) {
        println!("afk archive list: not implemented");
    }
}

impl ArchiveClearCommand {
    /// Execute the archive clear command (stub).
    pub fn execute(&self) {
        println!("afk archive clear: not implemented");
        println!("  yes: {}", self.yes);
    }
}

impl UpdateCommand {
    /// Execute the update command (stub).
    pub fn execute(&self) {
        println!("afk update: not implemented");
        println!("  beta: {}", self.beta);
        println!("  check: {}", self.check);
    }
}

impl CompletionsCommand {
    /// Execute the completions command (stub).
    pub fn execute(&self) {
        println!("afk completions: not implemented");
        println!("  shell: {}", self.shell);
    }
}

impl StartCommand {
    /// Execute the start command (stub).
    pub fn execute(&self) {
        println!("afk start: not implemented");
        println!("  iterations: {}", self.iterations);
        println!("  branch: {:?}", self.branch);
        println!("  yes: {}", self.yes);
    }
}

impl ResumeCommand {
    /// Execute the resume command (stub).
    pub fn execute(&self) {
        println!("afk resume: not implemented");
        println!("  iterations: {}", self.iterations);
        println!("  until_complete: {}", self.until_complete);
        println!("  timeout: {:?}", self.timeout);
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
                assert!(cmd.feedback.is_none());
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
        let cli =
            Cli::try_parse_from(["afk", "go", "-n", "-u", "--feedback", "minimal", "--no-mascot"])
                .unwrap();
        match cli.command {
            Some(Commands::Go(cmd)) => {
                assert!(cmd.dry_run);
                assert!(cmd.until_complete);
                assert_eq!(cmd.feedback, Some("minimal".to_string()));
                assert!(cmd.no_mascot);
            }
            _ => panic!("Expected Go command"),
        }
    }

    #[test]
    fn test_run_command_default() {
        let cli = Cli::try_parse_from(["afk", "run"]).unwrap();
        match cli.command {
            Some(Commands::Run(cmd)) => {
                assert_eq!(cmd.iterations, 5);
                assert!(!cmd.until_complete);
                assert!(cmd.timeout.is_none());
                assert!(cmd.branch.is_none());
                assert!(!cmd.resume_session);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_run_command_with_options() {
        let cli = Cli::try_parse_from([
            "afk",
            "run",
            "10",
            "-u",
            "-t",
            "60",
            "-b",
            "feature/test",
            "-c",
            "--feedback",
            "full",
            "--no-mascot",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Run(cmd)) => {
                assert_eq!(cmd.iterations, 10);
                assert!(cmd.until_complete);
                assert_eq!(cmd.timeout, Some(60));
                assert_eq!(cmd.branch, Some("feature/test".to_string()));
                assert!(cmd.resume_session);
                assert_eq!(cmd.feedback, Some("full".to_string()));
                assert!(cmd.no_mascot);
            }
            _ => panic!("Expected Run command"),
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
        assert!(matches!(cli.command, Some(Commands::Status(_))));
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
    fn test_prd_parse_command() {
        let cli = Cli::try_parse_from(["afk", "prd", "parse", "requirements.md", "-c"]).unwrap();
        match cli.command {
            Some(Commands::Prd(PrdCommands::Parse(cmd))) => {
                assert_eq!(cmd.input_file, "requirements.md");
                assert!(cmd.copy);
            }
            _ => panic!("Expected Prd Parse command"),
        }
    }

    #[test]
    fn test_prd_sync_command() {
        let cli = Cli::try_parse_from(["afk", "prd", "sync", "-b", "feature/test"]).unwrap();
        match cli.command {
            Some(Commands::Prd(PrdCommands::Sync(cmd))) => {
                assert_eq!(cmd.branch, Some("feature/test".to_string()));
            }
            _ => panic!("Expected Prd Sync command"),
        }
    }

    #[test]
    fn test_prd_show_command() {
        let cli = Cli::try_parse_from(["afk", "prd", "show", "-p"]).unwrap();
        match cli.command {
            Some(Commands::Prd(PrdCommands::Show(cmd))) => {
                assert!(cmd.pending);
            }
            _ => panic!("Expected Prd Show command"),
        }
    }

    #[test]
    fn test_sync_command() {
        let cli = Cli::try_parse_from(["afk", "sync", "-b", "main"]).unwrap();
        match cli.command {
            Some(Commands::Sync(cmd)) => {
                assert_eq!(cmd.branch, Some("main".to_string()));
            }
            _ => panic!("Expected Sync command"),
        }
    }

    #[test]
    fn test_next_command() {
        let cli = Cli::try_parse_from(["afk", "next", "-c", "-b", "-l", "20"]).unwrap();
        match cli.command {
            Some(Commands::Next(cmd)) => {
                assert!(cmd.copy);
                assert!(cmd.bootstrap);
                assert_eq!(cmd.limit, Some(20));
            }
            _ => panic!("Expected Next command"),
        }
    }

    #[test]
    fn test_explain_command() {
        let cli = Cli::try_parse_from(["afk", "explain", "-v"]).unwrap();
        match cli.command {
            Some(Commands::Explain(cmd)) => {
                assert!(cmd.verbose);
            }
            _ => panic!("Expected Explain command"),
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
    fn test_archive_create_command() {
        let cli = Cli::try_parse_from(["afk", "archive", "create", "-r", "switching branches"])
            .unwrap();
        match cli.command {
            Some(Commands::Archive(ArchiveCommands::Create(cmd))) => {
                assert_eq!(cmd.reason, "switching branches");
            }
            _ => panic!("Expected Archive Create command"),
        }
    }

    #[test]
    fn test_archive_list_command() {
        let cli = Cli::try_parse_from(["afk", "archive", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Archive(ArchiveCommands::List(_)))
        ));
    }

    #[test]
    fn test_archive_clear_command() {
        let cli = Cli::try_parse_from(["afk", "archive", "clear", "-y"]).unwrap();
        match cli.command {
            Some(Commands::Archive(ArchiveCommands::Clear(cmd))) => {
                assert!(cmd.yes);
            }
            _ => panic!("Expected Archive Clear command"),
        }
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
    fn test_start_command() {
        let cli = Cli::try_parse_from(["afk", "start", "20", "-b", "my-feature", "-y"]).unwrap();
        match cli.command {
            Some(Commands::Start(cmd)) => {
                assert_eq!(cmd.iterations, 20);
                assert_eq!(cmd.branch, Some("my-feature".to_string()));
                assert!(cmd.yes);
            }
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_resume_command() {
        let cli = Cli::try_parse_from(["afk", "resume", "15", "-u", "-t", "30"]).unwrap();
        match cli.command {
            Some(Commands::Resume(cmd)) => {
                assert_eq!(cmd.iterations, 15);
                assert!(cmd.until_complete);
                assert_eq!(cmd.timeout, Some(30));
            }
            _ => panic!("Expected Resume command"),
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
}
