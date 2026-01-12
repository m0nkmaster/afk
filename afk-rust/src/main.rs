//! afk - Autonomous AI coding loops, Ralph Wiggum style.
//!
//! This is the main entry point for the afk CLI tool.

use clap::Parser;

/// Autonomous AI coding loops - Ralph Wiggum style.
#[derive(Parser, Debug)]
#[command(name = "afk")]
#[command(author, version, about, long_about = None)]
struct Cli {
    // Commands will be added here in future stories
}

fn main() {
    let _cli = Cli::parse();

    // No subcommands implemented yet - just version and help work
    println!("afk: Autonomous AI coding loops");
    println!("Run 'afk --help' for available commands.");
}
