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
        use crate::bootstrap::ensure_ai_cli_configured;
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

        // Ensure AI CLI is configured (first-run experience)
        if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config)) {
            config.ai_cli = ai_cli;
        } else {
            // No AI CLI available - exit
            eprintln!("\x1b[31mError:\x1b[0m No AI CLI configured. Install one and try again.");
            std::process::exit(1);
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
    /// Execute the init command.
    pub fn execute(&self) {
        use crate::bootstrap::{analyse_project, generate_config, infer_sources};
        use std::fs;
        use std::path::Path;

        let afk_dir = Path::new(".afk");
        let config_path = afk_dir.join("config.json");

        // Check if already initialised
        if config_path.exists() && !self.force {
            eprintln!("\x1b[33mAlready initialised.\x1b[0m Use --force to reinitialise.");
            return;
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

        // Infer AI CLI using bootstrap detection (gets proper args per CLI)
        if config.ai_cli.command.is_empty() {
            if let Some(ai_cli) = crate::bootstrap::detect_ai_cli() {
                config.ai_cli = ai_cli;
            }
        }

        // Show what would be written
        println!("\n\x1b[1mConfiguration:\x1b[0m");
        println!("  AI CLI: {} {}", config.ai_cli.command, config.ai_cli.args.join(" "));
        println!("  Sources: {:?}", config.sources.iter().map(|s| &s.source_type).collect::<Vec<_>>());
        if let Some(ref cmd) = config.feedback_loops.test {
            println!("  Test: {cmd}");
        }
        if let Some(ref cmd) = config.feedback_loops.lint {
            println!("  Lint: {cmd}");
        }

        // Dry run mode
        if self.dry_run {
            println!("\n\x1b[2mDry run - no files written.\x1b[0m");
            return;
        }

        // Create .afk directory
        if let Err(e) = fs::create_dir_all(afk_dir) {
            eprintln!("\x1b[31mError creating .afk directory:\x1b[0m {e}");
            std::process::exit(1);
        }

        // Write config
        if let Err(e) = config.save(Some(&config_path)) {
            eprintln!("\x1b[31mError saving config:\x1b[0m {e}");
            std::process::exit(1);
        }

        // Create empty prd.json
        let prd_path = afk_dir.join("prd.json");
        if !prd_path.exists() {
            let empty_prd = r#"{
  "project": "",
  "branchName": "",
  "description": "",
  "userStories": []
}"#;
            if let Err(e) = fs::write(&prd_path, empty_prd) {
                eprintln!("\x1b[31mError creating prd.json:\x1b[0m {e}");
            }
        }

        println!("\n\x1b[32m✓ Initialised afk\x1b[0m");
        println!("  Config: {}", config_path.display());

        // Suggest next steps
        println!("\n\x1b[1mNext steps:\x1b[0m");
        if config.sources.is_empty() {
            println!("  1. Add a task source:");
            println!("     afk source add beads      # Use beads issues");
            println!("     afk prd parse spec.md     # Parse a requirements doc");
        } else {
            println!("  1. afk go   # Start working through tasks");
        }
    }
}


