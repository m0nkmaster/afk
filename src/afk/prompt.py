"""Prompt generation for afk."""

from __future__ import annotations

from jinja2 import BaseLoader, Environment

from afk.config import AfkConfig
from afk.progress import SessionProgress, check_limits
from afk.sources import aggregate_tasks

DEFAULT_TEMPLATE = """\
# afk Iteration {{ iteration }}

## Context Files
{% for file in context_files -%}
@{{ file }}
{% endfor %}

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
1. Choose ONE task based on priority:
   - Architectural decisions and core abstractions first
   - Integration points between modules
   - Standard features and implementation
   - Polish and cleanup last

2. Implement the task completely

3. Run feedback loops:
{% for name, cmd in feedback_loops.items() -%}
   - {{ name }}: `{{ cmd }}`
{% endfor %}

4. Mark complete: `afk done <task-id>`

5. Commit your changes with a descriptive message

{% for instruction in custom_instructions -%}
- {{ instruction }}
{% endfor %}

{% if bootstrap -%}
## Loop Mode (Bootstrap)
You are running in autonomous loop mode. After completing this task:
1. Run `afk done <task-id>` to mark it complete
2. Run `afk next --bootstrap` to get the next task
3. Repeat until you see AFK_COMPLETE or AFK_LIMIT_REACHED

If you encounter an error, run `afk fail <task-id>` and move to the next task.
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
## Tasks
{% for task in tasks -%}
- {{ task.id }}: {{ task.description }}
{% endfor %}

Complete ONE task, run `afk done <id>`, commit.
{% endif %}
"""
