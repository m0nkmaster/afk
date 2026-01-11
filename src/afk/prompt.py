"""Prompt generation for afk."""

from __future__ import annotations

from jinja2 import BaseLoader, Environment

from afk.config import AfkConfig
from afk.prd_store import all_stories_complete, get_pending_stories, load_prd
from afk.progress import SessionProgress

DEFAULT_TEMPLATE = """\
# afk Autonomous Agent

You are an autonomous coding agent working on a software project.

## Your Task

1. Read `.afk/progress.json` for session state and prior learnings
2. Read `.afk/prd.json` for the task list
3. Check you're on the correct branch from PRD `branchName`. If not, check it out or create.
4. Pick the **highest priority** user story where `passes: false`
5. Implement that single user story according to its `acceptanceCriteria`
6. Run `afk verify` to check quality gates (see below)
7. If verify fails, fix the issues and run `afk verify` again until it passes
8. Once verify passes, commit ALL changes with message: `feat: [Story ID] - [Story Title]`
9. Update `.afk/prd.json` to set `passes: true` for the completed story
10. Record learnings (see below)

## Key Files

- `.afk/progress.json` - Session state with per-task learnings (short-term memory)
- `.afk/prd.json` - Task list with priorities and acceptance criteria
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
{% for name, cmd in feedback_loops.items() -%}
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
      "learnings": [
        "OAuth tokens stored in secure cookies, not localStorage",
        "Must call refreshToken before API requests if >30min old"
      ]
    }
  }
}
```

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

After completing a user story, check if ALL stories have `passes: true` in `.afk/prd.json`.

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
"""


def generate_prompt(
    config: AfkConfig,
    bootstrap: bool = False,
    limit_override: int | None = None,
) -> str:
    """Generate the prompt for the next iteration.

    Note: Limit checking is handled by the loop controller, not here.
    This function only checks for story completion to include the stop signal.
    """
    # Load progress
    progress = SessionProgress.load()

    # Load PRD (Ralph pattern - AI reads this file directly)
    prd = load_prd()
    pending_stories = get_pending_stories(prd)
    total_stories = len(prd.userStories)
    completed_count = total_stories - len(pending_stories)

    # Max iterations for display only (limit enforcement is in loop controller)
    max_iterations = limit_override or config.limits.max_iterations

    # Check if all stories are complete (this is the only stop condition here)
    stop_signal = None
    if all_stories_complete(prd):
        stop_signal = "AFK_COMPLETE - All stories have passes: true"

    # Increment iteration for tracking
    iteration = progress.increment_iteration()

    # Build feedback loops dict (filter out None values)
    feedback_loops = {}
    if config.feedback_loops.types:
        feedback_loops["types"] = config.feedback_loops.types
    if config.feedback_loops.lint:
        feedback_loops["lint"] = config.feedback_loops.lint
    if config.feedback_loops.test:
        feedback_loops["test"] = config.feedback_loops.test
    if config.feedback_loops.build:
        feedback_loops["build"] = config.feedback_loops.build
    feedback_loops.update(config.feedback_loops.custom)

    # Get template
    template_str = _get_template(config)

    # Render
    env = Environment(loader=BaseLoader())
    template = env.from_string(template_str)

    # Get next story for context
    next_story = pending_stories[0] if pending_stories else None

    context = {
        "iteration": iteration,
        "max_iterations": max_iterations,
        "completed_count": completed_count,
        "total_count": total_stories,
        "next_story": next_story,
        "context_files": config.prompt.context_files,
        "feedback_loops": feedback_loops,
        "custom_instructions": config.prompt.instructions,
        "bootstrap": bootstrap,
        "stop_signal": stop_signal,
    }

    return template.render(**context)


def _get_template(config: AfkConfig) -> str:
    """Get the template string based on config."""
    if config.prompt.custom_path:
        from pathlib import Path

        custom_path = Path(config.prompt.custom_path)
        if custom_path.exists():
            return custom_path.read_text()

    # Built-in templates
    if config.prompt.template == "minimal":
        return _MINIMAL_TEMPLATE
    elif config.prompt.template == "verbose":
        return DEFAULT_TEMPLATE  # verbose is the default for now

    return DEFAULT_TEMPLATE


_MINIMAL_TEMPLATE = """\
# afk {{ iteration }}/{{ max_iterations }}

{% if stop_signal -%}
{{ stop_signal }}
{% else -%}
Read `.afk/progress.json` → `.afk/prd.json` → implement highest priority `passes: false` story →
run `afk verify` until it passes → set `passes: true` → commit → record learnings.

If all stories pass, reply with: <promise>COMPLETE</promise>
{% endif %}
"""
