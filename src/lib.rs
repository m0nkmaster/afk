//! afk - Autonomous AI coding loops, Ralph Wiggum style.
//!
//! This library provides the core functionality for the afk CLI tool,
//! implementing the Ralph Wiggum pattern for autonomous AI coding.
//!

// Include the generated version string
include!(concat!(env!("OUT_DIR"), "/version.rs"));

// Module declarations - to be implemented in future stories
pub mod bootstrap;
pub mod cli;
pub mod config;
pub mod feedback;
pub mod git;
pub mod parser;
pub mod prd;
pub mod progress;
pub mod prompt;
pub mod runner;
pub mod sources;
pub mod watcher;

// Re-export key types for convenience
pub use sources::{aggregate_tasks, SourceError};
