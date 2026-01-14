//! Prompt templates for afk.
//!
//! This module defines the default prompt template and provides
//! functionality to load custom templates.

use crate::config::AfkConfig;
use std::fs;
use std::path::Path;

/// Default prompt template, mirroring the Python version exactly.
///
/// Uses Tera template syntax (similar to Jinja2).
pub const DEFAULT_TEMPLATE: &str = r#"# afk Autonomous Agent

You are an autonomous coding agent working on a software project.

## Your Task

1. Read `.afk/progress.json` for session state and prior learnings
2. Read `.afk/tasks.json` for the task list
3. Check you're on the correct branch from PRD `branchName`. If not, check it out or create.
4. Pick the **highest priority** user story where `passes: false`
5. Implement that single user story according to its `acceptanceCriteria`
6. Run `afk verify` to check quality gates (see below)
7. If verify fails, fix the issues and run `afk verify` again until it passes
8. Once verify passes, commit ALL changes with message: `feat: [Story ID] - [Story Title]`
9. Update `.afk/tasks.json` to set `passes: true` for the completed story
10. Record learnings (see below)

## Key Files

- `.afk/progress.json` - Session state with per-task learnings (short-term memory)
- `.afk/tasks.json` - Task list with priorities and acceptance criteria
- `AGENTS.md` - Project-wide conventions and patterns (long-term memory)
{% for file in context_files -%}
- `{{ file }}`
{% endfor %}

## Progress
- Iteration: {{ iteration }}/{{ max_iterations }}
- Completed: {{ completed_count }}/{{ total_count }} stories
{% if next_story -%}
- Next story: {{ next_story.id }} (priority {{ next_story.priority }})
{% endif %}

## Quality Gates

**IMPORTANT**: Run `afk verify` before marking any story complete.
Do NOT set `passes: true` until verify passes.

```bash
afk verify           # Run all quality gates
afk verify --verbose # Show failure details
```

{% if feedback_loops -%}
Configured gates:
{% for name, cmd in feedback_loops -%}
- {{ name }}: `{{ cmd }}`
{% endfor %}
{% else -%}
No gates configured. Run whatever quality checks your project requires (typecheck, lint, test).
{% endif %}

## Recording Learnings

As you work, record discoveries appropriately:

### Short-term: `.afk/progress.json`

Add task-specific learnings to the `learnings` array for that task's entry:
- Gotchas specific to this task
- Context needed for related work
- Why certain approaches didn't work

Example structure:
```json
{
  "tasks": {
    "auth-login": {
      "id": "auth-login",
      "source": "prd",
      "status": "in_progress",
      "learnings": [
        "OAuth tokens stored in secure cookies, not localStorage",
        "Must call refreshToken before API requests if >30min old"
      ]
    }
  }
}
```

Task fields:
- `id`: Task identifier (must match the key)
- `source`: Where the task came from (e.g. "prd", "beads", "github")
- `status`: One of `pending`, `in_progress`, `completed`, `failed`, `skipped`
- `learnings`: Array of strings with discoveries from this task

### Long-term: `AGENTS.md`

Update `AGENTS.md` for discoveries that benefit future sessions:
- Project conventions and patterns
- Architectural decisions
- Gotchas that affect the whole codebase

If working deep in a subfolder with its own concerns, create a local `AGENTS.md` there instead.

{% for instruction in custom_instructions -%}
- {{ instruction }}
{% endfor %}

## Stop Condition

After completing a user story, check if ALL stories have `passes: true` in `.afk/tasks.json`.

If ALL stories are complete and passing, reply with:
<promise>COMPLETE</promise>

If there are still stories with `passes: false`, end your response normally.

{% if bootstrap -%}
## Autonomous Loop

You are running autonomously. Work on ONE story per iteration.
After completing this task, the loop will continue automatically.
{% endif %}

{% if stop_signal -%}
## STOP
{{ stop_signal }}
{% endif %}
"#;

/// Get the template string based on config.
///
/// If a custom_path is specified in the config and the file exists,
/// its contents are returned. Otherwise, the default template is used.
///
/// # Arguments
///
/// * `config` - The afk configuration
///
/// # Returns
///
/// The template string to use for prompt generation.
pub fn get_template(config: &AfkConfig) -> String {
    get_template_with_root(config, None)
}