impl StatusCommand {
    /// Execute the status command.
    pub fn execute(&self) {
        use crate::config::AfkConfig;
        use crate::prd::PrdDocument;
        use crate::progress::SessionProgress;
        use std::path::Path;

        // Check if initialised
        if !Path::new(".afk").exists() {
            println!("\x1b[33mafk not initialised.\x1b[0m");
            println!("Run \x1b[1mafk init\x1b[0m to get started.");
            return;
        }

        let config = AfkConfig::load(None).unwrap_or_default();
        let prd = PrdDocument::load(None).unwrap_or_default();
        let progress = SessionProgress::load(None).unwrap_or_default();

        println!("\x1b[1m=== afk status ===\x1b[0m");
        println!();

        // Task summary
        let (pending, completed) = prd.get_story_counts();
        let total = pending + completed;
        
        println!("\x1b[1mTasks\x1b[0m");
        if total == 0 {
            println!("  No tasks configured.");
        } else {
            println!("  Total: {total} ({completed} complete, {pending} pending)");
            if let Some(next) = prd.get_next_story() {
                let title = if next.title.len() > 50 {
                    format!("{}...", &next.title[..47])
                } else {
                    next.title.clone()
                };
                println!("  Next: \x1b[36m{}\x1b[0m - {}", next.id, title);
            }
        }
        println!();

        // Session progress
        println!("\x1b[1mSession\x1b[0m");
        println!("  Started: {}", &progress.started_at[..19].replace('T', " "));
        println!("  Iterations: {}", progress.iterations);
        let (pend, in_prog, comp, fail, skip) = progress.get_task_counts();
        if pend + in_prog + comp + fail + skip > 0 {
            println!(
                "  Tasks: {} pending, {} in-progress, {} complete, {} failed, {} skipped",
                pend, in_prog, comp, fail, skip
            );
        }
        println!();

        // Sources
        println!("\x1b[1mSources\x1b[0m");
        if config.sources.is_empty() {
            println!("  (none configured)");
        } else {
            for (i, source) in config.sources.iter().enumerate() {
                let desc = match &source.source_type {
                    crate::config::SourceType::Beads => "beads".to_string(),
                    crate::config::SourceType::Json => {
                        format!("json: {}", source.path.as_deref().unwrap_or("?"))
                    }
                    crate::config::SourceType::Markdown => {
                        format!("markdown: {}", source.path.as_deref().unwrap_or("?"))
                    }
                    crate::config::SourceType::Github => {
                        format!("github: {}", source.repo.as_deref().unwrap_or("current repo"))
                    }
                };
                println!("  {}. {}", i + 1, desc);
            }
        }
        println!();

        // Limits
        println!("\x1b[1mLimits\x1b[0m");
        println!("  Max iterations: {}", config.limits.max_iterations);
        println!("  Max task failures: {}", config.limits.max_task_failures);
        println!("  Timeout: {} minutes", config.limits.timeout_minutes);
        println!();

        // AI CLI
        println!("\x1b[1mAI CLI\x1b[0m");
        println!("  Command: {} {}", config.ai_cli.command, config.ai_cli.args.join(" "));
        println!();

        // Git integration
        println!("\x1b[1mGit Integration\x1b[0m");
        println!("  Auto-commit: {}", if config.git.auto_commit { "yes" } else { "no" });
        println!("  Auto-branch: {}", if config.git.auto_branch { "yes" } else { "no" });
        if config.git.auto_branch {
            println!("  Branch prefix: {}", config.git.branch_prefix);
        }
        println!();

        // Archiving
        println!("\x1b[1mArchiving\x1b[0m");
        println!("  Enabled: {}", if config.archive.enabled { "yes" } else { "no" });
        if config.archive.enabled {
            println!("  Directory: {}", config.archive.directory);
            println!("  On branch change: {}", if config.archive.on_branch_change { "yes" } else { "no" });
        }
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
    /// Execute the sync command (alias for prd sync).
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

impl NextCommand {
    /// Execute the next command.
    pub fn execute(&self) {
        use crate::cli::output::output_prompt;
        use crate::config::{AfkConfig, OutputMode};
        use crate::prompt::generate_prompt;

        // Load config
        let config = AfkConfig::load(None).unwrap_or_default();

        // Generate the prompt (but don't increment iteration in future - for now it does)
        let result = match generate_prompt(&config, self.bootstrap, self.limit) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[31mError generating prompt:\x1b[0m {e}");
                std::process::exit(1);
            }
        };

        // Determine output mode
        let mode = if self.stdout {
            OutputMode::Stdout
        } else if self.file {
            OutputMode::File
        } else if self.copy {
            OutputMode::Clipboard
        } else {
            config.output.default.clone()
        };

        // Output the prompt
        let is_stdout = mode == OutputMode::Stdout;
        let _ = output_prompt(&result.prompt, mode, &config);

        // Show info unless going to stdout
        if !self.stdout && !is_stdout {
            println!("\x1b[2mIteration {}\x1b[0m", result.iteration);
            if result.all_complete {
                println!("\x1b[32m✓ All tasks complete!\x1b[0m");
            }
        }
    }
}

impl ExplainCommand {
    /// Execute the explain command.
    pub fn execute(&self) {
        use crate::config::AfkConfig;
        use crate::prd::PrdDocument;
        use crate::progress::SessionProgress;

        // Load everything
        let config = AfkConfig::load(None).unwrap_or_default();
        let prd = PrdDocument::load(None).unwrap_or_default();
        let progress = SessionProgress::load(None).unwrap_or_default();

        println!("\x1b[1m=== afk explain ===\x1b[0m\n");

        // Config summary
        println!("\x1b[1mConfiguration:\x1b[0m");
        println!("  AI CLI: {} {}", config.ai_cli.command, config.ai_cli.args.join(" "));
        println!("  Sources: {:?}", config.sources.iter().map(|s| &s.source_type).collect::<Vec<_>>());
        println!("  Output: {:?}", config.output.default);

        // PRD summary
        println!("\n\x1b[1mPRD:\x1b[0m");
        let (pending, completed) = prd.get_story_counts();
        let total = pending + completed;
        println!("  Stories: {completed}/{total} complete ({pending} pending)");
        
        if let Some(next) = prd.get_next_story() {
            println!("  Next: \x1b[1m{}\x1b[0m - {}", next.id, next.title);
        }

        // Progress summary
        println!("\n\x1b[1mProgress:\x1b[0m");
        println!("  Session started: {}", progress.started_at);
        println!("  Iterations: {}", progress.iterations);
        let (pend, in_prog, comp, fail, skip) = progress.get_task_counts();
        println!("  Tasks: {comp} complete, {in_prog} in-progress, {pend} pending, {fail} failed, {skip} skipped");

        // Verbose: show more details
        if self.verbose {
            println!("\n\x1b[1mFeedback Loops:\x1b[0m");
            let fb = &config.feedback_loops;
            let has_any = fb.types.is_some() || fb.lint.is_some() || fb.test.is_some() || fb.build.is_some() || !fb.custom.is_empty();
            if has_any {
                if let Some(ref cmd) = fb.types { println!("  types: {cmd}"); }
                if let Some(ref cmd) = fb.lint { println!("  lint: {cmd}"); }
                if let Some(ref cmd) = fb.test { println!("  test: {cmd}"); }
                if let Some(ref cmd) = fb.build { println!("  build: {cmd}"); }
                for (name, cmd) in &fb.custom {
                    println!("  {name}: {cmd}");
                }
            } else {
                println!("  (none configured)");
            }

            println!("\n\x1b[1mPending Stories:\x1b[0m");
            for story in prd.get_pending_stories().iter().take(5) {
                println!("  - {} (P{})", story.id, story.priority);
            }
            if prd.get_pending_stories().len() > 5 {
                println!("  ... and {} more", prd.get_pending_stories().len() - 5);
            }
        }
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
    /// Execute the done command.
    pub fn execute(&self) {
        use crate::prd::PrdDocument;
        use crate::progress::{SessionProgress, TaskStatus};

        // Load progress
        let mut progress = match SessionProgress::load(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[31mError loading progress:\x1b[0m {e}");
                std::process::exit(1);
            }
        };

        // Mark task as completed in progress
        progress.set_task_status(
            &self.task_id,
            TaskStatus::Completed,
            "manual",
            self.message.clone(),
        );

        if let Err(e) = progress.save(None) {
            eprintln!("\x1b[31mError saving progress:\x1b[0m {e}");
            std::process::exit(1);
        }

        // Also mark as passed in PRD
        if let Ok(mut prd) = PrdDocument::load(None) {
            prd.mark_story_complete(&self.task_id);
            let _ = prd.save(None);
        }

        println!("\x1b[32m✓\x1b[0m Task \x1b[1m{}\x1b[0m marked complete", self.task_id);
        if let Some(ref msg) = self.message {
            println!("  \x1b[2m{msg}\x1b[0m");
        }
    }
}

impl FailCommand {
    /// Execute the fail command.
    pub fn execute(&self) {
        use crate::progress::{SessionProgress, TaskStatus};

        // Load progress
        let mut progress = match SessionProgress::load(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[31mError loading progress:\x1b[0m {e}");
                std::process::exit(1);
            }
        };

        // Mark task as failed in progress
        progress.set_task_status(
            &self.task_id,
            TaskStatus::Failed,
            "manual",
            self.message.clone(),
        );

        if let Err(e) = progress.save(None) {
            eprintln!("\x1b[31mError saving progress:\x1b[0m {e}");
            std::process::exit(1);
        }

        let task = progress.get_task(&self.task_id);
        let count = task.map(|t| t.failure_count).unwrap_or(1);

        println!("\x1b[31m✗\x1b[0m Task \x1b[1m{}\x1b[0m marked failed (attempt {count})", self.task_id);
        if let Some(ref msg) = self.message {
            println!("  \x1b[2m{msg}\x1b[0m");
        }
    }
}

impl ResetCommand {
    /// Execute the reset command.
    pub fn execute(&self) {
        use crate::prd::PrdDocument;
        use crate::progress::{SessionProgress, TaskStatus};

        // Load progress
        let mut progress = match SessionProgress::load(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[31mError loading progress:\x1b[0m {e}");
                std::process::exit(1);
            }
        };

        // Reset task to pending
        progress.set_task_status(&self.task_id, TaskStatus::Pending, "manual", None);

        // Clear failure count if the task exists
        if let Some(task) = progress.get_task_mut(&self.task_id) {
            task.failure_count = 0;
            task.started_at = None;
            task.completed_at = None;
        }

        if let Err(e) = progress.save(None) {
            eprintln!("\x1b[31mError saving progress:\x1b[0m {e}");
            std::process::exit(1);
        }

        // Also reset passes in PRD
        if let Ok(mut prd) = PrdDocument::load(None) {
            if let Some(story) = prd.user_stories.iter_mut().find(|s| s.id == self.task_id) {
                story.passes = false;
            }
            let _ = prd.save(None);
        }

        println!("\x1b[33m↺\x1b[0m Task \x1b[1m{}\x1b[0m reset to pending", self.task_id);
    }
}

impl ArchiveCreateCommand {
    /// Execute the archive create command.
    pub fn execute(&self) {
        use crate::progress::archive_session;

        match archive_session(&self.reason) {
            Ok(Some(path)) => {
                println!("\x1b[32m✓\x1b[0m Session archived to: {}", path.display());
            }
            Ok(None) => {
                println!("\x1b[33mNo session to archive.\x1b[0m");
            }
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m Failed to archive session: {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ArchiveListCommand {
    /// Execute the archive list command.
    pub fn execute(&self) {
        use crate::progress::list_archives;

        match list_archives() {
            Ok(archives) => {
                if archives.is_empty() {
                    println!("No archived sessions found.");
                    return;
                }

                println!("\x1b[1mArchived Sessions\x1b[0m");
                println!();
                println!(
                    "{:<24} {:<20} {:<8} {:<10} {}",
                    "DATE", "BRANCH", "ITERS", "COMPLETED", "REASON"
                );
                println!("{}", "-".repeat(75));

                for (name, metadata) in archives.iter().take(20) {
                    let branch = metadata.branch.as_deref().unwrap_or("-");
                    let date = &metadata.archived_at[..19]; // Trim microseconds
                    println!(
                        "{:<24} {:<20} {:<8} {:<10} {}",
                        date.replace('T', " "),
                        if branch.len() > 18 { &branch[..18] } else { branch },
                        metadata.iterations,
                        format!("{}/{}", metadata.tasks_completed, metadata.tasks_completed + metadata.tasks_pending),
                        metadata.reason
                    );
                }

                if archives.len() > 20 {
                    println!();
                    println!("\x1b[2m... and {} more\x1b[0m", archives.len() - 20);
                }
            }
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m Failed to list archives: {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ArchiveClearCommand {
    /// Execute the archive clear command.
    pub fn execute(&self) {
        use crate::progress::{archive_session, clear_session};
        use std::io::{self, Write};

        // Confirm unless --yes
        if !self.yes {
            print!("Archive current session before clearing? [Y/n]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim().to_lowercase();
                if input != "n" && input != "no" {
                    // Archive first
                    match archive_session("clear") {
                        Ok(Some(path)) => {
                            println!("\x1b[32m✓\x1b[0m Archived to: {}", path.display());
                        }
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("\x1b[31mError:\x1b[0m Failed to archive: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        // Clear session
        match clear_session() {
            Ok(()) => {
                println!("\x1b[32m✓\x1b[0m Session cleared");
            }
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m Failed to clear session: {e}");
                std::process::exit(1);
            }
        }
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
    /// Execute the completions command - generates shell completions.
    pub fn execute(&self) {
        use clap::CommandFactory;
        use clap_complete::{generate, Shell};
        use std::io;

        let shell = match self.shell.as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            _ => {
                eprintln!("\x1b[31mError:\x1b[0m Unsupported shell: {}", self.shell);
                std::process::exit(1);
            }
        };

        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "afk", &mut io::stdout());
    }
}

impl StartCommand {
    /// Execute the start command (init if needed + run).
    pub fn execute(&self) {
        use crate::bootstrap::{analyse_project, ensure_ai_cli_configured, generate_config, infer_sources};
        use crate::config::AfkConfig;
        use crate::runner::run_loop;
        use std::fs;
        use std::io::{self, Write};
        use std::path::Path;

        let afk_dir = Path::new(".afk");
        let config_path = afk_dir.join("config.json");

        // Init if needed
        let config = if config_path.exists() {
            AfkConfig::load(Some(&config_path)).unwrap_or_default()
        } else {
            println!("\x1b[1mInitialising afk...\x1b[0m");
            
            let analysis = analyse_project(None);
            let mut config = generate_config(&analysis);
            config.sources = infer_sources(None);
            
            // Ensure AI CLI is configured (first-run experience)
            if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config)) {
                config.ai_cli = ai_cli;
            } else {
                eprintln!("\x1b[31mError:\x1b[0m No AI CLI configured.");
                std::process::exit(1);
            }

            // Create .afk directory
            if let Err(e) = fs::create_dir_all(afk_dir) {
                eprintln!("\x1b[31mError:\x1b[0m Failed to create .afk directory: {e}");
                std::process::exit(1);
            }

            // Save config
            if let Err(e) = config.save(Some(&config_path)) {
                eprintln!("\x1b[31mError:\x1b[0m Failed to save config: {e}");
                std::process::exit(1);
            }

            println!("\x1b[32m✓\x1b[0m Initialised");
            config
        };

        // Check for sources
        if config.sources.is_empty() {
            // Check for existing PRD
            let prd = crate::prd::PrdDocument::load(None).unwrap_or_default();
            if prd.user_stories.is_empty() {
                eprintln!("\x1b[33mNo task sources configured.\x1b[0m");
                eprintln!();
                eprintln!("Add a source first:");
                eprintln!("  afk source add beads      # Use beads issues");
                eprintln!("  afk prd parse spec.md     # Parse a requirements doc");
                std::process::exit(1);
            }
        }

        // Confirm unless --yes
        if !self.yes {
            println!();
            println!("Ready to start afk with {} iterations.", self.iterations);
            if let Some(ref branch) = self.branch {
                println!("Branch: {branch}");
            }
            print!("Continue? [Y/n]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim().to_lowercase();
                if input == "n" || input == "no" {
                    println!("Aborted.");
                    return;
                }
            }
        }

        // Run the loop
        let result = run_loop(
            &config,
            Some(self.iterations),
            self.branch.as_deref(),
            false, // until_complete
            None,  // timeout
            false, // resume
        );

        match result.stop_reason {
            crate::runner::StopReason::Complete => std::process::exit(0),
            crate::runner::StopReason::MaxIterations => std::process::exit(0),
            crate::runner::StopReason::UserInterrupt => std::process::exit(130),
            _ => std::process::exit(1),
        }
    }
}

impl ResumeCommand {
    /// Execute the resume command (continue from last session).
    pub fn execute(&self) {
        use crate::config::AfkConfig;
        use crate::progress::SessionProgress;
        use crate::runner::run_loop;
        use std::path::Path;

        // Check for existing progress
        let progress_path = Path::new(".afk/progress.json");
        if !progress_path.exists() {
            eprintln!("\x1b[33mNo session to resume.\x1b[0m");
            eprintln!("Run \x1b[1mafk start\x1b[0m or \x1b[1mafk go\x1b[0m to begin.");
            std::process::exit(1);
        }

        let progress = SessionProgress::load(None).unwrap_or_default();
        let config = AfkConfig::load(None).unwrap_or_default();

        println!("\x1b[1mResuming session...\x1b[0m");
        println!("  Started: {}", &progress.started_at[..19].replace('T', " "));
        println!("  Iterations so far: {}", progress.iterations);
        println!();

        // Run the loop with resume flag
        let result = run_loop(
            &config,
            Some(self.iterations),
            None, // branch - don't switch when resuming
            self.until_complete,
            self.timeout,
            true, // resume = true
        );

        match result.stop_reason {
            crate::runner::StopReason::Complete => std::process::exit(0),
            crate::runner::StopReason::MaxIterations => std::process::exit(0),
            crate::runner::StopReason::UserInterrupt => std::process::exit(130),
            _ => std::process::exit(1),
        }
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
