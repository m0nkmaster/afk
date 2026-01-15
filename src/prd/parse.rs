//! PRD parsing prompt generation.
//!
//! This module provides functionality to convert raw PRD documents
//! into structured JSON feature lists using AI prompts.

use std::fs;
use std::path::Path;

use tera::{Context, Tera};

/// PRD parse template for converting requirements documents to JSON.
///
/// Uses Tera template syntax.
pub const PRD_PARSE_TEMPLATE: &str = r#"# Parse PRD into Structured Feature List

You are an AI assistant tasked with converting a product requirements document
into a structured JSON feature list.

## Input PRD

```
{{ prd_content }}
```

## Output Format

Create a JSON file at `{{ output_path }}` with the following structure:

```json
{
  "tasks": [
    {
      "id": "kebab-case-feature-id",
      "category": "functional|non-functional|technical|ux|security",
      "description": "Clear, actionable description of the feature",
      "priority": 1,
      "steps": [
        "Step 1: Navigate to...",
        "Step 2: Perform action...",
        "Step 3: Verify result..."
      ],
      "passes": false
    }
  ]
}
```

## Field Definitions

- **id**: Unique kebab-case identifier (e.g., `user-auth-login`, `api-rate-limiting`)
- **category**: One of:
  - `functional` - Core user-facing features
  - `non-functional` - Performance, scalability, reliability
  - `technical` - Infrastructure, architecture, tooling
  - `ux` - User experience, design, accessibility
  - `security` - Authentication, authorisation, data protection
- **description**: Single sentence describing what the feature does (not how)
- **priority**: Integer 1-5 (1 = highest priority, implement first)
- **steps**: Array of verification steps to confirm the feature works end-to-end
- **passes**: Always `false` initially (will be marked `true` when verified)

## Guidelines

1. **Be comprehensive**: Extract ALL features, requirements, and acceptance criteria
2. **Be atomic**: Each task should be a single, implementable unit of work
3. **Be testable**: Every task must have clear verification steps
4. **Prioritise wisely**:
   - Priority 1: Core architecture, dependencies, blocking features
   - Priority 2: Essential user-facing functionality
   - Priority 3: Standard features and integrations
   - Priority 4: Nice-to-have features, polish
   - Priority 5: Future considerations, stretch goals
5. **Order by dependency**: If feature B requires feature A, A should have higher priority
6. **Include edge cases**: Error handling, validation, and edge cases as separate tasks

## Task Size (CRITICAL)

Each task MUST complete in a single AI context window. Tasks that are too large cause:
- Context overflow mid-implementation
- Incomplete features
- Poor code quality

**Right-sized tasks (single session):**
- Add a database column and migration
- Add a UI component to an existing page
- Update a server action with new logic
- Add a filter dropdown to a list
- Add a new API endpoint
- Write tests for a module

**Too large (SPLIT THESE):**
- "Build the entire dashboard" → Split into: layout, navigation, each widget
- "Add authentication" → Split into: login form, session handling, protected routes
- "Refactor the API" → Split into: each endpoint or module separately

When in doubt, split into smaller tasks. 5 small tasks are better than 1 large task.

## Output

Write the complete JSON to `{{ output_path }}` and confirm the number of tasks extracted.
"#;

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
        assert!(PRD_PARSE_TEMPLATE.contains("# Parse PRD into Structured Feature List"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Input PRD"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Output Format"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Field Definitions"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Guidelines"));
        assert!(PRD_PARSE_TEMPLATE.contains("## Task Size (CRITICAL)"));
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
        assert!(prompt.contains("# Parse PRD into Structured Feature List"));
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
        assert!(prompt.contains("# Parse PRD into Structured Feature List"));
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
