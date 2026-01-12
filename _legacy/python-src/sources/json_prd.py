"""JSON PRD task source adapter."""

from __future__ import annotations

import json
from pathlib import Path

from afk.prd_store import UserStory


def load_json_tasks(path: str | None) -> list[UserStory]:
    """Load tasks from a JSON PRD file.

    Supports formats:

    1. Full afk style:
    {
        "tasks": [
            {
                "id": "feature-id",
                "title": "Feature title",
                "description": "Feature description",
                "priority": 1,
                "acceptanceCriteria": ["Step 1", "Step 2"],
                "passes": false
            }
        ]
    }

    2. Simple array:
    [
        {"id": "...", "title": "..."}
    ]
    """
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
    items: list[dict] = []
    if isinstance(data, list):
        items = data
    elif isinstance(data, dict):
        items = data.get("tasks") or data.get("userStories") or data.get("items") or []
    else:
        return []

    tasks = []
    for item in items:
        # Skip completed tasks
        if item.get("passes", False):
            continue

        task_id = item.get("id") or _generate_id(item.get("title", item.get("description", "")))
        title = item.get("title") or item.get("summary") or item.get("description", "")
        description = item.get("description") or title
        priority = _map_priority(item.get("priority"))

        # Get acceptance criteria (support both camelCase and snake_case)
        acceptance_criteria = (
            item.get("acceptanceCriteria")
            or item.get("acceptance_criteria")
            or item.get("steps")
            or []
        )
        if isinstance(acceptance_criteria, str):
            acceptance_criteria = [acceptance_criteria]
        if not acceptance_criteria:
            acceptance_criteria = [f"Complete: {title}"]

        if task_id:
            tasks.append(
                UserStory(
                    id=task_id,
                    title=title,
                    description=description,
                    acceptance_criteria=acceptance_criteria,
                    priority=priority,
                    source=f"json:{path}",
                    notes=item.get("notes", ""),
                )
            )

    return tasks


def _generate_id(text: str) -> str:
    """Generate an ID from text."""
    clean = text[:30].lower()
    clean = "".join(c if c.isalnum() or c == " " else "" for c in clean)
    return clean.replace(" ", "-").strip("-") or "task"


def _map_priority(priority: str | int | None) -> int:
    """Map various priority formats to int (1-5)."""
    if priority is None:
        return 3

    if isinstance(priority, int):
        return max(1, min(5, priority))

    priority_lower = str(priority).lower()
    if priority_lower in ("high", "critical", "urgent", "1", "p0", "p1"):
        return 1
    elif priority_lower in ("medium", "normal", "2", "p2"):
        return 2
    elif priority_lower in ("low", "minor", "3", "4", "5", "p3", "p4"):
        return 4
    else:
        return 3
