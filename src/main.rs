//! afk - Autonomous AI coding loops, Ralph Wiggum style.
//!
//! This is the main entry point for the afk CLI tool.

use afk::cli::{
    handle_result, ArchiveCommands, Cli, CliResult, Commands, ConfigCommands, ExitCode,
    SourceCommands, TasksCommands, TasksSyncCommand,
};
use clap::Parser;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    let result: CliResult = match cli.command {
        None => {
            // No subcommand provided - show help
            println!("afk - Autonomous AI coding loops, Ralph Wiggum style.");
            println!();
            println!("Run 'afk --help' for available commands.");
            println!();
            println!("Quick start:");
            println!("  afk go                 # Auto-detect, run 10 iterations");
            println!("  afk go 20              # Run 20 iterations");
            println!("  afk go TODO.md 5       # Use TODO.md, run 5 iterations");
            Ok(ExitCode::SUCCESS)
        }
        Some(cmd) => match cmd {
            Commands::Go(c) => c.execute(),
            Commands::Init(c) => c.execute(),
            Commands::Status(c) => c.execute(),
            Commands::Task(c) => c.execute(),
            Commands::Prompt(c) => c.execute(),
            Commands::Verify(c) => c.execute(),
            Commands::Done(c) => c.execute(),
            Commands::Fail(c) => c.execute(),
            Commands::Reset(c) => c.execute(),
            Commands::Source(subcmd) => match subcmd {
                SourceCommands::Add(c) => c.execute(),
                SourceCommands::List(c) => c.execute(),
                SourceCommands::Remove(c) => c.execute(),
            },
            Commands::Import(c) => c.execute(),
            Commands::Tasks {
                command,
                pending,
                complete,
                limit,
            } => match command {
                Some(TasksCommands::Sync(c)) => c.execute(),
                None => afk::cli::execute_tasks(pending, complete, limit),
            },
            Commands::Sync => {
                // Alias for `afk tasks sync`
                TasksSyncCommand {}.execute()
            }
            Commands::Archive {
                command,
                reason,
                yes,
            } => match command {
                Some(ArchiveCommands::List) => afk::cli::execute_archive_list(),
                None => afk::cli::execute_archive_now(&reason, yes),
            },
            Commands::Config(subcmd) => match subcmd {
                ConfigCommands::Show(c) => c.execute(),
                ConfigCommands::Get(c) => c.execute(),
                ConfigCommands::Set(c) => c.execute(),
                ConfigCommands::Reset(c) => c.execute(),
                ConfigCommands::Edit(c) => c.execute(),
                ConfigCommands::Explain(c) => c.execute(),
                ConfigCommands::Keys(c) => c.execute(),
            },
            Commands::Update(c) => c.execute(),
            Commands::Completions(c) => c.execute(),
            Commands::Use(c) => c.execute(),
        },
    };

    handle_result(result)
}
