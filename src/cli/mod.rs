//! CLI commands and argument handling.
//!
//! This module contains the clap CLI definitions and command implementations.

pub mod commands;
pub mod output;
pub mod update;

use clap::{Args, Parser, Subcommand};

/// Autonomous AI coding loops - Ralph Wiggum style.
///
/// Run AI coding tasks in a loop with fresh context each iteration.
/// Memory persists via git history, progress.json, and task sources.
#[derive(Parser, Debug)]
#[command(name = "afk")]
#[command(author, version = crate::VERSION, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
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

    /// Delete config and re-run setup.
    ///
    /// Wipes existing .afk/config.json and prompts for reconfiguration.
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

    /// Overwrite existing config.
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
pub struct TasksSyncCommand {}

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

// ============================================================================
// Command implementations
// ============================================================================

impl GoCommand {
    /// Execute the go command.
    pub fn execute(&self) {
        use crate::bootstrap::{
            analyse_project, ensure_ai_cli_configured, generate_config,
            infer_sources as bootstrap_infer_sources,
        };
        use crate::config::{AfkConfig, SourceConfig};
        use crate::prd::PrdDocument;
        use crate::runner::{run_loop_with_options, run_loop_with_tui, RunOptions};
        use std::fs;
        use std::path::Path;

        let afk_dir = Path::new(".afk");
        let config_path = afk_dir.join("config.json");

        // Handle --init flag: delete config and re-run setup
        if self.init && config_path.exists() {
            if let Err(e) = fs::remove_file(&config_path) {
                eprintln!("\x1b[31mError:\x1b[0m Failed to remove config: {e}");
                std::process::exit(1);
            }
            println!("\x1b[2mCleared existing configuration.\x1b[0m");
        }

        // Handle --fresh flag: clear session progress to start fresh
        if self.fresh {
            let progress_path = afk_dir.join("progress.json");
            if progress_path.exists() {
                if let Err(e) = fs::remove_file(&progress_path) {
                    eprintln!("\x1b[31mError:\x1b[0m Failed to clear progress: {e}");
                    std::process::exit(1);
                }
                println!("\x1b[2mCleared session progress.\x1b[0m");
            }
        }

        // Parse iterations_or_source
        let (iterations, explicit_source) = self.parse_args();

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
                if let Err(e) = fs::create_dir_all(afk_dir) {
                    eprintln!("\x1b[31mError:\x1b[0m Failed to create .afk directory: {e}");
                    std::process::exit(1);
                }
            }

