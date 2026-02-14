//! Plan command implementation.
//!
//! This module implements `afk plan` - AI-assisted task decomposition with
//! an enhanced planning prompt that includes per-task verification.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::bootstrap::ensure_ai_cli_configured;
use crate::cli::output::{get_effective_mode, output_prompt};
use crate::config::AfkConfig;
use crate::feedback::Spinner;
use crate::prd::{load_prd_file, PrdDocument, PrdError};

use tera::{Context, Tera};

/// Planning prompt template for AI-assisted task decomposition.
///
/// Enhanced version of the PRD parse template that includes
/// `verifyCommand` and `doneCriteria` fields.
pub const PLANNING_TEMPLATE: &str = include_str!("../../prompt/sop_planning.md");

/// Result type for plan command operations.
pub type PlanCommandResult = Result<(), PlanCommandError>;

/// Error type for plan command operations.
#[derive(Debug, thiserror::Error)]
pub enum PlanCommandError {
    /// Error during PRD import operation.
    #[error("Plan error: {0}")]
    ImportError(#[from] PrdError),
    /// Error parsing PRD document.
    #[error("Parse error: {0}")]
    ParseError(#[from] crate::prd::PrdParseError),
    /// Configuration error.
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
    /// No AI CLI configured or available.
    #[error("No AI CLI configured. Run `afk init` first.")]
    NoAiCli,
    /// Tera template rendering error.
    #[error("Template error: {0}")]
    TemplateError(#[from] tera::Error),
    /// Error writing output.
    #[error("Output error: {0}")]
    OutputError(#[from] crate::cli::output::OutputError),
    /// Input file was not found.
    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Generate the planning prompt from requirements content.
///
/// Uses the enhanced planning template that includes verification fields.
pub fn generate_planning_prompt(
    prd_content: &str,
    output_path: &str,
) -> Result<String, PlanCommandError> {
    let mut tera = Tera::default();
    tera.add_raw_template("planning", PLANNING_TEMPLATE)?;

    let mut context = Context::new();
    context.insert("prd_content", prd_content);
    context.insert("output_path", output_path);

    let prompt = tera.render("planning", &context)?;
    Ok(prompt)
}

/// Plan tasks from a requirements file.
///
/// Takes a requirements document and generates an AI-assisted task
/// decomposition using the enhanced planning prompt.
///
/// # Arguments
///
/// * `input_file` - Path to the requirements file
/// * `output` - Path for the generated JSON output
/// * `copy` - Copy prompt to clipboard
/// * `file` - Write prompt to file
/// * `stdout` - Print prompt to stdout
pub fn plan(
    input_file: &str,
    output: &str,
    copy: bool,
    file: bool,
    stdout: bool,
) -> PlanCommandResult {
    plan_impl(input_file, output, copy, file, stdout, None)
}

/// Internal implementation of plan with optional config path for testing.
pub fn plan_impl(
    input_file: &str,
    output: &str,
    copy: bool,
    file: bool,
    stdout: bool,
    config_path: Option<&Path>,
) -> PlanCommandResult {
    let mut config = AfkConfig::load(config_path)?;

    // Load the input file
    let input_path = Path::new(input_file);
    if !input_path.exists() {
        return Err(PlanCommandError::FileNotFound(input_file.to_string()));
    }

    let prd_content = load_prd_file(input_path)?;

    // Generate the planning prompt (uses enhanced template)
    let prompt = generate_planning_prompt(&prd_content, output)?;

    // If any output flag is specified, output the prompt for manual use
    if copy || file || stdout {
        let mode = get_effective_mode(copy, file, stdout, &config);
        output_prompt(&prompt, mode, &config)?;

        // Show next steps
        println!();
        println!("\x1b[2mRun the prompt with your AI tool, then start working:\x1b[0m");
        println!("  \x1b[36mafk go\x1b[0m");

        return Ok(());
    }

    // No output flags - run the AI CLI directly
    if let Some(ai_cli) = ensure_ai_cli_configured(Some(&mut config), false) {
        config.ai_cli = ai_cli;
    } else {
        return Err(PlanCommandError::NoAiCli);
    }

    // Run the AI CLI with the planning prompt
    run_ai_cli_for_plan(&config, &prompt, output)
}

/// Run the AI CLI with the planning prompt.
fn run_ai_cli_for_plan(config: &AfkConfig, prompt: &str, output: &str) -> PlanCommandResult {
    let command = &config.ai_cli.command;
    let args: Vec<&str> = config.ai_cli.args.iter().map(|s| s.as_str()).collect();

    // Start spinner whilst AI CLI initialises
    let mut spinner = Some(Spinner::start(&format!(
        "Planning tasks with {}...",
        config.ai_cli.command
    )));

    // Build the command
    let mut cmd = Command::new(command);
    cmd.args(&args)
        .arg(prompt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Spawn process
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                if let Some(s) = spinner.take() {
                    s.stop_with_error(&format!("AI CLI not found: {}", command));
                }
                eprintln!("\x1b[2mIs it installed and in your PATH?\x1b[0m");
                return Err(PlanCommandError::ImportError(PrdError::ReadError(e)));
            }
            if let Some(s) = spinner.take() {
                s.stop_with_error("Failed to start AI CLI");
            }
            return Err(PlanCommandError::ImportError(PrdError::ReadError(e)));
        }
    };

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            // Stop spinner on first output
            if let Some(s) = spinner.take() {
                s.stop();
                println!();
            }

            match line {
                Ok(line) => {
                    println!("{line}");
                }
                Err(e) => {
                    eprintln!("\x1b[33mWarning:\x1b[0m Error reading output: {e}");
                    break;
                }
            }
        }
    }

    // If no output was received, stop the spinner now
    if let Some(s) = spinner.take() {
        s.stop();
    }

    // Wait for process to finish
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                let exit_code = status.code().unwrap_or(-1);
                eprintln!("\x1b[31mError:\x1b[0m AI CLI exited with code {exit_code}");
                return Err(PlanCommandError::ImportError(PrdError::ReadError(
                    std::io::Error::other(format!("AI CLI exited with code {exit_code}")),
                )));
            }
        }
        Err(e) => {
            return Err(PlanCommandError::ImportError(PrdError::ReadError(e)));
        }
    }

    // Check if output file was created and show summary
    let output_path = Path::new(output);
    if output_path.exists() {
        println!();
        println!("\x1b[32m✓\x1b[0m Tasks planned successfully");
        println!("  Output: \x1b[36m{output}\x1b[0m");

        // Show task summary
        if let Ok(prd) = PrdDocument::load(Some(output_path)) {
            let (_, total) = prd.get_story_counts();
            let with_verify = prd
                .user_stories
                .iter()
                .filter(|s| s.verify_command.is_some())
                .count();

            println!();
            println!("  \x1b[36mTasks:\x1b[0m       {total}");
            if with_verify > 0 {
                println!("  \x1b[36mWith verify:\x1b[0m  {with_verify}");
            }

            // Show task list
            println!();
            for story in &prd.user_stories {
                let check = if story.passes { "✓" } else { "○" };
                println!("  \x1b[2m{check}\x1b[0m {} — {}", story.id, story.title);
            }
        }

        println!();
        println!("\x1b[2mReview the tasks, then start working:\x1b[0m");
        println!("  \x1b[36mafk tasks\x1b[0m       \x1b[2m# Review task list\x1b[0m");
        println!("  \x1b[36mafk go\x1b[0m          \x1b[2m# Start executing\x1b[0m");
    } else {
        println!();
        println!("\x1b[33mNote:\x1b[0m Output file not found at {output}");
        println!("\x1b[2mThe AI may have written to a different location.\x1b[0m");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_planning_template_not_empty() {
        assert!(!PLANNING_TEMPLATE.is_empty());
    }

    #[test]
    fn test_planning_template_has_key_sections() {
        assert!(PLANNING_TEMPLATE.contains("# AI-Assisted Task Planning"));
        assert!(PLANNING_TEMPLATE.contains("## Input Requirements"));
        assert!(PLANNING_TEMPLATE.contains("## Output Format"));
        assert!(PLANNING_TEMPLATE.contains("## Per-Task Verification"));
        assert!(PLANNING_TEMPLATE.contains("## Task Sizing (CRITICAL)"));
        assert!(PLANNING_TEMPLATE.contains("verifyCommand"));
        assert!(PLANNING_TEMPLATE.contains("doneCriteria"));
    }

    #[test]
    fn test_planning_template_has_variables() {
        assert!(PLANNING_TEMPLATE.contains("{{ prd_content }}"));
        assert!(PLANNING_TEMPLATE.contains("{{ output_path }}"));
    }

    #[test]
    fn test_generate_planning_prompt() {
        let content = "# My App\n\nBuild a todo list app with authentication.";
        let output_path = ".afk/tasks.json";

        let result = generate_planning_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("Build a todo list app with authentication"));
        assert!(prompt.contains(".afk/tasks.json"));
        assert!(prompt.contains("# AI-Assisted Task Planning"));
    }

    #[test]
    fn test_generate_planning_prompt_with_special_chars() {
        let content = "Features:\n- Use `code blocks`\n- Handle \"quotes\"";
        let output_path = "output/tasks.json";

        let result = generate_planning_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("Use `code blocks`"));
        assert!(prompt.contains("Handle \"quotes\""));
    }

    #[test]
    fn test_generate_planning_prompt_empty_content() {
        let content = "";
        let output_path = ".afk/tasks.json";

        let result = generate_planning_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("# AI-Assisted Task Planning"));
    }

    #[test]
    fn test_plan_file_not_found() {
        let (_temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");
        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = plan_impl(
            "/nonexistent/file.md",
            ".afk/tasks.json",
            false,
            false,
            true,
            Some(&config_path),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            PlanCommandError::FileNotFound(path) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_plan_generates_prompt_to_stdout() {
        let (temp, afk_dir) = setup_temp_dir();
        let config_path = afk_dir.join("config.json");

        // Create input file
        let input_file = temp.path().join("requirements.md");
        let content = "# My App\n\nBuild a todo list with user authentication.";
        fs::write(&input_file, content).unwrap();

        let config = AfkConfig::default();
        config.save(Some(&config_path)).unwrap();

        let result = plan_impl(
            input_file.to_str().unwrap(),
            ".afk/tasks.json",
            false,
            false,
            true,
            Some(&config_path),
        );

        // Should succeed with stdout mode
        assert!(result.is_ok());
    }

    #[test]
    fn test_plan_command_error_display() {
        let err = PlanCommandError::NoAiCli;
        assert!(err.to_string().contains("No AI CLI configured"));

        let err = PlanCommandError::FileNotFound("test.md".to_string());
        assert!(err.to_string().contains("File not found"));
        assert!(err.to_string().contains("test.md"));
    }

    /// Helper to set up a temp directory with .afk subdirectory.
    fn setup_temp_dir() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        (temp, afk_dir)
    }
}
