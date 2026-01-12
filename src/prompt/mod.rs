//! Prompt generation with templates.
//!
//! This module generates prompts for AI CLI tools using Tera templates.

pub mod template;

use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};

use crate::config::AfkConfig;
use crate::prd::PrdDocument;
use crate::progress::SessionProgress;

// Re-export key types and functions for convenience.
pub use template::{DEFAULT_TEMPLATE, get_template, get_template_with_root};

/// Error type for prompt generation operations.
#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("Failed to load progress: {0}")]
    ProgressError(#[from] crate::progress::ProgressError),
    #[error("Failed to load PRD: {0}")]
    PrdError(#[from] crate::prd::PrdError),
    #[error("Template rendering failed: {0}")]
    TemplateError(#[from] tera::Error),
}

/// A simplified story struct for template rendering.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NextStoryContext {
    pub id: String,
    pub priority: i32,
}

/// Result of prompt generation.
pub struct PromptResult {
    /// The generated prompt string.
    pub prompt: String,
    /// The current iteration number.
    pub iteration: u32,
    /// Whether all stories are complete.
    pub all_complete: bool,
}

/// Generate the prompt for the next iteration.
///
/// This function loads progress and PRD, increments the iteration,
/// builds the template context, and renders the prompt.
///
/// # Arguments
///
/// * `config` - The afk configuration
/// * `bootstrap` - Whether to include the autonomous loop section
/// * `limit_override` - Optional override for max iterations display
///
/// # Returns
///
/// Result containing the generated prompt and metadata, or an error.
pub fn generate_prompt(
    config: &AfkConfig,
    bootstrap: bool,
    limit_override: Option<u32>,
) -> Result<PromptResult, PromptError> {
    generate_prompt_with_root(config, bootstrap, limit_override, None)
}

/// Generate the prompt for the next iteration with an explicit root path.
///
/// This variant is useful for testing where the working directory
/// may not be the project root.
///
/// # Arguments
///
/// * `config` - The afk configuration
/// * `bootstrap` - Whether to include the autonomous loop section
/// * `limit_override` - Optional override for max iterations display
/// * `root` - Optional root path to resolve file paths against
///
/// # Returns
///
/// Result containing the generated prompt and metadata, or an error.
pub fn generate_prompt_with_root(
    config: &AfkConfig,
    bootstrap: bool,
    limit_override: Option<u32>,
    root: Option<&Path>,
) -> Result<PromptResult, PromptError> {
    // Load progress
    let progress_path = root.map(|r| r.join(".afk/progress.json"));
    let mut progress = SessionProgress::load(progress_path.as_deref())?;

    // Load PRD
    let prd_path = root.map(|r| r.join(".afk/prd.json"));
    let prd = PrdDocument::load(prd_path.as_deref())?;

    // Calculate counts
    let pending_stories = prd.get_pending_stories();
    let total_stories = prd.user_stories.len();
    let completed_count = total_stories - pending_stories.len();

    // Max iterations for display (limit enforcement is in loop controller)
    let max_iterations = limit_override.unwrap_or(config.limits.max_iterations);

    // Check if all stories are complete
    let all_complete = prd.all_stories_complete();
    let stop_signal: Option<String> = if all_complete {
        Some("AFK_COMPLETE - All stories have passes: true".to_string())
    } else {
        None
    };

    // Increment iteration for tracking
    let iteration = progress.increment_iteration();

    // Save the updated progress
    let progress_save_path = root.map(|r| r.join(".afk/progress.json"));
    progress.save(progress_save_path.as_deref())?;

    // Build feedback loops dict (filter out None values)
    let mut feedback_loops: HashMap<String, String> = HashMap::new();
    if let Some(ref types_cmd) = config.feedback_loops.types {
        feedback_loops.insert("types".to_string(), types_cmd.clone());
    }
    if let Some(ref lint_cmd) = config.feedback_loops.lint {
        feedback_loops.insert("lint".to_string(), lint_cmd.clone());
    }
    if let Some(ref test_cmd) = config.feedback_loops.test {
        feedback_loops.insert("test".to_string(), test_cmd.clone());
    }
    if let Some(ref build_cmd) = config.feedback_loops.build {
        feedback_loops.insert("build".to_string(), build_cmd.clone());
    }
    // Add custom commands
    for (name, cmd) in &config.feedback_loops.custom {
        feedback_loops.insert(name.clone(), cmd.clone());
    }

    // Get template
    let template_str = get_template_with_root(config, root);

    // Set up Tera and render
    let mut tera = Tera::default();
    tera.add_raw_template("prompt", &template_str)?;

    // Get next story for context
    let next_story: Option<NextStoryContext> = pending_stories.first().map(|s| NextStoryContext {
        id: s.id.clone(),
        priority: s.priority,
    });

    // Build context
    let mut context = Context::new();
    context.insert("iteration", &iteration);
    context.insert("max_iterations", &max_iterations);
    context.insert("completed_count", &completed_count);
    context.insert("total_count", &total_stories);
    context.insert("next_story", &next_story);
    context.insert("context_files", &config.prompt.context_files);
    context.insert("feedback_loops", &feedback_loops);
    context.insert("custom_instructions", &config.prompt.instructions);
    context.insert("bootstrap", &bootstrap);
    context.insert("stop_signal", &stop_signal);

    let prompt = tera.render("prompt", &context)?;

    Ok(PromptResult {
        prompt,
        iteration,
        all_complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FeedbackLoopsConfig, LimitsConfig, PromptConfig};
    use crate::prd::UserStory;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env(temp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let progress_path = afk_dir.join("progress.json");
        let prd_path = afk_dir.join("prd.json");

        (progress_path, prd_path)
    }

    #[test]
    fn test_generate_prompt_basic() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        // Create initial progress
        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // Create PRD with stories
        let prd = PrdDocument {
            project: "test-project".to_string(),
            branch_name: "main".to_string(),
            user_stories: vec![
                UserStory {
                    id: "story-1".to_string(),
                    title: "First Story".to_string(),
                    priority: 1,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "story-2".to_string(),
                    title: "Second Story".to_string(),
                    priority: 2,
                    passes: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert_eq!(result.iteration, 1);
        assert!(!result.all_complete);
        assert!(result.prompt.contains("# afk Autonomous Agent"));
        assert!(result.prompt.contains("Iteration: 1/200"));
        assert!(result.prompt.contains("Completed: 1/2 stories"));
        assert!(result.prompt.contains("Next story: story-1 (priority 1)"));
    }

    #[test]
    fn test_generate_prompt_increments_iteration() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        // Create initial progress with 5 iterations
        let mut progress = SessionProgress::new();
        progress.iterations = 5;
        progress.save(Some(&progress_path)).unwrap();

        // Create PRD
        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        // Should increment from 5 to 6
        assert_eq!(result.iteration, 6);
        assert!(result.prompt.contains("Iteration: 6/200"));

        // Verify progress was saved
        let loaded_progress = SessionProgress::load(Some(&progress_path)).unwrap();
        assert_eq!(loaded_progress.iterations, 6);
    }

    #[test]
    fn test_generate_prompt_with_limit_override() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result =
            generate_prompt_with_root(&config, false, Some(50), Some(temp.path())).unwrap();

        assert!(result.prompt.contains("Iteration: 1/50"));
    }

    #[test]
    fn test_generate_prompt_with_bootstrap() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, true, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("## Autonomous Loop"));
        assert!(result.prompt.contains("You are running autonomously"));
    }

    #[test]
    fn test_generate_prompt_without_bootstrap() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(!result.prompt.contains("## Autonomous Loop"));
    }

    #[test]
    fn test_generate_prompt_all_complete() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // All stories complete
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "story-1".to_string(),
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "story-2".to_string(),
                    passes: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.all_complete);
        assert!(result.prompt.contains("## STOP"));
        assert!(
            result
                .prompt
                .contains("AFK_COMPLETE - All stories have passes: true")
        );
    }

    #[test]
    fn test_generate_prompt_empty_prd() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // Empty PRD (no stories)
        let prd = PrdDocument::default();
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        // Empty PRD is considered complete
        assert!(result.all_complete);
        assert!(result.prompt.contains("Completed: 0/0 stories"));
    }

    #[test]
    fn test_generate_prompt_with_feedback_loops() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig {
            feedback_loops: FeedbackLoopsConfig {
                lint: Some("cargo clippy".to_string()),
                test: Some("cargo test".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("Configured gates:"));
        assert!(result.prompt.contains("lint: `cargo clippy`"));
        assert!(result.prompt.contains("test: `cargo test`"));
    }

    #[test]
    fn test_generate_prompt_no_feedback_loops() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("No gates configured"));
    }

    #[test]
    fn test_generate_prompt_with_custom_feedback_loops() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let mut custom = HashMap::new();
        custom.insert("format".to_string(), "cargo fmt --check".to_string());

        let config = AfkConfig {
            feedback_loops: FeedbackLoopsConfig {
                lint: Some("cargo clippy".to_string()),
                custom,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("lint: `cargo clippy`"));
        assert!(result.prompt.contains("format: `cargo fmt --check`"));
    }

    #[test]
    fn test_generate_prompt_with_context_files() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig {
            prompt: PromptConfig {
                context_files: vec!["AGENTS.md".to_string(), "README.md".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("- `AGENTS.md`"));
        assert!(result.prompt.contains("- `README.md`"));
    }

    #[test]
    fn test_generate_prompt_with_custom_instructions() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig {
            prompt: PromptConfig {
                instructions: vec![
                    "Use British English".to_string(),
                    "Always run tests".to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("- Use British English"));
        assert!(result.prompt.contains("- Always run tests"));
    }

    #[test]
    fn test_generate_prompt_with_custom_limits() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig {
            limits: LimitsConfig {
                max_iterations: 100,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("Iteration: 1/100"));
    }

    #[test]
    fn test_generate_prompt_with_custom_template() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        // Create custom template
        let template_path = temp.path().join(".afk/custom-prompt.txt");
        fs::write(
            &template_path,
            "Custom Template\nIteration: {{ iteration }}/{{ max_iterations }}",
        )
        .unwrap();

        let config = AfkConfig {
            prompt: PromptConfig {
                custom_path: Some(".afk/custom-prompt.txt".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(result.prompt.contains("Custom Template"));
        assert!(result.prompt.contains("Iteration: 1/200"));
    }

    #[test]
    fn test_generate_prompt_missing_prd() {
        let temp = TempDir::new().unwrap();
        let (progress_path, _prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // Don't create PRD file - it should load as empty

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        // Should work with empty/missing PRD
        assert!(result.prompt.contains("Completed: 0/0 stories"));
        assert!(result.all_complete); // Empty PRD is considered complete
    }

    #[test]
    fn test_generate_prompt_missing_progress() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let prd_path = afk_dir.join("prd.json");

        // Don't create progress file - it should create a new one

        let prd = PrdDocument {
            user_stories: vec![UserStory::new("story-1", "Test Story")],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        // Should start at iteration 1
        assert_eq!(result.iteration, 1);
    }

    #[test]
    fn test_generate_prompt_stories_sorted_by_priority() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // Create stories out of priority order
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "low-priority".to_string(),
                    priority: 3,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "high-priority".to_string(),
                    priority: 1,
                    passes: false,
                    ..Default::default()
                },
                UserStory {
                    id: "medium-priority".to_string(),
                    priority: 2,
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        // Next story should be the highest priority (priority 1)
        assert!(
            result
                .prompt
                .contains("Next story: high-priority (priority 1)")
        );
    }

    #[test]
    fn test_generate_prompt_only_pending_stories_in_next() {
        let temp = TempDir::new().unwrap();
        let (progress_path, prd_path) = setup_test_env(&temp);

        let progress = SessionProgress::new();
        progress.save(Some(&progress_path)).unwrap();

        // High-priority story is complete, next should be medium
        let prd = PrdDocument {
            user_stories: vec![
                UserStory {
                    id: "completed-high".to_string(),
                    priority: 1,
                    passes: true,
                    ..Default::default()
                },
                UserStory {
                    id: "pending-medium".to_string(),
                    priority: 2,
                    passes: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        prd.save(Some(&prd_path)).unwrap();

        let config = AfkConfig::default();
        let result = generate_prompt_with_root(&config, false, None, Some(temp.path())).unwrap();

        assert!(
            result
                .prompt
                .contains("Next story: pending-medium (priority 2)")
        );
        assert!(result.prompt.contains("Completed: 1/2 stories"));
    }

    #[test]
    fn test_next_story_context_fields() {
        let next_story = NextStoryContext {
            id: "test-123".to_string(),
            priority: 2,
        };

        // Verify it can be serialised (needed for template)
        let json = serde_json::to_string(&next_story).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("2"));
    }
}
