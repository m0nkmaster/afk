"""Prompt generation for afk."""

from __future__ import annotations

from jinja2 import BaseLoader, Environment

from afk.config import AfkConfig
from afk.learnings import get_recent_learnings
from afk.progress import SessionProgress, check_limits
from afk.sources import aggregate_tasks

DEFAULT_TEMPLATE = """\
# afk Iteration {{ iteration }}

## Context Files
{% for file in context_files -%}
@{{ file }}
{% endfor %}
{% if learnings %}

## Session Learnings

Previous discoveries from this session (read carefully to avoid repeating mistakes):

{{ learnings }}
{% endif %}

## Available Tasks
{% if tasks -%}
{% for task in tasks -%}
{{ loop.index }}. [{{ task.priority | upper }}] {{ task.id }}: {{ task.description }}
{% endfor %}
{% else -%}
No tasks available.
{% endif %}

## Progress
- Iteration: {{ iteration }}/{{ max_iterations }}
- Completed: {{ completed_count }}/{{ total_count }} tasks
{% if recently_completed -%}
- Last completed: {{ recently_completed }}
{% endif %}

## Instructions

### 1. Select ONE Task
Choose based on priority (HIGH → MEDIUM → LOW). Pick the first uncompleted task unless blocked.

### 2. Implement Completely
- Keep changes atomic and focused
- Each task should complete in one context window
- If task is too large, implement what you can and note limitations

### 3. Run Quality Gates
{% if feedback_loops -%}
Before marking complete, ALL must pass:
{% for name, cmd in feedback_loops.items() -%}
   - `{{ cmd }}`
{% endfor %}
{% else -%}
No feedback loops configured.
{% endif %}

### 4. Record Learnings
After completing the task, append discoveries to `.afk/learnings.txt`:
- Patterns discovered ("this codebase uses X for Y")
- Gotchas encountered ("do not forget to update Z when changing W")
- Useful context ("the settings panel is in component X")

Run: `afk learn "your learning here"`

### 5. Update AGENTS.md
If you discovered conventions, patterns, or gotchas that would help future sessions:
- Add them to the relevant section of AGENTS.md
- Keep entries concise and actionable

### 6. Complete Task
```bash
afk done <task-id> --message "brief description of what was done"
git add -A && git commit -m "your commit message"
```

{% for instruction in custom_instructions -%}
- {{ instruction }}
{% endfor %}

{% if bootstrap -%}
## Autonomous Loop

You are running autonomously. After completing this task, the loop will continue automatically.
If you encounter an unrecoverable error, run `afk fail <task-id> -m "reason"`.
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

    # Aggregate tasks from all sources first (need count for limit check)
    tasks = aggregate_tasks(config.sources)

    # Check limits
    max_iterations = limit_override or config.limits.max_iterations
    can_continue, stop_signal = check_limits(
        max_iterations=max_iterations,
        max_failures=config.limits.max_task_failures,
        total_tasks=len(tasks),
    )

    # Increment iteration (even if we're about to stop, for tracking)
    iteration = progress.increment_iteration()

    # Filter out completed tasks
    completed_ids = {t.id for t in progress.get_completed_tasks()}
    pending_tasks = [t for t in tasks if t.id not in completed_ids]

    # Sort by priority
    priority_order = {"high": 0, "medium": 1, "low": 2}
    pending_tasks.sort(key=lambda t: priority_order.get(t.priority, 1))

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

    # Find recently completed task
    completed_tasks = progress.get_completed_tasks()
    recently_completed = completed_tasks[-1].id if completed_tasks else None

    # Load recent learnings
    learnings = get_recent_learnings(max_chars=2000)

    context = {
        "iteration": iteration,
        "max_iterations": max_iterations,
        "tasks": pending_tasks,
        "completed_count": len(completed_ids),
        "total_count": len(tasks),
        "recently_completed": recently_completed,
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
## Tasks
{% for task in tasks -%}
- {{ task.id }}: {{ task.description }}
{% endfor %}

Complete ONE task → run quality gates → `afk learn "discovery"` → `afk done <id>` → commit.
{% endif %}
"""