            new_config
        };

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
                println!(
                    "\x1b[2mUsing existing .afk/tasks.json ({} tasks)\x1b[0m",
                    prd.user_stories.len()
                );
            } else {
                // Try to infer sources
                let inferred = infer_sources();
                if inferred.is_empty() {
                    eprintln!("\x1b[33mNo task sources found.\x1b[0m");
                    eprintln!();
                    eprintln!("Try one of:");
                    eprintln!("  afk go TODO.md           # Use a markdown file");
                    eprintln!("  afk import spec.md       # Import a requirements doc");
                    eprintln!("  afk source add beads     # Use beads issues");
                    std::process::exit(1);
                }
                config.sources = inferred;
            }
        }

        // Ensure AI CLI is configured (first-run experience)
        // This will prompt the user to select if not already configured
        if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config)) {
            config.ai_cli = ai_cli;
        } else {
            // No AI CLI available - exit
            eprintln!("\x1b[31mError:\x1b[0m No AI CLI configured. Install one and try again.");
            std::process::exit(1);
        }

        // Save config if it was newly created or modified
        if !config_path.exists() || self.init {
            if let Err(e) = config.save(Some(&config_path)) {
                eprintln!("\x1b[31mError:\x1b[0m Failed to save config: {e}");
                std::process::exit(1);
            }
            println!(
                "\x1b[32m✓\x1b[0m Configuration saved to {}",
                config_path.display()
            );
        }

        // Dry run mode
        if self.dry_run {
            let effective_iterations = iterations.unwrap_or(config.limits.max_iterations);
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
            return;
        }

        // Build run options with feedback settings
        // Use config.limits.max_iterations as default when not explicitly specified
        let effective_iterations = iterations.or(Some(config.limits.max_iterations));
        let options = RunOptions::new()
            .with_iterations(effective_iterations)
            .with_until_complete(self.until_complete)
            .with_timeout(self.timeout)
            .with_resume(false)
            .with_feedback_mode(RunOptions::parse_feedback_mode(self.feedback.as_deref()))
            .with_mascot(!self.no_mascot);

        // Run the loop - use TUI if requested
        let result = if RunOptions::is_tui_mode(self.feedback.as_deref()) {
            run_loop_with_tui(&config, options)
        } else {
            run_loop_with_options(&config, options)
        };

        // Exit with appropriate code
        match result.stop_reason {
            crate::runner::StopReason::Complete => std::process::exit(0),
            crate::runner::StopReason::MaxIterations => std::process::exit(0),
            crate::runner::StopReason::UserInterrupt => std::process::exit(130),
            _ => std::process::exit(1),
        }
    }

    /// Parse iterations_or_source argument.
    /// Returns (iterations, source_path). When iterations is None, caller should
    /// fall back to config.limits.max_iterations.
    fn parse_args(&self) -> (Option<u32>, Option<String>) {
        match &self.iterations_or_source {
            Some(arg) => {
                // Try to parse as number first
                if let Ok(n) = arg.parse::<u32>() {
                    (Some(n), None)
                } else {
                    // It's a path - use iterations_if_source if provided
                    (self.iterations_if_source, Some(arg.clone()))
                }
            }
            None => (None, None), // Use config.limits.max_iterations as default
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

impl InitCommand {
    /// Execute the init command.
    pub fn execute(&self) {
        use crate::bootstrap::{
            analyse_project, ensure_ai_cli_configured, generate_config, infer_sources,
        };
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

        // Handle AI CLI selection
        // In dry-run mode, just detect without prompting or saving
        // Otherwise, prompt user to select (first-run experience)
        if self.dry_run {
            if let Some(ai_cli) = crate::bootstrap::detect_ai_cli() {
                config.ai_cli = ai_cli;
            }
        } else if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config)) {
            config.ai_cli = ai_cli;
        } else {
            eprintln!("\x1b[31mError:\x1b[0m No AI CLI configured.");
            std::process::exit(1);
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

        // Create empty tasks.json
        let tasks_path = afk_dir.join("tasks.json");
        if !tasks_path.exists() {
            let empty_tasks = r#"{
  "project": "",
  "branchName": "",
  "description": "",
  "userStories": []
}"#;
            if let Err(e) = fs::write(&tasks_path, empty_tasks) {
                eprintln!("\x1b[31mError creating tasks.json:\x1b[0m {e}");
            }
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
            println!("Run \x1b[1mafk init\x1b[0m or \x1b[1mafk go\x1b[0m to get started.");
            return;
        }

        let config = AfkConfig::load(None).unwrap_or_default();
        let prd = PrdDocument::load(None).unwrap_or_default();
        let progress = SessionProgress::load(None).unwrap_or_default();

        println!("\x1b[1m=== afk status ===\x1b[0m");
        println!();

        // Task summary
        let (completed, total) = prd.get_story_counts();
        let pending = total - completed;

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
        println!(
            "  Started: {}",
            &progress.started_at[..19].replace('T', " ")
        );
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
                        format!(
                            "github: {}",
                            source.repo.as_deref().unwrap_or("current repo")
                        )
                    }
                };
                println!("  {}. {}", i + 1, desc);
            }
        }
        println!();

        // AI CLI
        println!("\x1b[1mAI CLI\x1b[0m");
        println!(
            "  Command: {} {}",
            config.ai_cli.command,
            config.ai_cli.args.join(" ")
        );

        // Verbose mode: show additional details
        if self.verbose {
            println!();

            // Feedback Loops
            println!("\x1b[1mFeedback Loops\x1b[0m");
            let fb = &config.feedback_loops;
            let has_any = fb.types.is_some()
                || fb.lint.is_some()
                || fb.test.is_some()
                || fb.build.is_some()
                || !fb.custom.is_empty();
            if has_any {
                if let Some(ref cmd) = fb.types {
                    println!("  types: {cmd}");
                }
                if let Some(ref cmd) = fb.lint {
                    println!("  lint: {cmd}");
                }
                if let Some(ref cmd) = fb.test {
                    println!("  test: {cmd}");
                }
                if let Some(ref cmd) = fb.build {
                    println!("  build: {cmd}");
                }
                for (name, cmd) in &fb.custom {
                    println!("  {name}: {cmd}");
                }
            } else {
                println!("  (none configured)");
            }
            println!();

            // Pending Stories
            println!("\x1b[1mPending Stories\x1b[0m");
            let pending_stories = prd.get_pending_stories();
            if pending_stories.is_empty() {
                println!("  (none)");
            } else {
                for story in pending_stories.iter().take(5) {
                    println!("  - {} (P{}) {}", story.id, story.priority, story.title);
                }
                if pending_stories.len() > 5 {
                    println!("  ... and {} more", pending_stories.len() - 5);
                }
            }
            println!();

            // Recent Learnings
            println!("\x1b[1mRecent Learnings\x1b[0m");
            let learnings = progress.get_recent_learnings(5);
            if learnings.is_empty() {
                println!("  (none recorded)");
            } else {
                for (i, (task_id, learning)) in learnings.iter().enumerate() {
                    let truncated = if learning.len() > 60 {
                        format!("{}...", &learning[..57])
                    } else {
                        learning.clone()
                    };
                    println!("  {}. [{}] {}", i + 1, task_id, truncated);
                }
            }
        }
    }
}

