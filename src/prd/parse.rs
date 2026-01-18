//! PRD parsing prompt generation.
//!
//! This module provides functionality to convert raw PRD documents
//! into structured JSON user stories using AI prompts.

use std::fs;
use std::path::Path;

use tera::{Context, Tera};

/// PRD parse template for converting requirements documents to user stories.
///
/// Generates Ralph-style user stories with specific acceptance criteria,
/// quality gates, and browser verification for UI tasks.
///
/// Uses Tera template syntax. Loaded from `parse_template.md`.
pub const PRD_PARSE_TEMPLATE: &str = include_str!("parse_template.md");

/// Error type for PRD parse operations.
#[derive(Debug, thiserror::Error)]
pub enum PrdParseError {
    /// File I/O error.
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
    /// Tera template rendering error.
    #[error("Template rendering failed: {0}")]
    TemplateError(#[from] tera::Error),
}

/// Generate a PRD parsing prompt from content.
///
/// Takes raw PRD content and generates an AI prompt that will convert
/// it into the structured JSON format.
///
/// # Arguments
///
/// * `prd_content` - The raw PRD content (markdown, text, etc.)
/// * `output_path` - Target path for the generated JSON file
///
/// # Returns
///
/// The generated prompt string, or an error.
pub fn generate_prd_prompt(prd_content: &str, output_path: &str) -> Result<String, PrdParseError> {
    let mut tera = Tera::default();
    tera.add_raw_template("prd_parse", PRD_PARSE_TEMPLATE)?;

    let mut context = Context::new();
    context.insert("prd_content", prd_content);
    context.insert("output_path", output_path);

    let prompt = tera.render("prd_parse", &context)?;
    Ok(prompt)
}

/// Load a PRD file from disk.
///
/// Reads the contents of a file as a string.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// The file contents as a string, or an error if the file cannot be read.
pub fn load_prd_file(path: &Path) -> Result<String, PrdParseError> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_prd_parse_template_not_empty() {
        assert!(!PRD_PARSE_TEMPLATE.is_empty());
    }

    #[test]
    fn test_prd_parse_template_has_key_sections() {
        assert!(PRD_PARSE_TEMPLATE.contains("# Parse PRD into Structured User Stories"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Input PRD"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Output Format"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Field Definitions"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Acceptance Criteria Guidelines"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Task Sizing (CRITICAL)"));
    }

    #[test]
    fn test_prd_parse_template_has_variables() {
        assert!(PRD_PARSE_TEMPLATE.contains("{{ prd_content }}"));
        assert!(PRD_PARSE_TEMPLATE.contains("{{ output_path }}"));
    }

    #[test]
    fn test_generate_prd_prompt() {
        let content = "# My App\n\nBuild a todo list app with user authentication.";
        let output_path = ".afk/tasks.json";

        let result = generate_prd_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("Build a todo list app with user authentication"));
        assert!(prompt.contains(".afk/tasks.json"));
        assert!(prompt.contains("# Parse PRD into Structured User Stories"));
    }

    #[test]
    fn test_generate_prd_prompt_with_special_chars() {
        let content = "Features:\n- Use `code blocks`\n- Handle \"quotes\"";
        let output_path = "output/tasks.json";

        let result = generate_prd_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        assert!(prompt.contains("Use `code blocks`"));
        assert!(prompt.contains("Handle \"quotes\""));
    }

    #[test]
    fn test_generate_prd_prompt_empty_content() {
        let content = "";
        let output_path = ".afk/tasks.json";

        let result = generate_prd_prompt(content, output_path);
        assert!(result.is_ok());

        let prompt = result.unwrap();
        // Should still have the template structure
        assert!(prompt.contains("# Parse PRD into Structured User Stories"));
    }

    #[test]
    fn test_load_prd_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("requirements.md");

        let content = "# Requirements\n\n- Feature 1\n- Feature 2";
        fs::write(&file_path, content).unwrap();

        let result = load_prd_file(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_load_prd_file_not_found() {
        let result = load_prd_file(Path::new("/nonexistent/path/file.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prd_file_multiline() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("prd.md");

        let content = r#"# Product Requirements Document

## Overview
This document describes the requirements for the new system.

## Features

### User Management
- Users can register
- Users can log in
- Users can reset password

### Dashboard
- Display summary widgets
- Real-time updates
"#;
        fs::write(&file_path, content).unwrap();

        let result = load_prd_file(&file_path);
        assert!(result.is_ok());

        let loaded = result.unwrap();
        assert!(loaded.contains("Product Requirements Document"));
        assert!(loaded.contains("User Management"));
        assert!(loaded.contains("Dashboard"));
    }

    #[test]
    fn test_prd_parse_error_display() {
        let err = PrdParseError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("Failed to read file"));
    }
}
