"""Learnings file management for afk.

The learnings file is an append-only record of discoveries made during
autonomous coding sessions. Unlike progress.json (structured, session-scoped),
learnings.txt persists across sessions and provides human-readable context.
"""

from __future__ import annotations

from datetime import datetime
from pathlib import Path

from afk.config import LEARNINGS_FILE


def load_learnings(path: Path | None = None) -> str:
    """Load learnings from file.

    Args:
        path: Optional path override (defaults to .afk/learnings.txt)

    Returns:
        The learnings content, or empty string if file doesn't exist
    """
    if path is None:
        path = LEARNINGS_FILE

    if not path.exists():
        return ""

    return path.read_text()


def append_learning(
    content: str,
    task_id: str | None = None,
    path: Path | None = None,
) -> None:
    """Append a learning entry to the learnings file.

    Args:
        content: The learning to record
        task_id: Optional task ID this learning relates to
        path: Optional path override
    """
    if path is None:
        path = LEARNINGS_FILE

    # Ensure directory exists
    path.parent.mkdir(parents=True, exist_ok=True)

    # Format the entry
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M")
    task_prefix = f"[{task_id}] " if task_id else ""
    entry = f"\n## {timestamp} {task_prefix}\n\n{content.strip()}\n"

    # Append to file
    with open(path, "a") as f:
        f.write(entry)


def clear_learnings(path: Path | None = None) -> None:
    """Clear the learnings file (typically only on explicit user request).

    Args:
        path: Optional path override
    """
    if path is None:
        path = LEARNINGS_FILE

    if path.exists():
        path.unlink()


def get_recent_learnings(max_chars: int = 2000, path: Path | None = None) -> str:
    """Get the most recent learnings, truncated to max_chars.

    Prioritises recent entries by reading from the end.

    Args:
        max_chars: Maximum characters to return
        path: Optional path override

    Returns:
        Recent learnings content, possibly truncated
    """
    content = load_learnings(path)

    if len(content) <= max_chars:
        return content

    # Truncate from the start, keeping recent entries
    truncated = content[-max_chars:]

    # Find the first complete entry (starts with ##)
    first_entry = truncated.find("\n## ")
    if first_entry > 0:
        truncated = truncated[first_entry + 1 :]

    return f"[...truncated...]\n{truncated}"
