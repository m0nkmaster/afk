"""JSON PRD task source adapter."""

from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from afk.sources import Task


def load_json_tasks(path: str | None) -> list[Task]:
    """Load tasks from a JSON PRD file.

    Supports two formats:

    1. Anthropic style:
    {
        "tasks": [
            {"id": "...", "description": "...", "passes": false}
        ]
    }

    2. Simple array:
    [
        {"id": "...", "description": "..."}
    ]
    """
    from afk.sources import Task

    if not path:
        # Try default locations
        for default_path in ["prd.json", "tasks.json", ".afk/prd.json"]:
            if Path(default_path).exists():
                path = default_path
                break
        else:
            return []

    file_path = Path(path)
    if not file_path.exists():
        return []

    with open(file_path) as f:
        data = json.load(f)

    # Handle both formats
    if isinstance(data, list):
        items = data
    elif isinstance(data, dict):
        items = data.get("tasks", data.get("items", []))
    else:
        return []

    tasks = []
    for item in items:
        # Skip completed tasks (Anthropic style)
        if item.get("passes", False):
            continue

        task_id = item.get("id") or _generate_id(item.get("description", ""))
        description = item.get("description") or item.get("title") or item.get("summary", "")
        priority = _map_priority(item.get("priority"))

        if task_id and description:
            tasks.append(
                Task(
                    id=task_id,
                    description=description,
                    priority=priority,
                    source=f"json:{path}",
                    metadata=item,
                )
            )

    return tasks


def _generate_id(description: str) -> str:
    """Generate an ID from description."""
    # Take first 30 chars, lowercase, replace spaces with dashes
    clean = description[:30].lower()
    clean = "".join(c if c.isalnum() or c == " " else "" for c in clean)
    return clean.replace(" ", "-").strip("-") or "task"


def _map_priority(priority: str | int | None) -> str:
    """Map various priority formats to afk priority."""
    if priority is None:
        return "medium"

    if isinstance(priority, int):
        if priority <= 1:
            return "high"
        elif priority <= 2:
            return "medium"
        else:
            return "low"

    priority_lower = str(priority).lower()
    if priority_lower in ("high", "critical", "urgent", "1"):
        return "high"
    elif priority_lower in ("low", "minor", "3", "4", "5"):
        return "low"
    else:
        return "medium"
