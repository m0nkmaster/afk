"""Markdown task source adapter."""

from __future__ import annotations

import re
from pathlib import Path

from afk.prd_store import UserStory


def load_markdown_tasks(path: str | None) -> list[UserStory]:
    """Load tasks from a markdown file with checkboxes.

    Supports formats:
    - [ ] Task description
    - [x] Completed task (skipped)
    - [ ] [HIGH] Task with priority
    - [ ] task-id: Task with explicit ID
    """
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
    pattern = r"^[\s]*[-*]\s*\[([ xX])\]\s*(.+)$"

    for match in re.finditer(pattern, content, re.MULTILINE):
        checked = match.group(1).lower() == "x"
        text = match.group(2).strip()

        if checked:
            # Skip completed tasks
            continue

        task_id, title, priority = _parse_task_line(text)

        tasks.append(
            UserStory(
                id=task_id,
                title=title,
                description=title,
                acceptance_criteria=[f"Complete: {title}"],
                priority=priority,
                source=f"markdown:{path}",
            )
        )

    return tasks


def _parse_task_line(text: str) -> tuple[str, str, int]:
    """Parse a task line to extract ID, title, and priority.

    Returns (id, title, priority)
    """
    priority = 3
    title = text

    # Check for priority tag: [HIGH], [LOW], [P0], etc.
    priority_match = re.match(r"^\[([A-Z0-9]+)\]\s*(.+)$", text)
    if priority_match:
        tag = priority_match.group(1).upper()
        title = priority_match.group(2).strip()

        if tag in ("HIGH", "CRITICAL", "URGENT", "P0", "P1"):
            priority = 1
        elif tag in ("LOW", "MINOR", "P3", "P4"):
            priority = 4
        else:
            priority = 3

    # Check for explicit ID: "task-id: description"
    id_match = re.match(r"^([a-z0-9_-]+):\s*(.+)$", title, re.IGNORECASE)
    if id_match:
        task_id = id_match.group(1).lower()
        title = id_match.group(2).strip()
    else:
        # Generate ID from title
        task_id = _generate_id(title)

    return task_id, title, priority


def _generate_id(text: str) -> str:
    """Generate an ID from text."""
    clean = text[:30].lower()
    clean = "".join(c if c.isalnum() or c == " " else "" for c in clean)
    return clean.replace(" ", "-").strip("-") or "task"
