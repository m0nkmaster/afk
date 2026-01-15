//! Output handlers for afk prompts.
//!
//! This module provides functionality to output prompts to clipboard, file, or stdout.

use std::fs;
use std::path::Path;

use crate::config::AfkConfig;

// Re-export OutputMode for convenience
pub use crate::config::OutputMode;

/// Error type for output operations.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    /// Failed to copy content to the system clipboard.
    #[error("Failed to copy to clipboard: {0}")]
    ClipboardError(String),
    /// Failed to write output to a file.
    #[error("Failed to write to file: {0}")]
    IoError(#[from] std::io::Error),
}

/// Output the generated prompt using the specified mode.
///
/// # Arguments
///
/// * `prompt` - The prompt text to output
/// * `mode` - The output mode (clipboard, file, stdout)
/// * `config` - The afk configuration (used for file path when mode is File)
///
/// # Returns
///
/// Ok(()) on success, or an error if the output fails.
pub fn output_prompt(
    prompt: &str,
    mode: OutputMode,
    config: &AfkConfig,
) -> Result<(), OutputError> {
    match mode {
        OutputMode::Clipboard => copy_to_clipboard(prompt),
        OutputMode::File => write_to_file(prompt, &config.output.file_path),
        OutputMode::Stdout => {
            print_to_stdout(prompt);
            Ok(())
        }
    }
}

/// Copy prompt to system clipboard.
///
/// If clipboard access fails, falls back to stdout and returns an error.
pub fn copy_to_clipboard(prompt: &str) -> Result<(), OutputError> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(prompt.to_string()) {
            Ok(()) => {
                println!("\x1b[32mPrompt copied to clipboard!\x1b[0m");
                println!("\x1b[2m({} characters)\x1b[0m", prompt.len());
                Ok(())
            }
            Err(e) => {
                eprintln!("\x1b[31mFailed to copy to clipboard:\x1b[0m {e}");
                eprintln!("\x1b[2mFalling back to stdout...\x1b[0m");
                print_to_stdout(prompt);
                Err(OutputError::ClipboardError(e.to_string()))
            }
        },
        Err(e) => {
            eprintln!("\x1b[31mFailed to access clipboard:\x1b[0m {e}");
            eprintln!("\x1b[2mFalling back to stdout...\x1b[0m");
            print_to_stdout(prompt);
            Err(OutputError::ClipboardError(e.to_string()))
        }
    }
}

/// Write prompt to a file.
///
/// Creates parent directories if they don't exist.
pub fn write_to_file(prompt: &str, file_path: &str) -> Result<(), OutputError> {
    let path = Path::new(file_path);

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, prompt)?;

    println!("\x1b[32mPrompt written to:\x1b[0m {file_path}");
    println!("\x1b[2mInclude with: @{file_path}\x1b[0m");
    Ok(())
}

/// Print prompt to stdout.
pub fn print_to_stdout(prompt: &str) {
    println!("{prompt}");
}

/// Get the effective output mode.
///
/// Returns the explicit mode if provided, otherwise uses the config default.
pub fn get_effective_mode(copy: bool, file: bool, stdout: bool, config: &AfkConfig) -> OutputMode {
    if copy {
        OutputMode::Clipboard
    } else if file {
        OutputMode::File
    } else if stdout {
        OutputMode::Stdout
    } else {
        // Use config default
        config.output.default.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputConfig;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_write_to_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("output/prompt.txt");

        let result = write_to_file("Test prompt content", file_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(file_path.exists());

        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "Test prompt content");
    }

    #[test]
    fn test_write_to_file_creates_directories() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("nested/deep/path/prompt.txt");

        let result = write_to_file("Nested content", file_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_write_to_file_overwrites() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("prompt.txt");

        write_to_file("First content", file_path.to_str().unwrap()).unwrap();
        write_to_file("Second content", file_path.to_str().unwrap()).unwrap();

        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "Second content");
    }

    #[test]
    fn test_get_effective_mode_explicit_flags() {
        let config = AfkConfig::default();

        assert_eq!(
            get_effective_mode(true, false, false, &config),
            OutputMode::Clipboard
        );
        assert_eq!(
            get_effective_mode(false, true, false, &config),
            OutputMode::File
        );
        assert_eq!(
            get_effective_mode(false, false, true, &config),
            OutputMode::Stdout
        );
    }

    #[test]
    fn test_get_effective_mode_uses_config_default() {
        let config = AfkConfig {
            output: OutputConfig {
                default: OutputMode::File,
                file_path: ".afk/prompt.txt".to_string(),
            },
            ..Default::default()
        };

        assert_eq!(
            get_effective_mode(false, false, false, &config),
            OutputMode::File
        );
    }

    #[test]
    fn test_get_effective_mode_priority() {
        let config = AfkConfig::default();

        // If multiple flags set, priority is: copy > file > stdout
        assert_eq!(
            get_effective_mode(true, true, true, &config),
            OutputMode::Clipboard
        );
        assert_eq!(
            get_effective_mode(false, true, true, &config),
            OutputMode::File
        );
    }

    #[test]
    fn test_output_error_display() {
        let err = OutputError::ClipboardError("access denied".to_string());
        assert!(err.to_string().contains("Failed to copy to clipboard"));
        assert!(err.to_string().contains("access denied"));

        let io_err = OutputError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("Failed to write to file"));
    }

    // Note: We can't easily test clipboard operations in CI environments
    // as they typically don't have a display server. The clipboard tests
    // would need to be run manually on a system with a GUI.
}
