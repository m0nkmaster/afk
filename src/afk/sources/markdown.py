"""Markdown task source adapter."""

from __future__ import annotations

import re
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from afk.sources import Task


def load_markdown_tasks(path: str | None) -> list[Task]:
    """Load tasks from a markdown file with checkboxes.

    Supports formats:
    - [ ] Task description
    - [x] Completed task (skipped)
    - [ ] [HIGH] Task with priority
    - [ ] task-id: Task with explicit ID
    """
    from afk.sources import Task

    if not path:
        # Try default locations
        for default_path in ["tasks.md", "TODO.md", "prd.md", ".afk/tasks.md"]:
            if Path(default_path).exists():
                path = default_path
                break
        else:
            return []

    file_path = Path(path)
    if not file_path.exists():
        return []

    content = file_path.read_text()
    tasks = []

    # Match markdown checkboxes
    # - [ ] unchecked, - [x] checked
    pattern = r"^[\s]*[-*]\s*\[([ xX])\]\s*(.+)$"

    for match in re.finditer(pattern, content, re.MULTILINE):
        checked = match.group(1).lower() == "x"
        text = match.group(2).strip()

        if checked:
            # Skip completed tasks
            continue

        task_id, description, priority = _parse_task_line(text)

        tasks.append(
            Task(
                id=task_id,
                description=description,
                priority=priority,
                source=f"markdown:{path}",
            )
        )

    return tasks


def _parse_task_line(text: str) -> tuple[str, str, str]:
    """Parse a task line to extract ID, description, and priority.

    Returns (id, description, priority)
    """
    priority = "medium"
    description = text

    # Check for priority tag: [HIGH], [LOW], etc.
    priority_match = re.match(r"^\[([A-Z]+)\]\s*(.+)$", text)
    if priority_match:
        tag = priority_match.group(1).upper()
        description = priority_match.group(2).strip()

        if tag in ("HIGH", "CRITICAL", "URGENT", "P0", "P1"):
            priority = "high"
        elif tag in ("LOW", "MINOR", "P3", "P4"):
            priority = "low"
        else:
            priority = "medium"

    # Check for explicit ID: "task-id: description"
    id_match = re.match(r"^([a-z0-9_-]+):\s*(.+)$", description, re.IGNORECASE)
    if id_match:
        task_id = id_match.group(1).lower()
        description = id_match.group(2).strip()
    else:
        # Generate ID from description
        task_id = _generate_id(description)

    return task_id, description, priority


def _generate_id(description: str) -> str:
    """Generate an ID from description."""
    # Take first 30 chars, lowercase, replace spaces with dashes
    clean = description[:30].lower()
    clean = "".join(c if c.isalnum() or c == " " else "" for c in clean)
    return clean.replace(" ", "-").strip("-") or "task"