impl TaskCommand {
    /// Execute the task command.
    pub fn execute(&self) {
        use crate::prd::PrdDocument;
        use crate::progress::SessionProgress;

        let prd = PrdDocument::load(None).unwrap_or_default();
        let progress = SessionProgress::load(None).unwrap_or_default();

        // Find the story
        let story = match prd.user_stories.iter().find(|s| s.id == self.task_id) {
            Some(s) => s,
            None => {
                eprintln!("\x1b[31mError:\x1b[0m Task not found: {}", self.task_id);
                std::process::exit(1);
            }
        };

        // Get task progress if available
        let task_progress = progress.get_task(&self.task_id);

        println!("\x1b[1m=== {} ===\x1b[0m", story.id);
        println!();

        println!("\x1b[1mTitle:\x1b[0m {}", story.title);
        println!(
            "\x1b[1mStatus:\x1b[0m {}",
            if story.passes { "complete" } else { "pending" }
        );
        println!("\x1b[1mPriority:\x1b[0m {}", story.priority);
        println!();

        if !story.description.is_empty() {
            println!("\x1b[1mDescription:\x1b[0m");
            for line in story.description.lines() {
                println!("  {line}");
            }
            println!();
        }

        if !story.acceptance_criteria.is_empty() {
            println!("\x1b[1mAcceptance Criteria:\x1b[0m");
            for criterion in &story.acceptance_criteria {
                let check = if story.passes { "✓" } else { "○" };
                println!("  {check} {criterion}");
            }
            println!();
        }

        // Show learnings from progress
        if let Some(task) = task_progress {
            if !task.learnings.is_empty() {
                println!("\x1b[1mLearnings:\x1b[0m");
                for learning in &task.learnings {
                    println!("  - {learning}");
                }
                println!();
            }

            println!("\x1b[1mAttempts:\x1b[0m {}", task.failure_count + 1);
            if let Some(ref started) = task.started_at {
                println!(
                    "\x1b[1mStarted:\x1b[0m {}",
                    &started[..19].replace('T', " ")
                );
            }
            if let Some(ref completed) = task.completed_at {
                println!(
                    "\x1b[1mCompleted:\x1b[0m {}",
                    &completed[..19].replace('T', " ")
                );
            }
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

impl ImportCommand {
    /// Execute the import command.
    pub fn execute(&self) {
        match commands::import::import(
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

impl TasksSyncCommand {
    /// Execute the tasks sync command.
    pub fn execute(&self) {
        match commands::import::tasks_sync() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Execute the tasks command (list tasks).
pub fn execute_tasks(pending: bool, complete: bool, limit: usize) {
    match commands::import::tasks_show(pending, complete, limit) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("\x1b[31mError:\x1b[0m {e}");
            std::process::exit(1);
        }
    }
}

impl PromptCommand {
    /// Execute the prompt command.
    pub fn execute(&self) {
        use crate::cli::output::output_prompt;
        use crate::config::{AfkConfig, OutputMode};
        use crate::prompt::generate_prompt;

        // Load config
        let config = AfkConfig::load(None).unwrap_or_default();

        // Generate the prompt
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

        println!(
            "\x1b[32m✓\x1b[0m Task \x1b[1m{}\x1b[0m marked complete",
            self.task_id
        );
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

        println!(
            "\x1b[31m✗\x1b[0m Task \x1b[1m{}\x1b[0m marked failed (attempt {count})",
            self.task_id
        );
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

        println!(
            "\x1b[33m↺\x1b[0m Task \x1b[1m{}\x1b[0m reset to pending",
            self.task_id
        );
    }
}

/// Execute the archive command (archive and clear session).
///
/// Prompts for confirmation, then moves session files to archive.
pub fn execute_archive_now(reason: &str, yes: bool) {
    use crate::progress::archive_session;
    use std::io::{self, Write};

    // Check if there's anything to archive
    let progress_exists = std::path::Path::new(".afk/progress.json").exists();
    let tasks_exists = std::path::Path::new(".afk/tasks.json").exists();

    if !progress_exists && !tasks_exists {
        println!("\x1b[33mNo session to archive.\x1b[0m");
        return;
    }

    // Confirm unless --yes
    if !yes {
        print!("Archive and clear current session? [Y/n]: ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input == "n" || input == "no" {
                println!("Cancelled.");
                return;
            }
        }
    }

    match archive_session(reason) {
        Ok(Some(path)) => {
            println!("\x1b[32m✓\x1b[0m Session archived to: {}", path.display());
            println!("\x1b[32m✓\x1b[0m Session cleared, ready for fresh work");
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

/// Execute the archive list command.
pub fn execute_archive_list() {
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
                "{:<24} {:<20} {:<8} {:<10} REASON",
                "DATE", "BRANCH", "ITERS", "COMPLETED"
            );
            println!("{}", "-".repeat(75));

            for (_name, metadata) in archives.iter().take(20) {
                let branch = metadata.branch.as_deref().unwrap_or("-");
                let date = &metadata.archived_at[..19]; // Trim microseconds
                println!(
                    "{:<24} {:<20} {:<8} {:<10} {}",
                    date.replace('T', " "),
                    if branch.len() > 18 {
                        &branch[..18]
                    } else {
                        branch
                    },
                    metadata.iterations,
                    format!(
                        "{}/{}",
                        metadata.tasks_completed,
                        metadata.tasks_completed + metadata.tasks_pending
                    ),
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

impl ConfigShowCommand {
    /// Execute the config show command.
    pub fn execute(&self) {
        match commands::config::config_show(self.section.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigGetCommand {
    /// Execute the config get command.
    pub fn execute(&self) {
        match commands::config::config_get(&self.key) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigSetCommand {
    /// Execute the config set command.
    pub fn execute(&self) {
        match commands::config::config_set(&self.key, &self.value) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigResetCommand {
    /// Execute the config reset command.
    pub fn execute(&self) {
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
                    return;
                }
            }
        }

        match commands::config::config_reset(self.key.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigEditCommand {
    /// Execute the config edit command.
    pub fn execute(&self) {
        match commands::config::config_edit() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigExplainCommand {
    /// Execute the config explain command.
    pub fn execute(&self) {
        match commands::config::config_explain(self.key.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl ConfigKeysCommand {
    /// Execute the config keys command.
    pub fn execute(&self) {
        match commands::config::config_keys() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {e}");
                std::process::exit(1);
            }
        }
    }
}

impl UpdateCommand {
    /// Execute the update command - checks for and installs updates.
    pub fn execute(&self) {
        if let Err(e) = update::execute_update(self.beta, self.check) {
            eprintln!("\x1b[31mError:\x1b[0m {e}");
            std::process::exit(1);
        }
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
}
