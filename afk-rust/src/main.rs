//! afk - Autonomous AI coding loops, Ralph Wiggum style.
//!
//! This is the main entry point for the afk CLI tool.

use afk::cli::{
    ArchiveCommands, Cli, Commands, PrdCommands, SourceCommands,
};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    match cli.command {
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
        }
        Some(cmd) => match cmd {
            Commands::Go(c) => c.execute(),
            Commands::Run(c) => c.execute(),
            Commands::Init(c) => c.execute(),
            Commands::Status(c) => c.execute(),
            Commands::Source(subcmd) => match subcmd {
                SourceCommands::Add(c) => c.execute(),
                SourceCommands::List(c) => c.execute(),
                SourceCommands::Remove(c) => c.execute(),
            },
            Commands::Prd(subcmd) => match subcmd {
                PrdCommands::Parse(c) => c.execute(),
                PrdCommands::Sync(c) => c.execute(),
                PrdCommands::Show(c) => c.execute(),
            },
            Commands::Sync(c) => c.execute(),
            Commands::Next(c) => c.execute(),
            Commands::Explain(c) => c.execute(),
            Commands::Verify(c) => c.execute(),
            Commands::Done(c) => c.execute(),
            Commands::Fail(c) => c.execute(),
            Commands::Reset(c) => c.execute(),
            Commands::Archive(subcmd) => match subcmd {
                ArchiveCommands::Create(c) => c.execute(),
                ArchiveCommands::List(c) => c.execute(),
                ArchiveCommands::Clear(c) => c.execute(),
            },
            Commands::Update(c) => c.execute(),
            Commands::Completions(c) => c.execute(),
            Commands::Start(c) => c.execute(),
            Commands::Resume(c) => c.execute(),
        },
    }
}
