//! Archive command implementations.
//!
//! This module implements the `afk archive` and `afk archive list` commands.

use std::io::{self, Write};
use std::path::Path;

use crate::progress::{archive_session, list_archives};

/// Result type for archive command operations.
pub type ArchiveCommandResult = Result<(), ArchiveCommandError>;

/// Error type for archive command operations.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveCommandError {
    #[error("Failed to archive session: {0}")]
    ArchiveError(String),
    #[error("Failed to list archives: {0}")]
    ListError(String),
}

/// Execute the archive command (archive and clear session).
pub fn archive_now(reason: &str, yes: bool) -> ArchiveCommandResult {
    // Check if there's anything to archive
    let progress_exists = Path::new(".afk/progress.json").exists();
    let tasks_exists = Path::new(".afk/tasks.json").exists();

    if !progress_exists && !tasks_exists {
        println!("\x1b[33mNo session to archive.\x1b[0m");
        return Ok(());
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
                return Ok(());
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
            return Err(ArchiveCommandError::ArchiveError(e.to_string()));
        }
    }

    Ok(())
}

/// Execute the archive list command.
pub fn archive_list() -> ArchiveCommandResult {
    let archives = list_archives().map_err(|e| ArchiveCommandError::ListError(e.to_string()))?;

    if archives.is_empty() {
        println!("No archived sessions found.");
        return Ok(());
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_command_error_display() {
        let err = ArchiveCommandError::ArchiveError("test error".to_string());
        assert!(err.to_string().contains("Failed to archive session"));

        let err = ArchiveCommandError::ListError("test error".to_string());
        assert!(err.to_string().contains("Failed to list archives"));
    }
}
