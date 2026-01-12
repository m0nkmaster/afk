"""Beads (bd) task source adapter."""

from __future__ import annotations

import json
import re
import subprocess

from afk.prd_store import UserStory


def load_beads_tasks() -> list[UserStory]:
    """Load tasks from beads (bd ready)."""
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
            title = item.get("title") or item.get("summary", "")
            description = item.get("description") or item.get("body") or title
            priority = _map_beads_priority(item.get("priority"))

            # Extract acceptance criteria from description if present
            acceptance_criteria = _extract_acceptance_criteria(description)
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
                        source="beads",
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


def _parse_beads_text_output() -> list[UserStory]:
    """Parse text output from bd ready (fallback)."""
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
                title = parts[1].strip() if len(parts) > 1 else task_id
            else:
                # Generate ID from description
                task_id = line[:20].replace(" ", "-").lower()
                title = line

            tasks.append(
                UserStory(
                    id=task_id,
                    title=title,
                    description=title,
                    acceptance_criteria=[f"Complete: {title}"],
                    priority=3,
                    source="beads",
                )
            )

        return tasks

    except (FileNotFoundError, subprocess.TimeoutExpired):
        return []


def _map_beads_priority(priority: str | int | None) -> int:
    """Map beads priority to int (1-5)."""
    if priority is None:
        return 3

    if isinstance(priority, int):
        # Clamp to 1-5
        return max(1, min(5, priority))

    priority_lower = str(priority).lower()
    if priority_lower in ("high", "critical", "urgent", "p0", "p1"):
        return 1
    elif priority_lower in ("low", "minor", "p3", "p4"):
        return 4
    else:
        return 3


def _extract_acceptance_criteria(text: str) -> list[str]:
    """Extract acceptance criteria from text."""
    if not text:
        return []

    criteria: list[str] = []

    # Look for acceptance criteria section
    ac_patterns = [
        r"(?:acceptance\s*criteria|ac|definition\s*of\s*done|dod|requirements?)[\s:]*\n"
        r"((?:[-*\d.]+\s*.+\n?)+)",
        r"##\s*(?:acceptance\s*criteria|ac|dod)\s*\n((?:[-*\d.]+\s*.+\n?)+)",
    ]

    for pattern in ac_patterns:
        match = re.search(pattern, text, re.IGNORECASE | re.MULTILINE)
        if match:
            section = match.group(1)
            for line in section.split("\n"):
                line = line.strip()
                cleaned = re.sub(r"^[-*\d.]+\s*(\[[ x]\])?\s*", "", line)
                if cleaned:
                    criteria.append(cleaned)
            break

    # If no section found, look for checkbox items
    if not criteria:
        checkbox_pattern = r"[-*]\s*\[[ ]\]\s*(.+)"
        matches = re.findall(checkbox_pattern, text)
        criteria = [m.strip() for m in matches if m.strip()]

    return criteria


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