/// Get the template string based on config, with an explicit root path.
///
/// This variant is useful for testing where the working directory
/// may not be the project root.
///
/// # Arguments
///
/// * `config` - The afk configuration
/// * `root` - Optional root path to resolve custom_path against
///
/// # Returns
///
/// The template string to use for prompt generation.
pub fn get_template_with_root(config: &AfkConfig, root: Option<&Path>) -> String {
    if let Some(custom_path) = &config.prompt.custom_path {
        let path = if let Some(root) = root {
            root.join(custom_path)
        } else {
            Path::new(custom_path).to_path_buf()
        };

        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                return contents;
            }
        }
    }

    DEFAULT_TEMPLATE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PromptConfig;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tera::{Context, Tera};

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_default_template_not_empty() {
        assert!(!DEFAULT_TEMPLATE.is_empty());
    }

    #[test]
    fn test_default_template_has_key_sections() {
        assert!(DEFAULT_TEMPLATE.contains("# afk Autonomous Agent"));
        assert!(DEFAULT_TEMPLATE.contains("## Your Task"));
        assert!(DEFAULT_TEMPLATE.contains("## Key Files"));
        assert!(DEFAULT_TEMPLATE.contains("## Progress"));
        assert!(DEFAULT_TEMPLATE.contains("## Quality Gates"));
        assert!(DEFAULT_TEMPLATE.contains("## Recording Learnings"));
        assert!(DEFAULT_TEMPLATE.contains("## Stop Condition"));
        assert!(DEFAULT_TEMPLATE.contains("<promise>COMPLETE</promise>"));
    }

    #[test]
    fn test_default_template_has_template_variables() {
        assert!(DEFAULT_TEMPLATE.contains("{{ iteration }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ max_iterations }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ completed_count }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ total_count }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ next_story.id }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ next_story.priority }}"));
        assert!(DEFAULT_TEMPLATE.contains("{{ stop_signal }}"));
    }

    #[test]
    fn test_default_template_has_loops() {
        assert!(DEFAULT_TEMPLATE.contains("{% for file in context_files -%}"));
        assert!(DEFAULT_TEMPLATE.contains("{% for name, cmd in feedback_loops -%}"));
        assert!(DEFAULT_TEMPLATE.contains("{% for instruction in custom_instructions -%}"));
    }

    #[test]
    fn test_default_template_has_conditionals() {
        assert!(DEFAULT_TEMPLATE.contains("{% if next_story -%}"));
        assert!(DEFAULT_TEMPLATE.contains("{% if feedback_loops -%}"));
        assert!(DEFAULT_TEMPLATE.contains("{% if bootstrap -%}"));
        assert!(DEFAULT_TEMPLATE.contains("{% if stop_signal -%}"));
    }

    #[test]
    fn test_get_template_returns_default_when_no_custom_path() {
        let config = AfkConfig::default();
        let template = get_template(&config);
        assert_eq!(template, DEFAULT_TEMPLATE);
    }

    #[test]
    fn test_get_template_returns_default_when_custom_path_not_found() {
        let config = AfkConfig {
            prompt: PromptConfig {
                custom_path: Some("nonexistent/path/template.txt".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let template = get_template(&config);
        assert_eq!(template, DEFAULT_TEMPLATE);
    }

    #[test]
    fn test_get_template_loads_custom_path() {
        let temp = TempDir::new().unwrap();
        let custom_template = "Custom template content here";
        let template_path = temp.path().join("custom.jinja2");
        fs::write(&template_path, custom_template).unwrap();

        let config = AfkConfig {
            prompt: PromptConfig {
                custom_path: Some(template_path.to_string_lossy().to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let template = get_template(&config);
        assert_eq!(template, custom_template);
    }

    #[test]
    fn test_get_template_with_root() {
        let temp = TempDir::new().unwrap();
        let custom_template = "Template with root";
        let template_path = temp.path().join(".afk/prompt.jinja2");
        fs::create_dir_all(template_path.parent().unwrap()).unwrap();
        fs::write(&template_path, custom_template).unwrap();

        let config = AfkConfig {
            prompt: PromptConfig {
                custom_path: Some(".afk/prompt.jinja2".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let template = get_template_with_root(&config, Some(temp.path()));
        assert_eq!(template, custom_template);
    }

    #[test]
    fn test_template_renders_with_tera_minimal() {
        let mut tera = Tera::default();
        tera.add_raw_template("prompt", DEFAULT_TEMPLATE).unwrap();

        let mut context = Context::new();
        context.insert("iteration", &1);
        context.insert("max_iterations", &10);
        context.insert("completed_count", &0);
        context.insert("total_count", &5);
        context.insert("context_files", &Vec::<String>::new());
        context.insert("feedback_loops", &HashMap::<String, String>::new());
        context.insert("custom_instructions", &Vec::<String>::new());
        context.insert("bootstrap", &false);
        context.insert("next_story", &None::<()>);
        context.insert("stop_signal", &None::<String>);

        let result = tera.render("prompt", &context);
        assert!(
            result.is_ok(),
            "Template failed to render: {:?}",
            result.err()
        );

        let rendered = result.unwrap();
        assert!(rendered.contains("# afk Autonomous Agent"));
        assert!(rendered.contains("Iteration: 1/10"));
        assert!(rendered.contains("Completed: 0/5 stories"));
    }

    #[test]
    fn test_template_renders_with_tera_full_context() {
        let mut tera = Tera::default();
        tera.add_raw_template("prompt", DEFAULT_TEMPLATE).unwrap();

        #[derive(serde::Serialize)]
        struct NextStory {
            id: String,
            priority: u32,
        }

        let next_story = NextStory {
            id: "story-001".to_string(),
            priority: 1,
        };

        let mut feedback_loops = HashMap::new();
        feedback_loops.insert("lint".to_string(), "cargo clippy".to_string());
        feedback_loops.insert("test".to_string(), "cargo test".to_string());

        let mut context = Context::new();
        context.insert("iteration", &5);
        context.insert("max_iterations", &20);
        context.insert("completed_count", &3);
        context.insert("total_count", &10);
        context.insert("context_files", &vec!["AGENTS.md", "README.md"]);
        context.insert("feedback_loops", &feedback_loops);
        context.insert(
            "custom_instructions",
            &vec!["Use British English", "Always run tests"],
        );
        context.insert("bootstrap", &true);
        context.insert("next_story", &next_story);
        context.insert("stop_signal", &None::<String>);

        let result = tera.render("prompt", &context);
        assert!(
            result.is_ok(),
            "Template failed to render: {:?}",
            result.err()
        );

        let rendered = result.unwrap();
        assert!(rendered.contains("Iteration: 5/20"));
        assert!(rendered.contains("Completed: 3/10 stories"));
        assert!(rendered.contains("Next story: story-001 (priority 1)"));
        assert!(rendered.contains("- `AGENTS.md`"));
        assert!(rendered.contains("- `README.md`"));
        assert!(rendered.contains("Configured gates:"));
        assert!(rendered.contains("- Use British English"));
        assert!(rendered.contains("- Always run tests"));
        assert!(rendered.contains("## Autonomous Loop"));
    }

    #[test]
    fn test_template_renders_with_stop_signal() {
        let mut tera = Tera::default();
        tera.add_raw_template("prompt", DEFAULT_TEMPLATE).unwrap();

        let mut context = Context::new();
        context.insert("iteration", &10);
        context.insert("max_iterations", &10);
        context.insert("completed_count", &5);
        context.insert("total_count", &5);
        context.insert("context_files", &Vec::<String>::new());
        context.insert("feedback_loops", &HashMap::<String, String>::new());
        context.insert("custom_instructions", &Vec::<String>::new());
        context.insert("bootstrap", &false);
        context.insert("next_story", &None::<()>);
        context.insert(
            "stop_signal",
            &Some("AFK_COMPLETE - All stories have passes: true"),
        );

        let result = tera.render("prompt", &context);
        assert!(
            result.is_ok(),
            "Template failed to render: {:?}",
            result.err()
        );

        let rendered = result.unwrap();
        assert!(rendered.contains("## STOP"));
        assert!(rendered.contains("AFK_COMPLETE - All stories have passes: true"));
    }

    #[test]
    fn test_template_renders_no_gates_message() {
        let mut tera = Tera::default();
        tera.add_raw_template("prompt", DEFAULT_TEMPLATE).unwrap();

        let mut context = Context::new();
        context.insert("iteration", &1);
        context.insert("max_iterations", &10);
        context.insert("completed_count", &0);
        context.insert("total_count", &5);
        context.insert("context_files", &Vec::<String>::new());
        // Pass empty HashMap for feedback_loops - should trigger "No gates configured" message
        let empty_loops: HashMap<String, String> = HashMap::new();
        context.insert("feedback_loops", &empty_loops);
        context.insert("custom_instructions", &Vec::<String>::new());
        context.insert("bootstrap", &false);
        context.insert("next_story", &None::<()>);
        context.insert("stop_signal", &None::<String>);

        let result = tera.render("prompt", &context);
        assert!(
            result.is_ok(),
            "Template failed to render: {:?}",
            result.err()
        );

        let rendered = result.unwrap();
        assert!(rendered.contains("No gates configured"));
    }
}
