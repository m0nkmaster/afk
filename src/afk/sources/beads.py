"""Beads (bd) task source adapter."""

from __future__ import annotations

import json
import subprocess
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from afk.sources import Task


def load_beads_tasks() -> list[Task]:
    """Load tasks from beads (bd ready)."""
    from afk.sources import Task

    try:
        # Run bd ready --json to get tasks
        result = subprocess.run(
            ["bd", "ready", "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            # Try without --json flag (older versions)
            return _parse_beads_text_output()

        data = json.loads(result.stdout)
        tasks = []

        for item in data:
            # beads format varies, try common fields
            task_id = item.get("id") or item.get("key") or str(item.get("number", ""))
            description = item.get("title") or item.get("description") or item.get("summary", "")
            priority = _map_beads_priority(item.get("priority"))

            if task_id and description:
                tasks.append(
                    Task(
                        id=task_id,
                        description=description,
                        priority=priority,
                        source="beads",
                        metadata=item,
                    )
                )

        return tasks

    except FileNotFoundError:
        # bd not installed
        return []
    except subprocess.TimeoutExpired:
        return []
    except json.JSONDecodeError:
        return _parse_beads_text_output()


def _parse_beads_text_output() -> list[Task]:
    """Parse text output from bd ready (fallback)."""
    from afk.sources import Task

    try:
        result = subprocess.run(
            ["bd", "ready"],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            return []

        tasks = []
        for line in result.stdout.strip().split("\n"):
            line = line.strip()
            if not line:
                continue

            # Try to parse "ID: description" or just use the whole line
            if ":" in line:
                parts = line.split(":", 1)
                task_id = parts[0].strip()
                description = parts[1].strip() if len(parts) > 1 else task_id
            else:
                # Generate ID from description
                task_id = line[:20].replace(" ", "-").lower()
                description = line

            tasks.append(
                Task(
                    id=task_id,
                    description=description,
                    priority="medium",
                    source="beads",
                )
            )

        return tasks

    except (FileNotFoundError, subprocess.TimeoutExpired):
        return []


def _map_beads_priority(priority: str | int | None) -> str:
    """Map beads priority to afk priority."""
    if priority is None:
        return "medium"

    if isinstance(priority, int):
        if priority <= 1:
            return "high"
        elif priority <= 3:
            return "medium"
        else:
            return "low"

    priority_lower = str(priority).lower()
    if priority_lower in ("high", "critical", "urgent", "p0", "p1"):
        return "high"
    elif priority_lower in ("low", "minor", "p3", "p4"):
        return "low"
    else:
        return "medium"


def close_beads_issue(issue_id: str) -> bool:
    """Close a beads issue by ID.

    Args:
        issue_id: The beads issue ID to close

    Returns:
        True if successfully closed, False otherwise
    """
    try:
        result = subprocess.run(
            ["bd", "close", issue_id],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False
