"""Prompt generation for afk."""

from __future__ import annotations

from jinja2 import BaseLoader, Environment

from afk.config import AfkConfig
from afk.learnings import get_recent_learnings
from afk.prd_store import all_stories_complete, get_pending_stories, load_prd
from afk.progress import SessionProgress, check_limits

DEFAULT_TEMPLATE = """\
# afk Autonomous Agent

You are an autonomous coding agent working on a software project.

## Your Task

1. Read the PRD at `.afk/prd.json`
2. Check you're on the correct branch from PRD `branchName`. If not, check it out or create from main.
3. Pick the **highest priority** user story where `passes: false`
4. Implement that single user story according to its `acceptanceCriteria`
5. Run quality checks (see below)
6. If checks pass, commit ALL changes with message: `feat: [Story ID] - [Story Title]`
7. Update `.afk/prd.json` to set `passes: true` for the completed story
8. Record learnings (see below)

## Context Files
{% for file in context_files -%}
@{{ file }}
{% endfor %}
{% if learnings %}

## Session Learnings

Previous discoveries from this session (read carefully to avoid repeating mistakes):

{{ learnings }}
{% endif %}

## Progress
- Iteration: {{ iteration }}/{{ max_iterations }}
- Completed: {{ completed_count }}/{{ total_count }} stories
{% if next_story -%}
- Next story: {{ next_story.id }} (priority {{ next_story.priority }})
{% endif %}

## Quality Gates
{% if feedback_loops -%}
Before marking complete, ALL must pass:
{% for name, cmd in feedback_loops.items() -%}
- `{{ cmd }}`
{% endfor %}
{% else -%}
Run whatever quality checks your project requires (typecheck, lint, test).
{% endif %}

## Record Learnings

After completing a story, record useful discoveries:

1. **Append to `.afk/learnings.txt`**:
   - Patterns discovered ("this codebase uses X for Y")
   - Gotchas encountered ("don't forget to update Z when changing W")
   - Useful context ("the settings panel is in component X")

2. **Update AGENTS.md** if you discovered conventions, patterns, or gotchas that would help future sessions.

{% for instruction in custom_instructions -%}
- {{ instruction }}
{% endfor %}

## Stop Condition

After completing a user story, check if ALL stories have `passes: true` in `.afk/prd.json`.

If ALL stories are complete and passing, reply with:
<promise>COMPLETE</promise>

If there are still stories with `passes: false`, end your response normally (another iteration will pick up the next story).

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
    """Generate the prompt for the next iteration."""
    # Load progress
    progress = SessionProgress.load()

    # Load PRD (Ralph pattern - AI reads this file directly)
    prd = load_prd()
    pending_stories = get_pending_stories(prd)
    total_stories = len(prd.userStories)
    completed_count = total_stories - len(pending_stories)

    # Check limits
    max_iterations = limit_override or config.limits.max_iterations
    can_continue, stop_signal = check_limits(
        max_iterations=max_iterations,
        max_failures=config.limits.max_task_failures,
        total_tasks=total_stories,
    )

    # Check if all stories are complete
    if all_stories_complete(prd):
        stop_signal = "AFK_COMPLETE - All stories have passes: true"
        can_continue = False

    # Increment iteration (even if we're about to stop, for tracking)
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

    # Load recent learnings
    learnings = get_recent_learnings(max_chars=2000)

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
        "stop_signal": stop_signal if not can_continue else None,
        "learnings": learnings,
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
{% if learnings -%}
## Learnings
{{ learnings }}

{% endif -%}
Read `.afk/prd.json` → pick highest priority story where `passes: false` → implement according to `acceptanceCriteria` → run quality gates → set `passes: true` → commit.

If all stories pass, reply with: <promise>COMPLETE</promise>
{% endif %}
"""
